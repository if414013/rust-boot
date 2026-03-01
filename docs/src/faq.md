# FAQ

## General

### What is rust-boot?

rust-boot is a Rust framework for building CRUD-heavy web APIs. It provides a layered architecture (repository, service, handler) with a plugin system inspired by Bevy, Axum integration for HTTP routing, and built-in plugins for authentication, caching, monitoring, and event sourcing.

### How does rust-boot compare to Spring Boot?

rust-boot borrows several concepts from Spring Boot but adapts them for Rust's ownership model and async ecosystem:

| Spring Boot Concept | rust-boot Equivalent |
|---|---|
| `@SpringBootApplication` | `PluginRegistry` + `init_all()` |
| `@Component` / `@Service` | `CrudPlugin` trait |
| `ApplicationContext` | `PluginContext` |
| `@Autowired` | `ctx.get::<T>("key").await` |
| `@Repository` | `CrudRepository` trait |
| `@RestController` | Axum `Router` + handler functions |
| `application.properties` | `AppConfig` with TOML/env support |
| Spring Security | `AuthPlugin` with JWT + RBAC |
| Spring Cache | `CachingPlugin` with Moka/Redis backends |
| Spring Actuator | `MonitoringPlugin` with Prometheus metrics |

The key difference is that rust-boot uses explicit registration and dependency declaration rather than annotation-based auto-discovery. You register plugins in code, declare dependencies through `PluginMeta`, and the registry handles initialization ordering.

### What is the minimum supported Rust version (MSRV)?

rust-boot requires Rust 1.88 or later. This is because the framework uses `std::sync::LazyLock`, which was stabilized in Rust 1.80, along with other features from recent stable releases. The MSRV is enforced in CI and specified in each crate's `Cargo.toml` via `rust-version = "1.88"`.

To check your Rust version:

```bash
rustc --version
```

To update:

```bash
rustup update stable
```

## Plugin System

### What order are plugins initialized in?

Plugins are initialized in topological (dependency) order using Kahn's algorithm. If plugin B depends on plugin A, then A's `build()` is always called before B's `build()`. Shutdown (`finish()` and `cleanup()`) happens in reverse order.

Plugins with no dependency relationship between them may be initialized in any order — don't rely on registration order for independent plugins.

### Can I have circular dependencies between plugins?

No. The registry uses topological sorting, which detects circular dependencies and returns an error. If you have plugins that need to reference each other, consider:

1. Extracting the shared state into a third plugin that both depend on
2. Using the `ready()` hook for cross-plugin initialization (since all plugins are built by then)
3. Restructuring so the dependency flows in one direction

### What happens if I register a plugin whose dependency isn't registered yet?

Registration fails immediately with an error like `"Plugin 'rate-limiter' depends on 'request-counter' which is not registered"`. Always register dependencies before the plugins that depend on them.

### Can I register the same plugin twice?

No. Plugin names must be unique. Attempting to register a second plugin with the same name returns an error.

### How do I share state between plugins?

Use the `PluginContext`. Insert state during `build()` and retrieve it in other plugins' `build()` or `ready()` methods:

```rust
// In plugin A's build():
ctx.insert("shared_data", Arc::new(MyData::new())).await;

// In plugin B's ready():
let data: Option<Arc<MyData>> = ctx.get("shared_data").await;
```

Always wrap shared state in `Arc<T>` for cheap cloning. Use descriptive keys to avoid collisions.

### Why does `ctx.get()` return `None` even though I inserted a value?

The most common cause is a type mismatch. `PluginContext::get<T>()` uses `downcast_ref::<T>()` internally — if the type parameter doesn't exactly match what was inserted, it returns `None` silently.

```rust
// Inserted as Arc<MyService>
ctx.insert("service", Arc::new(MyService::new())).await;

// This returns None — wrong type!
let svc: Option<MyService> = ctx.get("service").await;

// This works — exact type match
let svc: Option<Arc<MyService>> = ctx.get("service").await;
```

## Compilation and Build Issues

### I'm getting `LazyLock` not found errors

You need Rust 1.80 or later for `std::sync::LazyLock`. The framework's MSRV is 1.88. Update your toolchain:

```bash
rustup update stable
```

### Compilation is slow — how can I speed it up?

A few strategies:

- Use `cargo check` instead of `cargo build` during development
- Enable incremental compilation (on by default)
- Use the `mold` or `lld` linker for faster linking:
  ```toml
  # .cargo/config.toml
  [target.x86_64-unknown-linux-gnu]
  linker = "clang"
  rustflags = ["-C", "link-arg=-fuse-ld=mold"]
  ```
- Only enable the features you need — the `redis` feature pulls in additional dependencies

### I'm getting trait bound errors with `CrudPlugin`

The `CrudPlugin` trait requires `Send + Sync`. Make sure your plugin struct and all its fields are `Send + Sync`. Common issues:

- Using `Rc` instead of `Arc`
- Using `RefCell` instead of `RwLock` or `Mutex`
- Holding non-Send types across `.await` points

```rust
// Bad: Rc is not Send
struct MyPlugin {
    data: Rc<MyData>,
}

// Good: Arc is Send + Sync
struct MyPlugin {
    data: Arc<MyData>,
}
```

### How do I enable the Redis cache backend?

The Redis backend requires the `redis` feature flag:

```toml
[dependencies]
rust-boot-plugins = { version = "0.1", features = ["redis"] }
```

Without this feature, `RedisBackend` is still available in the code but the `redis` crate dependency won't be compiled.

## Architecture

### Why is the framework split into multiple crates?

The multi-crate structure keeps concerns separated and allows you to depend on only what you need:

| Crate | Purpose |
|---|---|
| `rust-boot-core` | Core traits (CrudPlugin, Repository, Service, Error types) |
| `rust-boot-plugins` | Built-in plugins (auth, cache, monitoring, events) |
| `rust-boot-axum` | Axum integration (router, handlers) |
| `rust-boot` | Umbrella crate that re-exports everything |

If you only need the plugin system without Axum, depend on `rust-boot-core` directly.

### Can I use rust-boot without Axum?

Yes. The core plugin system, repository traits, and service layer are all in `rust-boot-core`, which has no dependency on Axum. You can use the plugin system with any HTTP framework or even in non-web applications.

### How does error handling work?

rust-boot uses a unified `RustBootError` enum that covers all error categories (validation, not found, authentication, internal, cache, etc.). All fallible operations return `Result<T, RustBootError>`. The Axum integration automatically converts these errors into appropriate HTTP status codes.

See [Error Handling](./core/error-handling.md) for details.

## Performance

### Is the plugin context a bottleneck?

The `PluginContext` uses `Arc<RwLock<HashMap<...>>>`. For typical usage (reading shared state in request handlers), the read lock is very cheap — multiple readers can hold it simultaneously. Write operations (which only happen during plugin initialization and cleanup) take an exclusive lock.

For hot-path data, retrieve the `Arc<T>` from the context once at startup and clone it into your handlers, rather than calling `ctx.get()` on every request.

### How does the Moka cache perform?

Moka is one of the fastest concurrent caches available in Rust. It uses a lock-free concurrent hash map with O(1) reads and writes. For most applications, cache operations complete in single-digit microseconds.

The cache handles eviction automatically based on your `max_capacity` and `default_ttl` settings. Expired entries are cleaned up lazily on access and periodically in the background.

### Should I use Moka or Redis for caching?

| Scenario | Recommendation |
|---|---|
| Single instance, low latency needed | Moka (in-memory) |
| Multiple instances, shared cache | Redis (distributed) |
| Development and testing | Moka (no external dependencies) |
| Cache persistence across restarts | Redis |
| Sub-microsecond access times | Moka |

You can start with Moka and switch to Redis later without changing application code — just swap the backend at plugin registration time.

## Troubleshooting

### My health checks always return healthy

If you haven't registered any health checks with the `MonitoringPlugin`, `check_health()` returns `Healthy` by default (no checks = nothing unhealthy). Make sure you're adding checks:

```rust
let plugin = MonitoringPlugin::new(MetricsConfig::new())
    .with_health_check(my_db_check)
    .with_health_check(my_cache_check);
```

### Events fail with "Version conflict"

The `InMemoryEventStore` enforces strict sequential versioning. Each event's version must be exactly `previous_version + 1`. If you're getting version conflicts:

- Make sure you're reading the latest version before appending: `store.get_latest_version(id).await?`
- Don't reuse version numbers
- In concurrent scenarios, implement optimistic concurrency control (read version, append, retry on conflict)

### The plugin registry panics on shutdown

Make sure you call the shutdown methods in order:

```rust
registry.finish_all().await?;  // First: complete pending work
registry.cleanup_all().await?; // Then: release resources
```

Calling `cleanup_all()` without `finish_all()` may cause issues if plugins expect the finish phase to run first.
