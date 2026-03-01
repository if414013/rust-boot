# Core Overview

The `rust-boot-core` crate is the foundation of the entire framework. It defines the plugin system, configuration management, error handling, and the repository and service abstractions that all other crates build upon. If rust-boot were a building, `rust-boot-core` would be the foundation, structural beams, and plumbing — everything else is built on top of it.

## What Lives in Core

The crate is organized into six modules:

```
rust-boot-core/src/
├── lib.rs          # Module declarations
├── plugin.rs       # CrudPlugin trait, PluginState, PluginMeta, PluginContext
├── registry.rs     # PluginRegistry with dependency resolution
├── config.rs       # RustBootConfig, ServerConfig, DatabaseConfig
├── error.rs        # RustBootError enum and Result type alias
├── repository.rs   # CrudRepository, Transaction, DatabaseConnection traits
└── service.rs      # CrudService trait, pagination, filtering, sorting
```

Each module has a clear responsibility and minimal coupling to the others. The plugin system (`plugin.rs` + `registry.rs`) is the most important piece — it is the mechanism through which all framework capabilities are composed.

## Plugin System

The plugin system is covered in depth on the [Plugin Lifecycle](../architecture/plugin-lifecycle.md) page. Here is a brief summary of the key types:

- `CrudPlugin` — The trait that all plugins implement. Provides lifecycle hooks: `meta()`, `build()`, `ready()`, `finish()`, `cleanup()`.
- `PluginState` — Enum representing the four lifecycle states: `Adding` → `Ready` → `Finished` → `Cleaned`.
- `PluginMeta` — Metadata struct with `name`, `version`, and `dependencies`.
- `PluginContext` — Thread-safe key-value store (`Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>`) for inter-plugin communication.
- `PluginRegistry` — Manages registration, topological sorting of dependencies, and lifecycle orchestration.

## Configuration

The configuration system provides layered configuration with three levels of precedence (highest wins):

1. Environment variables (`RUST_BOOT_*` prefix)
2. Configuration file (TOML, YAML, or JSON)
3. Default values

### RustBootConfig

The top-level configuration struct:

```rust
pub struct RustBootConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub plugins: HashMap<String, serde_json::Value>,
}
```

- `ServerConfig` — `host` (default `"127.0.0.1"`) and `port` (default `3000`).
- `DatabaseConfig` — `url` (default `"sqlite::memory:"`), `max_connections` (default `10`), `min_connections` (default `1`).
- `plugins` — A flexible map for plugin-specific configuration that doesn't fit into the typed fields.

### Loading Configuration

```rust
// From defaults
let config = RustBootConfig::default();

// From a file (TOML, YAML, or JSON) with env overrides
let config = RustBootConfig::from_file("config.toml")?;

// From environment variables only
let config = RustBootConfig::from_env();

// Programmatically via builder
let config = RustBootConfig::builder()
    .server_host("0.0.0.0".to_string())
    .server_port(8080)
    .database_url("postgresql://localhost/mydb".to_string())
    .database_max_connections(25)
    .build();
```

### Environment Variable Mapping

| Environment Variable | Config Field |
|---|---|
| `RUST_BOOT_SERVER_HOST` | `server.host` |
| `RUST_BOOT_SERVER_PORT` | `server.port` |
| `RUST_BOOT_DATABASE_URL` | `database.url` |
| `RUST_BOOT_DATABASE_MAX_CONNECTIONS` | `database.max_connections` |
| `RUST_BOOT_DATABASE_MIN_CONNECTIONS` | `database.min_connections` |

Invalid values (e.g., a non-numeric port) are silently ignored, and the default or file-based value is used instead.

## Error Handling

The `RustBootError` enum provides a unified error type across the entire framework:

```rust
pub enum RustBootError {
    Config(String),
    Database(String),
    Plugin(String),
    Validation(String),
    Serialization(String),
    Http(u16, String),
    Cache(String),
    Auth(String),
    Internal(String),
}
```

Each variant carries a descriptive message string. The `Http` variant also carries a status code.

### Result Type Alias

```rust
pub type Result<T> = std::result::Result<T, RustBootError>;
```

This alias is used throughout the framework. Import it as `RustBootResult` from the prelude to avoid conflicts with `std::result::Result`:

```rust
use rust_boot::prelude::*;

async fn my_function() -> RustBootResult<()> {
    // ...
    Ok(())
}
```

### Automatic Conversions

`RustBootError` implements `From` for common error types, enabling the `?` operator:

| Source Type | Maps To |
|---|---|
| `std::io::Error` | `RustBootError::Database` |
| `serde_json::Error` | `RustBootError::Serialization` |
| `config::ConfigError` | `RustBootError::Config` |

## Repository Layer

The repository module defines traits for database access following the Repository pattern. This decouples your business logic from the specific database implementation.

### CrudRepository

```rust
#[async_trait]
pub trait CrudRepository: Send + Sync {
    type Entity: Send + Sync;
    type Id: Send + Sync;
    type Connection: DatabaseConnection;

    fn connection(&self) -> &Self::Connection;
    async fn insert(&self, entity: &Self::Entity) -> Result<Self::Entity>;
    async fn find_by_id(&self, id: Self::Id) -> Result<Option<Self::Entity>>;
    async fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Self::Entity>>;
    async fn find_with_filter(&self, filter: &dyn Filter, limit: usize, offset: usize) -> Result<Vec<Self::Entity>>;
    async fn update(&self, entity: &Self::Entity) -> Result<Self::Entity>;
    async fn delete(&self, id: Self::Id) -> Result<()>;
    async fn count(&self) -> Result<u64>;
    async fn count_with_filter(&self, filter: &dyn Filter) -> Result<u64>;
    async fn exists(&self, id: Self::Id) -> Result<bool>;
}
```

The three associated types let you parameterize the repository for any entity, ID type, and database connection.

### Transaction Support

```rust
#[async_trait]
pub trait Transaction: Send + Sync {
    async fn commit(self: Box<Self>) -> Result<()>;
    async fn rollback(self: Box<Self>) -> Result<()>;
}

#[async_trait]
pub trait DatabaseConnection: Send + Sync {
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>>;
    async fn execute(&self, query: &str) -> Result<u64>;
    fn is_connected(&self) -> bool;
}
```

Transactions consume `Box<Self>` on commit/rollback, ensuring they can only be finalized once. This is a Rust idiom that prevents accidental double-commit or use-after-commit bugs.

## Service Layer

The service module sits between your handlers and repositories, providing business logic with built-in pagination and soft-delete support.

### CrudService

```rust
#[async_trait]
pub trait CrudService: Send + Sync {
    type Entity: Send + Sync;
    type Id: Send + Sync;
    type CreateDto: Send + Sync;
    type UpdateDto: Send + Sync;

    async fn create(&self, dto: Self::CreateDto) -> Result<Self::Entity>;
    async fn find_by_id(&self, id: Self::Id) -> Result<Option<Self::Entity>>;
    async fn find_all(&self, pagination: PaginationParams) -> Result<PaginatedResult<Self::Entity>>;
    async fn find_with_filter(&self, filter: &dyn Filter, pagination: PaginationParams) -> Result<PaginatedResult<Self::Entity>>;
    async fn update(&self, id: Self::Id, dto: Self::UpdateDto) -> Result<Self::Entity>;
    async fn delete(&self, id: Self::Id) -> Result<()>;
    async fn hard_delete(&self, id: Self::Id) -> Result<()>;
    async fn restore(&self, id: Self::Id) -> Result<Self::Entity>;
    async fn find_all_including_deleted(&self, pagination: PaginationParams) -> Result<PaginatedResult<Self::Entity>>;
}
```

The service trait uses DTOs (`CreateDto`, `UpdateDto`) rather than raw entities for create and update operations. This separation ensures that clients cannot set fields like `id` or `created_at` directly.

### Pagination

```rust
// Create pagination parameters
let params = PaginationParams::new(2, 20); // page 2, 20 items per page
let offset = params.offset(); // 20
let limit = params.limit();   // 20

// Paginated results include metadata
let result = PaginatedResult::new(items, total_count, params);
assert!(result.has_next_page());
assert!(result.has_prev_page());
```

`PaginationParams` defaults to page 1 with 20 items per page. The `offset()` method handles the math for database queries.

### Filtering

The `Filter` trait and `FilterOp` enum provide a database-agnostic way to express query conditions:

```rust
pub enum FilterOp {
    Eq(String),       // field = value
    Ne(String),       // field != value
    Gt(String),       // field > value
    Lt(String),       // field < value
    Gte(String),      // field >= value
    Lte(String),      // field <= value
    Like(String),     // field LIKE pattern
    In(Vec<String>),  // field IN (values)
    IsNull,           // field IS NULL
    IsNotNull,        // field IS NOT NULL
}

pub trait Filter: Send + Sync {
    fn apply(&self, field: &str) -> Option<FilterOp>;
    fn fields(&self) -> Vec<String>;
}
```

The `NoFilter` struct is a default implementation that matches everything (returns `None` for all fields).

### Sorting

```rust
let sort = SortParams::asc("name");        // ORDER BY name ASC
let sort = SortParams::desc("created_at"); // ORDER BY created_at DESC
```

`SortDirection` defaults to `Asc`.

## How Core Relates to Other Crates

Every other crate in the workspace depends on `rust-boot-core`:

- `rust-boot-axum` depends on core for `CrudService`, `Filter`, and error types.
- `rust-boot-plugins` depends on core for `CrudPlugin`, `PluginMeta`, `PluginContext`, and error types.
- `rust-boot` (facade) re-exports everything from core through its `prelude` module.

The `rust-boot-macros` and `rust-boot-cli` crates are standalone tools that generate code targeting core's traits and types, but they don't have a runtime dependency on core.

## What's Next

- [Configuration](./configuration.md) — Deep dive into the configuration system.
- [Error Handling](./error-handling.md) — Complete reference for `RustBootError` and error conversion.
- [Repository](./repository.md) — Implementing the `CrudRepository` trait for your database.
- [Service](./service.md) — Building service layers with pagination and filtering.
