# API Reference

Complete type catalog for the rust-boot framework, organized by crate. All public types listed here are accessible through `rust_boot::prelude::*` unless noted otherwise.

---

## rust-boot-core

The core crate provides configuration, the plugin system, repository traits, service abstractions, and error handling.

### Configuration (`rust_boot::core::config`)

#### `RustBootConfig`

Main application configuration. Holds server, database, and plugin settings.

```rust
pub struct RustBootConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub plugins: HashMap<String, serde_json::Value>,
}
```

| Method | Description |
|--------|-------------|
| `RustBootConfig::default()` | Default config (localhost:3000, SQLite in-memory) |
| `RustBootConfig::builder()` | Returns a `RustBootConfigBuilder` |
| `RustBootConfig::from_file(path)` | Load from TOML/YAML/JSON with env overrides |
| `RustBootConfig::from_env()` | Load from defaults + environment variables |

#### `RustBootConfigBuilder`

Fluent builder for `RustBootConfig`.

| Method | Description |
|--------|-------------|
| `.server_host(String)` | Set server bind address |
| `.server_port(u16)` | Set server port |
| `.database_url(String)` | Set database connection URL |
| `.database_max_connections(u32)` | Set max pool size |
| `.database_min_connections(u32)` | Set min pool size |
| `.plugin(String, Value)` | Add plugin-specific config |
| `.build()` | Build the `RustBootConfig` |

#### `ServerConfig`

```rust
pub struct ServerConfig {
    pub host: String,  // default: "127.0.0.1"
    pub port: u16,     // default: 3000
}
```

#### `DatabaseConfig`

```rust
pub struct DatabaseConfig {
    pub url: String,            // default: "sqlite::memory:"
    pub max_connections: u32,   // default: 10
    pub min_connections: u32,   // default: 1
}
```

#### `ConfigError`

```rust
pub enum ConfigError {
    FileReadError(String),
    ParseError(String),
    InvalidFileFormat,
}
```

### Plugin System (`rust_boot::core::plugin`)

#### `CrudPlugin` (trait)

The core trait for all plugins. Requires `async_trait`.

```rust
#[async_trait]
pub trait CrudPlugin: Send + Sync {
    fn meta(&self) -> PluginMeta;
    async fn build(&mut self, ctx: &mut PluginContext) -> Result<()>;
    async fn ready(&mut self, ctx: &mut PluginContext) -> Result<()>;
    async fn finish(&mut self, ctx: &mut PluginContext) -> Result<()>;
    async fn cleanup(&mut self, ctx: &mut PluginContext) -> Result<()>;
}
```

#### `PluginMeta`

Plugin metadata including name, version, and dependencies.

| Method | Description |
|--------|-------------|
| `PluginMeta::new(name, version)` | Create with name and version |
| `.with_dependency(name)` | Declare a dependency on another plugin |

#### `PluginContext`

Async key-value store for sharing state between plugins.

| Method | Description |
|--------|-------------|
| `ctx.insert(key, Arc<T>).await` | Store a value |
| `ctx.get::<T>(key).await` | Retrieve a value (returns `Option<Arc<T>>`) |

#### `PluginState`

Enum representing the current state of a plugin in its lifecycle.

### Registry (`rust_boot::core::registry`)

#### `PluginRegistry`

Manages plugin registration and lifecycle execution.

| Method | Description |
|--------|-------------|
| `PluginRegistry::new()` | Create empty registry |
| `.register(plugin)` | Register a plugin (validates dependencies) |
| `.init_all().await` | Call `build()` on all plugins in dependency order |
| `.ready_all().await` | Call `ready()` on all plugins |
| `.finish_all().await` | Call `finish()` in reverse dependency order |
| `.cleanup_all().await` | Call `cleanup()` in reverse dependency order |
| `.context()` | Access the shared `PluginContext` |

### Repository (`rust_boot::core::repository`)

#### `CrudRepository` (trait)

Generic CRUD repository for entity persistence.

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

#### `DatabaseConnection` (trait)

```rust
#[async_trait]
pub trait DatabaseConnection: Send + Sync {
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>>;
    async fn execute(&self, query: &str) -> Result<u64>;
    fn is_connected(&self) -> bool;
}
```

#### `Transaction` (trait)

```rust
#[async_trait]
pub trait Transaction: Send + Sync {
    async fn commit(self: Box<Self>) -> Result<()>;
    async fn rollback(self: Box<Self>) -> Result<()>;
}
```

### Service Layer (`rust_boot::core::service`)

#### `CrudService` (trait)

Service layer with pagination, soft delete, and DTO support.

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

#### `PaginationParams`

```rust
pub struct PaginationParams {
    pub page: u64,      // default: 1
    pub per_page: u64,  // default: 20
}
```

| Method | Description |
|--------|-------------|
| `PaginationParams::new(page, per_page)` | Create with explicit values |
| `.offset()` | Calculate DB offset: `(page - 1) * per_page` |
| `.limit()` | Returns `per_page` |

#### `PaginatedResult<T>`

```rust
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
    pub total_pages: u64,
}
```

| Method | Description |
|--------|-------------|
| `.has_next_page()` | `page < total_pages` |
| `.has_prev_page()` | `page > 1` |
| `.is_empty()` | No items on this page |
| `.len()` | Number of items on this page |

#### `SortParams` / `SortDirection`

```rust
pub struct SortParams {
    pub field: String,
    pub direction: SortDirection,
}

pub enum SortDirection { Asc, Desc }
```

| Method | Description |
|--------|-------------|
| `SortParams::asc(field)` | Ascending sort |
| `SortParams::desc(field)` | Descending sort |

#### `Filter` (trait) and `FilterOp`

```rust
pub trait Filter: Send + Sync {
    fn apply(&self, field: &str) -> Option<FilterOp>;
    fn fields(&self) -> Vec<String>;
}

pub enum FilterOp {
    Eq(String), Ne(String), Gt(String), Lt(String),
    Gte(String), Lte(String), Like(String),
    In(Vec<String>), IsNull, IsNotNull,
}
```

`NoFilter` — a built-in empty filter that matches everything.

### Error Handling (`rust_boot::core::error`)

#### `RustBootError`

The framework's error type. Variants include `Database(String)` and others.

#### `Result<T>`

Type alias: `type Result<T> = std::result::Result<T, RustBootError>;`

Re-exported as `RustBootResult<T>` in the prelude.

---

## rust-boot-axum

Axum integration layer providing routers, response types, and pagination.

### Router (`rust_boot::axum`)

#### `CrudRouterBuilder<S>`

Fluent builder for CRUD route sets. Generic over the application state type `S`.

| Method | Description |
|--------|-------------|
| `CrudRouterBuilder::<S>::new(config)` | Create with a `CrudRouterConfig` |
| `.list(handler)` | `GET /base_path` |
| `.get(handler)` | `GET /base_path/:id` |
| `.create(handler)` | `POST /base_path` |
| `.update(handler)` | `PUT /base_path/:id` |
| `.delete(handler)` | `DELETE /base_path/:id` |
| `.build()` | Build the `axum::Router<S>` |

#### `CrudRouterConfig`

```rust
pub struct CrudRouterConfig {
    pub base_path: String,
    pub enable_soft_delete: bool,
    pub enable_delete: bool,
}
```

| Method | Description |
|--------|-------------|
| `CrudRouterConfig::new(path)` | Create with base path |
| `.with_soft_delete()` | Enable soft delete support |
| `.disable_delete()` | Remove the DELETE endpoint |

### Response Types

#### `ApiResponse<T>`

Standard JSON response envelope with `success`, `data`, and optional `error` fields.

#### `ApiResult<T>`

Type alias for `axum::Json<ApiResponse<T>>`.

#### `PaginatedResponse<T>` / `PaginatedResult<T>`

Paginated response wrapper with items and page metadata.

#### `ApiError`

Error type that integrates with Axum's error handling.

### Response Helpers

| Function | Returns | HTTP Status |
|----------|---------|-------------|
| `ok(value)` | `ApiResult<T>` | 200 OK |
| `created(value)` | `(StatusCode, Json<ApiResponse<T>>)` | 201 Created |
| `no_content()` | `StatusCode` | 204 No Content |
| `paginated(items, page, per_page, total)` | `PaginatedResult<T>` | 200 OK |

### Query Types

#### `PaginationQuery`

Axum query extractor for pagination parameters. Parses `?page=1&per_page=20` from the URL.

### Traits

#### `CrudHandlers` / `CrudRouter`

Traits for implementing CRUD handler sets and routers.

---

## rust-boot-plugins

Built-in plugins for caching, authentication, monitoring, and event sourcing.

### Caching

#### `CachingPlugin`

Plugin that provides cache backend management.

```rust
let plugin = CachingPlugin::new(CacheConfig::default());
```

#### `CacheConfig`

```rust
pub struct CacheConfig {
    pub name: String,
    pub default_ttl: Duration,
    pub max_capacity: u64,
}
```

| Method | Description |
|--------|-------------|
| `CacheConfig::new(name)` | Create with a cache name |
| `.with_ttl(duration)` | Set default TTL |
| `.with_max_capacity(n)` | Set max entries |

#### `CacheBackend` (trait)

```rust
#[async_trait]
pub trait CacheBackend: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()>;
    // ... delete, exists, clear
}
```

#### `MokaBackend`

In-memory cache using the [Moka](https://github.com/moka-rs/moka) library.

```rust
let cache = MokaBackend::new(CacheConfig::new("my-cache").with_max_capacity(10_000));
```

#### `RedisBackend`

Redis-backed cache implementation.

### Authentication

#### `AuthPlugin`

Plugin that provides JWT authentication.

```rust
let plugin = AuthPlugin::new(JwtConfig::new("secret"));
```

#### `JwtConfig`

| Method | Description |
|--------|-------------|
| `JwtConfig::new(secret)` | Create with signing secret |
| `.with_access_token_ttl(duration)` | Set access token lifetime |
| `.with_refresh_token_ttl(duration)` | Set refresh token lifetime |
| `.with_issuer(issuer)` | Set token issuer claim |
| `.with_audience(audience)` | Set token audience claim |

#### `JwtManager`

| Method | Description |
|--------|-------------|
| `JwtManager::new(config)` | Create from config |
| `.create_access_token(claims)` | Generate an access token |
| `.create_refresh_token(claims)` | Generate a refresh token |
| `.verify_access_token(token)` | Verify and decode an access token |
| `.refresh_tokens(refresh_token)` | Exchange refresh token for new token pair |

#### `Claims`

JWT claims payload.

| Method | Description |
|--------|-------------|
| `Claims::new(sub, iat, exp)` | Create with subject and timestamps |
| `.with_role(role)` | Add a role |
| `.with_email(email)` | Set email claim |
| `.with_name(name)` | Set name claim |
| `.has_role(role)` | Check if claims include a role |

Fields: `sub`, `email` (`Option<String>`), `name` (`Option<String>`), `roles`.

#### `Role`

| Method | Description |
|--------|-------------|
| `Role::admin()` | Built-in admin role |
| `Role::user()` | Built-in user role |
| `Role::custom(name)` | Custom named role |

### Monitoring

#### `MonitoringPlugin`

Provides Prometheus metrics and health checks.

```rust
let plugin = MonitoringPlugin::new(MetricsConfig::default());
```

#### `MetricsConfig`

Configuration for the monitoring plugin. Use `MetricsConfig::default()` for standard settings.

#### `MetricsRecorder`

Records application metrics (request counts, latencies, etc.).

#### `HealthCheck` / `HealthStatus` / `ReadinessCheck`

Health check traits and status types for liveness and readiness probes.

### Event Sourcing

#### `EventSourcingPlugin`

Plugin for domain event capture and replay.

#### `DomainEvent` / `EventEnvelope` / `EventMetadata`

Types for modeling domain events with metadata envelopes.

#### `EventStore` (trait) / `InMemoryEventStore`

Event persistence abstraction and its in-memory implementation.

---

## rust-boot-macros

#### `#[derive(CrudModel)]`

Procedural macro that generates SeaORM entities, DTOs, and OpenAPI schemas. See the [CrudModel Macro Reference](./crud-model-macro.md) for full documentation of attributes and generated code.

---

## Prelude Re-exports

`use rust_boot::prelude::*` imports all of the following:

**From rust-boot-core:**
`DatabaseConfig`, `RustBootConfig`, `RustBootConfigBuilder`, `ServerConfig`, `RustBootError`, `RustBootResult`, `CrudPlugin`, `PluginContext`, `PluginMeta`, `PluginState`, `PluginRegistry`, `CrudRepository`, `CrudService`

**From rust-boot-axum:**
`created`, `no_content`, `ok`, `paginated`, `ApiError`, `ApiResponse`, `ApiResult`, `CrudHandlers`, `CrudRouter`, `CrudRouterBuilder`, `CrudRouterConfig`, `PaginatedResponse`, `PaginatedResult`, `PaginationQuery`

**From rust-boot-plugins:**
`AuthPlugin`, `CacheBackend`, `CacheConfig`, `CachingPlugin`, `Claims`, `DomainEvent`, `EventEnvelope`, `EventMetadata`, `EventSourcingPlugin`, `EventStore`, `HealthCheck`, `HealthStatus`, `InMemoryEventStore`, `JwtConfig`, `JwtManager`, `MetricsConfig`, `MetricsRecorder`, `MokaBackend`, `MonitoringPlugin`, `ReadinessCheck`, `RedisBackend`, `Role`

**From external crates:**
`async_trait`, `Serialize`, `Deserialize` (serde), `Uuid`
