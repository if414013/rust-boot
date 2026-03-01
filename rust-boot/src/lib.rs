//! Rapid CRUD API framework inspired by Spring Boot.
//!
//! `rust-boot` is a batteries-included framework for building CRUD APIs in Rust.
//! It provides a plugin system, configuration management, caching, authentication,
//! monitoring, and seamless Axum integration.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use rust_boot::prelude::*;
//!
//! #[derive(CrudModel)]
//! struct User {
//!     #[primary_key]
//!     id: i32,
//!     name: String,
//!     email: String,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), RustBootError> {
//!     let app = Application::builder()
//!         .with_plugin(CachingPlugin::new(CacheConfig::default()))
//!         .with_plugin(MonitoringPlugin::new())
//!         .with_plugin(AuthPlugin::new(JwtConfig::default()))
//!         .build()
//!         .await?;
//!
//!     let router = CrudRouterBuilder::<User>::new()
//!         .all()
//!         .build();
//!
//!     // Start server...
//!     Ok(())
//! }
//! ```
//!
//! # Crate Structure
//!
//! - [`core`] - Core traits, plugin system, configuration, and error handling
//! - [`axum`] - Axum integration with routers and handlers
//! - [`plugins`] - Built-in plugins for caching, auth, monitoring, and events
//!
//! # Features
//!
//! - **Plugin System**: Extensible architecture with lifecycle hooks
//! - **Configuration**: Layered configuration from files and environment
//! - **Caching**: In-memory (Moka) and Redis backends
//! - **Authentication**: JWT-based auth with RBAC support
//! - **Monitoring**: Prometheus metrics and health checks
//! - **Event Sourcing**: Domain events with pluggable event stores

// Re-export core crate
pub mod core {
    //! Core traits, plugin system, shared types, and error handling.
    pub use rust_boot_core::*;
}

// Re-export axum integration
pub mod axum {
    //! Axum integration including routers, handlers, and middleware.
    pub use rust_boot_axum::*;
}

// Re-export plugins
pub mod plugins {
    //! Built-in plugins for caching, monitoring, authentication, and events.
    pub use rust_boot_plugins::*;
}

/// Convenient re-exports for common types and traits.
pub mod prelude {
    pub use rust_boot_core::config::{
        DatabaseConfig, RustBootConfig, RustBootConfigBuilder, ServerConfig,
    };
    pub use rust_boot_core::error::{Result as RustBootResult, RustBootError};
    pub use rust_boot_core::plugin::{CrudPlugin, PluginContext, PluginMeta, PluginState};
    pub use rust_boot_core::registry::PluginRegistry;
    pub use rust_boot_core::repository::CrudRepository;
    pub use rust_boot_core::service::CrudService;

    pub use rust_boot_axum::{
        created, no_content, ok, paginated, ApiError, ApiResponse, ApiResult, CrudHandlers,
        CrudRouter, CrudRouterBuilder, CrudRouterConfig, PaginatedResponse, PaginatedResult,
        PaginationQuery,
    };

    pub use rust_boot_plugins::{
        AuthPlugin, CacheBackend, CacheConfig, CachingPlugin, Claims, DomainEvent, EventEnvelope,
        EventMetadata, EventSourcingPlugin, EventStore, HealthCheck, HealthStatus,
        InMemoryEventStore, JwtConfig, JwtManager, MetricsConfig, MetricsRecorder, MokaBackend,
        MonitoringPlugin, ReadinessCheck, RedisBackend, Role,
    };

    pub use async_trait::async_trait;
    pub use serde::{Deserialize, Serialize};
    pub use uuid::Uuid;
}

pub use rust_boot_core::config::{
    DatabaseConfig, RustBootConfig, RustBootConfigBuilder, ServerConfig,
};
pub use rust_boot_core::error::{Result as RustBootResult, RustBootError};
pub use rust_boot_core::plugin::{CrudPlugin, PluginContext, PluginMeta, PluginState};
pub use rust_boot_core::registry::PluginRegistry;
pub use rust_boot_core::repository::CrudRepository;
pub use rust_boot_core::service::CrudService;

pub use rust_boot_axum::{
    created, no_content, ok, paginated, ApiError, ApiResponse, ApiResult, CrudHandlers, CrudRouter,
    CrudRouterBuilder, CrudRouterConfig, PaginatedResponse, PaginatedResult, PaginationQuery,
};

pub use rust_boot_plugins::{
    AuthPlugin, CacheBackend, CacheConfig, CachingPlugin, Claims, DomainEvent, EventEnvelope,
    EventMetadata, EventSourcingPlugin, EventStore, HealthCheck, HealthStatus, InMemoryEventStore,
    JwtConfig, JwtManager, MetricsConfig, MetricsRecorder, MokaBackend, MonitoringPlugin,
    ReadinessCheck, RedisBackend, Role,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_imports() {
        use crate::prelude::*;

        let _: fn() -> RustBootResult<()> = || Ok(());
    }

    #[test]
    fn test_top_level_imports() {
        let _config = ServerConfig::default();
        let _cache_config = CacheConfig::default();
        let _jwt_config = JwtConfig::new("test-secret");
        let _metrics_config = MetricsConfig::default();
    }

    #[test]
    fn test_module_imports() {
        use crate::core::error::RustBootError;
        use crate::plugins::CachingPlugin;

        let _ = std::any::type_name::<RustBootError>();
        let _ = std::any::type_name::<CachingPlugin>();
    }
}
