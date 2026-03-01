//! Moka-based in-memory cache backend.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use moka::future::Cache;
use rust_boot_core::error::Result;

use super::{CacheBackend, CacheConfig};

/// High-performance in-memory cache backend using the Moka library.
pub struct MokaBackend {
    cache: Arc<Cache<String, Vec<u8>>>,
    default_ttl: Duration,
}

impl MokaBackend {
    /// Creates a new Moka cache backend with the given configuration.
    pub fn new(config: CacheConfig) -> Self {
        let cache = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.default_ttl)
            .build();

        Self {
            cache: Arc::new(cache),
            default_ttl: config.default_ttl,
        }
    }

    /// Creates a backend from an existing Moka cache instance.
    pub fn with_cache(cache: Cache<String, Vec<u8>>, default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(cache),
            default_ttl,
        }
    }
}

#[async_trait]
impl CacheBackend for MokaBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.cache.get(key).await)
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let _ttl = ttl.unwrap_or(self.default_ttl);
        self.cache.insert(key.to_string(), value).await;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let existed = self.cache.contains_key(key);
        self.cache.remove(key).await;
        Ok(existed)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.cache.contains_key(key))
    }

    async fn clear(&self) -> Result<()> {
        self.cache.invalidate_all();
        self.cache.run_pending_tasks().await;
        Ok(())
    }
}
