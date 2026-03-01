# Axum Integration Overview

The `rust-boot-axum` crate connects rust-boot's core abstractions to the [Axum](https://github.com/tokio-rs/axum) web framework. It provides two things: a router builder that generates standard CRUD endpoints from configuration, and a set of response helpers that give your API a consistent JSON structure.

The crate re-exports everything from two submodules so you can import from the top level:

```rust
use rust_boot_axum::{
    // Router building
    crud_router, crud_router_with_config,
    CrudRouter, CrudRouterBuilder, CrudRouterConfig,

    // Response helpers
    ok, created, no_content, paginated,
    ApiResponse, ApiError, ApiResult,
    PaginatedResponse, PaginatedResult, PaginationQuery,
    CrudHandlers,
};
```

## What It Gives You

Instead of manually wiring up `GET /`, `GET /:id`, `POST /`, `PUT /:id`, `DELETE /:id` for every resource, you describe what you want and the router builder generates the routes:

```rust
use rust_boot_axum::{crud_router, ok, created, no_content, paginated};
use axum::extract::{State, Path, Query};
use axum::Json;

// Build a full CRUD router in a few lines
let router = crud_router::<AppState>("/api/users")
    .list(list_users)
    .get(get_user)
    .create(create_user)
    .update(update_user)
    .delete(delete_user)
    .build();

// Your handlers use the response helpers for consistent JSON output
async fn list_users(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> PaginatedResult<User> {
    let users = state.user_service.find_all(query.page, query.per_page).await?;
    paginated(users.items, query.page, query.per_page, users.total)
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> ApiResult<User> {
    let user = state.user_service.find_by_id(id).await?
        .ok_or_else(|| ApiError::not_found("User"))?;
    ok(user)
}
```

## Module Structure

| Module | Purpose | Key Exports |
|--------|---------|-------------|
| [`router`](router.md) | Route generation and configuration | `CrudRouterConfig`, `CrudRouterBuilder`, `crud_router`, `crud_router_with_config` |
| [`handlers`](handlers.md) | Response types and helper functions | `ApiResponse`, `ApiError`, `PaginationQuery`, `ok`, `created`, `no_content`, `paginated` |

## How the Pieces Fit Together

The typical flow in a rust-boot Axum application:

1. **Define your state** — an `AppState` struct holding your services
2. **Write handler functions** — using Axum extractors (`State`, `Path`, `Query`, `Json`) and rust-boot response helpers (`ok`, `created`, `paginated`)
3. **Build routes** — using `crud_router` or `CrudRouterBuilder` to wire handlers to HTTP methods and paths
4. **Merge into your app** — the builder produces a standard `axum::Router` that you can merge with other routes

```rust
use axum::Router;
use rust_boot_axum::{crud_router, CrudRouterConfig, crud_router_with_config};

#[derive(Clone)]
struct AppState {
    // your services here
}

// Simple: all CRUD endpoints enabled
let users = crud_router::<AppState>("/api/users")
    .list(list_users)
    .get(get_user)
    .create(create_user)
    .update(update_user)
    .delete(delete_user)
    .build();

// Advanced: selective endpoints with soft delete
let posts = crud_router_with_config::<AppState>(
    CrudRouterConfig::new("/api/posts")
        .with_soft_delete()
        .disable_update()
)
    .list(list_posts)
    .get(get_post)
    .create(create_post)
    .delete(delete_post)
    .restore(restore_post)
    .build();

// Merge into your application
let app = Router::new()
    .merge(users)
    .merge(posts)
    .with_state(AppState { /* ... */ });
```

## Relationship to Core

The Axum integration layer sits on top of the [core service layer](../core/service.md). Your handlers typically call `CrudService` methods and translate the results into HTTP responses using the helpers from this crate. The core `RustBootError` can be mapped to `ApiError` for consistent error responses.
