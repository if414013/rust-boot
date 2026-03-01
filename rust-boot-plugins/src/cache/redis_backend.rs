//! Redis-based distributed cache backend.

use std::time::Duration;

use async_trait::async_trait;
use redis::AsyncCommands;
use rust_boot_core::error::{Result, RustBootError};

use super::{CacheBackend, CacheConfig};

/// Distributed cache backend using Redis.
pub struct RedisBackend {
    client: redis::Client,
    default_ttl: Duration,
}

impl RedisBackend {
    /// Creates a new Redis backend connecting to the given URL.
    pub fn new(url: &str, config: CacheConfig) -> Result<Self> {
        let client = redis::Client::open(url)
            .map_err(|e| RustBootError::Cache(format!("Failed to connect to Redis: {e}")))?;

        Ok(Self {
            client,
            default_ttl: config.default_ttl,
        })
    }

    /// Creates a backend from an existing Redis client.
    pub const fn with_client(client: redis::Client, default_ttl: Duration) -> Self {
        Self {
            client,
            default_ttl,
        }
    }

    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| RustBootError::Cache(format!("Redis connection error: {e}")))
    }
}

#[async_trait]
impl CacheBackend for RedisBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut conn = self.get_connection().await?;
        let result: Option<Vec<u8>> = conn
            .get(key)
            .await
            .map_err(|e| RustBootError::Cache(format!("Redis GET error: {e}")))?;
        Ok(result)
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let ttl_secs = ttl.unwrap_or(self.default_ttl).as_secs();

        let _: () = conn
            .set_ex(key, value, ttl_secs)
            .await
            .map_err(|e| RustBootError::Cache(format!("Redis SET error: {e}")))?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let deleted: i64 = conn
            .del(key)
            .await
            .map_err(|e| RustBootError::Cache(format!("Redis DEL error: {e}")))?;
        Ok(deleted > 0)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let exists: bool = conn
            .exists(key)
            .await
            .map_err(|e| RustBootError::Cache(format!("Redis EXISTS error: {e}")))?;
        Ok(exists)
    }

    async fn clear(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let _: () = redis::cmd("FLUSHDB")
            .query_async(&mut conn)
            .await
            .map_err(|e| RustBootError::Cache(format!("Redis FLUSHDB error: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_backend_creation_invalid_url() {
        let config = CacheConfig::default();
        let result = RedisBackend::new("invalid://url", config);
        assert!(result.is_err());
    }

    #[test]
    fn test_redis_backend_creation_valid_url() {
        let config = CacheConfig::default();
        let result = RedisBackend::new("redis://127.0.0.1:6379", config);
        assert!(result.is_ok());
    }
}
