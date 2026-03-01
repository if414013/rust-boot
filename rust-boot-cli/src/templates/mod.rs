//! Template management module for rust-boot CLI.

pub mod project;

use anyhow::{Context, Result};
use tera::Tera;

pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();

        tera.add_raw_template("cargo_toml", project::CARGO_TOML_TEMPLATE)
            .context("Failed to register Cargo.toml template")?;
        tera.add_raw_template("main_rs", project::MAIN_RS_TEMPLATE)
            .context("Failed to register main.rs template")?;
        tera.add_raw_template("gitignore", project::GITIGNORE_TEMPLATE)
            .context("Failed to register .gitignore template")?;
        tera.add_raw_template("lib_rs", project::LIB_RS_TEMPLATE)
            .context("Failed to register lib.rs template")?;
        tera.add_raw_template("model", project::MODEL_TEMPLATE)
            .context("Failed to register model template")?;

        Ok(Self { tera })
    }

    pub fn render(&self, template_name: &str, context: &tera::Context) -> Result<String> {
        self.tera
            .render(template_name, context)
            .with_context(|| format!("Failed to render template: {template_name}"))
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new().expect("Failed to initialize template engine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_engine_creation() {
        let engine = TemplateEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_render_cargo_toml() {
        let engine = TemplateEngine::new().unwrap();
        let mut context = tera::Context::new();
        context.insert("project_name", "my-project");
        context.insert("rust_boot_version", "0.1.0");

        let result = engine.render("cargo_toml", &context);
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("my-project"));
    }

    #[test]
    fn test_render_main_rs() {
        let engine = TemplateEngine::new().unwrap();
        let mut context = tera::Context::new();
        context.insert("project_name", "my_project");

        let result = engine.render("main_rs", &context);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_model() {
        let engine = TemplateEngine::new().unwrap();
        let mut context = tera::Context::new();
        context.insert("model_name", "User");
        context.insert("model_name_snake", "user");

        let result = engine.render("model", &context);
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("User"));
        assert!(content.contains("CrudModel"));
    }
}
