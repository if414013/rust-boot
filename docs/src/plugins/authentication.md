# Authentication Plugin

The `AuthPlugin` provides JWT-based authentication with full role-based access control (RBAC). It wraps the `JwtManager` for token creation, verification, and refresh flows, and provides a `Claims` system with fine-grained role checking.

## Quick Start

```rust
use rust_boot::prelude::*;
use std::time::Duration;

// Configure JWT
let jwt_config = JwtConfig::new("your-secret-key")
    .with_access_token_ttl(Duration::from_secs(15 * 60))   // 15 minutes
    .with_refresh_token_ttl(Duration::from_secs(7 * 24 * 60 * 60)) // 7 days
    .with_issuer("my-app")
    .with_audience("my-api");

// Register with the plugin system
let mut registry = PluginRegistry::new();
registry.register(AuthPlugin::new(jwt_config))?;
registry.init_all().await?;
```

## JwtConfig

`JwtConfig` controls all aspects of token generation and validation. It uses the builder pattern for ergonomic configuration.

```rust
let config = JwtConfig::new("secret");
```

| Method | Default | Description |
|---|---|---|
| `new(secret)` | — | Creates config with the given signing secret |
| `with_access_token_ttl(Duration)` | 15 minutes | How long access tokens remain valid |
| `with_refresh_token_ttl(Duration)` | 7 days | How long refresh tokens remain valid |
| `with_issuer(String)` | `None` | Sets the `iss` claim for validation |
| `with_audience(String)` | `None` | Sets the `aud` claim for validation |

```rust
let config = JwtConfig::new("my-secret-key-at-least-32-chars-long")
    .with_access_token_ttl(Duration::from_secs(5 * 60))    // 5 minutes
    .with_refresh_token_ttl(Duration::from_secs(24 * 60 * 60)) // 1 day
    .with_issuer("auth-service")
    .with_audience("api-gateway");
```

## JwtManager

`JwtManager` is the core engine for creating and verifying tokens. It holds the encoding and decoding keys derived from your secret.

```rust
let manager = JwtManager::new(config);
```

### Creating Tokens

```rust
// Build claims for a user
let claims = Claims::new("user-123", 0, 0)
    .with_role(Role::admin())
    .with_role(Role::user())
    .with_email("admin@example.com")
    .with_name("Admin User");

// Generate an access token (short-lived, for API calls)
let access_token = manager.create_access_token(claims.clone())?;

// Generate a refresh token (long-lived, for obtaining new access tokens)
let refresh_token = manager.create_refresh_token(claims)?;
```

Both methods automatically set `iat` (issued at) to the current time and `exp` (expiration) based on the configured TTL. The refresh token additionally sets the `refresh` flag to `true`.

### Verifying Tokens

```rust
// Verify an access token — rejects refresh tokens
let claims = manager.verify_access_token(&access_token)?;
println!("User: {}", claims.sub);

// Verify a refresh token — rejects access tokens
let claims = manager.verify_refresh_token(&refresh_token)?;
assert!(claims.is_refresh_token());

// Verify any token (doesn't check the refresh flag)
let claims = manager.verify_token(&token)?;
```

Token verification checks:
- Signature validity (using the configured secret)
- Expiration (`exp` claim)
- Issuer (if configured in `JwtConfig`)
- Audience (if configured in `JwtConfig`)
- Token type (access vs. refresh, for the specific verify methods)

### Refreshing Tokens

The `refresh_tokens` method exchanges a valid refresh token for a new access/refresh token pair. All claims (roles, email, name) are carried over to the new tokens.

```rust
let (new_access, new_refresh) = manager.refresh_tokens(&refresh_token)?;

// The old refresh token is still valid until it expires.
// In production, consider maintaining a token blacklist.
```

### Full API Reference

| Method | Returns | Description |
|---|---|---|
| `new(config)` | `JwtManager` | Creates a new manager from config |
| `create_access_token(claims)` | `Result<String>` | Creates a signed access token |
| `create_refresh_token(claims)` | `Result<String>` | Creates a signed refresh token |
| `verify_token(token)` | `Result<Claims>` | Verifies any token |
| `verify_access_token(token)` | `Result<Claims>` | Verifies, rejects refresh tokens |
| `verify_refresh_token(token)` | `Result<Claims>` | Verifies, rejects access tokens |
| `refresh_tokens(refresh_token)` | `Result<(String, String)>` | Exchanges refresh for new pair |
| `config()` | `&JwtConfig` | Returns the JWT configuration |

## Claims

`Claims` is the JWT payload structure. It carries user identity, roles, and optional profile information.

```rust
pub struct Claims {
    pub sub: String,              // Subject (user ID)
    pub exp: u64,                 // Expiration (Unix timestamp)
    pub iat: u64,                 // Issued at (Unix timestamp)
    pub roles: HashSet<Role>,     // User roles for RBAC
    pub email: Option<String>,    // Optional email
    pub name: Option<String>,     // Optional display name
    pub refresh: Option<bool>,    // Whether this is a refresh token
}
```

### Building Claims

Claims use the builder pattern. Start with `new()` and chain methods:

```rust
let claims = Claims::new("user-456", 0, 0)  // subject, exp, iat (overwritten by JwtManager)
    .with_role(Role::user())
    .with_roles([Role::new("editor"), Role::new("reviewer")])
    .with_email("user@example.com")
    .with_name("Jane Doe");
```

The `exp` and `iat` values you pass to `new()` are placeholders — `JwtManager` overwrites them with the current time and configured TTL when creating tokens.

### Claims Methods

| Method | Description |
|---|---|
| `new(subject, exp, iat)` | Creates claims with subject and timestamps |
| `with_role(Role)` | Adds a single role |
| `with_roles(impl IntoIterator<Item = Role>)` | Adds multiple roles |
| `with_email(impl Into<String>)` | Sets the email claim |
| `with_name(impl Into<String>)` | Sets the name claim |
| `as_refresh_token()` | Marks as a refresh token |
| `is_refresh_token()` | Returns `true` if this is a refresh token |
| `has_role(&Role)` | Checks for a single role |
| `has_any_role(&[Role])` | Checks if any of the given roles are present |
| `has_all_roles(&[Role])` | Checks if all of the given roles are present |
| `is_expired()` | Checks expiration against current system time |

### Checking Roles

```rust
let claims = Claims::new("user-123", 0, 0)
    .with_role(Role::admin())
    .with_role(Role::user());

// Single role check
claims.has_role(&Role::admin());  // true
claims.has_role(&Role::new("moderator"));  // false

// Any of these roles?
claims.has_any_role(&[Role::admin(), Role::new("superadmin")]);  // true

// All of these roles?
claims.has_all_roles(&[Role::admin(), Role::user()]);  // true
claims.has_all_roles(&[Role::admin(), Role::new("moderator")]);  // false

// Token type and expiration
claims.is_refresh_token();  // false
claims.is_expired();        // checks against current system time
```

### Refresh Token Claims

```rust
let refresh_claims = Claims::new("user-123", 0, 0)
    .with_role(Role::user())
    .as_refresh_token();

assert!(refresh_claims.is_refresh_token());
```

## Role

`Role` is a simple wrapper around a string name, with convenience constructors for common roles.

```rust
// Named constructors
let admin = Role::admin();   // "admin"
let user = Role::user();     // "user"

// Custom roles
let editor = Role::new("editor");
let moderator = Role::new("moderator");

// From string types
let role: Role = "reviewer".into();
let role: Role = String::from("manager").into();

// Get the name back
assert_eq!(admin.name(), "admin");
```

## Complete RBAC Example

Here's a full example showing token generation, verification, and role-based authorization:

```rust
use rust_boot::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Configure JWT with production-like settings
    let secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");
    let config = JwtConfig::new(secret)
        .with_access_token_ttl(Duration::from_secs(15 * 60))
        .with_refresh_token_ttl(Duration::from_secs(7 * 24 * 60 * 60))
        .with_issuer("my-service")
        .with_audience("my-api");

    let manager = JwtManager::new(config);

    // 2. User logs in — create tokens with their roles
    let claims = Claims::new("user-789", 0, 0)
        .with_role(Role::user())
        .with_role(Role::new("editor"))
        .with_email("editor@example.com")
        .with_name("Jane Editor");

    let access_token = manager.create_access_token(claims.clone())?;
    let refresh_token = manager.create_refresh_token(claims)?;

    // 3. On each API request — verify the access token
    let verified = manager.verify_access_token(&access_token)?;

    // 4. Check authorization
    if verified.has_role(&Role::new("editor")) {
        println!("User {} can edit content", verified.sub);
    }

    if verified.has_any_role(&[Role::admin(), Role::new("editor")]) {
        println!("User has elevated privileges");
    }

    // 5. When the access token expires — use refresh token
    let (new_access, new_refresh) = manager.refresh_tokens(&refresh_token)?;

    // The new tokens carry the same roles and claims
    let refreshed = manager.verify_access_token(&new_access)?;
    assert!(refreshed.has_role(&Role::new("editor")));
    assert_eq!(refreshed.email, Some("editor@example.com".to_string()));

    Ok(())
}
```

## AuthPlugin Lifecycle

When used as a plugin, `AuthPlugin` creates the `JwtManager` during `build()` and drops it during `cleanup()`:

- **build()** — Creates a `JwtManager` from the config and stores it as `Arc<JwtManager>` internally.
- **cleanup()** — Sets the internal `JwtManager` reference to `None`.

The plugin registers with the name `"auth"` and version `"0.1.0"`. It has no dependencies on other plugins.

```rust
let auth = AuthPlugin::new(JwtConfig::new("secret"));

// After build(), access the manager:
// auth.jwt_manager() -> Option<Arc<JwtManager>>
```

## Security Best Practices

- **Never hardcode secrets.** Load your JWT secret from environment variables or a secrets manager:
  ```rust
  let secret = std::env::var("JWT_SECRET")
      .expect("JWT_SECRET must be set");
  let config = JwtConfig::new(secret);
  ```

- **Use strong secrets.** Your secret should be at least 32 characters of random data. Short or predictable secrets can be brute-forced.

- **Keep access tokens short-lived.** 5-15 minutes is typical. This limits the damage window if a token is compromised.

- **Implement token rotation.** When refreshing tokens, consider invalidating the old refresh token (maintain a blacklist or use single-use refresh tokens).

- **Validate issuer and audience.** Always set these in production to prevent tokens from one service being accepted by another:
  ```rust
  let config = JwtConfig::new(secret)
      .with_issuer("auth-service")
      .with_audience("api-gateway");
  ```

- **Don't store tokens in localStorage.** For web applications, use HTTP-only cookies with the `Secure` and `SameSite` flags.

- **Log authentication events.** Combine with the [Monitoring Plugin](./monitoring.md) to track failed authentication attempts.
