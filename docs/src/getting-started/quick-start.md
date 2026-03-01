# Quick Start

This guide walks you through building a complete CRUD API from scratch using rust-boot. By the end, you will have a running application with JWT authentication, in-memory caching, and Prometheus monitoring — all in a single file.

The code below is based on the `basic_api` example that ships with the framework. You can run it directly with:

```bash
cargo run --example basic_api
```

## Step 1: Define Your Domain Model

Start by defining the data structures your API will work with. In a real application you would derive `CrudModel` from `rust-boot-macros` to get automatic CRUD operations. For this walkthrough, we define everything manually so you can see how the pieces connect.

```rust
use rust_boot::prelude::*;
use std::sync::Arc;
use std::time::Duration;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserDto {
    pub name: String,
    pub email: String,
}
```

The `Serialize`, `Deserialize`, and `Uuid` types all come from the `rust_boot::prelude` — no extra imports needed.

## Step 2: Define Application State

Application state holds references to your plugins and shared resources. It gets cloned into every request handler via Axum's state extraction.

```rust
#[derive(Clone)]
pub struct AppState {
    pub jwt_manager: Arc<JwtManager>,
    pub cache: Arc<MokaBackend>,
}
```

## Step 3: Write Handler Functions

Handler functions are plain async functions that use Axum extractors. rust-boot provides response helpers (`ok`, `created`, `no_content`, `paginated`) that wrap your data in a consistent `ApiResponse<T>` envelope.

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

async fn get_user(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> ApiResult<User> {
    let user = User { id, name: "Example User".into(), email: "user@example.com".into(), active: true };
    ok(user)
}

async fn create_user(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Json(dto): axum::extract::Json<CreateUserDto>,
) -> (axum::http::StatusCode, axum::Json<ApiResponse<User>>) {
    let user = User::new(dto.name, dto.email);
    created(user)
}
```

Notice the return types: `PaginatedResult<T>` for list endpoints, `ApiResult<T>` for single-item responses, and the tuple form for custom status codes like 201 Created.

## Step 4: Configure and Register Plugins

The `PluginRegistry` is the central coordinator. You create plugin configurations, instantiate plugins, and register them. The registry resolves dependencies and initializes everything in the correct order.

```rust
async fn setup_plugins() -> RustBootResult<(Arc<JwtManager>, Arc<MokaBackend>)> {
    let mut registry = PluginRegistry::new();

    // Caching — Moka in-memory cache with 5-minute TTL
    let cache_config = CacheConfig::new("api-cache")
        .with_ttl(Duration::from_secs(300))
        .with_max_capacity(10_000);
    registry.register(CachingPlugin::new(cache_config.clone()))?;

    // Monitoring — Prometheus metrics and health checks
    let metrics_config = MetricsConfig::default();
    registry.register(MonitoringPlugin::new(metrics_config))?;

    // Authentication — JWT with 15-minute access tokens
    let jwt_config = JwtConfig::new("your-super-secret-key-change-in-production")
        .with_access_token_ttl(Duration::from_secs(15 * 60))
        .with_refresh_token_ttl(Duration::from_secs(7 * 24 * 60 * 60))
        .with_issuer("rust-boot-example")
        .with_audience("rust-boot-api");
    registry.register(AuthPlugin::new(jwt_config.clone()))?;

    // Initialize all plugins in dependency order
    registry.init_all().await?;

    let jwt_manager = Arc::new(JwtManager::new(jwt_config));
    let cache_backend = Arc::new(MokaBackend::new(cache_config));

    Ok((jwt_manager, cache_backend))
}
```

> **Important:** In production, load your JWT secret from an environment variable or a secrets manager — never hard-code it.

## Step 5: Build the Router

`CrudRouterBuilder` maps HTTP verbs to your handler functions. It produces a standard Axum `Router` that you can merge with other routes.

```rust
fn create_router(state: AppState) -> axum::Router {
    let router_config = CrudRouterConfig::new("/api/users")
        .with_soft_delete();

    let users_router = CrudRouterBuilder::<AppState>::new(router_config)
        .list(list_users)       // GET    /api/users
        .get(get_user)          // GET    /api/users/:id
        .create(create_user)    // POST   /api/users
        .build();

    axum::Router::new()
        .merge(users_router)
        .with_state(state)
}
```

## Step 6: Start the Server

Tie everything together in `main()`:

```rust
#[tokio::main]
async fn main() -> RustBootResult<()> {
    tracing_subscriber::fmt::init();

    let (jwt_manager, cache) = setup_plugins().await?;
    let state = AppState { jwt_manager, cache };
    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
```

Your API is now running at `http://localhost:3000` with these endpoints:

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/users` | List users (paginated) |
| `GET` | `/api/users/:id` | Get user by ID |
| `POST` | `/api/users` | Create a new user |

## What's Next

- [Architecture Overview](../architecture/overview.md) — Understand how the crates and plugins fit together.
- [Plugins](../plugins/overview.md) — Deep dive into authentication, caching, monitoring, and events.
- [Guides](../guides/overview.md) — Step-by-step tutorials for common tasks.
