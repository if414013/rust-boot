//! Caching plugin with multiple backend support.
//!
//! Provides a unified caching interface with pluggable backends:
//! - [`MokaBackend`]: High-performance in-memory cache (default)
//! - [`RedisBackend`]: Distributed cache using Redis

mod moka_backend;
mod redis_backend;

pub use moka_backend::MokaBackend;
pub use redis_backend::RedisBackend;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rust_boot_core::error::Result;
use rust_boot_core::plugin::{CrudPlugin, PluginContext, PluginMeta};
use serde::{de::DeserializeOwned, Serialize};

/// Configuration for the caching system.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Default time-to-live for cached entries.
    pub default_ttl: Duration,
    /// Maximum number of entries the cache can hold.
    pub max_capacity: u64,
    /// Name identifier for this cache instance.
    pub name: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(300),
            max_capacity: 10_000,
            name: "default".to_string(),
        }
    }
}

impl CacheConfig {
    /// Creates a new cache configuration with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Sets the default TTL for cached entries.
    pub const fn with_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Sets the maximum capacity of the cache.
    pub const fn with_max_capacity(mut self, capacity: u64) -> Self {
        self.max_capacity = capacity;
        self
    }
}

/// Trait for cache backend implementations.
#[async_trait]
pub trait CacheBackend: Send + Sync {
    /// Retrieves a value from the cache by key.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Stores a value in the cache with an optional TTL.
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()>;

    /// Deletes a value from the cache, returning whether it existed.
    async fn delete(&self, key: &str) -> Result<bool>;

    /// Checks if a key exists in the cache.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Removes all entries from the cache.
    async fn clear(&self) -> Result<()>;
}

/// Retrieves and deserializes a typed value from the cache.
pub async fn get_typed<T: DeserializeOwned>(
    backend: &dyn CacheBackend,
    key: &str,
) -> Result<Option<T>> {
    match backend.get(key).await? {
        Some(bytes) => {
            let value: T = serde_json::from_slice(&bytes)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

/// Serializes and stores a typed value in the cache.
pub async fn set_typed<T: Serialize>(
    backend: &dyn CacheBackend,
    key: &str,
    value: &T,
    ttl: Option<Duration>,
) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    backend.set(key, bytes, ttl).await
}

/// Plugin providing caching functionality to the application.
pub struct CachingPlugin {
    config: CacheConfig,
    backend: Option<Arc<dyn CacheBackend>>,
}

impl CachingPlugin {
    /// Creates a new caching plugin with the given configuration.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            backend: None,
        }
    }

    /// Sets a custom cache backend.
    pub fn with_backend<B: CacheBackend + 'static>(mut self, backend: B) -> Self {
        self.backend = Some(Arc::new(backend));
        self
    }

    /// Returns a reference to the cache backend, if initialized.
    pub fn backend(&self) -> Option<&Arc<dyn CacheBackend>> {
        self.backend.as_ref()
    }
}

#[async_trait]
impl CrudPlugin for CachingPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("caching", "1.0.0")
    }

    async fn build(&mut self, ctx: &mut PluginContext) -> Result<()> {
        if self.backend.is_none() {
            let backend = MokaBackend::new(self.config.clone());
            self.backend = Some(Arc::new(backend));
        }

        if let Some(backend) = &self.backend {
            ctx.insert("cache_backend", backend.clone()).await;
        }

        Ok(())
    }

    async fn cleanup(&mut self, ctx: &mut PluginContext) -> Result<()> {
        if let Some(backend) = &self.backend {
            backend.clear().await?;
        }
        ctx.remove::<Arc<dyn CacheBackend>>("cache_backend").await;
        self.backend = None;
        Ok(())
    }
}

/// Generates a cache key with the given prefix and ID.
pub fn generate_cache_key(prefix: &str, id: &str) -> String {
    format!("{prefix}:{id}")
}

/// Generates a cache key for an entity using its type name and ID.
pub fn generate_entity_key<T>(entity_name: &str, id: &T) -> String
where
    T: std::fmt::Display,
{
    format!("entity:{entity_name}:{id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.default_ttl, Duration::from_secs(300));
        assert_eq!(config.max_capacity, 10_000);
        assert_eq!(config.name, "default");
    }

    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfig::new("test")
            .with_ttl(Duration::from_secs(60))
            .with_max_capacity(1000);

        assert_eq!(config.name, "test");
        assert_eq!(config.default_ttl, Duration::from_secs(60));
        assert_eq!(config.max_capacity, 1000);
    }

    #[test]
    fn test_generate_cache_key() {
        assert_eq!(generate_cache_key("user", "123"), "user:123");
        assert_eq!(generate_cache_key("session", "abc"), "session:abc");
    }

    #[test]
    fn test_generate_entity_key() {
        assert_eq!(generate_entity_key("User", &42), "entity:User:42");
        assert_eq!(
            generate_entity_key("Product", &"abc-123"),
            "entity:Product:abc-123"
        );
    }

    #[tokio::test]
    async fn test_caching_plugin_creation() {
        let config = CacheConfig::new("test");
        let plugin = CachingPlugin::new(config);
        assert!(plugin.backend().is_none());
    }

    #[tokio::test]
    async fn test_caching_plugin_build() {
        let config = CacheConfig::new("test");
        let mut plugin = CachingPlugin::new(config);
        let mut ctx = PluginContext::new();

        plugin.build(&mut ctx).await.unwrap();

        assert!(plugin.backend().is_some());
        assert!(ctx.contains("cache_backend").await);
    }

    #[tokio::test]
    async fn test_caching_plugin_cleanup() {
        let config = CacheConfig::new("test");
        let mut plugin = CachingPlugin::new(config);
        let mut ctx = PluginContext::new();

        plugin.build(&mut ctx).await.unwrap();
        plugin.cleanup(&mut ctx).await.unwrap();

        assert!(plugin.backend().is_none());
        assert!(!ctx.contains("cache_backend").await);
    }

    #[tokio::test]
    async fn test_moka_backend_basic_operations() {
        let config = CacheConfig::new("test");
        let backend = MokaBackend::new(config);

        assert!(!backend.exists("key1").await.unwrap());

        backend.set("key1", b"value1".to_vec(), None).await.unwrap();
        assert!(backend.exists("key1").await.unwrap());

        let value = backend.get("key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        let deleted = backend.delete("key1").await.unwrap();
        assert!(deleted);
        assert!(!backend.exists("key1").await.unwrap());
    }

    #[tokio::test]
    async fn test_moka_backend_typed_operations() {
        let config = CacheConfig::new("test");
        let backend = MokaBackend::new(config);

        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
        struct TestData {
            id: i32,
            name: String,
        }

        let data = TestData {
            id: 1,
            name: "test".to_string(),
        };

        set_typed(&backend, "typed_key", &data, None).await.unwrap();

        let retrieved: Option<TestData> = get_typed(&backend, "typed_key").await.unwrap();
        assert_eq!(retrieved, Some(data));
    }

    #[tokio::test]
    async fn test_moka_backend_clear() {
        let config = CacheConfig::new("test");
        let backend = MokaBackend::new(config);

        backend.set("key1", b"value1".to_vec(), None).await.unwrap();
        backend.set("key2", b"value2".to_vec(), None).await.unwrap();

        backend.clear().await.unwrap();

        assert!(!backend.exists("key1").await.unwrap());
        assert!(!backend.exists("key2").await.unwrap());
    }

    #[tokio::test]
    async fn test_moka_backend_get_nonexistent() {
        let config = CacheConfig::new("test");
        let backend = MokaBackend::new(config);

        let value = backend.get("nonexistent").await.unwrap();
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_moka_backend_delete_nonexistent() {
        let config = CacheConfig::new("test");
        let backend = MokaBackend::new(config);

        let deleted = backend.delete("nonexistent").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_moka_backend_overwrite() {
        let config = CacheConfig::new("test");
        let backend = MokaBackend::new(config);

        backend.set("key", b"value1".to_vec(), None).await.unwrap();
        backend.set("key", b"value2".to_vec(), None).await.unwrap();

        let value = backend.get("key").await.unwrap();
        assert_eq!(value, Some(b"value2".to_vec()));
    }
}
