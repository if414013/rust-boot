//! Configuration management for rust-boot applications.
//!
//! This module provides layered configuration support with the following precedence:
//! 1. Default values
//! 2. Configuration file (TOML, YAML, JSON)
//! 3. Environment variables (`RUST_BOOT`_* prefix)
//!
//! # Examples
//!
//! ```ignore
//! use rust_boot_core::config::RustBootConfig;
//!
//! // Create default configuration
//! let config = RustBootConfig::default();
//!
//! // Load from file with env overrides
//! let config = RustBootConfig::from_file("config.toml")?;
//!
//! // Create with builder
//! let config = RustBootConfig::builder()
//!     .server_host("0.0.0.0".to_string())
//!     .server_port(8080)
//!     .build();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host address
    pub host: String,
    /// Server port
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
        }
    }
}

impl ServerConfig {
    /// Creates a new server configuration
    pub const fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections to maintain
    pub min_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite::memory:".to_string(),
            max_connections: 10,
            min_connections: 1,
        }
    }
}

impl DatabaseConfig {
    /// Creates a new database configuration
    pub const fn new(url: String, max_connections: u32, min_connections: u32) -> Self {
        Self {
            url,
            max_connections,
            min_connections,
        }
    }
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RustBootConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Plugin-specific configuration
    #[serde(default)]
    pub plugins: HashMap<String, serde_json::Value>,
}

impl RustBootConfig {
    /// Creates a new `RustBootConfig` builder
    pub fn builder() -> RustBootConfigBuilder {
        RustBootConfigBuilder::default()
    }

    /// Load configuration from a file with environment variable overrides
    ///
    /// Supports TOML, YAML, and JSON file formats based on file extension.
    /// Environment variables with `RUST_BOOT`_ prefix override file values.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or(ConfigError::InvalidFileFormat)?;

        let config_str =
            std::fs::read_to_string(path).map_err(|e| ConfigError::FileReadError(e.to_string()))?;

        let mut config: Self = match extension {
            "toml" => {
                toml::from_str(&config_str).map_err(|e| ConfigError::ParseError(e.to_string()))?
            }
            "yaml" | "yml" => serde_yaml::from_str(&config_str)
                .map_err(|e| ConfigError::ParseError(e.to_string()))?,
            "json" => serde_json::from_str(&config_str)
                .map_err(|e| ConfigError::ParseError(e.to_string()))?,
            _ => return Err(ConfigError::InvalidFileFormat),
        };

        // Apply environment variable overrides
        config.apply_env_overrides();
        Ok(config)
    }

    /// Load configuration from environment variables only
    ///
    /// Uses default values for any configuration not set via environment variables.
    /// Environment variables must use the `RUST_BOOT`_ prefix.
    pub fn from_env() -> Self {
        let mut config = Self::default();
        config.apply_env_overrides();
        config
    }

    /// Apply environment variable overrides to the configuration
    fn apply_env_overrides(&mut self) {
        // Server configuration
        if let Ok(host) = std::env::var("RUST_BOOT_SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(port_str) = std::env::var("RUST_BOOT_SERVER_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                self.server.port = port;
            }
        }

        // Database configuration
        if let Ok(url) = std::env::var("RUST_BOOT_DATABASE_URL") {
            self.database.url = url;
        }
        if let Ok(max_conn_str) = std::env::var("RUST_BOOT_DATABASE_MAX_CONNECTIONS") {
            if let Ok(max_conn) = max_conn_str.parse::<u32>() {
                self.database.max_connections = max_conn;
            }
        }
        if let Ok(min_conn_str) = std::env::var("RUST_BOOT_DATABASE_MIN_CONNECTIONS") {
            if let Ok(min_conn) = min_conn_str.parse::<u32>() {
                self.database.min_connections = min_conn;
            }
        }
    }
}

/// Builder for `RustBootConfig`
#[derive(Default)]
pub struct RustBootConfigBuilder {
    server_host: Option<String>,
    server_port: Option<u16>,
    database_url: Option<String>,
    database_max_connections: Option<u32>,
    database_min_connections: Option<u32>,
    plugins: HashMap<String, serde_json::Value>,
}

impl RustBootConfigBuilder {
    /// Set the server host
    pub fn server_host(mut self, host: String) -> Self {
        self.server_host = Some(host);
        self
    }

    /// Set the server port
    pub const fn server_port(mut self, port: u16) -> Self {
        self.server_port = Some(port);
        self
    }

    /// Set the database URL
    pub fn database_url(mut self, url: String) -> Self {
        self.database_url = Some(url);
        self
    }

    /// Set the maximum number of database connections
    pub const fn database_max_connections(mut self, max: u32) -> Self {
        self.database_max_connections = Some(max);
        self
    }

    /// Set the minimum number of database connections
    pub const fn database_min_connections(mut self, min: u32) -> Self {
        self.database_min_connections = Some(min);
        self
    }

    /// Add a plugin configuration
    pub fn plugin(mut self, name: String, config: serde_json::Value) -> Self {
        self.plugins.insert(name, config);
        self
    }

    /// Build the configuration
    pub fn build(self) -> RustBootConfig {
        let mut config = RustBootConfig::default();

        if let Some(host) = self.server_host {
            config.server.host = host;
        }
        if let Some(port) = self.server_port {
            config.server.port = port;
        }
        if let Some(url) = self.database_url {
            config.database.url = url;
        }
        if let Some(max) = self.database_max_connections {
            config.database.max_connections = max;
        }
        if let Some(min) = self.database_min_connections {
            config.database.min_connections = min;
        }

        config.plugins = self.plugins;

        config
    }
}

/// Configuration errors
#[derive(Debug)]
pub enum ConfigError {
    /// File could not be read
    FileReadError(String),
    /// Error parsing configuration file
    ParseError(String),
    /// Invalid file format (unsupported extension)
    InvalidFileFormat,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileReadError(e) => write!(f, "Failed to read config file: {e}"),
            Self::ParseError(e) => write!(f, "Failed to parse config file: {e}"),
            Self::InvalidFileFormat => {
                write!(f, "Invalid file format. Supported: .toml, .yaml, .json")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn with_clean_env<F>(f: F)
    where
        F: FnOnce(),
    {
        let _lock = ENV_LOCK.lock().unwrap();
        std::env::remove_var("RUST_BOOT_SERVER_HOST");
        std::env::remove_var("RUST_BOOT_SERVER_PORT");
        std::env::remove_var("RUST_BOOT_DATABASE_URL");
        std::env::remove_var("RUST_BOOT_DATABASE_MAX_CONNECTIONS");
        std::env::remove_var("RUST_BOOT_DATABASE_MIN_CONNECTIONS");
        f();
        std::env::remove_var("RUST_BOOT_SERVER_HOST");
        std::env::remove_var("RUST_BOOT_SERVER_PORT");
        std::env::remove_var("RUST_BOOT_DATABASE_URL");
        std::env::remove_var("RUST_BOOT_DATABASE_MAX_CONNECTIONS");
        std::env::remove_var("RUST_BOOT_DATABASE_MIN_CONNECTIONS");
    }

    #[test]
    fn test_default_config() {
        let config = RustBootConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.database.url, "sqlite::memory:");
        assert_eq!(config.database.max_connections, 10);
        assert_eq!(config.database.min_connections, 1);
    }

    #[test]
    fn test_server_config_default() {
        let server = ServerConfig::default();
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.port, 3000);
    }

    #[test]
    fn test_server_config_new() {
        let server = ServerConfig::new("0.0.0.0".to_string(), 8080);
        assert_eq!(server.host, "0.0.0.0");
        assert_eq!(server.port, 8080);
    }

    #[test]
    fn test_database_config_default() {
        let db = DatabaseConfig::default();
        assert_eq!(db.url, "sqlite::memory:");
        assert_eq!(db.max_connections, 10);
        assert_eq!(db.min_connections, 1);
    }

    #[test]
    fn test_database_config_new() {
        let db = DatabaseConfig::new("postgresql://user:pass@localhost/dbname".to_string(), 20, 2);
        assert_eq!(db.url, "postgresql://user:pass@localhost/dbname");
        assert_eq!(db.max_connections, 20);
        assert_eq!(db.min_connections, 2);
    }

    #[test]
    fn test_builder_basic() {
        let config = RustBootConfig::builder()
            .server_host("0.0.0.0".to_string())
            .server_port(8080)
            .build();

        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        // Database should still have defaults
        assert_eq!(config.database.url, "sqlite::memory:");
    }

    #[test]
    fn test_builder_database() {
        let config = RustBootConfig::builder()
            .database_url("postgresql://localhost/test".to_string())
            .database_max_connections(25)
            .database_min_connections(5)
            .build();

        assert_eq!(config.database.url, "postgresql://localhost/test");
        assert_eq!(config.database.max_connections, 25);
        assert_eq!(config.database.min_connections, 5);
    }

    #[test]
    fn test_builder_with_plugin() {
        let plugin_config = serde_json::json!({
            "enabled": true,
            "ttl": 300
        });

        let config = RustBootConfig::builder()
            .plugin("cache".to_string(), plugin_config.clone())
            .build();

        assert!(config.plugins.contains_key("cache"));
        assert_eq!(config.plugins["cache"], plugin_config);
    }

    #[test]
    fn test_builder_multiple_plugins() {
        let config = RustBootConfig::builder()
            .plugin("cache".to_string(), serde_json::json!({"enabled": true}))
            .plugin("auth".to_string(), serde_json::json!({"secret": "key"}))
            .build();

        assert_eq!(config.plugins.len(), 2);
        assert!(config.plugins.contains_key("cache"));
        assert!(config.plugins.contains_key("auth"));
    }

    #[test]
    fn test_env_override_server_host() {
        with_clean_env(|| {
            std::env::set_var("RUST_BOOT_SERVER_HOST", "192.168.1.1");
            let config = RustBootConfig::from_env();
            assert_eq!(config.server.host, "192.168.1.1");
        });
    }

    #[test]
    fn test_env_override_server_port() {
        with_clean_env(|| {
            std::env::set_var("RUST_BOOT_SERVER_PORT", "9000");
            let config = RustBootConfig::from_env();
            assert_eq!(config.server.port, 9000);
        });
    }

    #[test]
    fn test_env_override_database_url() {
        with_clean_env(|| {
            std::env::set_var("RUST_BOOT_DATABASE_URL", "mysql://root:pass@localhost/db");
            let config = RustBootConfig::from_env();
            assert_eq!(config.database.url, "mysql://root:pass@localhost/db");
        });
    }

    #[test]
    fn test_env_override_database_connections() {
        with_clean_env(|| {
            std::env::set_var("RUST_BOOT_DATABASE_MAX_CONNECTIONS", "50");
            std::env::set_var("RUST_BOOT_DATABASE_MIN_CONNECTIONS", "10");
            let config = RustBootConfig::from_env();
            assert_eq!(config.database.max_connections, 50);
            assert_eq!(config.database.min_connections, 10);
        });
    }

    #[test]
    fn test_env_override_invalid_port() {
        with_clean_env(|| {
            std::env::set_var("RUST_BOOT_SERVER_PORT", "invalid");
            let config = RustBootConfig::from_env();
            // Should fall back to default when invalid
            assert_eq!(config.server.port, 3000);
        });
    }

    #[test]
    fn test_env_override_multiple() {
        with_clean_env(|| {
            std::env::set_var("RUST_BOOT_SERVER_HOST", "0.0.0.0");
            std::env::set_var("RUST_BOOT_SERVER_PORT", "5000");
            std::env::set_var("RUST_BOOT_DATABASE_URL", "postgres://localhost/mydb");

            let config = RustBootConfig::from_env();
            assert_eq!(config.server.host, "0.0.0.0");
            assert_eq!(config.server.port, 5000);
            assert_eq!(config.database.url, "postgres://localhost/mydb");
        });
    }

    #[test]
    fn test_load_toml_config() {
        with_clean_env(|| {
            let toml_content = r#"
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgresql://localhost/testdb"
max_connections = 20
min_connections = 5
"#;

            let temp_file = std::env::temp_dir().join("test_config.toml");
            std::fs::write(&temp_file, toml_content).unwrap();

            let config = RustBootConfig::from_file(temp_file.to_str().unwrap()).unwrap();
            assert_eq!(config.server.host, "0.0.0.0");
            assert_eq!(config.server.port, 8080);
            assert_eq!(config.database.url, "postgresql://localhost/testdb");
            assert_eq!(config.database.max_connections, 20);
            assert_eq!(config.database.min_connections, 5);

            std::fs::remove_file(&temp_file).unwrap();
        });
    }

    #[test]
    fn test_load_json_config() {
        with_clean_env(|| {
            let json_content = r#"
{
  "server": {
    "host": "192.168.1.1",
    "port": 3001
  },
  "database": {
    "url": "sqlite:./test.db",
    "max_connections": 15,
    "min_connections": 3
  },
  "plugins": {}
}
"#;

            let temp_file = std::env::temp_dir().join("test_config.json");
            std::fs::write(&temp_file, json_content).unwrap();

            let config = RustBootConfig::from_file(temp_file.to_str().unwrap()).unwrap();
            assert_eq!(config.server.host, "192.168.1.1");
            assert_eq!(config.server.port, 3001);
            assert_eq!(config.database.url, "sqlite:./test.db");

            std::fs::remove_file(&temp_file).unwrap();
        });
    }

    #[test]
    fn test_load_config_with_env_override() {
        with_clean_env(|| {
            let toml_content = r#"
[server]
host = "127.0.0.1"
port = 3000

[database]
url = "sqlite::memory:"
max_connections = 10
min_connections = 1
"#;

            let temp_file = std::env::temp_dir().join("test_config_override.toml");
            std::fs::write(&temp_file, toml_content).unwrap();

            std::env::set_var("RUST_BOOT_SERVER_PORT", "9000");

            let config = RustBootConfig::from_file(temp_file.to_str().unwrap()).unwrap();
            assert_eq!(config.server.host, "127.0.0.1");
            assert_eq!(config.server.port, 9000);

            std::fs::remove_file(&temp_file).unwrap();
        });
    }

    #[test]
    fn test_invalid_file_format() {
        let path = std::env::temp_dir().join("config.unknown");
        let result = RustBootConfig::from_file(path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_file() {
        let path = std::env::temp_dir().join("nonexistent_config_12345.toml");
        let result = RustBootConfig::from_file(path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_toml_syntax() {
        let bad_toml = "invalid [toml content";
        let temp_file = std::env::temp_dir().join("bad_config.toml");
        std::fs::write(&temp_file, bad_toml).unwrap();

        let result = RustBootConfig::from_file(temp_file.to_str().unwrap());
        assert!(result.is_err());

        std::fs::remove_file(&temp_file).unwrap();
    }

    #[test]
    fn test_config_serialization() {
        let config = RustBootConfig::builder()
            .server_host("0.0.0.0".to_string())
            .server_port(8080)
            .database_url("postgres://localhost/db".to_string())
            .build();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RustBootConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.server.host, deserialized.server.host);
        assert_eq!(config.server.port, deserialized.server.port);
        assert_eq!(config.database.url, deserialized.database.url);
    }
}
