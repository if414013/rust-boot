//! JWT token management.

use crate::auth::claims::Claims;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rust_boot_core::error::{Result, RustBootError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Configuration for JWT token generation and validation.
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens.
    pub secret: String,
    /// TTL for access tokens.
    pub access_token_ttl: Duration,
    /// TTL for refresh tokens.
    pub refresh_token_ttl: Duration,
    /// Optional issuer claim.
    pub issuer: Option<String>,
    /// Optional audience claim.
    pub audience: Option<String>,
}

impl JwtConfig {
    /// Creates a new JWT configuration with the given secret.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            access_token_ttl: Duration::from_secs(15 * 60),
            refresh_token_ttl: Duration::from_secs(7 * 24 * 60 * 60),
            issuer: None,
            audience: None,
        }
    }

    /// Sets the access token TTL.
    pub const fn with_access_token_ttl(mut self, ttl: Duration) -> Self {
        self.access_token_ttl = ttl;
        self
    }

    /// Sets the refresh token TTL.
    pub const fn with_refresh_token_ttl(mut self, ttl: Duration) -> Self {
        self.refresh_token_ttl = ttl;
        self
    }

    /// Sets the issuer claim.
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Sets the audience claim.
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }
}

/// Manages JWT token creation and verification.
pub struct JwtManager {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtManager {
    /// Creates a new JWT manager with the given configuration.
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// Creates a new access token from the given claims.
    pub fn create_access_token(&self, mut claims: Claims) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| RustBootError::Internal(e.to_string()))?;

        claims.iat = now.as_secs();
        claims.exp = now.as_secs() + self.config.access_token_ttl.as_secs();

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| RustBootError::Internal(format!("Failed to create token: {e}")))
    }

    /// Creates a new refresh token from the given claims.
    pub fn create_refresh_token(&self, mut claims: Claims) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| RustBootError::Internal(e.to_string()))?;

        claims.iat = now.as_secs();
        claims.exp = now.as_secs() + self.config.refresh_token_ttl.as_secs();
        claims.refresh = Some(true);

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| RustBootError::Internal(format!("Failed to create refresh token: {e}")))
    }

    /// Verifies a token and returns its claims.
    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::default();

        if let Some(issuer) = &self.config.issuer {
            validation.set_issuer(&[issuer]);
        }

        if let Some(audience) = &self.config.audience {
            validation.set_audience(&[audience]);
        }

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| RustBootError::Auth(format!("Invalid token: {e}")))?;

        Ok(token_data.claims)
    }

    /// Verifies that a token is a valid access token.
    pub fn verify_access_token(&self, token: &str) -> Result<Claims> {
        let claims = self.verify_token(token)?;

        if claims.is_refresh_token() {
            return Err(RustBootError::Auth(
                "Refresh token cannot be used as access token".to_string(),
            ));
        }

        Ok(claims)
    }

    /// Verifies that a token is a valid refresh token.
    pub fn verify_refresh_token(&self, token: &str) -> Result<Claims> {
        let claims = self.verify_token(token)?;

        if !claims.is_refresh_token() {
            return Err(RustBootError::Auth(
                "Access token cannot be used as refresh token".to_string(),
            ));
        }

        Ok(claims)
    }

    /// Exchanges a refresh token for new access and refresh tokens.
    pub fn refresh_tokens(&self, refresh_token: &str) -> Result<(String, String)> {
        let claims = self.verify_refresh_token(refresh_token)?;

        let new_claims = Claims::new(&claims.sub, 0, 0).with_roles(claims.roles.clone());

        let new_claims = if let Some(email) = &claims.email {
            new_claims.with_email(email)
        } else {
            new_claims
        };

        let new_claims = if let Some(name) = &claims.name {
            new_claims.with_name(name)
        } else {
            new_claims
        };

        let access_token = self.create_access_token(new_claims.clone())?;
        let refresh_token = self.create_refresh_token(new_claims)?;

        Ok((access_token, refresh_token))
    }

    /// Returns the JWT configuration.
    pub const fn config(&self) -> &JwtConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::claims::Role;

    fn create_test_manager() -> JwtManager {
        let config = JwtConfig::new("test-secret-key-that-is-long-enough-for-jwt");
        JwtManager::new(config)
    }

    #[test]
    fn test_jwt_config_default() {
        let config = JwtConfig::new("secret");
        assert_eq!(config.access_token_ttl, Duration::from_secs(15 * 60));
        assert_eq!(
            config.refresh_token_ttl,
            Duration::from_secs(7 * 24 * 60 * 60)
        );
    }

    #[test]
    fn test_jwt_config_builder() {
        let config = JwtConfig::new("secret")
            .with_access_token_ttl(Duration::from_secs(300))
            .with_refresh_token_ttl(Duration::from_secs(3600))
            .with_issuer("my-app")
            .with_audience("my-api");

        assert_eq!(config.access_token_ttl, Duration::from_secs(300));
        assert_eq!(config.refresh_token_ttl, Duration::from_secs(3600));
        assert_eq!(config.issuer, Some("my-app".to_string()));
        assert_eq!(config.audience, Some("my-api".to_string()));
    }

    #[test]
    fn test_create_and_verify_access_token() {
        let manager = create_test_manager();
        let claims = Claims::new("user123", 0, 0).with_role(Role::user());

        let token = manager.create_access_token(claims).unwrap();
        let verified = manager.verify_access_token(&token).unwrap();

        assert_eq!(verified.sub, "user123");
        assert!(verified.has_role(&Role::user()));
    }

    #[test]
    fn test_create_and_verify_refresh_token() {
        let manager = create_test_manager();
        let claims = Claims::new("user123", 0, 0);

        let token = manager.create_refresh_token(claims).unwrap();
        let verified = manager.verify_refresh_token(&token).unwrap();

        assert_eq!(verified.sub, "user123");
        assert!(verified.is_refresh_token());
    }

    #[test]
    fn test_access_token_cannot_be_used_as_refresh() {
        let manager = create_test_manager();
        let claims = Claims::new("user123", 0, 0);

        let token = manager.create_access_token(claims).unwrap();
        let result = manager.verify_refresh_token(&token);

        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_token_cannot_be_used_as_access() {
        let manager = create_test_manager();
        let claims = Claims::new("user123", 0, 0);

        let token = manager.create_refresh_token(claims).unwrap();
        let result = manager.verify_access_token(&token);

        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_tokens() {
        let manager = create_test_manager();
        let claims = Claims::new("user123", 0, 0)
            .with_role(Role::admin())
            .with_email("user@example.com");

        let refresh_token = manager.create_refresh_token(claims).unwrap();
        let (new_access, new_refresh) = manager.refresh_tokens(&refresh_token).unwrap();

        let access_claims = manager.verify_access_token(&new_access).unwrap();
        assert_eq!(access_claims.sub, "user123");
        assert!(access_claims.has_role(&Role::admin()));
        assert_eq!(access_claims.email, Some("user@example.com".to_string()));

        let refresh_claims = manager.verify_refresh_token(&new_refresh).unwrap();
        assert_eq!(refresh_claims.sub, "user123");
    }

    #[test]
    fn test_invalid_token() {
        let manager = create_test_manager();
        let result = manager.verify_token("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_with_different_secret() {
        let manager1 = JwtManager::new(JwtConfig::new("secret-one-long-enough"));
        let manager2 = JwtManager::new(JwtConfig::new("secret-two-long-enough"));

        let claims = Claims::new("user123", 0, 0);
        let token = manager1.create_access_token(claims).unwrap();

        let result = manager2.verify_token(&token);
        assert!(result.is_err());
    }
}
