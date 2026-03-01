# Plugin Lifecycle

The plugin system is the heart of rust-boot's extensibility. Every cross-cutting concern — authentication, caching, monitoring, event sourcing — is implemented as a plugin that follows a well-defined lifecycle. This page provides a deep dive into the state machine, metadata system, shared context, the `CrudPlugin` trait, and the `PluginRegistry` that orchestrates everything.

## PluginState

Every plugin moves through exactly four states during its lifetime. The `PluginState` enum models this as a linear state machine:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginState {
    Adding,
    Ready,
    Finished,
    Cleaned,
}
```

The states represent:

- **Adding** — The plugin has been registered with the registry but has not been initialized. This is the initial state after calling `registry.register(plugin)`.
- **Ready** — The plugin has been built and is ready to serve requests. The registry transitions plugins to this state by calling `build()` then `ready()` during `init_all()`.
- **Finished** — The plugin has been told to stop serving. The registry calls `finish()` during `finish_all()`. The plugin should flush buffers and stop accepting new work.
- **Cleaned** — The plugin has released all resources. The registry calls `cleanup()` during `cleanup_all()`. After this, the plugin is fully torn down.

### State Transitions

The `PluginState` enum provides two methods for managing transitions:

```rust
impl PluginState {
    /// Returns the next state in the lifecycle, or None if already at the end.
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Adding => Some(Self::Ready),
            Self::Ready => Some(Self::Finished),
            Self::Finished => Some(Self::Cleaned),
            Self::Cleaned => None,
        }
    }

    /// Returns true if transitioning from the current state to `target` is valid.
    pub fn can_transition_to(&self, target: Self) -> bool {
        match (self, target) {
            (Self::Adding, Self::Ready) => true,
            (Self::Ready, Self::Finished) => true,
            (Self::Finished, Self::Cleaned) => true,
            _ => false,
        }
    }
}
```

Only forward transitions are allowed. A plugin in the `Ready` state can move to `Finished`, but never back to `Adding`. This ensures that the lifecycle is predictable and that resources are properly managed.

```
Adding ──► Ready ──► Finished ──► Cleaned
```

Each state also implements `Display`, producing human-readable names like `"Adding"`, `"Ready"`, `"Finished"`, and `"Cleaned"`.

## PluginMeta

Every plugin declares its identity and dependencies through `PluginMeta`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<String>,
}
```

- `name` — A unique identifier for the plugin (e.g., `"auth"`, `"cache"`, `"monitoring"`).
- `version` — A version string (e.g., `"0.1.0"`).
- `dependencies` — A list of plugin names that must be initialized before this plugin. The registry uses this to determine initialization order.

### Constructors and Builder

```rust
// Simple plugin with no dependencies
let meta = PluginMeta::new("auth", "0.1.0");

// Plugin with dependencies specified upfront
let meta = PluginMeta::with_dependencies(
    "api-handler",
    "0.1.0",
    vec!["auth".to_string(), "cache".to_string()],
);

// Builder pattern for adding dependencies one at a time
let meta = PluginMeta::new("api-handler", "0.1.0")
    .with_dependency("auth")
    .with_dependency("cache");
```

The `with_dependency()` method returns `self`, enabling fluent chaining. Dependencies are stored as plain strings that must match the `name` field of other registered plugins.

## PluginContext

The `PluginContext` is a thread-safe, type-erased key-value store that plugins use to share state with each other. It is the primary mechanism for inter-plugin communication.

```rust
#[derive(Debug, Clone)]
pub struct PluginContext {
    state: Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>,
}
```

The inner storage is wrapped in `Arc<RwLock<...>>`, making it safe to clone and share across threads. Values are stored as `Box<dyn Any + Send + Sync>`, which means any type that is `Send + Sync + 'static` can be stored.

### Methods

All methods are async because they acquire the `RwLock`:

```rust
// Insert a value (overwrites if key exists)
ctx.insert("jwt_manager", Arc::new(jwt_manager)).await;

// Get a cloned value (requires T: Clone)
let manager: Option<Arc<JwtManager>> = ctx.get("jwt_manager").await;

// Remove a value and return it
let removed: Option<Arc<JwtManager>> = ctx.remove("jwt_manager").await;

// Check if a key exists
let exists: bool = ctx.contains("jwt_manager").await;

// Get the number of entries
let count: usize = ctx.len().await;

// Check if empty
let empty: bool = ctx.is_empty().await;

// Remove all entries
ctx.clear().await;
```

The `get()` method uses `downcast_ref::<T>()` internally and clones the value if the type matches. If the key doesn't exist or the type doesn't match, it returns `None`.

### Usage Pattern

Plugins typically insert their resources during `build()` and remove them during `cleanup()`:

```rust
#[async_trait]
impl CrudPlugin for AuthPlugin {
    async fn build(&mut self, ctx: &mut PluginContext) -> Result<()> {
        let manager = Arc::new(JwtManager::new(&self.config));
        self.manager = Some(manager.clone());
        ctx.insert("jwt_manager", manager).await;
        Ok(())
    }

    async fn cleanup(&mut self, ctx: &mut PluginContext) -> Result<()> {
        self.manager = None;
        let _: Option<Arc<JwtManager>> = ctx.remove("jwt_manager").await;
        Ok(())
    }
}
```

Other plugins or application code can then retrieve the `JwtManager` from the context:

```rust
let manager: Arc<JwtManager> = ctx
    .get::<Arc<JwtManager>>("jwt_manager")
    .await
    .expect("AuthPlugin must be initialized first");
```

## CrudPlugin Trait

The `CrudPlugin` trait is the interface that all plugins must implement:

```rust
#[async_trait]
pub trait CrudPlugin: Send + Sync {
    /// Returns the plugin's metadata (name, version, dependencies).
    fn meta(&self) -> PluginMeta;

    /// Called during initialization. Create resources and store them in the context.
    async fn build(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called after build(). Perform final setup (e.g., install metrics recorder).
    async fn ready(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called during shutdown. Stop accepting new work, flush buffers.
    async fn finish(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    /// Called after finish(). Release all resources, remove context entries.
    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }
}
```

The `meta()` method is the only required method — it has no default implementation. All lifecycle hooks (`build`, `ready`, `finish`, `cleanup`) default to no-ops, so plugins only need to override the hooks they care about.

The trait requires `Send + Sync` because plugins are stored in the registry and may be accessed from multiple threads.

### Example: Implementing a Custom Plugin

```rust
use rust_boot_core::plugin::{CrudPlugin, PluginMeta, PluginContext};
use rust_boot_core::error::Result;

pub struct MyPlugin {
    config: MyConfig,
    resource: Option<Arc<MyResource>>,
}

impl MyPlugin {
    pub fn new(config: MyConfig) -> Self {
        Self {
            config,
            resource: None,
        }
    }
}

#[async_trait]
impl CrudPlugin for MyPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("my-plugin", "0.1.0")
            .with_dependency("cache") // depends on cache plugin
    }

    async fn build(&mut self, ctx: &mut PluginContext) -> Result<()> {
        // Create the resource using config
        let resource = Arc::new(MyResource::new(&self.config));
        self.resource = Some(resource.clone());

        // Share it via context so other plugins can access it
        ctx.insert("my_resource", resource).await;
        Ok(())
    }

    async fn cleanup(&mut self, ctx: &mut PluginContext) -> Result<()> {
        // Release the resource
        self.resource = None;
        let _: Option<Arc<MyResource>> = ctx.remove("my_resource").await;
        Ok(())
    }
}
```

## PluginRegistry

The `PluginRegistry` is the orchestrator that manages plugin registration, dependency resolution, and lifecycle execution.

```rust
pub struct PluginRegistry {
    plugins: HashMap<String, PluginEntry>,
    context: PluginContext,
}
```

Each `PluginEntry` contains the plugin instance, its current `PluginState`, and its `PluginMeta`.

### Registration

```rust
let mut registry = PluginRegistry::new();

// Register plugins in any order — dependencies are resolved automatically
registry.register(CachingPlugin::new(CacheConfig::default()))?;
registry.register(AuthPlugin::new(JwtConfig::new("secret")))?;
registry.register(MonitoringPlugin::new(MetricsConfig::default()))?;
```

During `register()`, the registry:
1. Calls `plugin.meta()` to get the plugin's name and dependencies.
2. Checks that no plugin with the same name is already registered.
3. Stores the plugin with an initial state of `Adding`.

### Dependency Resolution

The registry uses **Kahn's algorithm** for topological sorting to determine the correct initialization order. If plugin A depends on plugin B, then B will be initialized before A.

```rust
// Internal method — called automatically by init_all()
fn topological_order(&self) -> Result<Vec<String>> {
    // 1. Build in-degree map from dependency graph
    // 2. Start with plugins that have zero in-degree (no dependencies)
    // 3. Process queue: for each plugin, decrement in-degree of dependents
    // 4. If all plugins are processed, return the order
    // 5. If not, a circular dependency exists — return error
}
```

### Circular Dependency Detection

The registry also provides DFS-based cycle detection:

```rust
// Check if any circular dependencies exist
if registry.has_circular_dependency() {
    // The detect_cycle() method returns the cycle path
    // e.g., ["A", "B", "C", "A"] means A → B → C → A
}
```

This uses a standard three-color DFS algorithm (white/gray/black) to detect back edges in the dependency graph.

### Lifecycle Orchestration

The registry provides four async methods that drive all plugins through their lifecycle:

```rust
// Initialize all plugins: build() then ready() in dependency order
registry.init_all().await?;

// Mark all plugins as ready (called separately if needed)
registry.ready_all().await?;

// Shut down all plugins: finish() in REVERSE dependency order
registry.finish_all().await?;

// Clean up all plugins: cleanup() in REVERSE dependency order
registry.cleanup_all().await?;
```

The forward methods (`init_all`, `ready_all`) process plugins in topological order so that dependencies are satisfied before dependents. The reverse methods (`finish_all`, `cleanup_all`) process in reverse order so that dependents are torn down before their dependencies.

### Querying the Registry

```rust
// Get a reference to a plugin by name
let auth: Option<&dyn CrudPlugin> = registry.get("auth");

// Get a plugin's current state
let state: Option<PluginState> = registry.get_state("auth");

// List all registered plugin names
let names: Vec<String> = registry.plugin_names();

// Count registered plugins
let count: usize = registry.len();
let empty: bool = registry.is_empty();

// Access the shared context
let ctx: &PluginContext = registry.context();
let ctx_mut: &mut PluginContext = registry.context_mut();
```

### Full Lifecycle Example

```rust
use rust_boot_core::plugin::PluginContext;
use rust_boot_core::registry::PluginRegistry;

#[tokio::main]
async fn main() -> rust_boot_core::error::Result<()> {
    // 1. Create registry
    let mut registry = PluginRegistry::new();

    // 2. Register plugins (order doesn't matter)
    registry.register(CachingPlugin::new(CacheConfig::default()))?;
    registry.register(MonitoringPlugin::new(MetricsConfig::default()))?;
    registry.register(AuthPlugin::new(JwtConfig::new("my-secret")))?;

    // 3. Initialize all plugins (resolves dependencies, calls build + ready)
    registry.init_all().await?;

    // ... application runs, serves requests ...

    // 4. Graceful shutdown
    registry.finish_all().await?;
    registry.cleanup_all().await?;

    Ok(())
}
```

The registry ensures that if `AuthPlugin` depends on `CachingPlugin`, the cache is always built before auth and cleaned up after auth.
