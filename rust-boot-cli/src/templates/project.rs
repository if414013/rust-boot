//! Project template definitions for rust-boot scaffolding.

pub const CARGO_TOML_TEMPLATE: &str = r#"[package]
name = "{{ project_name }}"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[dependencies]
rust-boot = "{{ rust_boot_version }}"
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1.0"
"#;

pub const MAIN_RS_TEMPLATE: &str = r#"//! {{ project_name }} - A rust-boot application.

use anyhow::Result;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting {{ project_name }}...");

    // TODO: Add your rust-boot application logic here
    // Example:
    // let app = rust_boot::Application::builder()
    //     .with_config("config/application.toml")
    //     .build()
    //     .await?;
    //
    // app.run().await?;

    Ok(())
}
"#;

pub const LIB_RS_TEMPLATE: &str = r"//! {{ project_name }} library module.
//!
//! This module contains the core logic for your rust-boot application.

pub mod models;
pub mod handlers;

/// Re-export commonly used types
pub use models::*;
";

pub const GITIGNORE_TEMPLATE: &str = r"/target
Cargo.lock
**/*.rs.bk
*.pdb
.env
.env.local
*.log
.DS_Store
";

pub const MODEL_TEMPLATE: &str = r"//! {{ model_name }} model definition.

use serde::{Deserialize, Serialize};
// TODO: Uncomment when rust-boot-macros is available
// use rust_boot_macros::CrudModel;

/// {{ model_name }} entity.
///
/// This model represents a {{ model_name_snake }} in the database.
// TODO: Uncomment when rust-boot-macros is available
// #[derive(CrudModel)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{ model_name }} {
    /// Unique identifier
    pub id: i64,

    // TODO: Add your fields here
    // pub name: String,
    // pub created_at: chrono::DateTime<chrono::Utc>,
}

impl {{ model_name }} {
    /// Creates a new {{ model_name }} instance.
    #[must_use]
    pub fn new(id: i64) -> Self {
        Self { id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let entity = {{ model_name }}::new(1);
        assert_eq!(entity.id, 1);
    }
}
";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectTemplate {
    #[default]
    Basic,
    Full,
    Api,
}

impl ProjectTemplate {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Full => "full",
            Self::Api => "api",
        }
    }

    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "basic" => Ok(Self::Basic),
            "full" => Ok(Self::Full),
            "api" => Ok(Self::Api),
            _ => anyhow::bail!("Unknown template: {s}. Available templates: basic, full, api"),
        }
    }
}

impl std::fmt::Display for ProjectTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_template_from_str() {
        assert_eq!(
            ProjectTemplate::from_str("basic").unwrap(),
            ProjectTemplate::Basic
        );
        assert_eq!(
            ProjectTemplate::from_str("FULL").unwrap(),
            ProjectTemplate::Full
        );
        assert_eq!(
            ProjectTemplate::from_str("Api").unwrap(),
            ProjectTemplate::Api
        );
        assert!(ProjectTemplate::from_str("unknown").is_err());
    }

    #[test]
    fn test_project_template_display() {
        assert_eq!(ProjectTemplate::Basic.to_string(), "basic");
        assert_eq!(ProjectTemplate::Full.to_string(), "full");
        assert_eq!(ProjectTemplate::Api.to_string(), "api");
    }
}
