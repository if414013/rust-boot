//! JWT authentication plugin with RBAC support.
//!
//! Provides JWT token generation, verification, and role-based access control.

mod claims;
mod jwt;

pub use claims::{Claims, Role};
pub use jwt::{JwtConfig, JwtManager};

use async_trait::async_trait;
use rust_boot_core::{
    error::Result,
    plugin::{CrudPlugin, PluginContext, PluginMeta},
};
use std::sync::Arc;

/// Authentication plugin providing JWT-based authentication.
pub struct AuthPlugin {
    config: JwtConfig,
    jwt_manager: Option<Arc<JwtManager>>,
}

impl AuthPlugin {
    /// Creates a new authentication plugin with the given JWT configuration.
    pub const fn new(config: JwtConfig) -> Self {
        Self {
            config,
            jwt_manager: None,
        }
    }

    /// Returns the JWT manager instance, if the plugin has been built.
    pub fn jwt_manager(&self) -> Option<Arc<JwtManager>> {
        self.jwt_manager.clone()
    }
}

#[async_trait]
impl CrudPlugin for AuthPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("auth", "0.1.0")
    }

    async fn build(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        let manager = JwtManager::new(self.config.clone());
        self.jwt_manager = Some(Arc::new(manager));
        Ok(())
    }

    async fn ready(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    async fn finish(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    async fn cleanup(&mut self, _ctx: &mut PluginContext) -> Result<()> {
        self.jwt_manager = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_plugin_creation() {
        let config = JwtConfig::new("test-secret");
        let plugin = AuthPlugin::new(config);
        assert_eq!(plugin.meta().name, "auth");
    }

    #[tokio::test]
    async fn test_auth_plugin_jwt_manager_none_initially() {
        let config = JwtConfig::new("test-secret");
        let plugin = AuthPlugin::new(config);
        assert!(plugin.jwt_manager().is_none());
    }

    #[tokio::test]
    async fn test_auth_plugin_build() {
        let config = JwtConfig::new("test-secret-key-that-is-long-enough");
        let mut plugin = AuthPlugin::new(config);
        let mut ctx = PluginContext::new();

        plugin.build(&mut ctx).await.unwrap();
        assert!(plugin.jwt_manager().is_some());
    }
}
