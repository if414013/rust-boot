# Caching Plugin

The `CachingPlugin` provides a unified caching layer with pluggable backends. Out of the box it ships with two backends: `MokaBackend` for high-performance in-memory caching and `RedisBackend` for distributed caching across multiple application instances.

The plugin implements the `CacheBackend` trait, which means you can swap backends without changing any of your application code — just configure a different backend at startup.

## Quick Start

```rust
use rust_boot::prelude::*;
use std::time::Duration;

// Configure the cache
let cache_config = CacheConfig::new("my-cache")
    .with_ttl(Duration::from_secs(60))       // entries expire after 60 seconds
    .with_max_capacity(5_000);                // hold up to 5,000 entries

// Register with the plugin system (defaults to MokaBackend)
let mut registry = PluginRegistry::new();
registry.register(CachingPlugin::new(cache_config))?;
registry.init_all().await?;

// Access the backend from the context
let backend: Option<Arc<dyn CacheBackend>> = registry.context().get("cache_backend").await;
let backend = backend.expect("cache should be initialized");

// Store and retrieve data
backend.set("user:123", b"Alice".to_vec(), None).await?;
let value = backend.get("user:123").await?;
assert_eq!(value, Some(b"Alice".to_vec()));
```

## CacheConfig

`CacheConfig` controls cache behavior. All settings use the builder pattern.

```rust
let config = CacheConfig::new("my-cache");
```

| Field | Default | Description |
|---|---|---|
| `name` | `"default"` | Identifier for this cache instance |
| `default_ttl` | 300 seconds (5 min) | How long entries live before expiring |
| `max_capacity` | 10,000 | Maximum number of entries the cache can hold |

### Builder Methods

| Method | Description |
|---|---|
| `new(name)` | Creates a config with the given name and defaults |
| `with_ttl(Duration)` | Sets the default time-to-live for entries |
| `with_max_capacity(u64)` | Sets the maximum number of cached entries |

```rust
use std::time::Duration;

let config = CacheConfig::new("sessions")
    .with_ttl(Duration::from_secs(1800))   // 30 minutes
    .with_max_capacity(50_000);
```

## CacheBackend Trait

All cache backends implement the `CacheBackend` trait. This is the interface you interact with for all cache operations. Values are stored as raw bytes (`Vec<u8>`), giving you full control over serialization.

```rust
#[async_trait]
pub trait CacheBackend: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<bool>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn clear(&self) -> Result<()>;
}
```

| Method | Returns | Description |
|---|---|---|
| `get(key)` | `Result<Option<Vec<u8>>>` | Retrieves raw bytes by key, `None` if missing |
| `set(key, value, ttl)` | `Result<()>` | Stores bytes with an optional per-entry TTL override |
| `delete(key)` | `Result<bool>` | Removes an entry, returns whether it existed |
| `exists(key)` | `Result<bool>` | Checks if a key is present |
| `clear()` | `Result<()>` | Removes all entries from the cache |

### Typed Helpers

For convenience, the caching module provides two free functions that handle JSON serialization automatically:

```rust
use rust_boot::prelude::*;

#[derive(Serialize, Deserialize)]
struct User {
    id: i32,
    name: String,
}

let user = User { id: 1, name: "Alice".to_string() };

// Store a typed value (serializes to JSON bytes internally)
set_typed(&*backend, "user:1", &user, None).await?;

// Retrieve and deserialize
let cached: Option<User> = get_typed(&*backend, "user:1").await?;
```

| Function | Description |
|---|---|
| `get_typed<T: DeserializeOwned>(backend, key)` | Deserializes cached bytes into `T` via `serde_json` |
| `set_typed<T: Serialize>(backend, key, value, ttl)` | Serializes `T` to JSON bytes and stores it |

### Cache Key Helpers

Two utility functions help you build consistent cache keys:

```rust
// Simple prefix:id format
let key = generate_cache_key("user", "123");
// => "user:123"

// Entity-namespaced format
let key = generate_entity_key("Product", &42);
// => "entity:Product:42"
```

## MokaBackend

`MokaBackend` is the default backend, powered by the [Moka](https://github.com/moka-rs/moka) library. It's a high-performance, concurrent in-memory cache with automatic eviction based on TTL and capacity limits.

This is the right choice when:
- Your application runs as a single instance
- You need sub-millisecond cache access
- You don't need cache sharing across processes

```rust
// MokaBackend is used automatically when no backend is specified
let plugin = CachingPlugin::new(CacheConfig::new("fast-cache"));

// Or create one explicitly
let backend = MokaBackend::new(CacheConfig::new("explicit")
    .with_ttl(Duration::from_secs(120))
    .with_max_capacity(20_000));
```

Moka handles eviction internally — when the cache reaches `max_capacity`, the least-recently-used entries are evicted. Expired entries are cleaned up lazily on access and periodically in the background.

You can also wrap an existing Moka `Cache` instance:

```rust
use moka::future::Cache;

let cache = Cache::builder()
    .max_capacity(1000)
    .time_to_live(Duration::from_secs(60))
    .build();

let backend = MokaBackend::with_cache(cache, Duration::from_secs(60));
```

## RedisBackend

`RedisBackend` provides distributed caching through Redis. Use this when you need cache sharing across multiple application instances or cache persistence across restarts.

```rust
let config = CacheConfig::new("distributed")
    .with_ttl(Duration::from_secs(300));

// Connect to Redis
let backend = RedisBackend::new("redis://127.0.0.1:6379", config)?;

// Use with the plugin
let plugin = CachingPlugin::new(config).with_backend(backend);
```

You can also create a backend from an existing Redis client:

```rust
let client = redis::Client::open("redis://127.0.0.1:6379")?;
let backend = RedisBackend::with_client(client, Duration::from_secs(300));
```

All Redis operations use multiplexed async connections for efficient connection reuse. The `set` operation always uses `SET EX` (set with expiration) — if no per-entry TTL is provided, the `default_ttl` from the config is used.

The `clear()` method executes `FLUSHDB`, which removes all keys in the current Redis database. Use with caution in shared Redis instances.

## CachingPlugin Lifecycle

The `CachingPlugin` registers with the name `"caching"` and version `"1.0.0"`. It has no dependencies on other plugins.

- **build()** — If no backend was provided via `with_backend()`, creates a `MokaBackend` using the config. Inserts the backend into the `PluginContext` under the key `"cache_backend"` as `Arc<dyn CacheBackend>`.
- **cleanup()** — Calls `clear()` on the backend to flush all cached data, removes the `"cache_backend"` entry from the context, and drops the backend reference.

```rust
let config = CacheConfig::new("app-cache")
    .with_ttl(Duration::from_secs(600))
    .with_max_capacity(10_000);

// Default: in-memory with Moka
let plugin = CachingPlugin::new(config.clone());

// Or: distributed with Redis
let redis = RedisBackend::new("redis://localhost:6379", config.clone())?;
let plugin = CachingPlugin::new(config).with_backend(redis);
```

## Complete Example

```rust
use rust_boot::prelude::*;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Product {
    id: String,
    name: String,
    price: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Configure and register the caching plugin
    let config = CacheConfig::new("products")
        .with_ttl(Duration::from_secs(300))
        .with_max_capacity(10_000);

    let mut registry = PluginRegistry::new();
    registry.register(CachingPlugin::new(config))?;
    registry.init_all().await?;

    // 2. Get the cache backend from the context
    let backend: Arc<dyn CacheBackend> = registry
        .context()
        .get("cache_backend")
        .await
        .expect("cache backend should be available");

    // 3. Cache a product using typed helpers
    let product = Product {
        id: "prod-001".to_string(),
        name: "Rust Book".to_string(),
        price: 39.99,
    };

    let key = generate_entity_key("Product", &product.id);
    set_typed(&*backend, &key, &product, None).await?;

    // 4. Retrieve from cache
    let cached: Option<Product> = get_typed(&*backend, &key).await?;
    assert_eq!(cached.unwrap().name, "Rust Book");

    // 5. Check existence and delete
    assert!(backend.exists(&key).await?);
    backend.delete(&key).await?;
    assert!(!backend.exists(&key).await?);

    // 6. Cleanup
    registry.finish_all().await?;
    registry.cleanup_all().await?;

    Ok(())
}
```
