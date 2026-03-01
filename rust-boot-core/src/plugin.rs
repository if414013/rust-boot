//! Plugin system with Bevy-style lifecycle management.
//!
//! This module provides the core plugin abstraction for the rust-boot framework,
//! implementing a state machine-based lifecycle similar to Bevy's plugin system.
//!
//! # Plugin Lifecycle
//!
//! Plugins go through four states in order:
//! 1. **Adding** - Plugin is being registered, `build()` is called
//! 2. **Ready** - Plugin is initialized, `ready()` is called
//! 3. **Finished** - Plugin has completed its work, `finish()` is called
//! 4. **Cleaned** - Plugin resources are released, `cleanup()` is called
//!
//! # Example
//!
//! ```ignore
//! use rust_boot_core::plugin::{CrudPlugin, PluginContext, PluginMeta};
//! use rust_boot_core::error::Result;
//! use async_trait::async_trait;
//!
//! struct MyPlugin;
//!
//! #[async_trait]
//! impl CrudPlugin for MyPlugin {
//!     fn meta(&self) -> PluginMeta {
//!         PluginMeta::new("my-plugin", "1.0.0")
//!     }
//!
//!     async fn build(&mut self, ctx: &mut PluginContext) -> Result<()> {
//!         // Initialize plugin resources
//!         Ok(())
//!     }
//! }
//! ```

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::Result;

/// Represents the current state of a plugin in its lifecycle.
///
/// Plugins transition through these states in order during application
/// startup and shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginState {
    /// Plugin is being added to the registry, `build()` will be called.
    Adding,
    /// Plugin has been built and is ready, `ready()` will be called.
    Ready,
    /// Plugin has finished its main work, `finish()` will be called.
    Finished,
    /// Plugin has been cleaned up, `cleanup()` has been called.
    Cleaned,
}

impl fmt::Display for PluginState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Adding => write!(f, "Adding"),
            Self::Ready => write!(f, "Ready"),
            Self::Finished => write!(f, "Finished"),
            Self::Cleaned => write!(f, "Cleaned"),
        }
    }
}

impl PluginState {
    /// Returns the next state in the lifecycle, if any.
    pub const fn next(&self) -> Option<Self> {
        match self {
            Self::Adding => Some(Self::Ready),
            Self::Ready => Some(Self::Finished),
            Self::Finished => Some(Self::Cleaned),
            Self::Cleaned => None,
        }
    }

    /// Returns true if this state can transition to the given state.
    pub fn can_transition_to(&self, target: Self) -> bool {
        self.next() == Some(target)
    }
}

/// Metadata about a plugin including its name, version, and dependencies.
///
/// This information is used by the plugin registry to manage plugin
/// initialization order and dependency resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginMeta {
    /// Unique name identifying this plugin.
    pub name: String,
    /// Semantic version of the plugin.
    pub version: String,
    /// Names of plugins that must be initialized before this one.
    pub dependencies: Vec<String>,
}

impl PluginMeta {
    /// Creates a new `PluginMeta` with the given name and version.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for the plugin
    /// * `version` - Semantic version string (e.g., "1.0.0")
    ///
    /// # Example
    ///
    /// ```
    /// use rust_boot_core::plugin::PluginMeta;
    ///
    /// let meta = PluginMeta::new("my-plugin", "1.0.0");
    /// assert_eq!(meta.name, "my-plugin");
    /// assert_eq!(meta.version, "1.0.0");
    /// assert!(meta.dependencies.is_empty());
    /// ```
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            dependencies: Vec::new(),
        }
    }

    /// Creates a new `PluginMeta` with dependencies.
    ///
    /// # Arguments
    ///
    /// * `name` - Unique identifier for the plugin
    /// * `version` - Semantic version string
    /// * `dependencies` - Names of plugins this plugin depends on
    ///
    /// # Example
    ///
    /// ```
    /// use rust_boot_core::plugin::PluginMeta;
    ///
    /// let meta = PluginMeta::with_dependencies(
    ///     "auth-plugin",
    ///     "1.0.0",
    ///     vec!["database-plugin".to_string()]
    /// );
    /// assert_eq!(meta.dependencies.len(), 1);
    /// ```
    pub fn with_dependencies(
        name: impl Into<String>,
        version: impl Into<String>,
        dependencies: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            dependencies,
        }
    }

    /// Adds a dependency to this plugin.
    ///
    /// # Example
    ///
    /// ```
    /// use rust_boot_core::plugin::PluginMeta;
    ///
    /// let meta = PluginMeta::new("my-plugin", "1.0.0")
    ///     .with_dependency("other-plugin");
    /// assert_eq!(meta.dependencies, vec!["other-plugin"]);
    /// ```
    pub fn with_dependency(mut self, dependency: impl Into<String>) -> Self {
        self.dependencies.push(dependency.into());
        self
    }
}

/// Thread-safe container for shared state between plugins.
///
/// `PluginContext` allows plugins to share data with each other during
/// the application lifecycle. It uses interior mutability with `Arc<RwLock<...>>`
/// to allow safe concurrent access.
///
/// # Example
///
/// ```ignore
/// use rust_boot_core::plugin::PluginContext;
///
/// let mut ctx = PluginContext::new();
///
/// // Insert a value
/// ctx.insert("config", "database_url".to_string()).await;
///
/// // Retrieve the value
/// let url: Option<String> = ctx.get("config").await;
/// assert_eq!(url, Some("database_url".to_string()));
/// ```
#[derive(Debug, Clone)]
pub struct PluginContext {
    state: Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>,
}

impl Default for PluginContext {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginContext {
    /// Creates a new empty `PluginContext`.
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Inserts a value into the context with the given key.
    ///
    /// If a value already exists for the key, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to store the value under
    /// * `value` - The value to store (must be `Send + Sync + 'static`)
    pub async fn insert<T: Send + Sync + 'static>(&self, key: impl Into<String>, value: T) {
        let mut state = self.state.write().await;
        state.insert(key.into(), Box::new(value));
    }

    /// Retrieves a cloned value from the context.
    ///
    /// Returns `None` if the key doesn't exist or the type doesn't match.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The expected type of the value (must implement `Clone`)
    pub async fn get<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Option<T> {
        let state = self.state.read().await;
        state.get(key).and_then(|v| v.downcast_ref::<T>().cloned())
    }

    /// Removes a value from the context and returns it.
    ///
    /// Returns `None` if the key doesn't exist or the type doesn't match.
    pub async fn remove<T: Send + Sync + 'static>(&self, key: &str) -> Option<T> {
        let mut state = self.state.write().await;
        state
            .remove(key)
            .and_then(|v| v.downcast::<T>().ok().map(|b| *b))
    }

    /// Checks if a key exists in the context.
    pub async fn contains(&self, key: &str) -> bool {
        let state = self.state.read().await;
        state.contains_key(key)
    }

    /// Returns the number of entries in the context.
    pub async fn len(&self) -> usize {
        let state = self.state.read().await;
        state.len()
    }

    /// Returns true if the context is empty.
    pub async fn is_empty(&self) -> bool {
        let state = self.state.read().await;
        state.is_empty()
    }

    /// Clears all entries from the context.
    pub async fn clear(&self) {
        let mut state = self.state.write().await;
        state.clear();
    }
}

/// The core plugin trait for rust-boot framework.
///
/// Implement this trait to create plugins that can be registered with the
/// application. Plugins follow a Bevy-style lifecycle with four phases:
/// `build`, `ready`, `finish`, and `cleanup`.
///
/// All lifecycle methods have default implementations that do nothing,
/// so you only need to implement the methods relevant to your plugin.
///
/// # Example
///
/// ```
/// use rust_boot_core::plugin::{CrudPlugin, PluginContext, PluginMeta};
/// use rust_boot_core::error::Result;
/// use async_trait::async_trait;
///
/// struct LoggingPlugin {
///     log_level: String,
/// }
///
/// #[async_trait]
/// impl CrudPlugin for LoggingPlugin {
///     fn meta(&self) -> PluginMeta {
///         PluginMeta::new("logging", "1.0.0")
///     }
///
///     async fn build(&mut self, ctx: &mut PluginContext) -> Result<()> {
///         // Initialize logging system
///         ctx.insert("log_level", self.log_level.clone()).await;
///         Ok(())
///     }
///
///     async fn cleanup(&mut self, ctx: &mut PluginContext) -> Result<()> {
///         // Flush any pending logs
///         ctx.remove::<String>("log_level").await;
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait CrudPlugin: Send + Sync {
    /// Returns metadata about this plugin.
    ///
    /// This method must be implemented to provide the plugin's name,
    /// version, and any dependencies on other plugins.
    fn meta(&self) -> PluginMeta;

    /// Called when the plugin is being added to the application.
    ///
    /// Use this method to initialize resources, register services,
    /// or set up configuration. This is called during the `Adding` state.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Mutable reference to the shared plugin context
    ///
    /// # Default Implementation
    ///
    /// Does nothing and returns `Ok(())`.
    async fn build(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called after all plugins have been built.
    ///
    /// Use this method to perform initialization that depends on
    /// other plugins being available. This is called during the `Ready` state.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Mutable reference to the shared plugin context
    ///
    /// # Default Implementation
    ///
    /// Does nothing and returns `Ok(())`.
    async fn ready(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called when the application is shutting down.
    ///
    /// Use this method to complete any pending work before cleanup.
    /// This is called during the `Finished` state.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Mutable reference to the shared plugin context
    ///
    /// # Default Implementation
    ///
    /// Does nothing and returns `Ok(())`.
    async fn finish(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called to release plugin resources.
    ///
    /// Use this method to clean up any resources allocated by the plugin.
    /// This is called during the `Cleaned` state and is the last lifecycle
    /// method called.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Mutable reference to the shared plugin context
    ///
    /// # Default Implementation
    ///
    /// Does nothing and returns `Ok(())`.
    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_state_display() {
        assert_eq!(PluginState::Adding.to_string(), "Adding");
        assert_eq!(PluginState::Ready.to_string(), "Ready");
        assert_eq!(PluginState::Finished.to_string(), "Finished");
        assert_eq!(PluginState::Cleaned.to_string(), "Cleaned");
    }

    #[test]
    fn test_plugin_state_next() {
        assert_eq!(PluginState::Adding.next(), Some(PluginState::Ready));
        assert_eq!(PluginState::Ready.next(), Some(PluginState::Finished));
        assert_eq!(PluginState::Finished.next(), Some(PluginState::Cleaned));
        assert_eq!(PluginState::Cleaned.next(), None);
    }

    #[test]
    fn test_plugin_state_can_transition_to() {
        assert!(PluginState::Adding.can_transition_to(PluginState::Ready));
        assert!(!PluginState::Adding.can_transition_to(PluginState::Finished));
        assert!(!PluginState::Adding.can_transition_to(PluginState::Cleaned));

        assert!(PluginState::Ready.can_transition_to(PluginState::Finished));
        assert!(!PluginState::Ready.can_transition_to(PluginState::Adding));

        assert!(PluginState::Finished.can_transition_to(PluginState::Cleaned));
        assert!(!PluginState::Cleaned.can_transition_to(PluginState::Adding));
    }

    #[test]
    fn test_plugin_meta_new() {
        let meta = PluginMeta::new("test-plugin", "1.0.0");
        assert_eq!(meta.name, "test-plugin");
        assert_eq!(meta.version, "1.0.0");
        assert!(meta.dependencies.is_empty());
    }

    #[test]
    fn test_plugin_meta_with_dependencies() {
        let meta = PluginMeta::with_dependencies(
            "auth-plugin",
            "2.0.0",
            vec!["db-plugin".to_string(), "cache-plugin".to_string()],
        );
        assert_eq!(meta.name, "auth-plugin");
        assert_eq!(meta.version, "2.0.0");
        assert_eq!(meta.dependencies.len(), 2);
        assert!(meta.dependencies.contains(&"db-plugin".to_string()));
        assert!(meta.dependencies.contains(&"cache-plugin".to_string()));
    }

    #[test]
    fn test_plugin_meta_with_dependency_builder() {
        let meta = PluginMeta::new("my-plugin", "1.0.0")
            .with_dependency("dep1")
            .with_dependency("dep2");
        assert_eq!(meta.dependencies, vec!["dep1", "dep2"]);
    }

    #[tokio::test]
    async fn test_plugin_context_insert_and_get() {
        let ctx = PluginContext::new();

        ctx.insert("key1", "value1".to_string()).await;
        ctx.insert("key2", 42i32).await;

        let val1: Option<String> = ctx.get("key1").await;
        assert_eq!(val1, Some("value1".to_string()));

        let val2: Option<i32> = ctx.get("key2").await;
        assert_eq!(val2, Some(42));
    }

    #[tokio::test]
    async fn test_plugin_context_get_wrong_type() {
        let ctx = PluginContext::new();
        ctx.insert("key", "string_value".to_string()).await;

        // Try to get as wrong type
        let val: Option<i32> = ctx.get("key").await;
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_plugin_context_get_nonexistent() {
        let ctx = PluginContext::new();
        let val: Option<String> = ctx.get("nonexistent").await;
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_plugin_context_remove() {
        let ctx = PluginContext::new();
        ctx.insert("key", "value".to_string()).await;

        let removed: Option<String> = ctx.remove("key").await;
        assert_eq!(removed, Some("value".to_string()));

        // Should be gone now
        let val: Option<String> = ctx.get("key").await;
        assert_eq!(val, None);
    }

    #[tokio::test]
    async fn test_plugin_context_contains() {
        let ctx = PluginContext::new();
        assert!(!ctx.contains("key").await);

        ctx.insert("key", "value".to_string()).await;
        assert!(ctx.contains("key").await);
    }

    #[tokio::test]
    async fn test_plugin_context_len_and_is_empty() {
        let ctx = PluginContext::new();
        assert!(ctx.is_empty().await);
        assert_eq!(ctx.len().await, 0);

        ctx.insert("key1", 1).await;
        assert!(!ctx.is_empty().await);
        assert_eq!(ctx.len().await, 1);

        ctx.insert("key2", 2).await;
        assert_eq!(ctx.len().await, 2);
    }

    #[tokio::test]
    async fn test_plugin_context_clear() {
        let ctx = PluginContext::new();
        ctx.insert("key1", 1).await;
        ctx.insert("key2", 2).await;
        assert_eq!(ctx.len().await, 2);

        ctx.clear().await;
        assert!(ctx.is_empty().await);
    }

    #[tokio::test]
    async fn test_plugin_context_clone_shares_state() {
        let ctx1 = PluginContext::new();
        let ctx2 = ctx1.clone();

        ctx1.insert("key", "value".to_string()).await;

        let val: Option<String> = ctx2.get("key").await;
        assert_eq!(val, Some("value".to_string()));
    }

    // Mock plugin for testing
    struct MockPlugin {
        name: String,
        build_called: bool,
        ready_called: bool,
        finish_called: bool,
        cleanup_called: bool,
    }

    impl MockPlugin {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                build_called: false,
                ready_called: false,
                finish_called: false,
                cleanup_called: false,
            }
        }
    }

    #[async_trait]
    impl CrudPlugin for MockPlugin {
        fn meta(&self) -> PluginMeta {
            PluginMeta::new(&self.name, "1.0.0")
        }

        async fn build(&mut self, ctx: &mut PluginContext) -> Result<()> {
            self.build_called = true;
            ctx.insert(format!("{}_built", self.name), true).await;
            Ok(())
        }

        async fn ready(&mut self, _ctx: &mut PluginContext) -> Result<()> {
            self.ready_called = true;
            Ok(())
        }

        async fn finish(&mut self, _ctx: &mut PluginContext) -> Result<()> {
            self.finish_called = true;
            Ok(())
        }

        async fn cleanup(&mut self, ctx: &mut PluginContext) -> Result<()> {
            self.cleanup_called = true;
            ctx.remove::<bool>(&format!("{}_built", self.name)).await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_plugin_lifecycle() {
        let mut plugin = MockPlugin::new("test");
        let mut ctx = PluginContext::new();

        assert!(!plugin.build_called);
        assert!(!plugin.ready_called);
        assert!(!plugin.finish_called);
        assert!(!plugin.cleanup_called);

        plugin.build(&mut ctx).await.unwrap();
        assert!(plugin.build_called);
        assert!(ctx.contains("test_built").await);

        plugin.ready(&mut ctx).await.unwrap();
        assert!(plugin.ready_called);

        plugin.finish(&mut ctx).await.unwrap();
        assert!(plugin.finish_called);

        plugin.cleanup(&mut ctx).await.unwrap();
        assert!(plugin.cleanup_called);
        assert!(!ctx.contains("test_built").await);
    }

    #[tokio::test]
    async fn test_plugin_default_implementations() {
        struct MinimalPlugin;

        #[async_trait]
        impl CrudPlugin for MinimalPlugin {
            fn meta(&self) -> PluginMeta {
                PluginMeta::new("minimal", "1.0.0")
            }
        }

        let mut plugin = MinimalPlugin;
        let mut ctx = PluginContext::new();

        // All default implementations should succeed
        assert!(plugin.build(&mut ctx).await.is_ok());
        assert!(plugin.ready(&mut ctx).await.is_ok());
        assert!(plugin.finish(&mut ctx).await.is_ok());
        assert!(plugin.cleanup(&mut ctx).await.is_ok());
    }

    #[test]
    fn test_plugin_meta_equality() {
        let meta1 = PluginMeta::new("plugin", "1.0.0");
        let meta2 = PluginMeta::new("plugin", "1.0.0");
        let meta3 = PluginMeta::new("plugin", "2.0.0");

        assert_eq!(meta1, meta2);
        assert_ne!(meta1, meta3);
    }

    #[test]
    fn test_plugin_state_equality() {
        assert_eq!(PluginState::Adding, PluginState::Adding);
        assert_ne!(PluginState::Adding, PluginState::Ready);
    }
}
