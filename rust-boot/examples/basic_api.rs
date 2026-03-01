//! Basic API Example - Demonstrating rust-boot Framework Usage
//!
//! This example shows how to build a simple CRUD API using the rust-boot framework.
//! It demonstrates:
//! - Plugin setup (caching, authentication, monitoring)
//! - Router configuration with `CrudRouterBuilder`
//! - JWT token generation and verification
//! - Basic Axum application setup
//!
//! To run this example:
//! ```bash
//! cargo run --example basic_api
//! ```

#![allow(missing_docs)]

use rust_boot::prelude::*;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// STEP 1: Define your domain models
// ============================================================================

/// A simple User model for our API.
///
/// In a real application, you would derive `CrudModel` from rust-boot-macros
/// to get automatic CRUD operations. For this example, we define it manually.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier for the user
    pub id: Uuid,
    /// User's display name
    pub name: String,
    /// User's email address
    pub email: String,
    /// Whether the user account is active
    pub active: bool,
}

impl User {
    /// Creates a new user with the given name and email
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            email: email.into(),
            active: true,
        }
    }
}

/// Data Transfer Object for creating a new user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserDto {
    pub name: String,
    pub email: String,
}

/// Data Transfer Object for updating an existing user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserDto {
    pub name: Option<String>,
    pub email: Option<String>,
    pub active: Option<bool>,
}

// ============================================================================
// STEP 2: Define application state
// ============================================================================

/// Application state shared across all handlers.
///
/// This holds references to our plugins and any other shared resources.
#[derive(Clone)]
pub struct AppState {
    /// JWT manager for token operations
    pub jwt_manager: Arc<JwtManager>,
    /// Cache backend for data caching
    pub cache: Arc<MokaBackend>,
}

// ============================================================================
// STEP 3: Define handler functions
// ============================================================================

/// List all users with pagination support
async fn list_users(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Query(pagination): axum::extract::Query<PaginationQuery>,
) -> PaginatedResult<User> {
    // In a real app, you would fetch from a database
    let users = vec![
        User::new("Alice", "alice@example.com"),
        User::new("Bob", "bob@example.com"),
    ];

    // Return paginated response
    paginated(users, pagination.page, pagination.per_page, 2)
}

/// Get a single user by ID
async fn get_user(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> ApiResult<User> {
    // In a real app, you would fetch from a database
    // Here we simulate finding a user
    let user = User {
        id,
        name: "Example User".to_string(),
        email: "user@example.com".to_string(),
        active: true,
    };

    ok(user)
}

/// Create a new user
async fn create_user(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Json(dto): axum::extract::Json<CreateUserDto>,
) -> (axum::http::StatusCode, axum::Json<ApiResponse<User>>) {
    // Create the user
    let user = User::new(dto.name, dto.email);

    // Return 201 Created with the new user
    created(user)
}

/// Update an existing user
async fn update_user(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    axum::extract::Json(dto): axum::extract::Json<UpdateUserDto>,
) -> ApiResult<User> {
    // In a real app, you would update in the database
    let user = User {
        id,
        name: dto.name.unwrap_or_else(|| "Updated User".to_string()),
        email: dto
            .email
            .unwrap_or_else(|| "updated@example.com".to_string()),
        active: dto.active.unwrap_or(true),
    };

    ok(user)
}

/// Delete a user
async fn delete_user(
    axum::extract::State(_state): axum::extract::State<AppState>,
    axum::extract::Path(_id): axum::extract::Path<Uuid>,
) -> axum::http::StatusCode {
    // In a real app, you would delete from the database
    no_content()
}

// ============================================================================
// STEP 4: JWT Authentication demonstration
// ============================================================================

/// Demonstrates JWT token generation and verification.
///
/// This function shows how to:
/// - Create claims with user information and roles
/// - Generate access and refresh tokens
/// - Verify tokens and extract claims
fn demonstrate_jwt_auth(jwt_manager: &JwtManager) -> RustBootResult<()> {
    println!("\n=== JWT Authentication Demo ===\n");

    // Create claims for a user with admin role
    let claims = Claims::new("user-123", 0, 0)
        .with_role(Role::admin())
        .with_role(Role::user())
        .with_email("admin@example.com")
        .with_name("Admin User");

    // Generate access token
    let access_token = jwt_manager.create_access_token(claims.clone())?;
    println!(
        "Access Token (first 50 chars): {}...",
        &access_token[..50.min(access_token.len())]
    );

    // Generate refresh token
    let refresh_token = jwt_manager.create_refresh_token(claims)?;
    println!(
        "Refresh Token (first 50 chars): {}...",
        &refresh_token[..50.min(refresh_token.len())]
    );

    // Verify the access token
    let verified_claims = jwt_manager.verify_access_token(&access_token)?;
    println!("\nVerified Claims:");
    println!("  Subject: {}", verified_claims.sub);
    println!("  Email: {:?}", verified_claims.email);
    println!("  Name: {:?}", verified_claims.name);
    println!(
        "  Has admin role: {}",
        verified_claims.has_role(&Role::admin())
    );
    println!(
        "  Has user role: {}",
        verified_claims.has_role(&Role::user())
    );

    // Demonstrate token refresh
    let (new_access, new_refresh) = jwt_manager.refresh_tokens(&refresh_token)?;
    println!("\nTokens refreshed successfully!");
    println!(
        "New Access Token (first 50 chars): {}...",
        &new_access[..50.min(new_access.len())]
    );
    println!(
        "New Refresh Token (first 50 chars): {}...",
        &new_refresh[..50.min(new_refresh.len())]
    );

    Ok(())
}

// ============================================================================
// STEP 5: Plugin setup and initialization
// ============================================================================

/// Sets up and initializes all plugins using the `PluginRegistry`.
///
/// This demonstrates:
/// - Creating plugin configurations
/// - Registering plugins with the registry
/// - Initializing plugins in dependency order
async fn setup_plugins() -> RustBootResult<(Arc<JwtManager>, Arc<MokaBackend>)> {
    println!("\n=== Plugin Setup ===\n");

    // Create a plugin registry
    let mut registry = PluginRegistry::new();

    // Configure and register the caching plugin
    // The CachingPlugin uses Moka (in-memory cache) by default
    let cache_config = CacheConfig::new("api-cache")
        .with_ttl(Duration::from_secs(300)) // 5 minutes TTL
        .with_max_capacity(10_000); // Max 10k entries

    let caching_plugin = CachingPlugin::new(cache_config.clone());
    registry
        .register(caching_plugin)
        .expect("Failed to register caching plugin");
    println!("✓ Registered CachingPlugin");

    // Configure and register the monitoring plugin
    // This provides Prometheus metrics and health checks
    let metrics_config = MetricsConfig::default();
    let monitoring_plugin = MonitoringPlugin::new(metrics_config);
    registry
        .register(monitoring_plugin)
        .expect("Failed to register monitoring plugin");
    println!("✓ Registered MonitoringPlugin");

    // Configure and register the authentication plugin
    // IMPORTANT: In production, use a secure secret from environment variables!
    let jwt_config = JwtConfig::new("your-super-secret-key-change-in-production")
        .with_access_token_ttl(Duration::from_secs(15 * 60)) // 15 minutes
        .with_refresh_token_ttl(Duration::from_secs(7 * 24 * 60 * 60)) // 7 days
        .with_issuer("rust-boot-example")
        .with_audience("rust-boot-api");

    let auth_plugin = AuthPlugin::new(jwt_config.clone());
    registry
        .register(auth_plugin)
        .expect("Failed to register auth plugin");
    println!("✓ Registered AuthPlugin");

    // Initialize all plugins in dependency order
    registry.init_all().await?;
    println!("\n✓ All plugins initialized successfully!");

    // Create instances for use in the application
    let jwt_manager = Arc::new(JwtManager::new(jwt_config));
    let cache_backend = Arc::new(MokaBackend::new(cache_config));

    Ok((jwt_manager, cache_backend))
}

// ============================================================================
// STEP 6: Router configuration
// ============================================================================

/// Creates the API router using `CrudRouterBuilder`.
///
/// This demonstrates:
/// - Creating router configuration
/// - Adding CRUD handlers
/// - Building the final router
fn create_router(state: AppState) -> axum::Router {
    // Configure the CRUD router for the /api/users endpoint
    let router_config = CrudRouterConfig::new("/api/users").with_soft_delete(); // Enable soft delete support

    // Build the users router with all CRUD operations
    let users_router = CrudRouterBuilder::<AppState>::new(router_config)
        .list(list_users) // GET /api/users
        .get(get_user) // GET /api/users/:id
        .create(create_user) // POST /api/users
        .update(update_user) // PUT /api/users/:id
        .delete(delete_user) // DELETE /api/users/:id
        .build();

    // Create the main application router
    axum::Router::new().merge(users_router).with_state(state)
}

// ============================================================================
// STEP 7: Main application entry point
// ============================================================================

#[tokio::main]
async fn main() -> RustBootResult<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║           rust-boot Framework - Basic API Example          ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    // Step 1: Setup plugins
    let (jwt_manager, cache) = setup_plugins().await?;

    // Step 2: Demonstrate JWT authentication
    demonstrate_jwt_auth(&jwt_manager)?;

    // Step 3: Demonstrate caching
    println!("\n=== Caching Demo ===\n");

    // Store a value in cache
    let user = User::new("Cached User", "cached@example.com");
    let cache_key = format!("user:{}", user.id);
    let user_bytes = serde_json::to_vec(&user).expect("Failed to serialize user");
    cache
        .set(&cache_key, user_bytes, Some(Duration::from_secs(60)))
        .await?;
    println!("✓ Stored user in cache with key: {cache_key}");

    // Retrieve from cache
    if let Some(bytes) = cache.get(&cache_key).await? {
        let cached_user: User = serde_json::from_slice(&bytes).expect("Failed to deserialize user");
        println!(
            "✓ Retrieved from cache: {} <{}>",
            cached_user.name, cached_user.email
        );
    }

    // Step 4: Create the router
    let state = AppState { jwt_manager, cache };
    let app = create_router(state);

    println!("\n=== Application Ready ===\n");
    println!("Router configured with endpoints:");
    println!("  GET    /api/users       - List all users");
    println!("  GET    /api/users/:id   - Get user by ID");
    println!("  POST   /api/users       - Create new user");
    println!("  PUT    /api/users/:id   - Update user");
    println!("  DELETE /api/users/:id   - Delete user");

    // Step 5: Start the server
    // Uncomment the following lines to actually run the server:
    //
    // let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await
    //     .expect("Failed to bind to port 3000");
    // println!("\n🚀 Server running at http://localhost:3000");
    // axum::serve(listener, app).await.expect("Server error");

    // For the example, we just demonstrate the setup
    println!("\n✅ Example completed successfully!");
    println!("   (Server startup is commented out - uncomment to run)");

    // Prevent unused variable warning
    let _ = app;

    Ok(())
}

// ============================================================================
// ADDITIONAL EXAMPLES
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that the User model can be created and serialized
    #[test]
    fn test_user_creation() {
        let user = User::new("Test User", "test@example.com");
        assert!(!user.id.is_nil());
        assert_eq!(user.name, "Test User");
        assert_eq!(user.email, "test@example.com");
        assert!(user.active);
    }

    /// Test JWT claims creation
    #[test]
    fn test_claims_creation() {
        let claims = Claims::new("user-123", 0, 0)
            .with_role(Role::admin())
            .with_email("admin@example.com");

        assert_eq!(claims.sub, "user-123");
        assert!(claims.has_role(&Role::admin()));
        assert_eq!(claims.email, Some("admin@example.com".to_string()));
    }

    /// Test cache configuration
    #[test]
    fn test_cache_config() {
        let config = CacheConfig::new("test-cache")
            .with_ttl(Duration::from_secs(60))
            .with_max_capacity(100);

        assert_eq!(config.name, "test-cache");
        assert_eq!(config.default_ttl, Duration::from_secs(60));
        assert_eq!(config.max_capacity, 100);
    }

    /// Test router configuration
    #[test]
    fn test_router_config() {
        let config = CrudRouterConfig::new("/api/items")
            .with_soft_delete()
            .disable_delete();

        assert_eq!(config.base_path, "/api/items");
        assert!(config.enable_soft_delete);
        assert!(!config.enable_delete);
    }
}
