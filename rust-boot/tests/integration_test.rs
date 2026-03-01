//! Integration tests for the rust-boot facade crate verifying cross-crate functionality.

use std::time::Duration;

mod test_prelude_imports {
    use rust_boot::prelude::*;
    use std::time::Duration;

    #[test]
    fn test_core_types_accessible() {
        let _server_config = ServerConfig::default();
        let _db_config = DatabaseConfig::default();
        let _config = RustBootConfig::default();
        let _meta = PluginMeta::new("test", "1.0.0");
        let _ctx = PluginContext::new();
        let _state = PluginState::Adding;
    }

    #[test]
    fn test_axum_types_accessible() {
        let config = CrudRouterConfig::default();
        assert!(config.enable_list);
        assert!(config.enable_get);
        assert!(config.enable_create);
        assert!(config.enable_update);
        assert!(config.enable_delete);
    }

    #[test]
    fn test_plugin_types_accessible() {
        let _cache_config = CacheConfig::default();

        let jwt_config = JwtConfig::new("test-secret-key-long-enough-for-jwt-hmac");
        assert_eq!(jwt_config.access_token_ttl, Duration::from_secs(15 * 60));

        let _metrics_config = MetricsConfig::default();

        let role = Role::admin();
        assert_eq!(role.name(), "admin");

        let claims = Claims::new("user123", 0, 0).with_role(Role::user());
        assert!(claims.has_role(&Role::user()));
    }

    #[test]
    fn test_uuid_reexport() {
        let id = Uuid::new_v4();
        assert!(!id.is_nil());
    }

    #[test]
    fn test_async_trait_reexport() {
        use async_trait::async_trait;

        #[async_trait]
        trait TestTrait {
            async fn do_something(&self);
        }

        struct TestImpl;

        #[async_trait]
        impl TestTrait for TestImpl {
            async fn do_something(&self) {}
        }
    }
}

mod test_top_level_imports {
    use rust_boot::*;
    use std::time::Duration;

    #[test]
    fn test_config_types() {
        let config = RustBootConfig::builder()
            .server_host("0.0.0.0".to_string())
            .server_port(8080)
            .database_url("postgres://localhost/test".to_string())
            .build();

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.url, "postgres://localhost/test");
    }

    #[test]
    fn test_plugin_types() {
        let meta = PluginMeta::new("my-plugin", "1.0.0");
        assert_eq!(meta.name, "my-plugin");
        assert_eq!(meta.version, "1.0.0");
        assert!(meta.dependencies.is_empty());
    }

    #[test]
    fn test_cache_types() {
        let config = CacheConfig::new("test-cache")
            .with_ttl(Duration::from_secs(60))
            .with_max_capacity(1000);

        assert_eq!(config.name, "test-cache");
        assert_eq!(config.default_ttl, Duration::from_secs(60));
        assert_eq!(config.max_capacity, 1000);
    }
}

mod test_module_imports {
    #[test]
    fn test_core_module() {
        use rust_boot::core::error::RustBootError;
        use rust_boot::core::plugin::{CrudPlugin, PluginMeta};

        let _ = std::any::type_name::<RustBootError>();
        let _ = std::any::type_name::<dyn CrudPlugin>();
        let _ = PluginMeta::new("test", "1.0.0");
    }

    #[test]
    fn test_plugins_module() {
        use rust_boot::plugins::{CachingPlugin, JwtManager};

        let _ = std::any::type_name::<CachingPlugin>();
        let _ = std::any::type_name::<JwtManager>();
    }

    #[test]
    fn test_axum_module() {
        use rust_boot::axum::{CrudRouterBuilder, CrudRouterConfig};

        let _ = std::any::type_name::<CrudRouterConfig>();
        let config = CrudRouterConfig::new("/api/users");
        let _ = CrudRouterBuilder::<()>::new(config);
    }
}

mod test_plugin_lifecycle {
    use async_trait::async_trait;
    use rust_boot::prelude::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct LifecycleTrackingPlugin {
        name: String,
        tracker: Arc<Mutex<Vec<String>>>,
    }

    impl LifecycleTrackingPlugin {
        fn new(name: &str, tracker: Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name: name.to_string(),
                tracker,
            }
        }
    }

    #[async_trait]
    impl CrudPlugin for LifecycleTrackingPlugin {
        fn meta(&self) -> PluginMeta {
            PluginMeta::new(&self.name, "1.0.0")
        }

        async fn build(&mut self, ctx: &mut PluginContext) -> RustBootResult<()> {
            let mut tracker = self.tracker.lock().await;
            tracker.push(format!("{}_build", self.name));
            ctx.insert(format!("{}_initialized", self.name), true).await;
            Ok(())
        }

        async fn ready(&mut self, _ctx: &mut PluginContext) -> RustBootResult<()> {
            let mut tracker = self.tracker.lock().await;
            tracker.push(format!("{}_ready", self.name));
            Ok(())
        }

        async fn finish(&mut self, _ctx: &mut PluginContext) -> RustBootResult<()> {
            let mut tracker = self.tracker.lock().await;
            tracker.push(format!("{}_finish", self.name));
            Ok(())
        }

        async fn cleanup(&mut self, ctx: &mut PluginContext) -> RustBootResult<()> {
            let mut tracker = self.tracker.lock().await;
            tracker.push(format!("{}_cleanup", self.name));
            ctx.remove::<bool>(&format!("{}_initialized", self.name))
                .await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_plugin_registry_creation() {
        let registry = PluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[tokio::test]
    async fn test_plugin_registration_and_retrieval() {
        let mut registry = PluginRegistry::new();
        let tracker = Arc::new(Mutex::new(Vec::new()));

        registry
            .register(LifecycleTrackingPlugin::new("test-plugin", tracker))
            .expect("Failed to register plugin");

        assert_eq!(registry.len(), 1);
        assert!(registry.get("test-plugin").is_some());
        assert_eq!(registry.get_state("test-plugin"), Some(PluginState::Adding));
    }

    #[tokio::test]
    async fn test_plugin_lifecycle_init_ready_finish_cleanup() {
        let mut registry = PluginRegistry::new();
        let tracker = Arc::new(Mutex::new(Vec::new()));

        registry
            .register(LifecycleTrackingPlugin::new("plugin-a", tracker.clone()))
            .expect("Failed to register plugin");

        registry.init_all().await.expect("init_all failed");
        assert_eq!(registry.get_state("plugin-a"), Some(PluginState::Ready));

        registry.ready_all().await.expect("ready_all failed");

        registry.finish_all().await.expect("finish_all failed");
        assert_eq!(registry.get_state("plugin-a"), Some(PluginState::Finished));

        registry.cleanup_all().await.expect("cleanup_all failed");
        assert_eq!(registry.get_state("plugin-a"), Some(PluginState::Cleaned));

        let calls = tracker.lock().await;
        assert_eq!(
            *calls,
            vec![
                "plugin-a_build",
                "plugin-a_ready",
                "plugin-a_finish",
                "plugin-a_cleanup"
            ]
        );
    }

    #[tokio::test]
    async fn test_plugin_with_dependencies_initialization_order() {
        let mut registry = PluginRegistry::new();
        let tracker = Arc::new(Mutex::new(Vec::new()));

        struct DependentPlugin {
            name: String,
            deps: Vec<String>,
            tracker: Arc<Mutex<Vec<String>>>,
        }

        #[async_trait]
        impl CrudPlugin for DependentPlugin {
            fn meta(&self) -> PluginMeta {
                PluginMeta::with_dependencies(&self.name, "1.0.0", self.deps.clone())
            }

            async fn build(&mut self, _ctx: &mut PluginContext) -> RustBootResult<()> {
                let mut tracker = self.tracker.lock().await;
                tracker.push(self.name.clone());
                Ok(())
            }
        }

        registry
            .register(DependentPlugin {
                name: "database".to_string(),
                deps: vec![],
                tracker: tracker.clone(),
            })
            .expect("Failed to register database plugin");

        registry
            .register(DependentPlugin {
                name: "cache".to_string(),
                deps: vec!["database".to_string()],
                tracker: tracker.clone(),
            })
            .expect("Failed to register cache plugin");

        registry
            .register(DependentPlugin {
                name: "auth".to_string(),
                deps: vec!["cache".to_string()],
                tracker: tracker.clone(),
            })
            .expect("Failed to register auth plugin");

        registry.init_all().await.expect("init_all failed");

        let order = tracker.lock().await;
        let db_idx = order.iter().position(|x| x == "database").unwrap();
        let cache_idx = order.iter().position(|x| x == "cache").unwrap();
        let auth_idx = order.iter().position(|x| x == "auth").unwrap();

        assert!(
            db_idx < cache_idx,
            "database should be initialized before cache"
        );
        assert!(cache_idx < auth_idx, "cache should be initialized before auth");
    }

    #[tokio::test]
    async fn test_plugin_context_sharing() {
        let mut registry = PluginRegistry::new();
        let tracker = Arc::new(Mutex::new(Vec::new()));

        registry
            .register(LifecycleTrackingPlugin::new("context-test", tracker))
            .expect("Failed to register plugin");

        registry.init_all().await.expect("init_all failed");

        let initialized: Option<bool> = registry.context().get("context-test_initialized").await;
        assert_eq!(initialized, Some(true));
    }

    #[tokio::test]
    async fn test_duplicate_plugin_registration_fails() {
        let mut registry = PluginRegistry::new();
        let tracker = Arc::new(Mutex::new(Vec::new()));

        registry
            .register(LifecycleTrackingPlugin::new("duplicate", tracker.clone()))
            .expect("First registration should succeed");

        let result = registry.register(LifecycleTrackingPlugin::new("duplicate", tracker));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already registered"));
    }

    #[tokio::test]
    async fn test_missing_dependency_registration_fails() {
        let mut registry = PluginRegistry::new();

        struct PluginWithMissingDep;

        #[async_trait]
        impl CrudPlugin for PluginWithMissingDep {
            fn meta(&self) -> PluginMeta {
                PluginMeta::with_dependencies(
                    "dependent",
                    "1.0.0",
                    vec!["nonexistent".to_string()],
                )
            }
        }

        let result = registry.register(PluginWithMissingDep);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not registered"));
    }
}

mod test_crud_router_builder {
    use axum::extract::State;
    use rust_boot::prelude::*;

    #[derive(Clone)]
    struct AppState {
        name: String,
    }

    async fn list_handler(State(_state): State<AppState>) -> &'static str {
        "list"
    }

    async fn get_handler(State(_state): State<AppState>) -> &'static str {
        "get"
    }

    async fn create_handler(State(_state): State<AppState>) -> &'static str {
        "create"
    }

    async fn update_handler(State(_state): State<AppState>) -> &'static str {
        "update"
    }

    async fn delete_handler(State(_state): State<AppState>) -> &'static str {
        "delete"
    }

    async fn restore_handler(State(_state): State<AppState>) -> &'static str {
        "restore"
    }

    #[test]
    fn test_router_config_defaults() {
        let config = CrudRouterConfig::default();
        assert!(config.enable_list);
        assert!(config.enable_get);
        assert!(config.enable_create);
        assert!(config.enable_update);
        assert!(config.enable_delete);
        assert!(!config.enable_soft_delete);
        assert!(config.base_path.is_empty());
    }

    #[test]
    fn test_router_config_with_base_path() {
        let config = CrudRouterConfig::new("/api/v1/users");
        assert_eq!(config.base_path, "/api/v1/users");
    }

    #[test]
    fn test_router_config_with_soft_delete() {
        let config = CrudRouterConfig::new("/users").with_soft_delete();
        assert!(config.enable_soft_delete);
    }

    #[test]
    fn test_router_config_disable_operations() {
        let config = CrudRouterConfig::default()
            .disable_list()
            .disable_get()
            .disable_create()
            .disable_update()
            .disable_delete();

        assert!(!config.enable_list);
        assert!(!config.enable_get);
        assert!(!config.enable_create);
        assert!(!config.enable_update);
        assert!(!config.enable_delete);
    }

    #[test]
    fn test_build_full_crud_router() {
        let config = CrudRouterConfig::new("/api/users");
        let _router = CrudRouterBuilder::<AppState>::new(config)
            .list(list_handler)
            .get(get_handler)
            .create(create_handler)
            .update(update_handler)
            .delete(delete_handler)
            .build();
    }

    #[test]
    fn test_build_readonly_router() {
        let config = CrudRouterConfig::new("/api/products")
            .disable_create()
            .disable_update()
            .disable_delete();

        let _router = CrudRouterBuilder::<AppState>::new(config)
            .list(list_handler)
            .get(get_handler)
            .build();
    }

    #[test]
    fn test_build_router_with_soft_delete() {
        let config = CrudRouterConfig::new("/api/posts").with_soft_delete();

        let _router = CrudRouterBuilder::<AppState>::new(config)
            .list(list_handler)
            .get(get_handler)
            .create(create_handler)
            .update(update_handler)
            .delete(delete_handler)
            .restore(restore_handler)
            .build();
    }

    #[test]
    fn test_build_router_empty_base_path() {
        let config = CrudRouterConfig::new("");
        let _router = CrudRouterBuilder::<AppState>::new(config)
            .list(list_handler)
            .build();
    }
}

mod test_jwt_auth_flow {
    use rust_boot::prelude::*;
    use std::time::Duration;

    fn create_test_jwt_manager() -> JwtManager {
        let config =
            JwtConfig::new("test-secret-key-that-is-long-enough-for-jwt-validation-purposes");
        JwtManager::new(config)
    }

    #[test]
    fn test_jwt_config_creation() {
        let config = JwtConfig::new("my-secret");
        assert_eq!(config.secret, "my-secret");
        assert_eq!(config.access_token_ttl, Duration::from_secs(15 * 60));
        assert_eq!(
            config.refresh_token_ttl,
            Duration::from_secs(7 * 24 * 60 * 60)
        );
        assert!(config.issuer.is_none());
        assert!(config.audience.is_none());
    }

    #[test]
    fn test_jwt_config_builder_pattern() {
        let config = JwtConfig::new("secret")
            .with_access_token_ttl(Duration::from_secs(300))
            .with_refresh_token_ttl(Duration::from_secs(3600))
            .with_issuer("my-app")
            .with_audience("my-api");

        assert_eq!(config.access_token_ttl, Duration::from_secs(300));
        assert_eq!(config.refresh_token_ttl, Duration::from_secs(3600));
        assert_eq!(config.issuer, Some("my-app".to_string()));
        assert_eq!(config.audience, Some("my-api".to_string()));
    }

    #[test]
    fn test_claims_creation() {
        let claims = Claims::new("user-123", 0, 0);
        assert_eq!(claims.sub, "user-123");
        assert!(claims.roles.is_empty());
        assert!(claims.email.is_none());
        assert!(claims.name.is_none());
    }

    #[test]
    fn test_claims_with_roles() {
        let claims = Claims::new("user-123", 0, 0)
            .with_role(Role::admin())
            .with_role(Role::user());

        assert!(claims.has_role(&Role::admin()));
        assert!(claims.has_role(&Role::user()));
        assert!(!claims.has_role(&Role::new("moderator")));
    }

    #[test]
    fn test_claims_role_checks() {
        let claims = Claims::new("user-123", 0, 0)
            .with_roles([Role::admin(), Role::user()]);

        assert!(claims.has_any_role(&[Role::admin()]));
        assert!(claims.has_any_role(&[Role::new("guest"), Role::user()]));
        assert!(!claims.has_any_role(&[Role::new("guest")]));

        assert!(claims.has_all_roles(&[Role::admin(), Role::user()]));
        assert!(!claims.has_all_roles(&[Role::admin(), Role::new("moderator")]));
    }

    #[test]
    fn test_claims_with_email_and_name() {
        let claims = Claims::new("user-123", 0, 0)
            .with_email("user@example.com")
            .with_name("John Doe");

        assert_eq!(claims.email, Some("user@example.com".to_string()));
        assert_eq!(claims.name, Some("John Doe".to_string()));
    }

    #[test]
    fn test_create_and_verify_access_token() {
        let manager = create_test_jwt_manager();
        let claims = Claims::new("user-456", 0, 0)
            .with_role(Role::user())
            .with_email("test@example.com");

        let token = manager
            .create_access_token(claims)
            .expect("Failed to create access token");

        assert!(!token.is_empty());

        let verified_claims = manager
            .verify_access_token(&token)
            .expect("Failed to verify access token");

        assert_eq!(verified_claims.sub, "user-456");
        assert!(verified_claims.has_role(&Role::user()));
        assert_eq!(verified_claims.email, Some("test@example.com".to_string()));
        assert!(!verified_claims.is_refresh_token());
    }

    #[test]
    fn test_create_and_verify_refresh_token() {
        let manager = create_test_jwt_manager();
        let claims = Claims::new("user-789", 0, 0);

        let token = manager
            .create_refresh_token(claims)
            .expect("Failed to create refresh token");

        let verified_claims = manager
            .verify_refresh_token(&token)
            .expect("Failed to verify refresh token");

        assert_eq!(verified_claims.sub, "user-789");
        assert!(verified_claims.is_refresh_token());
    }

    #[test]
    fn test_access_token_rejected_as_refresh_token() {
        let manager = create_test_jwt_manager();
        let claims = Claims::new("user-123", 0, 0);

        let access_token = manager
            .create_access_token(claims)
            .expect("Failed to create access token");

        let result = manager.verify_refresh_token(&access_token);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot be used as refresh token"));
    }

    #[test]
    fn test_refresh_token_rejected_as_access_token() {
        let manager = create_test_jwt_manager();
        let claims = Claims::new("user-123", 0, 0);

        let refresh_token = manager
            .create_refresh_token(claims)
            .expect("Failed to create refresh token");

        let result = manager.verify_access_token(&refresh_token);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot be used as access token"));
    }

    #[test]
    fn test_refresh_tokens_flow() {
        let manager = create_test_jwt_manager();
        let claims = Claims::new("user-refresh", 0, 0)
            .with_role(Role::admin())
            .with_email("admin@example.com")
            .with_name("Admin User");

        let initial_refresh = manager
            .create_refresh_token(claims)
            .expect("Failed to create initial refresh token");

        let (new_access, new_refresh) = manager
            .refresh_tokens(&initial_refresh)
            .expect("Failed to refresh tokens");

        let access_claims = manager
            .verify_access_token(&new_access)
            .expect("Failed to verify new access token");
        assert_eq!(access_claims.sub, "user-refresh");
        assert!(access_claims.has_role(&Role::admin()));
        assert_eq!(access_claims.email, Some("admin@example.com".to_string()));

        let refresh_claims = manager
            .verify_refresh_token(&new_refresh)
            .expect("Failed to verify new refresh token");
        assert_eq!(refresh_claims.sub, "user-refresh");
    }

    #[test]
    fn test_invalid_token_verification() {
        let manager = create_test_jwt_manager();
        let result = manager.verify_token("invalid.token.here");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid token"));
    }

    #[test]
    fn test_token_with_wrong_secret_rejected() {
        let manager1 = JwtManager::new(JwtConfig::new("secret-one-long-enough-for-jwt"));
        let manager2 = JwtManager::new(JwtConfig::new("secret-two-long-enough-for-jwt"));

        let claims = Claims::new("cross-secret-test", 0, 0);
        let token = manager1
            .create_access_token(claims)
            .expect("Failed to create token");

        let result = manager2.verify_token(&token);
        assert!(result.is_err());
    }
}

mod test_caching {
    use rust_boot::prelude::*;
    use serde::{Deserialize, Serialize};
    use std::time::Duration;

    fn create_test_cache() -> MokaBackend {
        let config = CacheConfig::new("test-cache")
            .with_ttl(Duration::from_secs(300))
            .with_max_capacity(1000);
        MokaBackend::new(config)
    }

    #[tokio::test]
    async fn test_cache_config_creation() {
        let config = CacheConfig::default();
        assert_eq!(config.name, "default");
        assert_eq!(config.default_ttl, Duration::from_secs(300));
        assert_eq!(config.max_capacity, 10_000);
    }

    #[tokio::test]
    async fn test_cache_config_builder() {
        let config = CacheConfig::new("my-cache")
            .with_ttl(Duration::from_secs(60))
            .with_max_capacity(500);

        assert_eq!(config.name, "my-cache");
        assert_eq!(config.default_ttl, Duration::from_secs(60));
        assert_eq!(config.max_capacity, 500);
    }

    #[tokio::test]
    async fn test_moka_backend_set_and_get() {
        let cache = create_test_cache();

        cache
            .set("key1", b"value1".to_vec(), None)
            .await
            .expect("Failed to set value");

        let value = cache.get("key1").await.expect("Failed to get value");
        assert_eq!(value, Some(b"value1".to_vec()));
    }

    #[tokio::test]
    async fn test_moka_backend_get_nonexistent() {
        let cache = create_test_cache();
        let value = cache.get("nonexistent").await.expect("Failed to get value");
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_moka_backend_exists() {
        let cache = create_test_cache();

        assert!(
            !cache.exists("key").await.expect("Failed to check existence")
        );

        cache
            .set("key", b"value".to_vec(), None)
            .await
            .expect("Failed to set value");

        assert!(cache.exists("key").await.expect("Failed to check existence"));
    }

    #[tokio::test]
    async fn test_moka_backend_delete() {
        let cache = create_test_cache();

        cache
            .set("to-delete", b"value".to_vec(), None)
            .await
            .expect("Failed to set value");

        let deleted = cache.delete("to-delete").await.expect("Failed to delete");
        assert!(deleted);

        let exists = cache
            .exists("to-delete")
            .await
            .expect("Failed to check existence");
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_moka_backend_delete_nonexistent() {
        let cache = create_test_cache();
        let deleted = cache.delete("nonexistent").await.expect("Failed to delete");
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_moka_backend_clear() {
        let cache = create_test_cache();

        cache
            .set("key1", b"value1".to_vec(), None)
            .await
            .expect("Failed to set value");
        cache
            .set("key2", b"value2".to_vec(), None)
            .await
            .expect("Failed to set value");

        cache.clear().await.expect("Failed to clear cache");

        assert!(
            !cache
                .exists("key1")
                .await
                .expect("Failed to check existence")
        );
        assert!(
            !cache
                .exists("key2")
                .await
                .expect("Failed to check existence")
        );
    }

    #[tokio::test]
    async fn test_moka_backend_overwrite() {
        let cache = create_test_cache();

        cache
            .set("key", b"value1".to_vec(), None)
            .await
            .expect("Failed to set value");
        cache
            .set("key", b"value2".to_vec(), None)
            .await
            .expect("Failed to set value");

        let value = cache.get("key").await.expect("Failed to get value");
        assert_eq!(value, Some(b"value2".to_vec()));
    }

    #[tokio::test]
    async fn test_cache_with_typed_data() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct User {
            id: u64,
            name: String,
            email: String,
        }

        let cache = create_test_cache();
        let user = User {
            id: 1,
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
        };

        let serialized = serde_json::to_vec(&user).expect("Failed to serialize");
        cache
            .set("user:1", serialized, None)
            .await
            .expect("Failed to set value");

        let retrieved = cache.get("user:1").await.expect("Failed to get value");
        let deserialized: User =
            serde_json::from_slice(&retrieved.expect("Value not found")).expect("Failed to deserialize");

        assert_eq!(deserialized, user);
    }

    #[tokio::test]
    async fn test_caching_plugin_creation() {
        let config = CacheConfig::new("plugin-cache");
        let plugin = CachingPlugin::new(config);
        assert!(plugin.backend().is_none());
    }

    #[tokio::test]
    async fn test_caching_plugin_build() {
        let config = CacheConfig::new("plugin-cache");
        let mut plugin = CachingPlugin::new(config);
        let mut ctx = PluginContext::new();

        plugin.build(&mut ctx).await.expect("Failed to build plugin");

        assert!(plugin.backend().is_some());
        assert!(ctx.contains("cache_backend").await);
    }

    #[tokio::test]
    async fn test_caching_plugin_cleanup() {
        let config = CacheConfig::new("plugin-cache");
        let mut plugin = CachingPlugin::new(config);
        let mut ctx = PluginContext::new();

        plugin.build(&mut ctx).await.expect("Failed to build plugin");
        plugin
            .cleanup(&mut ctx)
            .await
            .expect("Failed to cleanup plugin");

        assert!(plugin.backend().is_none());
        assert!(!ctx.contains("cache_backend").await);
    }
}

mod test_event_sourcing {
    use rust_boot::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct UserCreated {
        user_id: String,
        username: String,
    }

    impl DomainEvent for UserCreated {
        fn event_type(&self) -> &'static str {
            "UserCreated"
        }

        fn aggregate_type(&self) -> &'static str {
            "User"
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct UserUpdated {
        user_id: String,
        new_email: String,
    }

    impl DomainEvent for UserUpdated {
        fn event_type(&self) -> &'static str {
            "UserUpdated"
        }

        fn aggregate_type(&self) -> &'static str {
            "User"
        }
    }

    #[tokio::test]
    async fn test_in_memory_event_store_creation() {
        let store = InMemoryEventStore::new();
        let events = store
            .load("nonexistent")
            .await
            .expect("Failed to load events");
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_in_memory_event_store_get_latest_version_empty() {
        let store = InMemoryEventStore::new();

        let version = store
            .get_latest_version("aggregate-1")
            .await
            .expect("Failed to get latest version");
        assert!(version.is_none());
    }

    #[test]
    fn test_event_metadata_creation() {
        let metadata = EventMetadata::new("agg-123", "User", "UserCreated", 1);

        assert_eq!(metadata.aggregate_id, "agg-123");
        assert_eq!(metadata.aggregate_type, "User");
        assert_eq!(metadata.event_type, "UserCreated");
        assert_eq!(metadata.version, 1);
        assert!(!metadata.event_id.is_empty());
        assert!(metadata.timestamp > 0);
        assert!(metadata.correlation_id.is_none());
        assert!(metadata.causation_id.is_none());
        assert!(metadata.user_id.is_none());
    }

    #[test]
    fn test_event_metadata_with_context() {
        let metadata = EventMetadata::new("agg-123", "User", "UserCreated", 1)
            .with_correlation_id("corr-456")
            .with_causation_id("cause-789")
            .with_user_id("user-001");

        assert_eq!(metadata.correlation_id, Some("corr-456".to_string()));
        assert_eq!(metadata.causation_id, Some("cause-789".to_string()));
        assert_eq!(metadata.user_id, Some("user-001".to_string()));
    }

    #[test]
    fn test_event_envelope_creation() {
        let event = UserCreated {
            user_id: "user-123".to_string(),
            username: "testuser".to_string(),
        };

        let envelope = EventEnvelope::new("agg-456", 1, event);

        assert_eq!(envelope.metadata.aggregate_id, "agg-456");
        assert_eq!(envelope.metadata.version, 1);
        assert_eq!(envelope.metadata.event_type, "UserCreated");
        assert_eq!(envelope.metadata.aggregate_type, "User");
        assert_eq!(envelope.payload.user_id, "user-123");
        assert_eq!(envelope.payload.username, "testuser");
    }

    #[test]
    fn test_event_envelope_serialization() {
        let event = UserCreated {
            user_id: "user-789".to_string(),
            username: "serialized_user".to_string(),
        };

        let envelope = EventEnvelope::new("agg-serialize", 1, event);

        let json = serde_json::to_string(&envelope).expect("Failed to serialize");
        assert!(json.contains("UserCreated"));
        assert!(json.contains("agg-serialize"));
        assert!(json.contains("serialized_user"));

        let deserialized: EventEnvelope<UserCreated> =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.metadata.aggregate_id, "agg-serialize");
        assert_eq!(deserialized.payload.username, "serialized_user");
    }
}

mod test_config {
    use rust_boot::prelude::*;

    #[test]
    fn test_default_config() {
        let config = RustBootConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.database.url, "sqlite::memory:");
        assert_eq!(config.database.max_connections, 10);
        assert_eq!(config.database.min_connections, 1);
        assert!(config.plugins.is_empty());
    }

    #[test]
    fn test_server_config() {
        let server = ServerConfig::default();
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.port, 3000);

        let custom = ServerConfig::new("0.0.0.0".to_string(), 8080);
        assert_eq!(custom.host, "0.0.0.0");
        assert_eq!(custom.port, 8080);
    }

    #[test]
    fn test_database_config() {
        let db = DatabaseConfig::default();
        assert_eq!(db.url, "sqlite::memory:");
        assert_eq!(db.max_connections, 10);
        assert_eq!(db.min_connections, 1);

        let custom = DatabaseConfig::new("postgres://localhost/mydb".to_string(), 50, 5);
        assert_eq!(custom.url, "postgres://localhost/mydb");
        assert_eq!(custom.max_connections, 50);
        assert_eq!(custom.min_connections, 5);
    }

    #[test]
    fn test_config_builder_server() {
        let config = RustBootConfig::builder()
            .server_host("192.168.1.1".to_string())
            .server_port(9000)
            .build();

        assert_eq!(config.server.host, "192.168.1.1");
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.database.url, "sqlite::memory:");
    }

    #[test]
    fn test_config_builder_database() {
        let config = RustBootConfig::builder()
            .database_url("mysql://localhost/app".to_string())
            .database_max_connections(100)
            .database_min_connections(10)
            .build();

        assert_eq!(config.database.url, "mysql://localhost/app");
        assert_eq!(config.database.max_connections, 100);
        assert_eq!(config.database.min_connections, 10);
    }

    #[test]
    fn test_config_builder_with_plugins() {
        let config = RustBootConfig::builder()
            .plugin(
                "cache".to_string(),
                serde_json::json!({"ttl": 300, "enabled": true}),
            )
            .plugin(
                "auth".to_string(),
                serde_json::json!({"secret": "my-secret"}),
            )
            .build();

        assert_eq!(config.plugins.len(), 2);
        assert!(config.plugins.contains_key("cache"));
        assert!(config.plugins.contains_key("auth"));
        assert_eq!(config.plugins["cache"]["ttl"], 300);
    }

    #[test]
    fn test_config_builder_complete() {
        let config = RustBootConfig::builder()
            .server_host("0.0.0.0".to_string())
            .server_port(8080)
            .database_url("postgres://db.example.com/production".to_string())
            .database_max_connections(200)
            .database_min_connections(20)
            .plugin(
                "monitoring".to_string(),
                serde_json::json!({"enabled": true}),
            )
            .build();

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.url, "postgres://db.example.com/production");
        assert_eq!(config.database.max_connections, 200);
        assert_eq!(config.database.min_connections, 20);
        assert!(config.plugins.contains_key("monitoring"));
    }

    #[test]
    fn test_config_serialization() {
        let config = RustBootConfig::builder()
            .server_host("test-host".to_string())
            .server_port(4000)
            .build();

        let json = serde_json::to_string(&config).expect("Failed to serialize config");
        let deserialized: RustBootConfig =
            serde_json::from_str(&json).expect("Failed to deserialize config");

        assert_eq!(config.server.host, deserialized.server.host);
        assert_eq!(config.server.port, deserialized.server.port);
    }
}

mod test_cross_crate_integration {
    use async_trait::async_trait;
    use rust_boot::prelude::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_full_plugin_lifecycle_with_caching() {
        let mut registry = PluginRegistry::new();

        let cache_config = CacheConfig::new("integration-cache");
        let caching_plugin = CachingPlugin::new(cache_config);

        registry
            .register(caching_plugin)
            .expect("Failed to register caching plugin");

        registry.init_all().await.expect("init_all failed");

        let backend: Option<Arc<dyn CacheBackend>> =
            registry.context().get("cache_backend").await;
        assert!(backend.is_some());

        let cache = backend.expect("Cache backend not found");
        cache
            .set("integration-key", b"integration-value".to_vec(), None)
            .await
            .expect("Failed to set cache value");

        let value = cache
            .get("integration-key")
            .await
            .expect("Failed to get cache value");
        assert_eq!(value, Some(b"integration-value".to_vec()));

        registry.cleanup_all().await.expect("cleanup_all failed");

        let backend_after: Option<Arc<dyn CacheBackend>> =
            registry.context().get("cache_backend").await;
        assert!(backend_after.is_none());
    }

    #[test]
    fn test_auth_with_role_based_access() {
        let jwt_manager = JwtManager::new(JwtConfig::new(
            "integration-secret-key-long-enough-for-jwt",
        ));

        let admin_claims = Claims::new("admin-user", 0, 0)
            .with_role(Role::admin())
            .with_role(Role::user())
            .with_email("admin@example.com");

        let user_claims = Claims::new("regular-user", 0, 0)
            .with_role(Role::user())
            .with_email("user@example.com");

        let admin_token = jwt_manager
            .create_access_token(admin_claims)
            .expect("Failed to create admin token");
        let user_token = jwt_manager
            .create_access_token(user_claims)
            .expect("Failed to create user token");

        let verified_admin = jwt_manager
            .verify_access_token(&admin_token)
            .expect("Failed to verify admin token");
        let verified_user = jwt_manager
            .verify_access_token(&user_token)
            .expect("Failed to verify user token");

        assert!(verified_admin.has_role(&Role::admin()));
        assert!(verified_admin.has_role(&Role::user()));
        assert!(verified_admin.has_all_roles(&[Role::admin(), Role::user()]));

        assert!(!verified_user.has_role(&Role::admin()));
        assert!(verified_user.has_role(&Role::user()));
    }

    #[tokio::test]
    async fn test_plugin_dependency_chain_with_context() {
        struct DatabasePlugin {
            connection_string: String,
        }

        #[async_trait]
        impl CrudPlugin for DatabasePlugin {
            fn meta(&self) -> PluginMeta {
                PluginMeta::new("database", "1.0.0")
            }

            async fn build(&mut self, ctx: &mut PluginContext) -> RustBootResult<()> {
                ctx.insert("db_connection", self.connection_string.clone())
                    .await;
                Ok(())
            }
        }

        struct CacheWithDbPlugin;

        #[async_trait]
        impl CrudPlugin for CacheWithDbPlugin {
            fn meta(&self) -> PluginMeta {
                PluginMeta::with_dependencies("cache-with-db", "1.0.0", vec!["database".to_string()])
            }

            async fn build(&mut self, ctx: &mut PluginContext) -> RustBootResult<()> {
                let db_conn: Option<String> = ctx.get("db_connection").await;
                if db_conn.is_none() {
                    return Err(RustBootError::Plugin(
                        "Database connection not found".to_string(),
                    ));
                }
                ctx.insert("cache_ready", true).await;
                Ok(())
            }
        }

        let mut registry = PluginRegistry::new();

        registry
            .register(DatabasePlugin {
                connection_string: "postgres://localhost/test".to_string(),
            })
            .expect("Failed to register database plugin");

        registry
            .register(CacheWithDbPlugin)
            .expect("Failed to register cache plugin");

        registry.init_all().await.expect("init_all failed");

        let db_conn: Option<String> = registry.context().get("db_connection").await;
        assert_eq!(db_conn, Some("postgres://localhost/test".to_string()));

        let cache_ready: Option<bool> = registry.context().get("cache_ready").await;
        assert_eq!(cache_ready, Some(true));
    }

    #[tokio::test]
    async fn test_event_sourcing_plugin_integration() {
        use rust_boot::plugins::EventSourcingPlugin;
        use rust_boot::prelude::CrudPlugin;

        let mut plugin = EventSourcingPlugin::new();
        let mut ctx = PluginContext::new();

        assert!(plugin.store().is_none());

        plugin.build(&mut ctx).await.expect("Failed to build plugin");

        assert!(plugin.store().is_some());

        plugin.cleanup(&mut ctx).await.expect("Failed to cleanup plugin");

        assert!(plugin.store().is_none());
    }

    #[test]
    fn test_domain_event_and_envelope_creation() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct OrderPlaced {
            order_id: String,
            total: f64,
        }

        impl DomainEvent for OrderPlaced {
            fn event_type(&self) -> &'static str {
                "OrderPlaced"
            }

            fn aggregate_type(&self) -> &'static str {
                "Order"
            }
        }

        let event = OrderPlaced {
            order_id: "order-123".to_string(),
            total: 99.99,
        };

        let envelope = EventEnvelope::new("order-aggregate-1", 1, event);

        assert_eq!(envelope.metadata.aggregate_id, "order-aggregate-1");
        assert_eq!(envelope.metadata.version, 1);
        assert_eq!(envelope.metadata.event_type, "OrderPlaced");
        assert_eq!(envelope.metadata.aggregate_type, "Order");
        assert_eq!(envelope.payload.order_id, "order-123");
        assert!((envelope.payload.total - 99.99).abs() < f64::EPSILON);
    }

    #[test]
    fn test_config_with_plugin_integration() {
        let config = RustBootConfig::builder()
            .server_host("0.0.0.0".to_string())
            .server_port(8080)
            .database_url("postgres://localhost/app".to_string())
            .plugin(
                "cache".to_string(),
                serde_json::json!({
                    "ttl_seconds": 300,
                    "max_capacity": 10000
                }),
            )
            .plugin(
                "auth".to_string(),
                serde_json::json!({
                    "jwt_secret": "production-secret",
                    "access_token_ttl_minutes": 15
                }),
            )
            .plugin(
                "monitoring".to_string(),
                serde_json::json!({
                    "metrics_enabled": true,
                    "health_check_path": "/health"
                }),
            )
            .build();

        assert!(config.plugins.contains_key("cache"));
        assert!(config.plugins.contains_key("auth"));
        assert!(config.plugins.contains_key("monitoring"));

        assert_eq!(config.plugins["cache"]["ttl_seconds"], 300);
        assert_eq!(config.plugins["auth"]["access_token_ttl_minutes"], 15);
        assert_eq!(config.plugins["monitoring"]["health_check_path"], "/health");
    }
}
