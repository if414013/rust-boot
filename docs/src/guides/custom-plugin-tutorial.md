# Writing Custom Plugins

rust-boot's plugin system lets you extend the framework with your own functionality. Plugins participate in a managed lifecycle, can declare dependencies on other plugins, and share state through a typed context.

This tutorial walks through three plugins from the `custom_plugin` example:

1. **RequestCounterPlugin** — tracks request counts with shared atomic state
2. **AuditLoggerPlugin** — provides in-memory audit logging
3. **RateLimiterPlugin** — demonstrates plugin dependencies

```bash
cargo run --example custom_plugin
```

---

## The CrudPlugin Trait

Every plugin implements the `CrudPlugin` trait, which defines metadata and four lifecycle hooks:

```rust
use async_trait::async_trait;
use rust_boot::prelude::*;

#[async_trait]
pub trait CrudPlugin: Send + Sync {
    /// Returns plugin name, version, and optional dependencies.
    fn meta(&self) -> PluginMeta;

    /// Called during initialization. Set up resources and register shared state.
    async fn build(&mut self, ctx: &mut PluginContext) -> rust_boot::core::error::Result<()>;

    /// Called after all plugins are built. Start background tasks, open connections.
    async fn ready(&mut self, ctx: &mut PluginContext) -> rust_boot::core::error::Result<()>;

    /// Called during shutdown. Flush buffers, stop accepting work.
    async fn finish(&mut self, ctx: &mut PluginContext) -> rust_boot::core::error::Result<()>;

    /// Called after finish. Release resources, close connections.
    async fn cleanup(&mut self, ctx: &mut PluginContext) -> rust_boot::core::error::Result<()>;
}
```

The lifecycle runs in this order:

```
register → build (init_all) → ready (ready_all) → finish (finish_all) → cleanup (cleanup_all)
```

`finish` and `cleanup` run in **reverse** dependency order, so plugins that depend on others shut down first.

---

## Plugin 1: RequestCounterPlugin

This plugin tracks how many requests have been processed. It stores shared state in the `PluginContext` so other plugins (and application code) can access it.

### Define the Shared State

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

pub struct RequestCounter {
    counter: AtomicU64,
    start_time: RwLock<Option<Instant>>,
}

impl RequestCounter {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
            start_time: RwLock::new(None),
        }
    }

    pub fn increment(&self) {
        self.counter.fetch_add(1, Ordering::SeqCst);
    }

    pub fn count(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }

    pub async fn start_timer(&self) {
        *self.start_time.write().await = Some(Instant::now());
    }

    pub async fn uptime_secs(&self) -> Option<f64> {
        self.start_time.read().await.map(|s| s.elapsed().as_secs_f64())
    }
}
```

### Implement the Plugin

```rust
pub struct RequestCounterPlugin {
    counter: Arc<RequestCounter>,
}

impl RequestCounterPlugin {
    pub fn new() -> Self {
        Self { counter: Arc::new(RequestCounter::new()) }
    }
}

#[async_trait]
impl CrudPlugin for RequestCounterPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("request-counter", "1.0.0")
    }

    async fn build(&mut self, ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        // Store shared state in the context under a string key
        ctx.insert("request_counter", self.counter.clone()).await;
        Ok(())
    }

    async fn ready(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        self.counter.start_timer().await;
        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        let count = self.counter.count();
        let uptime = self.counter.uptime_secs().await.unwrap_or(0.0);
        println!("RequestCounter shutting down: {count} requests in {uptime:.2}s");
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        Ok(())
    }
}
```

Key points:

- `meta()` returns a `PluginMeta` with the plugin's name and version. The name is used for dependency resolution.
- `build()` inserts the shared `Arc<RequestCounter>` into the `PluginContext`. Any code with access to the context can retrieve it by key.
- `ready()` starts the uptime timer after all plugins are initialized.

---

## Plugin 2: AuditLoggerPlugin

This plugin provides an in-memory audit log. It follows the same pattern: define shared state, store it in the context during `build()`.

### Shared State

```rust
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: String,
    pub action: String,
    pub details: String,
}

pub struct AuditLog {
    entries: RwLock<Vec<AuditEntry>>,
    entry_count: AtomicU64,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            entry_count: AtomicU64::new(0),
        }
    }

    pub async fn log(&self, action: &str, details: &str) {
        let count = self.entry_count.fetch_add(1, Ordering::SeqCst);
        let entry = AuditEntry {
            timestamp: format!("entry-{:04}", count + 1),
            action: action.to_string(),
            details: details.to_string(),
        };
        self.entries.write().await.push(entry);
    }

    pub async fn entries(&self) -> Vec<AuditEntry> {
        self.entries.read().await.clone()
    }
}
```

### Plugin Implementation

```rust
pub struct AuditLoggerPlugin {
    log: Arc<AuditLog>,
}

impl AuditLoggerPlugin {
    pub fn new() -> Self {
        Self { log: Arc::new(AuditLog::new()) }
    }
}

#[async_trait]
impl CrudPlugin for AuditLoggerPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("audit-logger", "1.0.0")
    }

    async fn build(&mut self, ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        ctx.insert("audit_log", self.log.clone()).await;
        Ok(())
    }

    async fn ready(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        self.log.log("system", "Audit logger initialized").await;
        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        let count = self.log.entries().await.len();
        println!("AuditLogger shutting down: {count} entries recorded");
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        Ok(())
    }
}
```

---

## Plugin 3: RateLimiterPlugin (with Dependencies)

The rate limiter depends on the request counter — it needs access to the counter's shared state to enforce limits. rust-boot handles this through declared dependencies.

```rust
pub struct RateLimiterPlugin {
    max_requests_per_minute: u64,
    enabled: bool,
}

impl RateLimiterPlugin {
    pub const fn new(max_requests_per_minute: u64) -> Self {
        Self { max_requests_per_minute, enabled: false }
    }
}

#[async_trait]
impl CrudPlugin for RateLimiterPlugin {
    fn meta(&self) -> PluginMeta {
        // Declare dependency on "request-counter"
        PluginMeta::new("rate-limiter", "1.0.0")
            .with_dependency("request-counter")
    }

    async fn build(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        println!("RateLimiter configured: max {} req/min", self.max_requests_per_minute);
        Ok(())
    }

    async fn ready(&mut self, ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        // Retrieve the counter from context (placed there by RequestCounterPlugin)
        let counter: Option<Arc<RequestCounter>> = ctx.get("request_counter").await;
        if counter.is_some() {
            self.enabled = true;
            println!("RateLimiter enabled — request counter found");
        } else {
            println!("RateLimiter disabled — no request counter available");
        }
        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> rust_boot::core::error::Result<()> {
        self.enabled = false;
        Ok(())
    }
}
```

The key line is `.with_dependency("request-counter")` in `meta()`. This tells the registry:

- **Registration order**: `request-counter` must be registered before `rate-limiter`. Registration will fail otherwise.
- **Init order**: `request-counter`'s `build()` runs before `rate-limiter`'s `build()`.
- **Shutdown order**: `rate-limiter`'s `finish()` and `cleanup()` run before `request-counter`'s.

---

## Putting It All Together

Register plugins with the `PluginRegistry` and run the lifecycle:

```rust
async fn run_example() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = PluginRegistry::new();

    // Register in dependency order
    registry.register(RequestCounterPlugin::new())?;
    registry.register(AuditLoggerPlugin::new())?;
    registry.register(RateLimiterPlugin::new(100))?;

    // Run lifecycle phases
    registry.init_all().await?;   // calls build() on all plugins
    registry.ready_all().await?;  // calls ready() on all plugins

    // Access shared state from the context
    let counter: Option<Arc<RequestCounter>> = registry.context().get("request_counter").await;
    let counter = counter.expect("Counter should exist");

    for i in 1..=5 {
        counter.increment();
        println!("Request #{}: count = {}", i, counter.count());
    }

    let audit_log: Option<Arc<AuditLog>> = registry.context().get("audit_log").await;
    let audit_log = audit_log.expect("Audit log should exist");
    audit_log.log("user.login", "User alice logged in").await;

    // Shutdown (reverse dependency order)
    registry.finish_all().await?;
    registry.cleanup_all().await?;

    Ok(())
}
```

---

## PluginContext Deep Dive

`PluginContext` is a type-safe, async key-value store that plugins use to share state. It uses `Arc<dyn Any + Send + Sync>` internally, so you can store any `Send + Sync + 'static` type.

### Inserting State

```rust
ctx.insert("my_key", Arc::new(MySharedState::new())).await;
```

### Retrieving State

```rust
let state: Option<Arc<MySharedState>> = ctx.get("my_key").await;
```

The type parameter on `get()` determines what type to downcast to. If the key doesn't exist or the type doesn't match, you get `None`.

### Accessing Context from the Registry

Outside of plugin lifecycle hooks, you can access the context through the registry:

```rust
let value: Option<Arc<MyType>> = registry.context().get("key").await;
```

---

## Best Practices

- **Name plugins with kebab-case** (`"request-counter"`, not `"RequestCounter"`). Names are used for dependency resolution and must be unique.
- **Store shared state as `Arc<T>`** in the context. This allows multiple plugins and application code to hold references concurrently.
- **Declare dependencies explicitly** via `with_dependency()`. Don't rely on registration order alone — the dependency declaration makes the contract clear and enforced.
- **Do heavy initialization in `build()`**, not in the constructor. The constructor should be cheap; `build()` is where you allocate resources, open connections, etc.
- **Use `ready()` for startup tasks** that depend on other plugins being fully initialized (e.g., checking that a dependency's state is available).
- **Clean up in `cleanup()`**, not `finish()`. Use `finish()` to stop accepting new work and flush buffers; use `cleanup()` to release resources.

---

## Next Steps

- [Basic API Tutorial](./basic-api-tutorial.md) — See how built-in plugins (CachingPlugin, AuthPlugin, MonitoringPlugin) are used in a real API
- [Database Setup Guide](./database-setup.md) — Connect to a database with SeaORM
- [API Reference](../reference/api-reference.md) — Full type catalog for the plugin system
