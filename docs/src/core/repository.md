# Repository

The repository layer in rust-boot provides a trait-based abstraction over data persistence. It follows the Repository pattern — your business logic talks to traits, not to a specific database driver. This means you can swap database backends, write in-memory implementations for testing, or layer caching on top without changing your service code.

The module lives in `rust_boot_core::repository` and defines three traits: `CrudRepository` (the main data access interface), `DatabaseConnection` (connection management), and `Transaction` (ACID transaction control).

## Architecture Overview

The relationship between the three traits forms a clean hierarchy:

- A `CrudRepository` holds a reference to a `DatabaseConnection`
- A `DatabaseConnection` can begin a `Transaction`
- A `Transaction` can be committed or rolled back

Your application code interacts primarily with `CrudRepository`. The connection and transaction traits are used when you need direct database access or explicit transaction boundaries.

## CrudRepository Trait

This is the core trait you'll implement for each entity in your application. It defines a standard set of CRUD operations plus counting and existence checks.

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
        &self,
        filter: &dyn Filter,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Self::Entity>>;
    async fn update(&self, entity: &Self::Entity) -> Result<Self::Entity>;
    async fn delete(&self, id: Self::Id) -> Result<()>;
    async fn count(&self) -> Result<u64>;
    async fn count_with_filter(&self, filter: &dyn Filter) -> Result<u64>;
    async fn exists(&self, id: Self::Id) -> Result<bool>;
}
```

### Associated Types

| Type | Constraint | Description |
|------|-----------|-------------|
| `Entity` | `Send + Sync` | The domain entity this repository manages (e.g., `User`, `Post`) |
| `Id` | `Send + Sync` | The identifier type for the entity (e.g., `u64`, `Uuid`) |
| `Connection` | `DatabaseConnection` | The concrete connection type used by this repository |

### Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `connection()` | `&self -> &Self::Connection` | Returns a reference to the underlying database connection |
| `insert(entity)` | `&Self::Entity -> Result<Self::Entity>` | Inserts a new entity, returns it with its assigned ID |
| `find_by_id(id)` | `Self::Id -> Result<Option<Self::Entity>>` | Finds by ID, returns `None` if not found |
| `find_all(limit, offset)` | `usize, usize -> Result<Vec<Self::Entity>>` | Retrieves a paginated list of all entities |
| `find_with_filter(filter, limit, offset)` | `&dyn Filter, usize, usize -> Result<Vec<...>>` | Retrieves a filtered, paginated list |
| `update(entity)` | `&Self::Entity -> Result<Self::Entity>` | Updates an existing entity, returns the updated version |
| `delete(id)` | `Self::Id -> Result<()>` | Deletes an entity by ID |
| `count()` | `-> Result<u64>` | Returns the total count of entities |
| `count_with_filter(filter)` | `&dyn Filter -> Result<u64>` | Returns the count of entities matching a filter |
| `exists(id)` | `Self::Id -> Result<bool>` | Checks whether an entity with the given ID exists |

## DatabaseConnection Trait

Represents an active database connection capable of executing queries and managing transactions.

```rust
#[async_trait]
pub trait DatabaseConnection: Send + Sync {
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>>;
    async fn execute(&self, query: &str) -> Result<u64>;
    fn is_connected(&self) -> bool;
}
```

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `begin_transaction()` | `Result<Box<dyn Transaction>>` | Starts a new database transaction |
| `execute(query)` | `Result<u64>` | Executes a raw SQL query, returns affected row count |
| `is_connected()` | `bool` | Returns whether the connection is currently active |

## Transaction Trait

Represents an active database transaction. Implementations must ensure ACID properties. Note that both `commit` and `rollback` consume the transaction (`self: Box<Self>`), preventing accidental reuse.

```rust
#[async_trait]
pub trait Transaction: Send + Sync {
    async fn commit(self: Box<Self>) -> Result<()>;
    async fn rollback(self: Box<Self>) -> Result<()>;
}
```

### Methods

| Method | Description |
|--------|-------------|
| `commit(self)` | Commits all changes made within the transaction |
| `rollback(self)` | Rolls back all changes made within the transaction |

## Implementing CrudRepository

Here's a complete example of implementing `CrudRepository` for a `User` entity. This shows the pattern you'd follow for any entity in your application:

```rust
use async_trait::async_trait;
use rust_boot_core::repository::{CrudRepository, DatabaseConnection, Transaction};
use rust_boot_core::service::Filter;
use rust_boot_core::error::{Result, RustBootError};

// Define your entity
#[derive(Debug, Clone)]
struct User {
    id: u64,
    name: String,
    email: String,
}

// Define your connection type (wrapping your actual DB driver)
struct PgConnection {
    // ... your database driver connection
}

#[async_trait]
impl DatabaseConnection for PgConnection {
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>> {
        // Start a transaction using your DB driver
        todo!()
    }

    async fn execute(&self, query: &str) -> Result<u64> {
        // Execute raw SQL
        todo!()
    }

    fn is_connected(&self) -> bool {
        true // Check actual connection state
    }
}

// Implement the repository
struct UserRepository {
    conn: PgConnection,
}

#[async_trait]
impl CrudRepository for UserRepository {
    type Entity = User;
    type Id = u64;
    type Connection = PgConnection;

    fn connection(&self) -> &Self::Connection {
        &self.conn
    }

    async fn insert(&self, entity: &Self::Entity) -> Result<Self::Entity> {
        // INSERT INTO users (name, email) VALUES ($1, $2) RETURNING *
        todo!()
    }

    async fn find_by_id(&self, id: Self::Id) -> Result<Option<Self::Entity>> {
        // SELECT * FROM users WHERE id = $1
        todo!()
    }

    async fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Self::Entity>> {
        // SELECT * FROM users LIMIT $1 OFFSET $2
        todo!()
    }

    async fn find_with_filter(
        &self,
        filter: &dyn Filter,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Self::Entity>> {
        // Build WHERE clause from filter.fields() and filter.apply(field)
        todo!()
    }

    async fn update(&self, entity: &Self::Entity) -> Result<Self::Entity> {
        // UPDATE users SET name = $1, email = $2 WHERE id = $3 RETURNING *
        todo!()
    }

    async fn delete(&self, id: Self::Id) -> Result<()> {
        // DELETE FROM users WHERE id = $1
        todo!()
    }

    async fn count(&self) -> Result<u64> {
        // SELECT COUNT(*) FROM users
        todo!()
    }

    async fn count_with_filter(&self, filter: &dyn Filter) -> Result<u64> {
        // SELECT COUNT(*) FROM users WHERE ...
        todo!()
    }

    async fn exists(&self, id: Self::Id) -> Result<bool> {
        // SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)
        todo!()
    }
}
```

## Using Transactions

Transactions give you explicit control over commit/rollback boundaries. The `Box<Self>` signature on `commit` and `rollback` ensures the transaction is consumed after use.

```rust
async fn transfer_funds(
    conn: &impl DatabaseConnection,
) -> Result<()> {
    // Begin a transaction
    let tx = conn.begin_transaction().await?;

    // Perform operations...
    // If anything fails, the transaction is dropped (implicit rollback
    // depends on your implementation)

    // Explicitly commit on success
    tx.commit().await?;
    Ok(())
}
```

## Relationship with the Service Layer

The repository layer is designed to work hand-in-hand with the [Service](service.md) layer. A typical `CrudService` implementation holds a repository and delegates data access to it, while adding business logic like pagination calculation, soft delete semantics, and validation:

```rust
// The service uses the repository for data access
struct UserService {
    repo: UserRepository,
}

// The service adds business logic on top of raw repository operations
impl UserService {
    async fn find_all_active(
        &self,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<User>> {
        let offset = pagination.offset() as usize;
        let limit = pagination.limit() as usize;
        let items = self.repo.find_all(limit, offset).await?;
        let total = self.repo.count().await?;
        Ok(PaginatedResult::new(items, total, pagination))
    }
}
```
