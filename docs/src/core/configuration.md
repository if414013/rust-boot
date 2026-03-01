# Configuration

rust-boot provides a layered configuration system that lets you define application settings through code defaults, configuration files, and environment variables. The system is designed so that each layer overrides the previous one, giving you flexibility across development, staging, and production environments without changing code.

The configuration lives in the `rust_boot_core::config` module and revolves around three main structs: `RustBootConfig` (the top-level container), `ServerConfig` (HTTP server settings), and `DatabaseConfig` (database connection settings).

## How Layered Configuration Works

Configuration values are resolved in the following order of precedence (highest wins):

1. **Environment variables** (`RUST_BOOT_*` prefix) — always applied last, always win
2. **Configuration file** (TOML, YAML, or JSON) — loaded from disk
3. **Default values** — hardcoded sensible defaults built into the structs

This means you can ship a `config.toml` with your application for base settings, then override specific values per-environment using environment variables — a common pattern for containerized deployments.

## RustBootConfig

`RustBootConfig` is the root configuration struct. It holds all sub-configurations and provides multiple ways to construct itself.

```rust
use rust_boot_core::config::RustBootConfig;

// 1. Use built-in defaults
let config = RustBootConfig::default();

// 2. Load from a file (with automatic env var overrides)
let config = RustBootConfig::from_file("config.toml").unwrap();

// 3. Load from environment variables only (defaults + env overrides)
let config = RustBootConfig::from_env();

// 4. Use the builder for programmatic construction
let config = RustBootConfig::builder()
    .server_host("0.0.0.0".to_string())
    .server_port(8080)
    .database_url("postgresql://localhost/myapp".to_string())
    .build();
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `server` | `ServerConfig` | HTTP server host and port settings |
| `database` | `DatabaseConfig` | Database connection URL and pool sizing |
| `plugins` | `HashMap<String, serde_json::Value>` | Arbitrary plugin-specific configuration as JSON values |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `default()` | `RustBootConfig` | Creates config with all default values |
| `builder()` | `RustBootConfigBuilder` | Returns a builder for fluent construction |
| `from_file(path)` | `Result<RustBootConfig, ConfigError>` | Loads from a file, then applies env var overrides |
| `from_env()` | `RustBootConfig` | Starts from defaults, then applies env var overrides |

## ServerConfig

Controls where the HTTP server binds.

```rust
use rust_boot_core::config::ServerConfig;

// Use defaults: 127.0.0.1:3000
let server = ServerConfig::default();

// Or construct directly
let server = ServerConfig::new("0.0.0.0".to_string(), 8080);
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `host` | `String` | `"127.0.0.1"` | The address the server listens on |
| `port` | `u16` | `3000` | The port the server listens on |

## DatabaseConfig

Controls the database connection pool.

```rust
use rust_boot_core::config::DatabaseConfig;

// Use defaults: in-memory SQLite with 10 max / 1 min connections
let db = DatabaseConfig::default();

// Or construct directly
let db = DatabaseConfig::new(
    "postgresql://user:pass@localhost/mydb".to_string(),
    20,  // max_connections
    2,   // min_connections
);
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | `"sqlite::memory:"` | Database connection URL |
| `max_connections` | `u32` | `10` | Maximum connections in the pool |
| `min_connections` | `u32` | `1` | Minimum connections to keep alive |

## RustBootConfigBuilder

The builder provides a fluent API for constructing `RustBootConfig` programmatically. Any field you don't set falls back to the default value.

```rust
use rust_boot_core::config::RustBootConfig;

let config = RustBootConfig::builder()
    .server_host("0.0.0.0".to_string())
    .server_port(8080)
    .database_url("postgresql://localhost/myapp".to_string())
    .database_max_connections(25)
    .database_min_connections(5)
    .plugin("cache".to_string(), serde_json::json!({
        "enabled": true,
        "ttl": 300
    }))
    .build();
```

### Builder Methods

| Method | Parameter | Description |
|--------|-----------|-------------|
| `server_host(host)` | `String` | Set the server bind address |
| `server_port(port)` | `u16` | Set the server port |
| `database_url(url)` | `String` | Set the database connection URL |
| `database_max_connections(max)` | `u32` | Set the max pool size |
| `database_min_connections(min)` | `u32` | Set the min pool size |
| `plugin(name, config)` | `String, serde_json::Value` | Add a plugin configuration entry |
| `build()` | — | Consume the builder and produce a `RustBootConfig` |

## Configuration Files

`RustBootConfig::from_file()` supports three file formats, detected by extension:

| Extension | Format |
|-----------|--------|
| `.toml` | TOML |
| `.yaml`, `.yml` | YAML |
| `.json` | JSON |

### TOML Example

```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgresql://localhost/myapp"
max_connections = 20
min_connections = 5

[plugins.cache]
enabled = true
ttl = 300
```

### JSON Example

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080
  },
  "database": {
    "url": "postgresql://localhost/myapp",
    "max_connections": 20,
    "min_connections": 5
  },
  "plugins": {}
}
```

After parsing the file, `from_file()` automatically calls `apply_env_overrides()`, so environment variables always take final precedence.

## Environment Variable Overrides

Any configuration value can be overridden at runtime using environment variables with the `RUST_BOOT_` prefix. This applies whether you loaded from a file or from defaults.

| Environment Variable | Config Field | Type |
|---------------------|--------------|------|
| `RUST_BOOT_SERVER_HOST` | `server.host` | `String` |
| `RUST_BOOT_SERVER_PORT` | `server.port` | `u16` |
| `RUST_BOOT_DATABASE_URL` | `database.url` | `String` |
| `RUST_BOOT_DATABASE_MAX_CONNECTIONS` | `database.max_connections` | `u32` |
| `RUST_BOOT_DATABASE_MIN_CONNECTIONS` | `database.min_connections` | `u32` |

If a numeric environment variable contains an unparseable value (e.g., `RUST_BOOT_SERVER_PORT=abc`), the override is silently skipped and the previous value is kept.

```bash
# Override just the port for production
RUST_BOOT_SERVER_PORT=9000 RUST_BOOT_DATABASE_URL=postgresql://prod-db/app ./my-app
```

## ConfigError

File-based loading can fail. The `ConfigError` enum covers the possible failure modes:

| Variant | Description |
|---------|-------------|
| `FileReadError(String)` | The file could not be read from disk |
| `ParseError(String)` | The file contents could not be parsed (syntax error, wrong structure) |
| `InvalidFileFormat` | The file extension is not `.toml`, `.yaml`, `.yml`, or `.json` |

`ConfigError` implements `Display` and `std::error::Error`, so it works with `?` and standard error handling patterns.

## Serialization

All configuration structs derive `Serialize` and `Deserialize`, so you can round-trip them through any serde-compatible format:

```rust
let config = RustBootConfig::builder()
    .server_host("0.0.0.0".to_string())
    .server_port(8080)
    .build();

// Serialize to JSON
let json = serde_json::to_string_pretty(&config).unwrap();

// Deserialize back
let restored: RustBootConfig = serde_json::from_str(&json).unwrap();
assert_eq!(config.server.port, restored.server.port);
```
