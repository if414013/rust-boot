//! Axum integration including routers, handlers, middleware, and OpenAPI documentation support.

pub mod handlers;
pub mod router;

pub use handlers::{
    ok, created, no_content, paginated,
    ApiError, ApiResponse, ApiResult, CrudHandlers,
    PaginatedResponse, PaginatedResult, PaginationQuery,
};
pub use router::{
    crud_router, crud_router_with_config,
    CrudRouter, CrudRouterBuilder, CrudRouterConfig,
};
