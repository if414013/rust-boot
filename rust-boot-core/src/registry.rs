//! Plugin registry with dependency resolution and lifecycle orchestration.
//!
//! The `PluginRegistry` is the central component for managing plugins in a rust-boot
//! application. It handles plugin registration, dependency resolution (topological sorting),
//! and coordinating plugin lifecycle transitions.
//!
//! # Features
//!
//! - **Dependency Resolution**: Automatically orders plugin initialization based on declared dependencies
//! - **Circular Dependency Detection**: Prevents registration of plugins that would create cycles
//! - **Lifecycle Management**: Coordinates `build`, `ready`, `finish`, and `cleanup` phases
//! - **Shared Context**: Provides a thread-safe context for plugins to share state
//!
//! # Example
//!
//! ```ignore
//! use rust_boot_core::registry::PluginRegistry;
//! use rust_boot_core::plugin::{CrudPlugin, PluginMeta, PluginContext};
//!
//! let mut registry = PluginRegistry::new();
//!
//! // Register plugins (order matters for dependencies)
//! registry.register(DatabasePlugin::new())?;
//! registry.register(CachePlugin::new())?;  // May depend on DatabasePlugin
//!
//! // Initialize all plugins in dependency order
//! registry.init_all().await?;
//!
//! // Later, shut down gracefully in reverse order
//! registry.finish_all().await?;
//! registry.cleanup_all().await?;
//! ```

use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::{Result, RustBootError};
use crate::plugin::{CrudPlugin, PluginContext, PluginState};

/// Internal representation of a registered plugin with its current state.
struct PluginEntry {
    plugin: Box<dyn CrudPlugin + Send + Sync>,
    state: PluginState,
}

/// Central registry for managing plugins and their lifecycle.
///
/// `PluginRegistry` maintains a collection of plugins, tracks their states,
/// and provides methods to coordinate their initialization and shutdown.
/// It uses topological sorting to ensure plugins are initialized in the
/// correct order based on their declared dependencies.
///
/// # Thread Safety
///
/// The registry itself is not thread-safe. For concurrent access, wrap it
/// in appropriate synchronization primitives like `Arc<RwLock<PluginRegistry>>`.
pub struct PluginRegistry {
    plugins: HashMap<String, PluginEntry>,
    context: PluginContext,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Creates a new empty `PluginRegistry`.
    ///
    /// The registry starts with no plugins and a fresh `PluginContext`.
    ///
    /// # Example
    ///
    /// ```
    /// use rust_boot_core::registry::PluginRegistry;
    ///
    /// let registry = PluginRegistry::new();
    /// assert!(registry.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            context: PluginContext::new(),
        }
    }

    /// Creates a new `PluginRegistry` with an existing `PluginContext`.
    ///
    /// This is useful when you need to pre-populate the context with
    /// shared state before registering plugins.
    ///
    /// # Arguments
    ///
    /// * `context` - An existing `PluginContext` to use for plugin communication
    pub fn with_context(context: PluginContext) -> Self {
        Self {
            plugins: HashMap::new(),
            context,
        }
    }

    /// Registers a plugin with the registry.
    ///
    /// The plugin will be validated for:
    /// - Unique name (no duplicate registrations)
    /// - Satisfied dependencies (all dependencies must be registered first)
    ///
    /// # Arguments
    ///
    /// * `plugin` - The plugin to register (must implement `CrudPlugin`)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A plugin with the same name is already registered
    /// - Any of the plugin's dependencies are not yet registered
    ///
    /// # Example
    ///
    /// ```ignore
    /// registry.register(MyPlugin::new())?;
    /// ```
    pub fn register<P: CrudPlugin + Send + Sync + 'static>(&mut self, plugin: P) -> Result<()> {
        let meta = plugin.meta();
        let name = meta.name.clone();

        if self.plugins.contains_key(&name) {
            return Err(RustBootError::Plugin(format!(
                "Plugin '{name}' is already registered"
            )));
        }

        for dep in &meta.dependencies {
            if !self.plugins.contains_key(dep) {
                return Err(RustBootError::Plugin(format!(
                    "Plugin '{name}' depends on '{dep}' which is not registered. Register dependencies first."
                )));
            }
        }

        self.plugins.insert(
            name,
            PluginEntry {
                plugin: Box::new(plugin),
                state: PluginState::Adding,
            },
        );

        Ok(())
    }

    /// Returns a reference to a registered plugin by name.
    pub fn get(&self, name: &str) -> Option<&(dyn CrudPlugin + Send + Sync)> {
        self.plugins.get(name).map(|e| e.plugin.as_ref())
    }

    /// Returns the current state of a plugin by name.
    pub fn get_state(&self, name: &str) -> Option<PluginState> {
        self.plugins.get(name).map(|e| e.state)
    }

    /// Returns a list of all registered plugin names.
    pub fn plugin_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Returns the number of registered plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Returns `true` if no plugins are registered.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Returns a reference to the shared plugin context.
    pub const fn context(&self) -> &PluginContext {
        &self.context
    }

    /// Returns a mutable reference to the shared plugin context.
    pub fn context_mut(&mut self) -> &mut PluginContext {
        &mut self.context
    }

    /// Computes the topological order of plugins based on dependencies.
    fn topological_order(&self) -> Result<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

        for (name, entry) in &self.plugins {
            in_degree.entry(name.clone()).or_insert(0);
            dependents.entry(name.clone()).or_default();

            for dep in &entry.plugin.meta().dependencies {
                *in_degree.entry(name.clone()).or_insert(0) += 1;
                dependents
                    .entry(dep.clone())
                    .or_default()
                    .push(name.clone());
            }
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut result = Vec::new();

        while let Some(name) = queue.pop_front() {
            result.push(name.clone());

            if let Some(deps) = dependents.get(&name) {
                for dependent in deps {
                    if let Some(deg) = in_degree.get_mut(dependent) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        if result.len() != self.plugins.len() {
            let remaining: Vec<_> = self
                .plugins
                .keys()
                .filter(|k| !result.contains(k))
                .cloned()
                .collect();
            return Err(RustBootError::Plugin(format!(
                "Circular dependency detected involving plugins: {remaining:?}"
            )));
        }

        Ok(result)
    }

    /// Initializes all plugins by calling their `build` methods in dependency order.
    ///
    /// After successful initialization, each plugin's state transitions to `Ready`.
    pub async fn init_all(&mut self) -> Result<()> {
        let order = self.topological_order()?;

        for name in order {
            if let Some(entry) = self.plugins.get_mut(&name) {
                entry.plugin.build(&mut self.context).await?;
                entry.state = PluginState::Ready;
            }
        }

        Ok(())
    }

    /// Calls the `ready` method on all plugins in dependency order.
    pub async fn ready_all(&mut self) -> Result<()> {
        let order = self.topological_order()?;

        for name in order {
            if let Some(entry) = self.plugins.get_mut(&name) {
                entry.plugin.ready(&mut self.context).await?;
            }
        }

        Ok(())
    }

    /// Finishes all plugins by calling their `finish` methods in reverse dependency order.
    pub async fn finish_all(&mut self) -> Result<()> {
        let mut order = self.topological_order()?;
        order.reverse();

        for name in order {
            if let Some(entry) = self.plugins.get_mut(&name) {
                entry.plugin.finish(&mut self.context).await?;
                entry.state = PluginState::Finished;
            }
        }

        Ok(())
    }

    /// Cleans up all plugins by calling their `cleanup` methods in reverse dependency order.
    pub async fn cleanup_all(&mut self) -> Result<()> {
        let mut order = self.topological_order()?;
        order.reverse();

        for name in order {
            if let Some(entry) = self.plugins.get_mut(&name) {
                entry.plugin.cleanup(&mut self.context).await?;
                entry.state = PluginState::Cleaned;
            }
        }

        Ok(())
    }

    /// Returns `true` if there is a circular dependency among registered plugins.
    pub fn has_circular_dependency(&self) -> bool {
        self.detect_cycle().is_some()
    }

    fn detect_cycle(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for name in self.plugins.keys() {
            if !visited.contains(name) {
                if let Some(cycle) =
                    self.detect_cycle_util(name, &mut visited, &mut rec_stack, &mut path)
                {
                    return Some(cycle);
                }
            }
        }

        None
    }

    fn detect_cycle_util(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(name.to_string());
        rec_stack.insert(name.to_string());
        path.push(name.to_string());

        if let Some(entry) = self.plugins.get(name) {
            for dep in &entry.plugin.meta().dependencies {
                if !visited.contains(dep) {
                    if let Some(cycle) = self.detect_cycle_util(dep, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    let start_idx = path.iter().position(|x| x == dep).unwrap_or(0);
                    return Some(path[start_idx..].to_vec());
                }
            }
        }

        path.pop();
        rec_stack.remove(name);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::PluginMeta;
    use async_trait::async_trait;

    struct TestPlugin {
        meta: PluginMeta,
        build_order: std::sync::Arc<tokio::sync::Mutex<Vec<String>>>,
    }

    impl TestPlugin {
        fn new(name: &str) -> Self {
            Self {
                meta: PluginMeta::new(name, "1.0.0"),
                build_order: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            }
        }

        fn with_deps(name: &str, deps: Vec<String>) -> Self {
            Self {
                meta: PluginMeta::with_dependencies(name, "1.0.0", deps),
                build_order: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
            }
        }

        fn with_order_tracker(
            name: &str,
            deps: Vec<String>,
            tracker: std::sync::Arc<tokio::sync::Mutex<Vec<String>>>,
        ) -> Self {
            Self {
                meta: PluginMeta::with_dependencies(name, "1.0.0", deps),
                build_order: tracker,
            }
        }
    }

    #[async_trait]
    impl CrudPlugin for TestPlugin {
        fn meta(&self) -> PluginMeta {
            self.meta.clone()
        }

        async fn build(&mut self, _ctx: &mut PluginContext) -> Result<()> {
            let mut order = self.build_order.lock().await;
            order.push(self.meta.name.clone());
            Ok(())
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = PluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register_single() {
        let mut registry = PluginRegistry::new();
        let plugin = TestPlugin::new("test");

        registry.register(plugin).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.get("test").is_some());
    }

    #[test]
    fn test_registry_duplicate_registration() {
        let mut registry = PluginRegistry::new();
        registry.register(TestPlugin::new("test")).unwrap();

        let result = registry.register(TestPlugin::new("test"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already registered"));
    }

    #[test]
    fn test_registry_missing_dependency() {
        let mut registry = PluginRegistry::new();
        let plugin = TestPlugin::with_deps("child", vec!["parent".to_string()]);

        let result = registry.register(plugin);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not registered"));
    }

    #[test]
    fn test_registry_with_dependencies() {
        let mut registry = PluginRegistry::new();

        registry.register(TestPlugin::new("database")).unwrap();
        registry
            .register(TestPlugin::with_deps("cache", vec!["database".to_string()]))
            .unwrap();
        registry
            .register(TestPlugin::with_deps("auth", vec!["cache".to_string()]))
            .unwrap();

        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn test_registry_get_state() {
        let mut registry = PluginRegistry::new();
        registry.register(TestPlugin::new("test")).unwrap();

        assert_eq!(registry.get_state("test"), Some(PluginState::Adding));
        assert_eq!(registry.get_state("nonexistent"), None);
    }

    #[test]
    fn test_registry_plugin_names() {
        let mut registry = PluginRegistry::new();
        registry.register(TestPlugin::new("a")).unwrap();
        registry.register(TestPlugin::new("b")).unwrap();

        let names = registry.plugin_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
    }

    #[tokio::test]
    async fn test_registry_init_all_order() {
        let mut registry = PluginRegistry::new();
        let tracker = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));

        registry
            .register(TestPlugin::with_order_tracker(
                "database",
                vec![],
                tracker.clone(),
            ))
            .unwrap();
        registry
            .register(TestPlugin::with_order_tracker(
                "cache",
                vec!["database".to_string()],
                tracker.clone(),
            ))
            .unwrap();
        registry
            .register(TestPlugin::with_order_tracker(
                "auth",
                vec!["cache".to_string()],
                tracker.clone(),
            ))
            .unwrap();

        registry.init_all().await.unwrap();

        let order = tracker.lock().await;
        let db_idx = order.iter().position(|x| x == "database").unwrap();
        let cache_idx = order.iter().position(|x| x == "cache").unwrap();
        let auth_idx = order.iter().position(|x| x == "auth").unwrap();

        assert!(db_idx < cache_idx);
        assert!(cache_idx < auth_idx);
    }

    #[tokio::test]
    async fn test_registry_state_after_init() {
        let mut registry = PluginRegistry::new();
        registry.register(TestPlugin::new("test")).unwrap();

        registry.init_all().await.unwrap();

        assert_eq!(registry.get_state("test"), Some(PluginState::Ready));
    }

    #[tokio::test]
    async fn test_registry_finish_reverse_order() {
        let mut registry = PluginRegistry::new();
        registry.register(TestPlugin::new("a")).unwrap();
        registry
            .register(TestPlugin::with_deps("b", vec!["a".to_string()]))
            .unwrap();

        registry.init_all().await.unwrap();
        registry.finish_all().await.unwrap();

        assert_eq!(registry.get_state("a"), Some(PluginState::Finished));
        assert_eq!(registry.get_state("b"), Some(PluginState::Finished));
    }

    #[tokio::test]
    async fn test_registry_cleanup_all() {
        let mut registry = PluginRegistry::new();
        registry.register(TestPlugin::new("test")).unwrap();

        registry.init_all().await.unwrap();
        registry.cleanup_all().await.unwrap();

        assert_eq!(registry.get_state("test"), Some(PluginState::Cleaned));
    }

    #[tokio::test]
    async fn test_registry_context_sharing() {
        let mut registry = PluginRegistry::new();

        struct ContextPlugin;

        #[async_trait]
        impl CrudPlugin for ContextPlugin {
            fn meta(&self) -> PluginMeta {
                PluginMeta::new("context-test", "1.0.0")
            }

            async fn build(&mut self, ctx: &mut PluginContext) -> Result<()> {
                ctx.insert("shared_key", "shared_value".to_string()).await;
                Ok(())
            }
        }

        registry.register(ContextPlugin).unwrap();
        registry.init_all().await.unwrap();

        let val: Option<String> = registry.context().get("shared_key").await;
        assert_eq!(val, Some("shared_value".to_string()));
    }

    #[test]
    fn test_topological_order_simple() {
        let mut registry = PluginRegistry::new();
        registry.register(TestPlugin::new("a")).unwrap();
        registry
            .register(TestPlugin::with_deps("b", vec!["a".to_string()]))
            .unwrap();
        registry
            .register(TestPlugin::with_deps("c", vec!["b".to_string()]))
            .unwrap();

        let order = registry.topological_order().unwrap();

        let a_idx = order.iter().position(|x| x == "a").unwrap();
        let b_idx = order.iter().position(|x| x == "b").unwrap();
        let c_idx = order.iter().position(|x| x == "c").unwrap();

        assert!(a_idx < b_idx);
        assert!(b_idx < c_idx);
    }

    #[test]
    fn test_topological_order_diamond() {
        let mut registry = PluginRegistry::new();
        registry.register(TestPlugin::new("a")).unwrap();
        registry
            .register(TestPlugin::with_deps("b", vec!["a".to_string()]))
            .unwrap();
        registry
            .register(TestPlugin::with_deps("c", vec!["a".to_string()]))
            .unwrap();
        registry
            .register(TestPlugin::with_deps(
                "d",
                vec!["b".to_string(), "c".to_string()],
            ))
            .unwrap();

        let order = registry.topological_order().unwrap();

        let a_idx = order.iter().position(|x| x == "a").unwrap();
        let b_idx = order.iter().position(|x| x == "b").unwrap();
        let c_idx = order.iter().position(|x| x == "c").unwrap();
        let d_idx = order.iter().position(|x| x == "d").unwrap();

        assert!(a_idx < b_idx);
        assert!(a_idx < c_idx);
        assert!(b_idx < d_idx);
        assert!(c_idx < d_idx);
    }

    #[test]
    fn test_empty_registry_operations() {
        let registry = PluginRegistry::new();
        assert!(registry.is_empty());
        assert!(registry.get("nonexistent").is_none());
        assert!(registry.plugin_names().is_empty());
    }

    #[tokio::test]
    async fn test_empty_registry_lifecycle() {
        let mut registry = PluginRegistry::new();
        assert!(registry.init_all().await.is_ok());
        assert!(registry.ready_all().await.is_ok());
        assert!(registry.finish_all().await.is_ok());
        assert!(registry.cleanup_all().await.is_ok());
    }
}
