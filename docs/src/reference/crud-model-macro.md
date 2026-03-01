# CrudModel Derive Macro

The `#[derive(CrudModel)]` macro generates SeaORM entities, DTOs, and OpenAPI schemas from a single annotated struct. It lives in the `rust-boot-macros` crate and is re-exported through `rust_boot::prelude::*`.

## Quick Example

```rust
use rust_boot_macros::CrudModel;

#[derive(CrudModel)]
#[crud_model(table_name = "users", soft_delete, timestamps)]
pub struct User {
    #[crud_field(primary_key)]
    pub id: i64,

    #[crud_field(validation = "email")]
    pub email: String,

    pub name: String,

    #[crud_field(nullable)]
    pub bio: Option<String>,

    #[crud_field(skip_dto)]
    pub password_hash: String,
}
```

This single struct generates:

1. A SeaORM entity module with `Model`, `Relation`, and `ActiveModel`
2. `CreateUserDto` — for creating new users (excludes `id` and `password_hash`)
3. `UpdateUserDto` — for partial updates (all fields wrapped in `Option`)
4. `UserResponse` — for API responses (excludes `password_hash`)
5. Const methods on `User`: `table_name()`, `soft_delete_enabled()`, `timestamps_enabled()`

---

## Struct-Level Attributes

Struct-level attributes are placed on the `#[crud_model(...)]` attribute above your struct.

### `table_name = "name"`

Sets the database table name for the generated SeaORM entity.

```rust
#[crud_model(table_name = "user_accounts")]
pub struct User { /* ... */ }
```

If omitted, the table name is derived by converting the struct name to snake_case:

| Struct Name | Default Table Name |
|-------------|-------------------|
| `User` | `user` |
| `UserProfile` | `user_profile` |
| `BlogPost` | `blog_post` |

### `soft_delete`

Enables soft delete support. Adds a `deleted_at: Option<DateTimeUtc>` column to the generated SeaORM entity with the `#[sea_orm(nullable)]` attribute.

```rust
#[crud_model(soft_delete)]
pub struct User { /* ... */ }
```

Also sets `User::soft_delete_enabled()` to return `true`.

### `timestamps`

Auto-generates `created_at` and `updated_at` timestamp columns in the SeaORM entity (as `DateTimeUtc`) and includes them in the response DTO (as `chrono::DateTime<chrono::Utc>`).

```rust
#[crud_model(timestamps)]
pub struct User { /* ... */ }
```

Also sets `User::timestamps_enabled()` to return `true`.

### Combining Attributes

All struct-level attributes can be combined:

```rust
#[crud_model(table_name = "users", soft_delete, timestamps)]
pub struct User { /* ... */ }
```

---

## Field-Level Attributes

Field-level attributes are placed on individual fields using `#[crud_field(...)]`.

### `primary_key`

Marks the field as the primary key. This field:

- Gets `#[sea_orm(primary_key)]` in the generated entity
- Is excluded from `CreateDto` (the database assigns it)
- Is included in `UpdateDto` and `ResponseDto`

```rust
#[crud_field(primary_key)]
pub id: i64,
```

### `column_name = "name"`

Overrides the database column name. By default, field names are used as-is (converted to snake_case if needed).

```rust
#[crud_field(column_name = "user_email")]
pub email: String,
```

Generates `#[sea_orm(column_name = "user_email")]` on the entity field.

### `nullable`

Marks the field as nullable in the database. Adds `#[sea_orm(nullable)]` to the entity field.

```rust
#[crud_field(nullable)]
pub bio: Option<String>,
```

### `skip_dto`

Excludes the field from all generated DTOs (Create, Update, and Response). Use this for internal fields like password hashes that should never appear in the API.

```rust
#[crud_field(skip_dto)]
pub password_hash: String,
```

The field still appears in the SeaORM entity — it's only hidden from the API layer.

### `default = "value"`

Sets a default value expression for the field. Stored as metadata in the intermediate representation.

```rust
#[crud_field(default = "true")]
pub active: bool,
```

### `validation = "rules"`

Adds validation attributes to the generated DTOs. Multiple rules can be comma-separated.

```rust
#[crud_field(validation = "email")]
pub email: String,

#[crud_field(validation = "min_length:3, max_length:50")]
pub username: String,
```

---

## Validation Rules

The `validation` attribute accepts a comma-separated string of rules:

| Rule | Generated Attribute | Example |
|------|-------------------|---------|
| `email` | `#[validate(email)]` | `validation = "email"` |
| `url` | `#[validate(url)]` | `validation = "url"` |
| `min_length:N` | `#[validate(length(min = N))]` | `validation = "min_length:3"` |
| `max_length:N` | `#[validate(length(max = N))]` | `validation = "max_length:255"` |
| `pattern:REGEX` | `#[validate(regex = "REGEX")]` | `validation = "pattern:^[a-z]+$"` |
| anything else | (ignored — stored as Custom) | `validation = "my_rule"` |

Multiple rules:

```rust
#[crud_field(validation = "email, min_length:5, max_length:100")]
pub email: String,
```

---

## Generated Code

### Entity Module

For a struct named `User` with `table_name = "users"`, the macro generates:

```rust
// Const methods on the original struct
impl User {
    pub const fn table_name() -> &'static str { "users" }
    pub const fn soft_delete_enabled() -> bool { false }  // or true
    pub const fn timestamps_enabled() -> bool { false }    // or true
}

// SeaORM entity module
pub mod entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub email: String,
        pub name: String,
        #[sea_orm(nullable)]
        pub bio: Option<String>,
        pub password_hash: String,
        // If timestamps: true
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
        // If soft_delete: true
        #[sea_orm(nullable)]
        pub deleted_at: Option<DateTimeUtc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
```

### CreateDto

Excludes `primary_key` and `skip_dto` fields. All remaining fields are required.

```rust
#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserDto {
    #[validate(email)]
    pub email: String,
    pub name: String,
    // bio included (nullable doesn't affect DTOs)
    // id excluded (primary_key)
    // password_hash excluded (skip_dto)
}
```

### UpdateDto

Same field filtering as CreateDto, but all fields are wrapped in `Option<T>` for partial updates. Fields that are already `Option<T>` are not double-wrapped.

```rust
#[derive(Debug, Clone, Default, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserDto {
    #[validate(email)]
    pub email: Option<String>,
    pub name: Option<String>,
    pub bio: Option<String>,  // Already Option, not double-wrapped
}
```

### ResponseDto

Includes all fields except `skip_dto` fields. If `timestamps` is enabled, adds `created_at` and `updated_at`.

```rust
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub bio: Option<String>,
    // password_hash excluded (skip_dto)
    // If timestamps: true
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

---

## Common Patterns

### Minimal Model

```rust
#[derive(CrudModel)]
pub struct Item {
    #[crud_field(primary_key)]
    pub id: i64,
    pub name: String,
}
```

Table name defaults to `item`. No timestamps, no soft delete.

### Full-Featured Model

```rust
#[derive(CrudModel)]
#[crud_model(table_name = "products", soft_delete, timestamps)]
pub struct Product {
    #[crud_field(primary_key)]
    pub id: i64,

    #[crud_field(validation = "min_length:1, max_length:200")]
    pub name: String,

    #[crud_field(nullable)]
    pub description: Option<String>,

    #[crud_field(column_name = "price_cents")]
    pub price: i64,

    #[crud_field(skip_dto)]
    pub internal_sku: String,
}
```

---

## See Also

- [Database Setup Guide](../guides/database-setup.md) — How to configure database connections
- [Basic API Tutorial](../guides/basic-api-tutorial.md) — Using models in a complete API
- [API Reference](./api-reference.md) — Full type catalog
