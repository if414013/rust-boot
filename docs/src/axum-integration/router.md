# Router

The router module provides a builder-based system for generating standard CRUD routes in Axum. Instead of manually defining each route, you describe which endpoints you want and wire up your handler functions — the builder takes care of HTTP method mapping, path construction, and nesting.

The module lives in `rust_boot_axum::router`.

## Quick Start

```rust
use rust_boot_axum::crud_router;
use axum::Router;

#[derive(Clone)]
struct AppState { /* your services */ }

let router: Router<AppState> = crud_router::<AppState>("/api/users")
    .list(list_users)       // GET    /api/users
    .get(get_user)          // GET    /api/users/:id
    .create(create_user)    // POST   /api/users
    .update(update_user)    // PUT    /api/users/:id
    .delete(delete_user)    // DELETE /api/users/:id
    .build();
```

## CrudRouterConfig

Controls which endpoints are generated and where they're mounted. By default, all standard CRUD endpoints are enabled and soft delete is disabled.

```rust
use rust_boot_axum::CrudRouterConfig;

// All endpoints enabled at /api/users
let config = CrudRouterConfig::new("/api/users");

// Enable soft delete (adds PATCH /:id/restore)
let config = CrudRouterConfig::new("/api/posts").with_soft_delete();

// Disable specific endpoints
let config = CrudRouterConfig::new("/api/logs")
    .disable_create()
    .disable_update()
    .disable_delete();
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `base_path` | `String` | `""` | Base path for all routes (e.g., `"/api/users"`) |
| `enable_soft_delete` | `bool` | `false` | Whether to enable the restore endpoint |
| `enable_list` | `bool` | `true` | Enable `GET /` |
| `enable_get` | `bool` | `true` | Enable `GET /:id` |
| `enable_create` | `bool` | `true` | Enable `POST /` |
| `enable_update` | `bool` | `true` | Enable `PUT /:id` |
| `enable_delete` | `bool` | `true` | Enable `DELETE /:id` |

### Configuration Methods

All methods consume and return `self` for chaining:

| Method | Description |
|--------|-------------|
| `new(base_path)` | Creates a config with the given base path, all endpoints enabled |
| `with_soft_delete()` | Enables the restore endpoint (`PATCH /:id/restore`) |
| `disable_list()` | Disables `GET /` |
| `disable_get()` | Disables `GET /:id` |
| `disable_create()` | Disables `POST /` |
| `disable_update()` | Disables `PUT /:id` |
| `disable_delete()` | Disables `DELETE /:id` |

## CrudRouterBuilder

The builder is where you attach your handler functions. Each builder method checks the corresponding config flag — if the endpoint is disabled, the handler is silently ignored.

```rust
use rust_boot_axum::{CrudRouterBuilder, CrudRouterConfig};

let config = CrudRouterConfig::new("/api/users").with_soft_delete();

let router = CrudRouterBuilder::<AppState>::new(config)
    .list(list_users)       // GET    /
    .get(get_user)          // GET    /:id
    .create(create_user)    // POST   /
    .update(update_user)    // PUT    /:id
    .delete(delete_user)    // DELETE /:id
    .restore(restore_user)  // PATCH  /:id/restore (only if soft delete enabled)
    .build();
```

### Builder Methods

Each method accepts any Axum handler (anything implementing `axum::handler::Handler<T, S>`):

| Method | Route | HTTP Method | Config Flag |
|--------|-------|-------------|-------------|
| `list(handler)` | `/` | GET | `enable_list` |
| `get(handler)` | `/:id` | GET | `enable_get` |
| `create(handler)` | `/` | POST | `enable_create` |
| `update(handler)` | `/:id` | PUT | `enable_update` |
| `delete(handler)` | `/:id` | DELETE | `enable_delete` |
| `restore(handler)` | `/:id/restore` | PATCH | `enable_soft_delete` |
| `build()` | — | — | Produces the final `Router<S>` |

### How `build()` Works

When you call `build()`, the builder checks the `base_path`:

- If `base_path` is empty, it returns the router as-is (routes at `/`, `/:id`, etc.)
- If `base_path` is set, it nests the router under that path using `Router::new().nest(&base_path, router)`

## Convenience Functions

Two free functions create builders without manually constructing a config:

### `crud_router(base_path)`

Creates a builder with default config (all endpoints enabled, no soft delete):

```rust
use rust_boot_axum::crud_router;

let router = crud_router::<AppState>("/api/users")
    .list(list_users)
    .get(get_user)
    .create(create_user)
    .update(update_user)
    .delete(delete_user)
    .build();
```

### `crud_router_with_config(config)`

Creates a builder with a custom config:

```rust
use rust_boot_axum::{crud_router_with_config, CrudRouterConfig};

let config = CrudRouterConfig::new("/api/posts")
    .with_soft_delete()
    .disable_update();

let router = crud_router_with_config::<AppState>(config)
    .list(list_posts)
    .get(get_post)
    .create(create_post)
    .delete(delete_post)
    .restore(restore_post)
    .build();
```

## CrudRouter Trait

For a more structured approach, you can implement the `CrudRouter` trait on a type to encapsulate route generation:

```rust
pub trait CrudRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn crud_routes(state: S, config: CrudRouterConfig) -> Router<S>;
}
```

This is useful when you want a resource type to own its route definitions:

```rust
use rust_boot_axum::{CrudRouter, CrudRouterConfig, crud_router_with_config};
use axum::Router;

struct UserRoutes;

impl CrudRouter<AppState> for UserRoutes {
    fn crud_routes(_state: AppState, config: CrudRouterConfig) -> Router<AppState> {
        crud_router_with_config(config)
            .list(list_users)
            .get(get_user)
            .create(create_user)
            .update(update_user)
            .delete(delete_user)
            .build()
    }
}

// Usage
let config = CrudRouterConfig::new("/api/users");
let router = UserRoutes::crud_routes(app_state, config);
```

## Generated Route Map

For a config with `base_path = "/api/users"` and all options enabled (including soft delete):

| HTTP Method | Path | Builder Method | Description |
|-------------|------|----------------|-------------|
| GET | `/api/users` | `.list()` | List with pagination |
| GET | `/api/users/:id` | `.get()` | Get by ID |
| POST | `/api/users` | `.create()` | Create new |
| PUT | `/api/users/:id` | `.update()` | Update by ID |
| DELETE | `/api/users/:id` | `.delete()` | Delete by ID |
| PATCH | `/api/users/:id/restore` | `.restore()` | Restore soft-deleted |

## Composing Multiple Routers

Since `build()` returns a standard `axum::Router`, you can merge multiple CRUD routers together:

```rust
use axum::Router;
use rust_boot_axum::crud_router;

let app = Router::new()
    .merge(
        crud_router::<AppState>("/api/users")
            .list(list_users)
            .get(get_user)
            .create(create_user)
            .update(update_user)
            .delete(delete_user)
            .build()
    )
    .merge(
        crud_router::<AppState>("/api/posts")
            .list(list_posts)
            .get(get_post)
            .create(create_post)
            .build()
    )
    .with_state(app_state);
```
