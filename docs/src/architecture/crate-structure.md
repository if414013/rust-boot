# Crate Structure

The rust-boot workspace contains six crates. This page describes each crate's purpose, its public modules, and the key types it exports.

## rust-boot (facade crate)

The top-level `rust-boot` crate is a thin facade that re-exports types from all sub-crates through a unified `prelude` module. Application code typically only needs a single import:

```rust
use rust_boot::prelude::*;
```

This gives access to the full framework API — plugin system, configuration, error types, Axum integration, and all built-in plugins — without needing to depend on individual sub-crates.

The facade crate depends on `rust-boot-core`, `rust-boot-axum`, and `rust-boot-plugins`. It does not contain any logic of its own.

## rust-boot-core

The foundational crate that defines the plugin system, configuration management, error handling, repository abstractions, and service layer. Every other feature crate depends on `rust-boot-core`.

### Modules

| Module | Description |
|--------|-------------|
| `plugin` | Plugin system: `CrudPlugin` trait, `PluginState`, `PluginMeta`, `PluginContext` |
| `registry` | `PluginRegistry` with dependency resolution and lifecycle management |
| `config` | `RustBootConfig`, `ServerConfig`, `DatabaseConfig`, `RustBootConfigBuilder`, `ConfigError` |
| `error` | `RustBootError` enum (9 variants) and `Result<T>` type alias |
| `repository` | `CrudRepository`, `Transaction`, `DatabaseConnection` traits |
| `service` | `CrudService` trait, `PaginationParams`, `PaginatedResult`, `SortParams`, `SortDirection`, `FilterOp`, `Filter`, `NoFilter` |

### Key Types

**Plugin System:**
- `CrudPlugin` — The core trait that all plugins implement. Provides lifecycle hooks: `meta()`, `build()`, `ready()`, `finish()`, `cleanup()`. All hooks except `meta()` have default no-op implementations.
- `PluginState` — Enum representing the four lifecycle states: `Adding`, `Ready`, `Finished`, `Cleaned`. Provides `next()` and `can_transition_to()` for state machine validation.
- `PluginMeta` — Metadata for a plugin: `name`, `version`, and `dependencies` (a `Vec<String>` of plugin names this plugin depends on). Supports builder pattern via `with_dependency()`.
- `PluginContext` — A thread-safe key-value store (`Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>`) shared across all plugins. Provides async `insert`, `get`, `remove`, `contains`, `len`, `is_empty`, and `clear` methods.
- `PluginRegistry` — Manages plugin registration, dependency resolution (topological sort via Kahn's algorithm), circular dependency detection (DFS), and lifecycle orchestration (`init_all`, `ready_all`, `finish_all`, `cleanup_all`).

**Configuration:**
- `RustBootConfig` — Top-level configuration containing `ServerConfig`, `DatabaseConfig`, and a `HashMap<String, serde_json::Value>` for plugin-specific config. Supports loading from file (`from_file()` — TOML, YAML, JSON) and environment variables (`from_env()` with `RUST_BOOT_*` prefix).
- `ServerConfig` — Server binding configuration with `host` (default: `"127.0.0.1"`) and `port` (default: `3000`).
- `DatabaseConfig` — Database connection configuration with `url` (default: `"sqlite::memory:"`), `max_connections` (default: `10`), and `min_connections` (default: `1`).
- `RustBootConfigBuilder` — Builder for constructing `RustBootConfig` programmatically.

**Error Handling:**
- `RustBootError` — Enum with 9 variants: `Config`, `Database`, `Plugin`, `Validation`, `Serialization`, `Http(u16, String)`, `Cache`, `Auth`, `Internal`. Implements `From` for `std::io::Error`, `serde_json::Error`, and `config::ConfigError`.
- `Result<T>` — Type alias for `std::result::Result<T, RustBootError>`.

**Repository:**
- `Transaction` — Async trait with `commit()` and `rollback()` methods that consume `Box<Self>`.
- `DatabaseConnection` — Async trait with `begin_transaction()`, `execute()`, and `is_connected()`.
- `CrudRepository` — Generic async trait with associated types `Entity`, `Id`, and `Connection`. Provides `insert`, `find_by_id`, `find_all`, `find_with_filter`, `update`, `delete`, `count`, `count_with_filter`, and `exists`.

**Service:**
- `CrudService` — Generic async trait with associated types `Entity`, `Id`, `CreateDto`, `UpdateDto`. Provides full CRUD plus soft-delete support (`hard_delete`, `restore`, `find_all_including_deleted`).
- `PaginationParams` — Value object with `page` and `per_page`. Provides `offset()` and `limit()`. Defaults: page=1, per_page=20.
- `PaginatedResult<T>` — Result container with `items`, `total`, `page`, `per_page`, `total_pages`. Provides `has_next_page()` and `has_prev_page()`.
- `FilterOp` — Enum: `Eq`, `Ne`, `Gt`, `Lt`, `Gte`, `Lte`, `Like`, `In(Vec<String>)`, `IsNull`, `IsNotNull`.
- `Filter` — Trait with `apply(field) -> Option<FilterOp>` and `fields() -> Vec<String>`.
- `NoFilter` — Default implementation that matches nothing.
- `SortDirection` — Enum: `Asc` (default), `Desc`.
- `SortParams` — Sort specification with `field` and `direction`. Factory methods: `asc()`, `desc()`.

## rust-boot-axum

Axum web framework integration providing CRUD router generation and standardized HTTP handlers.

### Modules

| Module | Description |
|--------|-------------|
| `router` | `CrudRouterBuilder`, `CrudRouterConfig`, `CrudRouter` trait, helper functions |
| `handlers` | `ApiResponse`, `ApiError`, `PaginationQuery`, response helpers, `CrudHandlers` trait |

### Key Types

**Router:**
- `CrudRouterConfig` — Configuration for CRUD route generation. Fields: `base_path`, `enable_soft_delete`, `enable_list`, `enable_get`, `enable_create`, `enable_update`, `enable_delete`. Builder methods: `with_soft_delete()`, `disable_list()`, `disable_get()`, `disable_create()`, `disable_update()`, `disable_delete()`.
- `CrudRouterBuilder<S>` — Generic builder that constructs an Axum `Router<S>`. Methods: `list()` (GET /), `get()` (GET /:id), `create()` (POST /), `update()` (PUT /:id), `delete()` (DELETE /:id), `restore()` (PATCH /:id/restore), `build()`.
- `crud_router()` — Convenience function that creates a `CrudRouterBuilder` with a base path.
- `crud_router_with_config()` — Creates a `CrudRouterBuilder` with a custom `CrudRouterConfig`.

**Handlers:**
- `ApiResponse<T>` — JSON response wrapper: `{ "data": T }`.
- `ApiError` — Structured error with `error` code, `message`, and optional `details`. Factory methods: `not_found()`, `bad_request()`, `internal_error()`, `validation_error()`, `conflict()`. Implements `IntoResponse` mapping error codes to HTTP status codes.
- `PaginationQuery` — Axum query parameters: `page` (default 1), `per_page` (default 20), `include_deleted` (default false).
- `PaginatedResponse<T>` — Paginated JSON response with `data`, `page`, `per_page`, `total`, `total_pages`.
- Response helpers: `ok(data)` returns 200, `created(data)` returns 201, `no_content()` returns 204, `paginated(data, page, per_page, total)` returns paginated 200.
- `CrudHandlers` — Async trait defining the standard CRUD handler signatures: `list`, `get`, `create`, `update`, `delete`, `restore`.

## rust-boot-plugins

A collection of ready-to-use plugins that implement the `CrudPlugin` trait from core.

### Modules

| Module | Description |
|--------|-------------|
| `auth` | JWT authentication with RBAC: `AuthPlugin`, `JwtConfig`, `JwtManager`, `Claims`, `Role` |
| `cache` | Caching abstraction: `CachingPlugin`, `CacheConfig`, `CacheBackend`, `MokaBackend`, `RedisBackend` |
| `monitoring` | Prometheus metrics and health checks: `MonitoringPlugin`, `MetricsConfig`, `MetricsRecorder`, `HealthCheck`, `HealthStatus` |
| `events` | Event sourcing: `EventSourcingPlugin`, `DomainEvent`, `EventMetadata`, `EventEnvelope`, `EventStore`, `InMemoryEventStore` |

### Key Types

**Auth:** `AuthPlugin`, `JwtConfig` (secret, TTLs, issuer, audience), `JwtManager` (create/verify/refresh tokens), `Claims` (sub, exp, iat, roles, email, name), `Role` (newtype around String with `admin()` and `user()` factories).

**Cache:** `CachingPlugin`, `CacheConfig` (default_ttl, max_capacity, name), `CacheBackend` trait (get/set/delete/exists/clear), `MokaBackend` (in-memory via Moka), `RedisBackend` (distributed via Redis).

**Monitoring:** `MonitoringPlugin`, `MetricsConfig` (prefix, labels, process metrics), `MetricsRecorder` (render/record_request/increment_counter/set_gauge/record_histogram/time), `HealthCheck` trait, `HealthStatus` (Healthy/Degraded/Unhealthy aggregation).

**Events:** `EventSourcingPlugin`, `DomainEvent` trait, `EventMetadata` (event_id, aggregate_id, version, timestamps, correlation/causation IDs), `EventEnvelope<E>`, `CrudEvent<T>` (Created/Updated/Deleted/Restored), `EventStore` trait, `InMemoryEventStore`.

## rust-boot-macros

Procedural macro crate providing the `#[derive(CrudModel)]` macro for compile-time code generation.

### What It Generates

Given a struct annotated with `#[derive(CrudModel)]`, the macro generates:

- A SeaORM entity module with `Model`, `Relation`, and `ActiveModelBehavior`
- `CreateDto`, `UpdateDto`, and `ResponseDto` structs
- OpenAPI schema annotations via utoipa

### Attributes

Struct-level (`#[crud_model(...)]`):
- `table_name = "..."` — Database table name (defaults to snake_case of struct name)
- `soft_delete` — Adds `deleted_at: Option<DateTimeUtc>` column
- `timestamps` — Adds `created_at` and `updated_at` columns

Field-level (`#[crud_field(...)]`):
- `primary_key` — Marks the field as primary key
- `column_name = "..."` — Custom database column name
- `nullable` — Allows NULL values
- `skip_dto` — Excludes from generated DTOs
- `validation = "..."` — Validation rules: `email`, `url`, `min_length:N`, `max_length:N`, `pattern:REGEX`, or custom

### Internal Architecture

The macro pipeline has three stages:
1. **Parse** (`parse.rs`) — Uses `darling` to extract struct and field attributes into an intermediate representation (`CrudModelIr`, `FieldIr`).
2. **IR** (`ir.rs`) — The intermediate representation with helper methods like `primary_key_field()`, `dto_fields()`, `required_fields()`.
3. **Generate** (`gen/`) — Produces `TokenStream` output for entities (`gen/entity.rs`) and DTOs (`gen/dto.rs`) using `quote`.

## rust-boot-cli

A command-line tool for scaffolding new rust-boot projects and generating model code. Built with `clap` for argument parsing and `tera` for template rendering.

The CLI provides commands for:
- Creating new projects with a standard directory structure
- Generating model files with CRUD boilerplate
- Scaffolding configuration files
