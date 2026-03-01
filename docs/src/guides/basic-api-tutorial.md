# Building a Basic CRUD API

This tutorial walks you through building a complete CRUD API with rust-boot. You'll create a User management REST API with JWT authentication, caching, monitoring, and paginated responses.

Based on the `basic_api` example shipped with the framework:

```bash
cargo run --example basic_api
```

## What You'll Build

| Method   | Path              | Description            |
|----------|-------------------|------------------------|
| `GET`    | `/api/users`      | List users (paginated) |
| `GET`    | `/api/users/:id`  | Get user by ID         |
| `POST`   | `/api/users`      | Create a new user      |
| `PUT`    | `/api/users/:id`  | Update a user          |
| `DELETE` | `/api/users/:id`  | Delete a user          |

---

## Step 1: Define Your Domain Models

Start with your data structures. The `prelude` module re-exports `Serialize`, `Deserialize`, `Uuid`, and other common types.

```rust
use rust_boot::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub active: bool,
}

impl User {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            email: email.into(),
            active: true,
        }
    }
}
```

Define DTOs to separate your API contract from your internal model. `CreateUserDto` has required fields; `UpdateUserDto` uses `Option` for partial updates.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserDto {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserDto {
    pub name: Option<String>,
    pub email: Option<String>,
    pub active: Option<bool>,
}
```

In production, use `#[derive(CrudModel)]` to auto-generate DTOs, SeaORM entities, and OpenAPI schemas. See the [CrudModel Macro Reference](../reference/crud-model-macro.md).

---

## Step 2: Define Application State

Axum handlers receive shared state through the `State` extractor. Wrap plugin instances in `Arc` for cheap, thread-safe cloning across requests.

```rust
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub jwt_manager: Arc<JwtManager>,
    pub cache: Arc<MokaBackend>,
}
```

---

## Step 3: Write Handler Functions

rust-boot provides helper functions for consistent API responses:

- `ok(value)` — HTTP 200 with data wrapped in `ApiResponse`
- `created(value)` — HTTP 201 with the created resource
- `no_content()` — HTTP 204 with no body
- `paginated(items, page, per_page, total)` — paginated response envelope

### List Users

```rust
async fn list_users(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> PaginatedResult<User> {
    let users = vec![
        User::new("Alice", "alice@example.com"),
        User::new("Bob", "bob@example.com"),
    ];
    paginated(users, pagination.page, pagination.per_page, 2)
}
```

`PaginationQuery` parses `?page=1&per_page=20` from the URL automatically.

### Get User

```rust
async fn get_user(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> ApiResult<User> {
    let user = User { id, name: "Example User".into(), email: "user@example.com".into(), active: true };
    ok(user)
}
```

### Create User

```rust
async fn create_user(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Json(dto): axum::extract::Json<CreateUserDto>,
) -> (axum::http::StatusCode, axum::Json<ApiResponse<User>>) {
    let user = User::new(dto.name, dto.email);
    created(user)
}
```

### Update User

```rust
async fn update_user(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    axum::extract::Json(dto): axum::extract::Json<UpdateUserDto>,
) -> ApiResult<User> {
    let user = User {
        id,
        name: dto.name.unwrap_or_else(|| "Updated User".into()),
        email: dto.email.unwrap_or_else(|| "updated@example.com".into()),
        active: dto.active.unwrap_or(true),
    };
    ok(user)
}
```

### Delete User

```rust
async fn delete_user(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Path(_id): axum::extract::Path<Uuid>,
) -> axum::http::StatusCode {
    no_content()
}
```

---

## Step 4: Set Up JWT Authentication

rust-boot includes JWT authentication out of the box. Configure it with `JwtConfig`, then use `JwtManager` for token operations.

```rust
use std::time::Duration;

let jwt_config = JwtConfig::new("your-secret-key-change-in-production")
    .with_access_token_ttl(Duration::from_secs(15 * 60))           // 15 minutes
    .with_refresh_token_ttl(Duration::from_secs(7 * 24 * 60 * 60)) // 7 days
    .with_issuer("rust-boot-example")
    .with_audience("rust-boot-api");

let jwt_manager = JwtManager::new(jwt_config);
```

### Create and Verify Tokens

```rust
let claims = Claims::new("user-123", 0, 0)
    .with_role(Role::admin())
    .with_role(Role::user())
    .with_email("admin@example.com")
    .with_name("Admin User");

let access_token = jwt_manager.create_access_token(claims.clone())?;
let refresh_token = jwt_manager.create_refresh_token(claims)?;

// Verify and extract claims
let verified = jwt_manager.verify_access_token(&access_token)?;
assert!(verified.has_role(&Role::admin()));

// Refresh tokens
let (new_access, new_refresh) = jwt_manager.refresh_tokens(&refresh_token)?;
```

---

## Step 5: Register and Initialize Plugins

The `PluginRegistry` manages plugin lifecycle. Register plugins, then call `init_all()` to run the `build()` phase in dependency order.

```rust
async fn setup_plugins() -> RustBootResult<(Arc<JwtManager>, Arc<MokaBackend>)> {
    let mut registry = PluginRegistry::new();

    // Caching — Moka in-memory cache
    let cache_config = CacheConfig::new("api-cache")
        .with_ttl(Duration::from_secs(300))
        .with_max_capacity(10_000);
    registry.register(CachingPlugin::new(cache_config.clone()))?;

    // Monitoring — Prometheus metrics + health checks
    registry.register(MonitoringPlugin::new(MetricsConfig::default()))?;

    // Authentication — JWT
    let jwt_config = JwtConfig::new("your-secret-key-change-in-production")
        .with_access_token_ttl(Duration::from_secs(15 * 60))
        .with_refresh_token_ttl(Duration::from_secs(7 * 24 * 60 * 60))
        .with_issuer("rust-boot-example")
        .with_audience("rust-boot-api");
    registry.register(AuthPlugin::new(jwt_config.clone()))?;

    registry.init_all().await?;

    Ok((
        Arc::new(JwtManager::new(jwt_config)),
        Arc::new(MokaBackend::new(cache_config)),
    ))
}
```

### Using the Cache

```rust
let user = User::new("Cached User", "cached@example.com");
let key = format!("user:{}", user.id);
let bytes = serde_json::to_vec(&user)?;
cache.set(&key, bytes, Some(Duration::from_secs(60))).await?;

if let Some(bytes) = cache.get(&key).await? {
    let cached_user: User = serde_json::from_slice(&bytes)?;
}
```

---

## Step 6: Configure the Router

`CrudRouterBuilder` declaratively wires handlers to standard CRUD routes.

```rust
fn create_router(state: AppState) -> axum::Router {
    let router_config = CrudRouterConfig::new("/api/users")
        .with_soft_delete();

    let users_router = CrudRouterBuilder::<AppState>::new(router_config)
        .list(list_users)       // GET    /api/users
        .get(get_user)          // GET    /api/users/:id
        .create(create_user)    // POST   /api/users
        .update(update_user)    // PUT    /api/users/:id
        .delete(delete_user)    // DELETE /api/users/:id
        .build();

    axum::Router::new().merge(users_router).with_state(state)
}
```

You can selectively disable operations:

```rust
let config = CrudRouterConfig::new("/api/items")
    .with_soft_delete()
    .disable_delete();  // No DELETE endpoint
```

---

## Step 7: Start the Server

```rust
#[tokio::main]
async fn main() -> RustBootResult<()> {
    tracing_subscriber::fmt::init();

    let (jwt_manager, cache) = setup_plugins().await?;
    let state = AppState { jwt_manager, cache };
    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Server running at http://localhost:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
```

---

## Testing Your API

```bash
# List users
curl http://localhost:3000/api/users

# Create a user
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Charlie", "email": "charlie@example.com"}'

# Get a user
curl http://localhost:3000/api/users/<uuid>

# Update a user
curl -X PUT http://localhost:3000/api/users/<uuid> \
  -H "Content-Type: application/json" \
  -d '{"name": "Charlie Updated"}'

# Delete a user
curl -X DELETE http://localhost:3000/api/users/<uuid>
```

---

## Next Steps

- [Custom Plugin Tutorial](./custom-plugin-tutorial.md) — Write your own plugins with lifecycle hooks and dependencies
- [Database Setup Guide](./database-setup.md) — Connect to a real database with SeaORM
- [CrudModel Macro Reference](../reference/crud-model-macro.md) — Auto-generate entities, DTOs, and OpenAPI schemas
