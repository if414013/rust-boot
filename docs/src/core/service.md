# Service

The service layer sits between your HTTP handlers and the repository layer. It's where business logic lives — pagination calculations, soft delete semantics, filtering, sorting, and any domain rules that don't belong in raw data access or HTTP response formatting.

The module lives in `rust_boot_core::service` and provides the `CrudService` trait along with supporting types for pagination, sorting, and filtering.

## CrudService Trait

`CrudService` defines the standard operations for managing entities with business logic applied. It mirrors the repository's CRUD operations but adds pagination awareness, soft delete vs. hard delete distinction, and restore capability.

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
        &self,
        filter: &dyn Filter,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<Self::Entity>>;
    async fn update(&self, id: Self::Id, dto: Self::UpdateDto) -> Result<Self::Entity>;
    async fn delete(&self, id: Self::Id) -> Result<()>;
    async fn hard_delete(&self, id: Self::Id) -> Result<()>;
    async fn restore(&self, id: Self::Id) -> Result<Self::Entity>;
    async fn find_all_including_deleted(
        &self,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<Self::Entity>>;
}
```

### Associated Types

| Type | Constraint | Description |
|------|-----------|-------------|
| `Entity` | `Send + Sync` | The domain entity this service manages |
| `Id` | `Send + Sync` | The entity identifier type |
| `CreateDto` | `Send + Sync` | Data transfer object for creating new entities |
| `UpdateDto` | `Send + Sync` | Data transfer object for updating existing entities |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `create(dto)` | `Result<Entity>` | Creates a new entity from the DTO |
| `find_by_id(id)` | `Result<Option<Entity>>` | Finds an entity by ID (excludes soft-deleted) |
| `find_all(pagination)` | `Result<PaginatedResult<Entity>>` | Lists entities with pagination (excludes soft-deleted) |
| `find_with_filter(filter, pagination)` | `Result<PaginatedResult<Entity>>` | Lists filtered entities with pagination |
| `update(id, dto)` | `Result<Entity>` | Updates an entity by ID |
| `delete(id)` | `Result<()>` | Soft deletes an entity (marks as deleted, keeps in DB) |
| `hard_delete(id)` | `Result<()>` | Permanently removes an entity from the database |
| `restore(id)` | `Result<Entity>` | Restores a soft-deleted entity |
| `find_all_including_deleted(pagination)` | `Result<PaginatedResult<Entity>>` | Lists all entities including soft-deleted ones |

## PaginationParams

Controls which page of results to fetch. Pages are 1-indexed.

```rust
use rust_boot_core::service::PaginationParams;

// Defaults: page 1, 20 items per page
let params = PaginationParams::default();

// Custom: page 3, 10 items per page
let params = PaginationParams::new(3, 10);

// Calculate database query values
assert_eq!(params.offset(), 20); // (3 - 1) * 10
assert_eq!(params.limit(), 10);  // same as per_page
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page` | `u64` | `1` | Page number (1-indexed) |
| `per_page` | `u64` | `20` | Number of items per page |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `new(page, per_page)` | `PaginationParams` | Creates new params |
| `offset()` | `u64` | Calculates `(page - 1) * per_page` for DB queries. Uses `saturating_sub` so page 0 returns offset 0 |
| `limit()` | `u64` | Returns `per_page` |

## PaginatedResult

Wraps a page of results with metadata about the full result set. This is what `CrudService` methods return for list operations.

```rust
use rust_boot_core::service::{PaginatedResult, PaginationParams};

let items = vec!["a", "b", "c"];
let result = PaginatedResult::new(
    items,
    100,                          // total items across all pages
    PaginationParams::new(1, 10), // current page params
);

assert_eq!(result.total_pages, 10);   // ceil(100 / 10)
assert!(result.has_next_page());       // page 1 < 10
assert!(!result.has_prev_page());      // page 1, no previous
assert_eq!(result.len(), 3);           // items on this page
assert!(!result.is_empty());
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `items` | `Vec<T>` | The items on the current page |
| `total` | `u64` | Total number of items across all pages |
| `page` | `u64` | Current page number |
| `per_page` | `u64` | Items per page |
| `total_pages` | `u64` | Total number of pages (calculated automatically) |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `new(items, total, params)` | `PaginatedResult<T>` | Creates a result, auto-calculates `total_pages` |
| `has_next_page()` | `bool` | `true` if `page < total_pages` |
| `has_prev_page()` | `bool` | `true` if `page > 1` |
| `is_empty()` | `bool` | `true` if `items` is empty |
| `len()` | `usize` | Number of items on this page |

## SortDirection and SortParams

Control the ordering of query results.

```rust
use rust_boot_core::service::{SortDirection, SortParams};

// Default sort direction is ascending
assert_eq!(SortDirection::default(), SortDirection::Asc);

// Convenience constructors
let sort = SortParams::asc("name");       // sort by name ascending
let sort = SortParams::desc("created_at"); // sort by created_at descending

// Or explicit construction
let sort = SortParams::new("email", SortDirection::Asc);
```

### SortDirection

| Variant | Description |
|---------|-------------|
| `Asc` | Ascending order (default) |
| `Desc` | Descending order |

### SortParams Fields

| Field | Type | Description |
|-------|------|-------------|
| `field` | `String` | The field name to sort by |
| `direction` | `SortDirection` | Ascending or descending |

## Filtering

The filtering system uses a trait-based approach that lets you build dynamic query filters without coupling to a specific query language.

### FilterOp Enum

Represents a single filter operation on a field:

| Variant | Description | Example |
|---------|-------------|---------|
| `Eq(String)` | Equals | `status = "active"` |
| `Ne(String)` | Not equals | `status != "deleted"` |
| `Gt(String)` | Greater than | `age > "18"` |
| `Lt(String)` | Less than | `price < "100"` |
| `Gte(String)` | Greater than or equal | `age >= "21"` |
| `Lte(String)` | Less than or equal | `score <= "50"` |
| `Like(String)` | Pattern match (SQL LIKE) | `name LIKE "%john%"` |
| `In(Vec<String>)` | Value in set | `role IN ("admin", "mod")` |
| `IsNull` | Is null | `deleted_at IS NULL` |
| `IsNotNull` | Is not null | `email IS NOT NULL` |

### Filter Trait

Implement this trait to create custom filter types:

```rust
pub trait Filter: Send + Sync {
    /// Returns the filter operation for a given field, or None if not filtered
    fn apply(&self, field: &str) -> Option<FilterOp>;
    /// Returns the list of fields this filter applies to
    fn fields(&self) -> Vec<String>;
}
```

### NoFilter

A built-in filter that matches everything — useful as a default:

```rust
use rust_boot_core::service::NoFilter;

let filter = NoFilter;
assert_eq!(filter.apply("any_field"), None); // no filtering
assert!(filter.fields().is_empty());         // no fields
```

### Custom Filter Example

```rust
use rust_boot_core::service::{Filter, FilterOp};
use std::collections::HashMap;

struct UserFilter {
    conditions: HashMap<String, FilterOp>,
}

impl UserFilter {
    fn active_users() -> Self {
        let mut conditions = HashMap::new();
        conditions.insert(
            "status".to_string(),
            FilterOp::Eq("active".to_string()),
        );
        Self { conditions }
    }
}

impl Filter for UserFilter {
    fn apply(&self, field: &str) -> Option<FilterOp> {
        self.conditions.get(field).cloned()
    }

    fn fields(&self) -> Vec<String> {
        self.conditions.keys().cloned().collect()
    }
}
```

## Implementing CrudService

Here's how a typical service implementation looks, delegating to a repository:

```rust
use async_trait::async_trait;
use rust_boot_core::service::*;
use rust_boot_core::error::{Result, RustBootError};

struct UserService {
    repo: UserRepository,
}

#[async_trait]
impl CrudService for UserService {
    type Entity = User;
    type Id = u64;
    type CreateDto = CreateUserDto;
    type UpdateDto = UpdateUserDto;

    async fn create(&self, dto: CreateUserDto) -> Result<User> {
        // Add business logic: validate, hash password, etc.
        let entity = User::from_dto(dto);
        self.repo.insert(&entity).await
    }

    async fn find_by_id(&self, id: u64) -> Result<Option<User>> {
        // Only return non-deleted entities
        self.repo.find_by_id(id).await
    }

    async fn find_all(
        &self,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<User>> {
        let offset = pagination.offset() as usize;
        let limit = pagination.limit() as usize;
        let items = self.repo.find_all(limit, offset).await?;
        let total = self.repo.count().await?;
        Ok(PaginatedResult::new(items, total, pagination))
    }

    // ... implement remaining methods
}
```
