//! Axum router generation for CRUD endpoints.

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};

/// Configuration for generating CRUD routes.
pub struct CrudRouterConfig {
    /// Base path for all routes (e.g., "/api/users").
    pub base_path: String,
    /// Whether to enable soft delete with restore endpoint.
    pub enable_soft_delete: bool,
    /// Whether to enable the list endpoint.
    pub enable_list: bool,
    /// Whether to enable the get-by-id endpoint.
    pub enable_get: bool,
    /// Whether to enable the create endpoint.
    pub enable_create: bool,
    /// Whether to enable the update endpoint.
    pub enable_update: bool,
    /// Whether to enable the delete endpoint.
    pub enable_delete: bool,
}

impl Default for CrudRouterConfig {
    fn default() -> Self {
        Self {
            base_path: String::new(),
            enable_soft_delete: false,
            enable_list: true,
            enable_get: true,
            enable_create: true,
            enable_update: true,
            enable_delete: true,
        }
    }
}

impl CrudRouterConfig {
    /// Creates a new config with the given base path.
    pub fn new(base_path: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
            ..Default::default()
        }
    }

    /// Enables soft delete with restore functionality.
    pub fn with_soft_delete(mut self) -> Self {
        self.enable_soft_delete = true;
        self
    }

    /// Disables the list endpoint.
    pub fn disable_list(mut self) -> Self {
        self.enable_list = false;
        self
    }

    /// Disables the get-by-id endpoint.
    pub fn disable_get(mut self) -> Self {
        self.enable_get = false;
        self
    }

    /// Disables the create endpoint.
    pub fn disable_create(mut self) -> Self {
        self.enable_create = false;
        self
    }

    /// Disables the update endpoint.
    pub fn disable_update(mut self) -> Self {
        self.enable_update = false;
        self
    }

    /// Disables the delete endpoint.
    pub fn disable_delete(mut self) -> Self {
        self.enable_delete = false;
        self
    }
}

/// Trait for generating CRUD routes from a configuration.
pub trait CrudRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Generates CRUD routes for the given state and configuration.
    fn crud_routes(state: S, config: CrudRouterConfig) -> Router<S>;
}

/// Builder for constructing CRUD routers with custom handlers.
pub struct CrudRouterBuilder<S>
where
    S: Clone + Send + Sync + 'static,
{
    router: Router<S>,
    config: CrudRouterConfig,
}

impl<S> CrudRouterBuilder<S>
where
    S: Clone + Send + Sync + 'static,
{
    /// Creates a new builder with the given configuration.
    pub fn new(config: CrudRouterConfig) -> Self {
        Self {
            router: Router::new(),
            config,
        }
    }

    /// Adds a list handler (GET /).
    pub fn list<H, T>(mut self, handler: H) -> Self
    where
        H: axum::handler::Handler<T, S>,
        T: 'static,
    {
        if self.config.enable_list {
            self.router = self.router.route("/", get(handler));
        }
        self
    }

    /// Adds a get-by-id handler (GET /:id).
    pub fn get<H, T>(mut self, handler: H) -> Self
    where
        H: axum::handler::Handler<T, S>,
        T: 'static,
    {
        if self.config.enable_get {
            self.router = self.router.route("/:id", get(handler));
        }
        self
    }

    /// Adds a create handler (POST /).
    pub fn create<H, T>(mut self, handler: H) -> Self
    where
        H: axum::handler::Handler<T, S>,
        T: 'static,
    {
        if self.config.enable_create {
            self.router = self.router.route("/", post(handler));
        }
        self
    }

    /// Adds an update handler (PUT /:id).
    pub fn update<H, T>(mut self, handler: H) -> Self
    where
        H: axum::handler::Handler<T, S>,
        T: 'static,
    {
        if self.config.enable_update {
            self.router = self.router.route("/:id", put(handler));
        }
        self
    }

    /// Adds a delete handler (DELETE /:id).
    pub fn delete<H, T>(mut self, handler: H) -> Self
    where
        H: axum::handler::Handler<T, S>,
        T: 'static,
    {
        if self.config.enable_delete {
            self.router = self.router.route("/:id", delete(handler));
        }
        self
    }

    /// Adds a restore handler (PATCH /:id/restore).
    pub fn restore<H, T>(mut self, handler: H) -> Self
    where
        H: axum::handler::Handler<T, S>,
        T: 'static,
    {
        if self.config.enable_soft_delete {
            self.router = self.router.route("/:id/restore", patch(handler));
        }
        self
    }

    /// Builds the final router.
    pub fn build(self) -> Router<S> {
        if self.config.base_path.is_empty() {
            self.router
        } else {
            Router::new().nest(&self.config.base_path, self.router)
        }
    }
}

/// Creates a CRUD router builder with default configuration.
pub fn crud_router<S>(base_path: &str) -> CrudRouterBuilder<S>
where
    S: Clone + Send + Sync + 'static,
{
    CrudRouterBuilder::new(CrudRouterConfig::new(base_path))
}

/// Creates a CRUD router builder with custom configuration.
pub fn crud_router_with_config<S>(config: CrudRouterConfig) -> CrudRouterBuilder<S>
where
    S: Clone + Send + Sync + 'static,
{
    CrudRouterBuilder::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::State;

    #[derive(Clone)]
    struct TestState;

    async fn list_handler(State(_state): State<TestState>) -> &'static str {
        "list"
    }

    async fn get_handler(State(_state): State<TestState>) -> &'static str {
        "get"
    }

    async fn create_handler(State(_state): State<TestState>) -> &'static str {
        "create"
    }

    async fn update_handler(State(_state): State<TestState>) -> &'static str {
        "update"
    }

    async fn delete_handler(State(_state): State<TestState>) -> &'static str {
        "delete"
    }

    async fn restore_handler(State(_state): State<TestState>) -> &'static str {
        "restore"
    }

    #[test]
    fn test_crud_router_config_default() {
        let config = CrudRouterConfig::default();
        assert!(config.enable_list);
        assert!(config.enable_get);
        assert!(config.enable_create);
        assert!(config.enable_update);
        assert!(config.enable_delete);
        assert!(!config.enable_soft_delete);
    }

    #[test]
    fn test_crud_router_config_with_soft_delete() {
        let config = CrudRouterConfig::new("/users").with_soft_delete();
        assert!(config.enable_soft_delete);
        assert_eq!(config.base_path, "/users");
    }

    #[test]
    fn test_crud_router_config_disable_operations() {
        let config = CrudRouterConfig::default()
            .disable_list()
            .disable_create()
            .disable_delete();

        assert!(!config.enable_list);
        assert!(config.enable_get);
        assert!(!config.enable_create);
        assert!(config.enable_update);
        assert!(!config.enable_delete);
    }

    #[test]
    fn test_crud_router_builder() {
        let _router: Router<TestState> = crud_router::<TestState>("/api/users")
            .list(list_handler)
            .get(get_handler)
            .create(create_handler)
            .update(update_handler)
            .delete(delete_handler)
            .build();
    }

    #[test]
    fn test_crud_router_with_restore() {
        let config = CrudRouterConfig::new("/api/users").with_soft_delete();
        let _router: Router<TestState> = crud_router_with_config::<TestState>(config)
            .list(list_handler)
            .get(get_handler)
            .create(create_handler)
            .update(update_handler)
            .delete(delete_handler)
            .restore(restore_handler)
            .build();
    }

    #[test]
    fn test_crud_router_empty_base_path() {
        let _router: Router<TestState> = crud_router::<TestState>("")
            .list(list_handler)
            .build();
    }
}
