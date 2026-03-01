# Custom Plugins

Building your own plugins is the primary way to extend rust-boot with application-specific functionality. A plugin is any struct that implements the `CrudPlugin` trait — it declares metadata (name, version, dependencies), hooks into the four lifecycle phases, and shares state with other plugins through the `PluginContext`.

This page walks through three complete example plugins of increasing complexity, all from the framework's `custom_plugin.rs` example.

## The CrudPlugin Trait

Every plugin implements this trait:

```rust
#[async_trait]
pub trait CrudPlugin: Send + Sync {
    /// Returns the plugin's name, version, and dependencies.
    fn meta(&self) -> PluginMeta;

    /// Called during registration — initialize resources, insert shared state.
    async fn build(&mut self, ctx: &mut PluginContext) -> Result<()> { Ok(()) }

    /// Called after all plugins are built — cross-plugin initialization.
    async fn ready(&mut self, ctx: &mut PluginContext) -> Result<()> { Ok(()) }

    /// Called during shutdown — complete pending work.
    async fn finish(&mut self, ctx: &mut PluginContext) -> Result<()> { Ok(()) }

    /// Called last — release resources, clean up context entries.
    async fn cleanup(&mut self, ctx: &mut PluginContext) -> Result<()> { Ok(()) }
}
```

All lifecycle methods have default no-op implementations, so you only need to override the ones relevant to your plugin.

## PluginMeta

`PluginMeta` identifies your plugin and declares its dependencies:

```rust
// Simple plugin with no dependencies
let meta = PluginMeta::new("my-plugin", "1.0.0");

// Plugin with dependencies (these must be registered first)
let meta = PluginMeta::new("rate-limiter", "1.0.0")
    .with_dependency("request-counter");

// Multiple dependencies
let meta = PluginMeta::with_dependencies(
    "dashboard",
    "1.0.0",
    vec!["auth".to_string(), "monitoring".to_string()],
);
```

| Method | Description |
|---|---|
| `new(name, version)` | Creates metadata with no dependencies |
| `with_dependencies(name, version, deps)` | Creates metadata with a list of dependencies |
| `with_dependency(name)` | Builder method to add a single dependency |

The registry validates dependencies at registration time. If a dependency hasn't been registered yet, registration fails immediately with a clear error.

## Example 1: RequestCounterPlugin

The simplest useful plugin — it maintains a shared atomic counter that other plugins and application code can use to track request counts.

### Shared State

First, define the state that will be shared through the `PluginContext`:

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Shared state for counting requests (stored in PluginContext)
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
        let mut start_time = self.start_time.write().await;
        *start_time = Some(Instant::now());
    }

    pub async fn uptime_secs(&self) -> Option<f64> {
        self.start_time.read().await
            .map(|start| start.elapsed().as_secs_f64())
    }
}
```

### Plugin Implementation

```rust
pub struct RequestCounterPlugin {
    counter: Arc<RequestCounter>,
}

impl RequestCounterPlugin {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(RequestCounter::new()),
        }
    }
}

#[async_trait]
impl CrudPlugin for RequestCounterPlugin {
    fn meta(&self) -> PluginMeta {
        // No dependencies — this plugin can be registered in any order
        PluginMeta::new("request-counter", "1.0.0")
    }

    async fn build(&mut self, ctx: &mut PluginContext) -> Result<()> {
        // Store the shared counter in the context so other plugins can access it
        ctx.insert("request_counter", self.counter.clone()).await;
        Ok(())
    }

    async fn ready(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        // Start the uptime timer when the plugin becomes ready
        self.counter.start_timer().await;
        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        // Log final stats during shutdown
        let count = self.counter.count();
        let uptime = self.counter.uptime_secs().await.unwrap_or(0.0);
        println!("Total requests: {count}, Uptime: {uptime:.2}s");
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        // Nothing to clean up — the Arc will be dropped naturally
        Ok(())
    }
}
```

Key patterns demonstrated:
- Wrap shared state in `Arc<T>` so it can be cloned cheaply from the context
- Insert state during `build()` so it's available to other plugins
- Use `ready()` for initialization that should happen after all plugins are built
- Use `finish()` for final reporting before shutdown

### Using the Counter

```rust
// From application code, after registry.init_all():
let counter: Option<Arc<RequestCounter>> = registry.context().get("request_counter").await;
let counter = counter.expect("request-counter plugin should be registered");

// In your request handlers:
counter.increment();
println!("Total requests: {}", counter.count());
```

## Example 2: AuditLoggerPlugin

A more practical plugin that provides an in-memory audit log. It demonstrates storing structured data and using the `ready()` hook to log an initialization event.

### Shared State

```rust
use tokio::sync::RwLock;

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
        Self {
            log: Arc::new(AuditLog::new()),
        }
    }
}

#[async_trait]
impl CrudPlugin for AuditLoggerPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("audit-logger", "1.0.0")
    }

    async fn build(&mut self, ctx: &mut PluginContext) -> Result<()> {
        // Share the audit log with the rest of the application
        ctx.insert("audit_log", self.log.clone()).await;
        Ok(())
    }

    async fn ready(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        // Log that the system has started
        self.log.log("system", "Audit logger initialized").await;
        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        let count = self.log.entries().await.len();
        println!("Audit logger shutting down with {count} entries");
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }
}
```

### Using the Audit Log

```rust
let audit_log: Arc<AuditLog> = registry.context()
    .get("audit_log").await
    .expect("audit-logger plugin should be registered");

audit_log.log("user.login", "User alice logged in").await;
audit_log.log("user.update", "User alice updated profile").await;

let entries = audit_log.entries().await;
for entry in entries {
    println!("[{}] {} - {}", entry.timestamp, entry.action, entry.details);
}
```

## Example 3: RateLimiterPlugin (with Dependencies)

This plugin demonstrates the dependency system. It depends on the `request-counter` plugin and uses the shared `RequestCounter` from the context during its `ready()` phase.

```rust
pub struct RateLimiterPlugin {
    max_requests_per_minute: u64,
    enabled: bool,
}

impl RateLimiterPlugin {
    pub const fn new(max_requests_per_minute: u64) -> Self {
        Self {
            max_requests_per_minute,
            enabled: false,
        }
    }
}

#[async_trait]
impl CrudPlugin for RateLimiterPlugin {
    fn meta(&self) -> PluginMeta {
        // Declare dependency on request-counter
        // Registration will fail if request-counter isn't registered first
        PluginMeta::new("rate-limiter", "1.0.0")
            .with_dependency("request-counter")
    }

    async fn build(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        println!("Rate limiter configured: max {} req/min", self.max_requests_per_minute);
        Ok(())
    }

    async fn ready(&mut self, ctx: &mut PluginContext) -> Result<()> {
        // Access the request counter from the context
        // It's guaranteed to exist because we declared the dependency
        let counter: Option<Arc<RequestCounter>> = ctx.get("request_counter").await;
        if counter.is_some() {
            self.enabled = true;
        }
        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        println!("Rate limiter shutting down. Enabled: {}", self.enabled);
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        self.enabled = false;
        Ok(())
    }
}
```

### Registration Order Matters

Because `RateLimiterPlugin` depends on `request-counter`, you must register the counter first:

```rust
let mut registry = PluginRegistry::new();

// These must be registered before the rate limiter
registry.register(RequestCounterPlugin::new())?;
registry.register(AuditLoggerPlugin::new())?;

// Now the rate limiter can find its dependency
registry.register(RateLimiterPlugin::new(100))?;

// init_all() calls build() in topological order
registry.init_all().await?;
// ready_all() calls ready() — rate limiter finds the counter here
registry.ready_all().await?;
```

## Lifecycle Hooks Summary

| Hook | When | Use For |
|---|---|---|
| `build()` | During `init_all()`, in dependency order | Allocate resources, insert state into context |
| `ready()` | During `ready_all()`, after all builds complete | Cross-plugin initialization, access other plugins' state |
| `finish()` | During `finish_all()`, in reverse dependency order | Flush buffers, log final stats, complete pending work |
| `cleanup()` | During `cleanup_all()`, in reverse dependency order | Release resources, remove context entries, close connections |

## PluginContext Patterns

### Inserting State

Always wrap shared state in `Arc<T>` before inserting. This lets consumers clone cheaply:

```rust
async fn build(&mut self, ctx: &mut PluginContext) -> Result<()> {
    ctx.insert("my_service", Arc::new(MyService::new())).await;
    Ok(())
}
```

### Reading State from Other Plugins

Use `get<T>()` with the exact type. A type mismatch returns `None`:

```rust
async fn ready(&mut self, ctx: &mut PluginContext) -> Result<()> {
    // Must match the exact type that was inserted
    let service: Option<Arc<MyService>> = ctx.get("my_service").await;
    Ok(())
}
```

### Cleaning Up

Remove your entries during `cleanup()` to prevent memory leaks:

```rust
async fn cleanup(&mut self, ctx: &mut PluginContext) -> Result<()> {
    ctx.remove::<Arc<MyService>>("my_service").await;
    Ok(())
}
```

## Best Practices

- **Name plugins with kebab-case** — e.g., `"request-counter"`, `"audit-logger"`. This is the convention used by all built-in plugins.
- **Use semantic versioning** — the version string in `PluginMeta` follows semver.
- **Declare all dependencies explicitly** — don't rely on registration order alone. The dependency system ensures correct initialization order even as your plugin set grows.
- **Keep `build()` fast** — avoid blocking I/O in `build()`. If you need async initialization, do the heavy lifting in `ready()`.
- **Use namespaced context keys** — prefer `"myplugin:state"` over generic names like `"state"` to avoid collisions.
- **Always implement `cleanup()`** for plugins that allocate resources — close connections, flush caches, remove context entries.
- **Make plugins `Send + Sync`** — the trait requires it. Use `Arc`, `RwLock`, and atomic types for interior mutability.
