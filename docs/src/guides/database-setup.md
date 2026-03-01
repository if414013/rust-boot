# Database Setup

rust-boot uses [SeaORM](https://www.sea-ql.org/SeaORM/) as its ORM layer and provides configuration, connection pooling, and repository abstractions out of the box. This guide covers how to configure your database connection, understand the repository pattern, and work with the service layer.

---

## Database Configuration

The `DatabaseConfig` struct holds your connection parameters. It lives inside the top-level `RustBootConfig` and supports three ways to set values.

### Default Values

```rust
use rust_boot::prelude::*;

let config = DatabaseConfig::default();
// url:             "sqlite::memory:"
// max_connections: 10
// min_connections: 1
```

The default is an in-memory SQLite database — useful for development and testing.

### Builder Pattern

Use `RustBootConfig::builder()` to configure the database alongside other settings:

```rust
let config = RustBootConfig::builder()
    .database_url("postgresql://user:password@localhost:5432/myapp".to_string())
    .database_max_connections(25)
    .database_min_connections(5)
    .server_host("0.0.0.0".to_string())
    .server_port(8080)
    .build();

// Access database config
println!("DB URL: {}", config.database.url);
println!("Pool: {}-{} connections", config.database.min_connections, config.database.max_connections);
```

### Configuration Files

Load from TOML, YAML, or JSON files:

```toml
# config.toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgresql://user:password@localhost:5432/myapp"
max_connections = 20
min_connections = 5
```

```rust
let config = RustBootConfig::from_file("config.toml")?;
```

Supported formats are determined by file extension: `.toml`, `.yaml`/`.yml`, `.json`.

### Environment Variable Overrides

Environment variables with the `RUST_BOOT_` prefix override any file or default values:

| Variable | Description |
|----------|-------------|
| `RUST_BOOT_DATABASE_URL` | Connection URL |
| `RUST_BOOT_DATABASE_MAX_CONNECTIONS` | Maximum pool size |
| `RUST_BOOT_DATABASE_MIN_CONNECTIONS` | Minimum pool size |

```bash
export RUST_BOOT_DATABASE_URL="postgresql://prod-user:secret@db.example.com/prod"
export RUST_BOOT_DATABASE_MAX_CONNECTIONS=50
```

```rust
// Loads defaults, then applies env overrides
let config = RustBootConfig::from_env();
```

When loading from a file, env overrides are applied automatically after parsing:

```rust
// File values + env overrides
let config = RustBootConfig::from_file("config.toml")?;
```

The precedence order is: **defaults < file values < environment variables**.

---

## Supported Databases

rust-boot supports any database that SeaORM supports through its connection URL scheme:

| Database   | URL Format |
|------------|------------|
| PostgreSQL | `postgresql://user:pass@host:5432/dbname` |
| MySQL      | `mysql://user:pass@host:3306/dbname` |
| SQLite     | `sqlite:./path/to/db.sqlite` or `sqlite::memory:` |

---

## The Repository Pattern

rust-boot defines a `CrudRepository` trait that provides a standard interface for data access. This decouples your business logic from the specific database implementation.

### CrudRepository Trait

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
    async fn find_with_filter(
        &self, filter: &dyn Filter, limit: usize, offset: usize,
    ) -> Result<Vec<Self::Entity>>;
    async fn update(&self, entity: &Self::Entity) -> Result<Self::Entity>;
    async fn delete(&self, id: Self::Id) -> Result<()>;
    async fn count(&self) -> Result<u64>;
    async fn count_with_filter(&self, filter: &dyn Filter) -> Result<u64>;
    async fn exists(&self, id: Self::Id) -> Result<bool>;
}
```

The three associated types let you customize:

- `Entity` — your domain model type
- `Id` — the primary key type (e.g., `i64`, `Uuid`)
- `Connection` — the database connection type

### DatabaseConnection and Transactions

The `DatabaseConnection` trait abstracts over the actual database driver:

```rust
#[async_trait]
pub trait DatabaseConnection: Send + Sync {
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>>;
    async fn execute(&self, query: &str) -> Result<u64>;
    fn is_connected(&self) -> bool;
}
```

Transactions follow a simple commit/rollback pattern:

```rust
let tx = connection.begin_transaction().await?;

// Perform operations...

tx.commit().await?;   // or tx.rollback().await?
```

---

## The Service Layer

Above the repository sits the `CrudService` trait, which adds business logic concerns like pagination, soft delete, and DTOs.

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
    async fn find_with_filter(
        &self, filter: &dyn Filter, pagination: PaginationParams,
    ) -> Result<PaginatedResult<Self::Entity>>;
    async fn update(&self, id: Self::Id, dto: Self::UpdateDto) -> Result<Self::Entity>;
    async fn delete(&self, id: Self::Id) -> Result<()>;
    async fn hard_delete(&self, id: Self::Id) -> Result<()>;
    async fn restore(&self, id: Self::Id) -> Result<Self::Entity>;
    async fn find_all_including_deleted(
        &self, pagination: PaginationParams,
    ) -> Result<PaginatedResult<Self::Entity>>;
}
```

Key differences from `CrudRepository`:

- **DTOs**: `create()` and `update()` accept DTOs instead of raw entities
- **Pagination**: `find_all()` returns `PaginatedResult<T>` with page metadata
- **Soft delete**: `delete()` soft-deletes, `hard_delete()` permanently removes, `restore()` recovers

### Pagination

```rust
let params = PaginationParams::new(1, 20);  // page 1, 20 per page
let result = service.find_all(params).await?;

println!("Page {} of {}", result.page, result.total_pages);
println!("Items on this page: {}", result.items.len());
println!("Total items: {}", result.total);
println!("Has next page: {}", result.has_next_page());
```

### Filtering

Implement the `Filter` trait for dynamic queries:

```rust
use rust_boot::prelude::*;
use rust_boot::core::service::{Filter, FilterOp};

struct UserFilter {
    name: Option<String>,
}

impl Filter for UserFilter {
    fn apply(&self, field: &str) -> Option<FilterOp> {
        match field {
            "name" => self.name.as_ref().map(|n| FilterOp::Eq(n.clone())),
            _ => None,
        }
    }

    fn fields(&self) -> Vec<String> {
        let mut fields = Vec::new();
        if self.name.is_some() { fields.push("name".to_string()); }
        fields
    }
}
```

Available filter operations:

| Operation | Description |
|-----------|-------------|
| `FilterOp::Eq(val)` | Equals |
| `FilterOp::Ne(val)` | Not equals |
| `FilterOp::Gt(val)` | Greater than |
| `FilterOp::Lt(val)` | Less than |
| `FilterOp::Gte(val)` | Greater than or equal |
| `FilterOp::Lte(val)` | Less than or equal |
| `FilterOp::Like(val)` | SQL LIKE pattern |
| `FilterOp::In(vals)` | Value in set |
| `FilterOp::IsNull` | Is NULL |
| `FilterOp::IsNotNull` | Is not NULL |

Use `NoFilter` when you don't need filtering:

```rust
let filter = NoFilter;
let result = service.find_with_filter(&filter, PaginationParams::default()).await?;
```

---

## Next Steps

- [CrudModel Macro Reference](../reference/crud-model-macro.md) — Auto-generate SeaORM entities from annotated structs
- [Basic API Tutorial](./basic-api-tutorial.md) — Build a full API with these database abstractions
- [API Reference](../reference/api-reference.md) — Complete type catalog
