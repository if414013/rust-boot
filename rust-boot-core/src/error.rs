//! Error types for the rust-boot framework.
//!
//! This module provides a comprehensive error type system using the `thiserror` crate
//! for ergonomic error handling across the framework.

use std::error::Error;
use std::fmt::{self, Display};

/// The main error type for rust-boot framework operations.
///
/// `RustBootError` encapsulates all error cases that can occur in the framework,
/// organized by category. Each variant includes context-specific information.
///
/// # Examples
///
/// ```
/// use rust_boot_core::error::{RustBootError, Result};
///
/// fn parse_config() -> Result<()> {
///     Err(RustBootError::Config(
///         "Invalid database URL format".to_string()
///     ))
/// }
/// ```
#[derive(Debug)]
pub enum RustBootError {
    /// Configuration-related errors (invalid settings, missing required config, etc.)
    Config(String),

    /// Database or ORM-related errors (query failures, connection issues, etc.)
    Database(String),

    /// Plugin lifecycle or loading errors (failed initialization, missing dependencies, etc.)
    Plugin(String),

    /// Input validation errors (invalid format, constraints violated, etc.)
    Validation(String),

    /// Serialization/deserialization errors (JSON encoding issues, etc.)
    Serialization(String),

    /// HTTP or request-related errors (bad request, not found, etc.)
    Http(u16, String),

    /// Cache operation errors (eviction failures, backend issues, etc.)
    Cache(String),

    /// Authentication or authorization errors (invalid token, insufficient permissions, etc.)
    Auth(String),

    /// Unexpected internal errors that should not occur in normal operation
    Internal(String),
}

impl Display for RustBootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RustBootError::Config(msg) => write!(f, "configuration error: {}", msg),
            RustBootError::Database(msg) => write!(f, "database error: {}", msg),
            RustBootError::Plugin(msg) => write!(f, "plugin error: {}", msg),
            RustBootError::Validation(msg) => write!(f, "validation error: {}", msg),
            RustBootError::Serialization(msg) => write!(f, "serialization error: {}", msg),
            RustBootError::Http(status, msg) => write!(f, "HTTP {} error: {}", status, msg),
            RustBootError::Cache(msg) => write!(f, "cache error: {}", msg),
            RustBootError::Auth(msg) => write!(f, "authentication error: {}", msg),
            RustBootError::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl Error for RustBootError {}

/// A specialized `Result` type for rust-boot operations.
///
/// This is an alias for `std::result::Result<T, RustBootError>` to reduce boilerplate
/// in function signatures across the framework.
///
/// # Examples
///
/// ```
/// use rust_boot_core::error::Result;
///
/// fn load_config(path: &str) -> Result<String> {
///     Ok("config".to_string())
/// }
/// ```
pub type Result<T> = std::result::Result<T, RustBootError>;

/// Conversion from `std::io::Error` to `RustBootError`.
///
/// Maps IO errors to database errors since they typically occur during database operations.
impl From<std::io::Error> for RustBootError {
    fn from(err: std::io::Error) -> Self {
        RustBootError::Database(format!("IO error: {}", err))
    }
}

/// Conversion from `serde_json::Error` to `RustBootError`.
///
/// Maps JSON serialization errors to `RustBootError::Serialization`.
impl From<serde_json::Error> for RustBootError {
    fn from(err: serde_json::Error) -> Self {
        RustBootError::Serialization(format!("JSON error: {}", err))
    }
}

/// Conversion from `config::ConfigError` to `RustBootError`.
///
/// Maps configuration crate errors to `RustBootError::Config`.
impl From<config::ConfigError> for RustBootError {
    fn from(err: config::ConfigError) -> Self {
        RustBootError::Config(format!("Config crate error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_display() {
        let err = RustBootError::Config("missing database url".to_string());
        assert_eq!(err.to_string(), "configuration error: missing database url");
    }

    #[test]
    fn test_database_error_display() {
        let err = RustBootError::Database("connection timeout".to_string());
        assert_eq!(err.to_string(), "database error: connection timeout");
    }

    #[test]
    fn test_plugin_error_display() {
        let err = RustBootError::Plugin("failed to initialize plugin".to_string());
        assert_eq!(err.to_string(), "plugin error: failed to initialize plugin");
    }

    #[test]
    fn test_validation_error_display() {
        let err = RustBootError::Validation("email format invalid".to_string());
        assert_eq!(err.to_string(), "validation error: email format invalid");
    }

    #[test]
    fn test_serialization_error_display() {
        let err = RustBootError::Serialization("unexpected type".to_string());
        assert_eq!(err.to_string(), "serialization error: unexpected type");
    }

    #[test]
    fn test_http_error_display() {
        let err = RustBootError::Http(404, "not found".to_string());
        assert_eq!(err.to_string(), "HTTP 404 error: not found");
    }

    #[test]
    fn test_http_error_various_status_codes() {
        let err_400 = RustBootError::Http(400, "bad request".to_string());
        assert_eq!(err_400.to_string(), "HTTP 400 error: bad request");

        let err_500 = RustBootError::Http(500, "internal server error".to_string());
        assert_eq!(err_500.to_string(), "HTTP 500 error: internal server error");
    }

    #[test]
    fn test_cache_error_display() {
        let err = RustBootError::Cache("eviction failed".to_string());
        assert_eq!(err.to_string(), "cache error: eviction failed");
    }

    #[test]
    fn test_auth_error_display() {
        let err = RustBootError::Auth("invalid token".to_string());
        assert_eq!(err.to_string(), "authentication error: invalid token");
    }

    #[test]
    fn test_internal_error_display() {
        let err = RustBootError::Internal("unexpected state".to_string());
        assert_eq!(err.to_string(), "internal error: unexpected state");
    }

    #[test]
    fn test_error_trait_implementation() {
        let err: Box<dyn Error> = Box::new(RustBootError::Config("test".to_string()));
        assert_eq!(err.to_string(), "configuration error: test");
    }

    #[test]
    fn test_result_type_with_ok() {
        let result: Result<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_with_err() {
        let result: Result<i32> = Err(RustBootError::Internal("test error".to_string()));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "internal error: test error"
        );
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let rust_boot_err: RustBootError = io_err.into();
        let error_string = rust_boot_err.to_string();
        assert!(error_string.starts_with("database error: IO error:"));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_str = r#"{"invalid json"#;
        let json_err: serde_json::Result<serde_json::Value> = serde_json::from_str(json_str);
        if let Err(e) = json_err {
            let rust_boot_err: RustBootError = e.into();
            let error_string = rust_boot_err.to_string();
            assert!(error_string.starts_with("serialization error: JSON error:"));
        } else {
            panic!("Expected JSON parsing to fail");
        }
    }

    #[test]
    fn test_from_config_error() {
        // Create a config::ConfigError by attempting to read a non-existent config file
        let config_err = config::Config::builder()
            .add_source(config::File::with_name("/nonexistent/path"))
            .build()
            .unwrap_err();
        let rust_boot_err: RustBootError = config_err.into();
        let error_string = rust_boot_err.to_string();
        assert!(error_string.starts_with("configuration error: Config crate error:"));
    }

    #[test]
    fn test_error_chain_with_result() {
        fn inner_func() -> Result<String> {
            Err(RustBootError::Validation("invalid input".to_string()))
        }

        fn outer_func() -> Result<String> {
            let _result = inner_func()?;
            Ok("success".to_string())
        }

        let result = outer_func();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "validation error: invalid input"
        );
    }

    #[test]
    fn test_error_debug_display() {
        let err = RustBootError::Plugin("plugin load error".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Plugin"));
        assert!(debug_str.contains("plugin load error"));
    }

    #[test]
    fn test_from_trait_in_question_mark_context() {
        fn json_to_error() -> Result<()> {
            let json_str = r#"{"valid": true}"#;
            let _: serde_json::Value = serde_json::from_str(json_str)?;
            Ok(())
        }

        assert!(json_to_error().is_ok());

        fn json_to_error_invalid() -> Result<()> {
            let json_str = r#"{"invalid json"#;
            let _: serde_json::Value = serde_json::from_str(json_str)?;
            Ok(())
        }

        let result = json_to_error_invalid();
        assert!(result.is_err());
    }

    #[test]
    fn test_result_map_operations() {
        let result: Result<i32> = Ok(10);
        let mapped = result.map(|x| x * 2);
        assert_eq!(mapped.unwrap(), 20);
    }

    #[test]
    fn test_result_map_err_operations() {
        let result: Result<i32> = Err(RustBootError::Internal("error".to_string()));
        let mapped = result.map_err(|_| RustBootError::Validation("mapped error".to_string()));
        assert_eq!(
            mapped.unwrap_err().to_string(),
            "validation error: mapped error"
        );
    }

    #[test]
    fn test_result_or_operations() {
        let result: Result<i32> = Err(RustBootError::Internal("error".to_string()));
        let fallback: Result<i32> = Ok(42);
        assert_eq!(result.or(fallback).unwrap(), 42);
    }

    #[test]
    fn test_result_unwrap_or_operations() {
        let result: Result<i32> = Err(RustBootError::Internal("error".to_string()));
        assert_eq!(result.unwrap_or(99), 99);
    }

    #[test]
    fn test_result_and_then_chaining() {
        fn step1() -> Result<i32> {
            Ok(5)
        }

        fn step2(val: i32) -> Result<i32> {
            Ok(val * 2)
        }

        let result = step1().and_then(step2);
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn test_all_error_variants_can_be_created() {
        let _config = RustBootError::Config("test".to_string());
        let _db = RustBootError::Database("test".to_string());
        let _plugin = RustBootError::Plugin("test".to_string());
        let _validation = RustBootError::Validation("test".to_string());
        let _serialization = RustBootError::Serialization("test".to_string());
        let _http = RustBootError::Http(500, "test".to_string());
        let _cache = RustBootError::Cache("test".to_string());
        let _auth = RustBootError::Auth("test".to_string());
        let _internal = RustBootError::Internal("test".to_string());
    }
}
