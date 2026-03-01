//! Generate command implementation for scaffolding entities.

use crate::templates::TemplateEngine;
use anyhow::{Context, Result};
use clap::Args;
use tera::Context as TeraContext;

#[derive(Args, Debug)]
pub struct GenerateCommand {
    /// Type of artifact to generate (model, handler, etc.)
    #[arg(value_name = "TYPE")]
    pub artifact_type: String,

    /// Name of the artifact (e.g., User, Product)
    #[arg(value_name = "NAME")]
    pub name: String,
}

#[derive(Debug)]
pub struct GeneratedModel {
    pub model_name: String,
    pub file_name: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactType {
    Model,
}

impl ArtifactType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "model" | "entity" => Ok(Self::Model),
            _ => anyhow::bail!("Unknown artifact type: {s}. Available types: model, entity"),
        }
    }
}

impl GenerateCommand {
    pub fn execute(&self) -> Result<GeneratedModel> {
        let artifact_type = ArtifactType::from_str(&self.artifact_type)?;

        match artifact_type {
            ArtifactType::Model => self.generate_model(),
        }
    }

    fn generate_model(&self) -> Result<GeneratedModel> {
        let engine = TemplateEngine::new()?;

        let model_name = to_pascal_case(&self.name);
        let model_name_snake = to_snake_case(&self.name);

        let mut context = TeraContext::new();
        context.insert("model_name", &model_name);
        context.insert("model_name_snake", &model_name_snake);

        let rendered_content = engine
            .render("model", &context)
            .context("Failed to render model template")?;

        let file_name = format!("{model_name_snake}.rs");

        Ok(GeneratedModel {
            model_name,
            file_name,
            content: rendered_content,
        })
    }
}

fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect::<String>()
            })
        })
        .collect()
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_was_upper = false;

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !prev_was_upper {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
            prev_was_upper = true;
        } else if c == '-' || c == ' ' {
            result.push('_');
            prev_was_upper = false;
        } else {
            result.push(c);
            prev_was_upper = false;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_model() {
        let cmd = GenerateCommand {
            artifact_type: "model".to_string(),
            name: "User".to_string(),
        };

        let result = cmd.execute();
        assert!(result.is_ok());

        let model = result.unwrap();
        assert_eq!(model.model_name, "User");
        assert_eq!(model.file_name, "user.rs");
        assert!(model.content.contains("struct User"));
        assert!(model.content.contains("CrudModel"));
    }

    #[test]
    fn test_generate_entity_alias() {
        let cmd = GenerateCommand {
            artifact_type: "entity".to_string(),
            name: "Product".to_string(),
        };

        let result = cmd.execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_artifact_type() {
        let cmd = GenerateCommand {
            artifact_type: "unknown".to_string(),
            name: "Test".to_string(),
        };

        let result = cmd.execute();
        assert!(result.is_err());
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("user"), "User");
        assert_eq!(to_pascal_case("user_profile"), "UserProfile");
        assert_eq!(to_pascal_case("user-profile"), "UserProfile");
        assert_eq!(to_pascal_case("UserProfile"), "UserProfile");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("User"), "user");
        assert_eq!(to_snake_case("UserProfile"), "user_profile");
        assert_eq!(to_snake_case("user-profile"), "user_profile");
        assert_eq!(to_snake_case("user_profile"), "user_profile");
    }
}
