# Handlers

The handlers module provides response types and helper functions that give your Axum API a consistent JSON structure. Every successful response wraps data in a standard envelope, every error returns a structured JSON error body with an error code and message, and paginated endpoints include page metadata automatically.

The module lives in `rust_boot_axum::handlers`.

## Response Helpers

Four free functions cover the most common HTTP response patterns. Use these in your handler return types for consistent API output.

### `ok(data)` — 200 OK

Wraps data in an `ApiResponse` envelope and returns 200:

```rust
use rust_boot_axum::{ok, ApiResult};

async fn get_user(Path(id): Path<u64>, State(state): State<AppState>) -> ApiResult<User> {
    let user = state.service.find_by_id(id).await
        .map_err(|_| ApiError::internal_error())?
        .ok_or_else(|| ApiError::not_found("User"))?;
    ok(user)
}
```

Response body:
```json
{
  "data": { "id": 1, "name": "Alice" }
}
```

### `created(data)` — 201 Created

Returns a 201 status with the created entity wrapped in `ApiResponse`:

```rust
use rust_boot_axum::created;
use axum::{http::StatusCode, Json};

async fn create_user(
    State(state): State<AppState>,
    Json(dto): Json<CreateUserDto>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let user = state.service.create(dto).await.unwrap();
    created(user)
}
```

Response body (with HTTP 201):
```json
{
  "data": { "id": 1, "name": "Alice" }
}
```

### `no_content()` — 204 No Content

Returns a bare 204 status with no body. Typically used for delete operations:

```rust
use rust_boot_axum::no_content;
use axum::http::StatusCode;

async fn delete_user(Path(id): Path<u64>, State(state): State<AppState>) -> StatusCode {
    state.service.delete(id).await.unwrap();
    no_content()
}
```

### `paginated(data, page, per_page, total)` — 200 OK with page metadata

Wraps a list of items in a `PaginatedResponse` with automatic page calculation:

```rust
use rust_boot_axum::{paginated, PaginatedResult, PaginationQuery};

async fn list_users(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> PaginatedResult<User> {
    let result = state.service.find_all(query.page, query.per_page).await.unwrap();
    paginated(result.items, query.page, query.per_page, result.total)
}
```

Response body:
```json
{
  "data": [{ "id": 1, "name": "Alice" }, { "id": 2, "name": "Bob" }],
  "page": 1,
  "per_page": 20,
  "total": 50,
  "total_pages": 3
}
```

## Type Aliases

Two type aliases simplify handler return types:

```rust
// For single-item responses
pub type ApiResult<T> = Result<Json<ApiResponse<T>>, ApiError>;

// For paginated responses
pub type PaginatedResult<T> = Result<Json<PaginatedResponse<T>>, ApiError>;
```

Use these as your handler return types:

```rust
async fn get_user(...) -> ApiResult<User> { ... }
async fn list_users(...) -> PaginatedResult<User> { ... }
```

## PaginationQuery

Deserializes pagination parameters from query strings. All fields have defaults, so callers can omit them.

```rust
use rust_boot_axum::PaginationQuery;

// GET /api/users                        → page=1, per_page=20, include_deleted=false
// GET /api/users?page=3&per_page=10     → page=3, per_page=10, include_deleted=false
// GET /api/users?include_deleted=true   → page=1, per_page=20, include_deleted=true
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `page` | `u64` | `1` | Page number (1-indexed) |
| `per_page` | `u64` | `20` | Items per page |
| `include_deleted` | `bool` | `false` | Whether to include soft-deleted items |

Use it as an Axum `Query` extractor:

```rust
use axum::extract::Query;
use rust_boot_axum::PaginationQuery;

async fn list_users(Query(query): Query<PaginationQuery>) -> PaginatedResult<User> {
    if query.include_deleted {
        // fetch including soft-deleted
    } else {
        // fetch only active
    }
    // ...
}
```

## ApiResponse

A generic wrapper that puts your data under a `"data"` key in the JSON response:

```rust
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
}
```

You typically don't construct this directly — use the `ok()` and `created()` helpers instead. But you can if needed:

```rust
use rust_boot_axum::ApiResponse;

let response = ApiResponse::new(user);
// Serializes to: { "data": { ... } }
```

## PaginatedResponse

Wraps a list of items with pagination metadata:

```rust
#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub page: u64,
    pub per_page: u64,
    pub total: u64,
    pub total_pages: u64,
}
```

The `total_pages` field is calculated automatically: `ceil(total / per_page)`. If `per_page` is 0, `total_pages` is 0.

```rust
use rust_boot_axum::PaginatedResponse;

let response = PaginatedResponse::new(
    vec![user1, user2],  // data
    1,                    // page
    20,                   // per_page
    50,                   // total
);
assert_eq!(response.total_pages, 3); // ceil(50 / 20)
```

## ApiError

A structured error type that serializes to a consistent JSON format and maps to appropriate HTTP status codes.

```rust
#[derive(Serialize)]
pub struct ApiError {
    pub error: String,                        // error code
    pub message: String,                      // human-readable message
    pub details: Option<serde_json::Value>,   // optional extra context
}
```

### Factory Methods

| Method | Error Code | HTTP Status | Description |
|--------|-----------|-------------|-------------|
| `ApiError::not_found(resource)` | `"not_found"` | 404 | Resource not found |
| `ApiError::bad_request(message)` | `"bad_request"` | 400 | Invalid request |
| `ApiError::validation_error(message)` | `"validation_error"` | 422 | Validation failure |
| `ApiError::conflict(message)` | `"conflict"` | 409 | Resource conflict |
| `ApiError::internal_error()` | `"internal_error"` | 500 | Internal server error |
| `ApiError::new(error, message)` | Custom | Depends on code | Generic constructor |

### Adding Details

Chain `.with_details()` to attach structured context to any error:

```rust
use rust_boot_axum::ApiError;

let error = ApiError::validation_error("Field validation failed")
    .with_details(serde_json::json!({
        "field": "email",
        "reason": "invalid format"
    }));
```

Serializes to:
```json
{
  "error": "validation_error",
  "message": "Field validation failed",
  "details": { "field": "email", "reason": "invalid format" }
}
```

The `details` field is omitted from the JSON when `None` (via `#[serde(skip_serializing_if = "Option::is_none")]`).

### HTTP Status Code Mapping

`ApiError` implements Axum's `IntoResponse` trait. The HTTP status code is determined by the `error` field:

| Error Code | HTTP Status |
|-----------|-------------|
| `"not_found"` | 404 Not Found |
| `"bad_request"` | 400 Bad Request |
| `"validation_error"` | 422 Unprocessable Entity |
| `"conflict"` | 409 Conflict |
| `"unauthorized"` | 401 Unauthorized |
| `"forbidden"` | 403 Forbidden |
| Any other value | 500 Internal Server Error |

## CrudHandlers Trait

Defines the standard handler signatures for a complete CRUD resource. This is useful when you want to enforce a consistent handler interface across your application:

```rust
#[async_trait]
pub trait CrudHandlers<S, Id, Entity, CreateDto, UpdateDto>
where
    S: Clone + Send + Sync + 'static,
    Id: Send + Sync + 'static,
    Entity: Serialize + Send + Sync + 'static,
    CreateDto: DeserializeOwned + Send + Sync + 'static,
    UpdateDto: DeserializeOwned + Send + Sync + 'static,
{
    async fn list(state: State<S>, query: Query<PaginationQuery>) -> PaginatedResult<Entity>;
    async fn get(state: State<S>, id: Path<Id>) -> ApiResult<Entity>;
    async fn create(state: State<S>, payload: Json<CreateDto>)
        -> (StatusCode, Json<ApiResponse<Entity>>);
    async fn update(state: State<S>, id: Path<Id>, payload: Json<UpdateDto>)
        -> ApiResult<Entity>;
    async fn delete(state: State<S>, id: Path<Id>) -> StatusCode;
    async fn restore(state: State<S>, id: Path<Id>) -> ApiResult<Entity>;
}
```

### Handler Signatures

| Method | Extractors | Return Type |
|--------|-----------|-------------|
| `list` | `State<S>`, `Query<PaginationQuery>` | `PaginatedResult<Entity>` |
| `get` | `State<S>`, `Path<Id>` | `ApiResult<Entity>` |
| `create` | `State<S>`, `Json<CreateDto>` | `(StatusCode, Json<ApiResponse<Entity>>)` |
| `update` | `State<S>`, `Path<Id>`, `Json<UpdateDto>` | `ApiResult<Entity>` |
| `delete` | `State<S>`, `Path<Id>` | `StatusCode` |
| `restore` | `State<S>`, `Path<Id>` | `ApiResult<Entity>` |

## Complete Handler Example

Putting it all together — a full set of CRUD handlers for a `User` resource:

```rust
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use rust_boot_axum::{
    ok, created, no_content, paginated,
    ApiResult, PaginatedResult, PaginationQuery,
    ApiResponse, ApiError,
};

async fn list_users(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> PaginatedResult<User> {
    let result = state.user_service
        .find_all(query.page, query.per_page)
        .await
        .map_err(|_| ApiError::internal_error())?;
    paginated(result.items, query.page, query.per_page, result.total)
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> ApiResult<User> {
    let user = state.user_service
        .find_by_id(id)
        .await
        .map_err(|_| ApiError::internal_error())?
        .ok_or_else(|| ApiError::not_found("User"))?;
    ok(user)
}

async fn create_user(
    State(state): State<AppState>,
    Json(dto): Json<CreateUserDto>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let user = state.user_service.create(dto).await.unwrap();
    created(user)
}

async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(dto): Json<UpdateUserDto>,
) -> ApiResult<User> {
    let user = state.user_service
        .update(id, dto)
        .await
        .map_err(|_| ApiError::internal_error())?;
    ok(user)
}

async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> StatusCode {
    state.user_service.delete(id).await.unwrap();
    no_content()
}
```
