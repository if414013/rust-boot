//! New project command implementation.

use crate::templates::{project::ProjectTemplate, TemplateEngine};
use anyhow::{Context, Result};
use clap::Args;
use tera::Context as TeraContext;

#[derive(Args, Debug)]
pub struct NewCommand {
    /// Name of the project to create
    pub name: String,

    /// Template to use for project generation
    #[arg(short, long, default_value = "basic")]
    pub template: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct GeneratedProject {
    pub project_name: String,
    pub cargo_toml: String,
    pub main_rs: String,
    pub lib_rs: String,
    pub gitignore: String,
}

impl NewCommand {
    pub fn execute(&self) -> Result<GeneratedProject> {
        let _template_type = ProjectTemplate::from_str(&self.template)?;
        let engine = TemplateEngine::new()?;

        let project_name = &self.name;
        let project_name_snake = project_name.replace('-', "_");

        let mut context = TeraContext::new();
        context.insert("project_name", project_name);
        context.insert("project_name_snake", &project_name_snake);
        context.insert("rust_boot_version", "0.1.0");

        let cargo_toml = engine
            .render("cargo_toml", &context)
            .context("Failed to render Cargo.toml")?;

        let main_rs = engine
            .render("main_rs", &context)
            .context("Failed to render main.rs")?;

        let lib_rs = engine
            .render("lib_rs", &context)
            .context("Failed to render lib.rs")?;

        let gitignore = engine
            .render("gitignore", &context)
            .context("Failed to render .gitignore")?;

        Ok(GeneratedProject {
            project_name: project_name.clone(),
            cargo_toml,
            main_rs,
            lib_rs,
            gitignore,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_command_basic_template() {
        let cmd = NewCommand {
            name: "my-test-project".to_string(),
            template: "basic".to_string(),
        };

        let result = cmd.execute();
        assert!(result.is_ok());

        let project = result.unwrap();
        assert_eq!(project.project_name, "my-test-project");
        assert!(project.cargo_toml.contains("my-test-project"));
        assert!(project.main_rs.contains("my-test-project"));
        assert!(project.gitignore.contains("/target"));
    }

    #[test]
    fn test_new_command_invalid_template() {
        let cmd = NewCommand {
            name: "test".to_string(),
            template: "nonexistent".to_string(),
        };

        let result = cmd.execute();
        assert!(result.is_err());
    }

    #[test]
    fn test_project_name_with_hyphens() {
        let cmd = NewCommand {
            name: "my-cool-app".to_string(),
            template: "basic".to_string(),
        };

        let result = cmd.execute().unwrap();
        assert!(result.cargo_toml.contains("my-cool-app"));
    }
}
