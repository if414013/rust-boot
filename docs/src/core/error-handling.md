# Error Handling

rust-boot uses a single, unified error type — `RustBootError` — to represent every kind of failure that can occur across the framework. Rather than scattering different error types through different modules, all errors flow through one enum, making it straightforward to propagate errors with `?` and match on specific failure categories when you need to.

The error system lives in `rust_boot_core::error` and provides two key exports: the `RustBootError` enum and a `Result<T>` type alias.

## RustBootError

`RustBootError` is an enum with nine variants, each representing a distinct category of failure. Every variant carries a `String` message with context about what went wrong, except `Http` which also carries a status code.

```rust
use rust_boot_core::error::{RustBootError, Result};

fn load_user(id: u64) -> Result<User> {
    let user = db.find(id)
        .map_err(|e| RustBootError::Database(e.to_string()))?;

    user.ok_or_else(|| RustBootError::Http(404, "User not found".to_string()))
}
```

### Variants

| Variant | Fields | When to Use |
|---------|--------|-------------|
| `Config(String)` | Error message | Invalid settings, missing required configuration, malformed config files |
| `Database(String)` | Error message | Query failures, connection issues, migration errors, ORM problems |
| `Plugin(String)` | Error message | Plugin initialization failures, missing plugin dependencies |
| `Validation(String)` | Error message | Input validation failures, constraint violations, invalid formats |
| `Serialization(String)` | Error message | JSON encoding/decoding issues, data format mismatches |
| `Http(u16, String)` | Status code, message | HTTP-layer errors like 404 Not Found, 400 Bad Request |
| `Cache(String)` | Error message | Cache backend failures, eviction errors |
| `Auth(String)` | Error message | Authentication failures, expired tokens, insufficient permissions |
| `Internal(String)` | Error message | Unexpected errors that shouldn't occur in normal operation |

### Display Format

Each variant formats its message with a descriptive prefix:

```rust
let err = RustBootError::Config("missing database url".to_string());
assert_eq!(err.to_string(), "configuration error: missing database url");

let err = RustBootError::Http(404, "not found".to_string());
assert_eq!(err.to_string(), "HTTP 404 error: not found");

let err = RustBootError::Auth("invalid token".to_string());
assert_eq!(err.to_string(), "authentication error: invalid token");
```

## Result Type Alias

The module exports a convenience type alias that saves you from writing `RustBootError` in every function signature:

```rust
// This:
pub type Result<T> = std::result::Result<T, RustBootError>;

// So instead of writing:
fn do_thing() -> std::result::Result<String, RustBootError> { ... }

// You write:
use rust_boot_core::error::Result;
fn do_thing() -> Result<String> { ... }
```

This alias is used throughout the framework — in repository traits, service traits, and anywhere that returns a fallible result.

## From Conversions

`RustBootError` implements `From` for several common error types, enabling seamless use of the `?` operator when working with external libraries.

### `From<std::io::Error>`

IO errors are mapped to `RustBootError::Database` since they most commonly occur during database file operations:

```rust
use rust_boot_core::error::Result;

fn read_data(path: &str) -> Result<String> {
    // The ? operator automatically converts io::Error → RustBootError::Database
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}
```

### `From<serde_json::Error>`

JSON serialization errors are mapped to `RustBootError::Serialization`:

```rust
use rust_boot_core::error::Result;

fn parse_payload(json: &str) -> Result<serde_json::Value> {
    // Automatically converts serde_json::Error → RustBootError::Serialization
    let value = serde_json::from_str(json)?;
    Ok(value)
}
```

### `From<config::ConfigError>`

Errors from the `config` crate are mapped to `RustBootError::Config`:

```rust
use rust_boot_core::error::Result;

fn load_settings() -> Result<config::Config> {
    // Automatically converts config::ConfigError → RustBootError::Config
    let settings = config::Config::builder()
        .add_source(config::File::with_name("settings"))
        .build()?;
    Ok(settings)
}
```

### Conversion Summary

| Source Type | Target Variant | Prefix in Message |
|-------------|---------------|-------------------|
| `std::io::Error` | `Database(...)` | `"IO error: "` |
| `serde_json::Error` | `Serialization(...)` | `"JSON error: "` |
| `config::ConfigError` | `Config(...)` | `"Config crate error: "` |

## Trait Implementations

`RustBootError` implements the standard Rust error traits:

- `Debug` — derived, for `{:?}` formatting
- `Display` — custom formatting with category prefixes (shown above)
- `std::error::Error` — makes it compatible with `Box<dyn Error>`, `anyhow`, and other error-handling ecosystems

```rust
// Works as a boxed trait object
let err: Box<dyn std::error::Error> = Box::new(
    RustBootError::Config("test".to_string())
);
assert_eq!(err.to_string(), "configuration error: test");
```

## Error Handling Patterns

### Propagation with `?`

The most common pattern — let errors bubble up through the call stack:

```rust
use rust_boot_core::error::Result;

fn process_request(data: &str) -> Result<Response> {
    let parsed: Request = serde_json::from_str(data)?;  // Serialization error
    let user = find_user(parsed.user_id)?;               // Database error
    let result = validate_and_process(&user, &parsed)?;   // Validation error
    Ok(result)
}
```

### Matching on Variants

When you need to handle specific error categories differently:

```rust
use rust_boot_core::error::RustBootError;

match service.find_by_id(id).await {
    Ok(Some(entity)) => ok(entity),
    Ok(None) => Err(RustBootError::Http(404, "Not found".to_string())),
    Err(RustBootError::Database(msg)) => {
        log::error!("Database failure: {msg}");
        Err(RustBootError::Internal("Service unavailable".to_string()))
    }
    Err(e) => Err(e),
}
```

### Creating Domain-Specific Errors

Wrap domain logic failures in the appropriate variant:

```rust
use rust_boot_core::error::{RustBootError, Result};

fn validate_email(email: &str) -> Result<()> {
    if !email.contains('@') {
        return Err(RustBootError::Validation(
            "Email must contain @".to_string()
        ));
    }
    Ok(())
}
```
