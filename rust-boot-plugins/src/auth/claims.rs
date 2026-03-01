//! JWT claims structures.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Represents a role for RBAC.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Role(pub String);

impl Role {
    /// Creates a new role with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Creates the admin role.
    pub fn admin() -> Self {
        Self::new("admin")
    }

    /// Creates the user role.
    pub fn user() -> Self {
        Self::new("user")
    }

    /// Returns the role name.
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Role {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Role {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// JWT claims payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID).
    pub sub: String,
    /// Expiration time (Unix timestamp).
    pub exp: u64,
    /// Issued at time (Unix timestamp).
    pub iat: u64,
    /// User roles.
    #[serde(default)]
    pub roles: HashSet<Role>,
    /// Optional email claim.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Optional name claim.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether this is a refresh token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<bool>,
}

impl Claims {
    /// Creates new claims with the given subject and timestamps.
    pub fn new(subject: impl Into<String>, expires_at: u64, issued_at: u64) -> Self {
        Self {
            sub: subject.into(),
            exp: expires_at,
            iat: issued_at,
            roles: HashSet::new(),
            email: None,
            name: None,
            refresh: None,
        }
    }

    /// Adds a role to the claims.
    pub fn with_role(mut self, role: Role) -> Self {
        self.roles.insert(role);
        self
    }

    /// Adds multiple roles to the claims.
    pub fn with_roles(mut self, roles: impl IntoIterator<Item = Role>) -> Self {
        self.roles.extend(roles);
        self
    }

    /// Sets the email claim.
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Sets the name claim.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Marks this as a refresh token.
    pub fn as_refresh_token(mut self) -> Self {
        self.refresh = Some(true);
        self
    }

    /// Returns true if this is a refresh token.
    pub fn is_refresh_token(&self) -> bool {
        self.refresh.unwrap_or(false)
    }

    /// Returns true if the claims include the given role.
    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }

    /// Returns true if the claims include any of the given roles.
    pub fn has_any_role(&self, roles: &[Role]) -> bool {
        roles.iter().any(|r| self.roles.contains(r))
    }

    /// Returns true if the claims include all of the given roles.
    pub fn has_all_roles(&self, roles: &[Role]) -> bool {
        roles.iter().all(|r| self.roles.contains(r))
    }

    /// Returns true if the token has expired.
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.exp < now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_creation() {
        let role = Role::new("admin");
        assert_eq!(role.name(), "admin");
    }

    #[test]
    fn test_role_presets() {
        assert_eq!(Role::admin().name(), "admin");
        assert_eq!(Role::user().name(), "user");
    }

    #[test]
    fn test_role_from_str() {
        let role: Role = "moderator".into();
        assert_eq!(role.name(), "moderator");
    }

    #[test]
    fn test_claims_creation() {
        let claims = Claims::new("user123", 1000, 500);
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.exp, 1000);
        assert_eq!(claims.iat, 500);
        assert!(claims.roles.is_empty());
    }

    #[test]
    fn test_claims_with_role() {
        let claims = Claims::new("user123", 1000, 500).with_role(Role::admin());
        assert!(claims.has_role(&Role::admin()));
        assert!(!claims.has_role(&Role::user()));
    }

    #[test]
    fn test_claims_with_multiple_roles() {
        let claims = Claims::new("user123", 1000, 500).with_roles([Role::admin(), Role::user()]);
        assert!(claims.has_role(&Role::admin()));
        assert!(claims.has_role(&Role::user()));
    }

    #[test]
    fn test_claims_has_any_role() {
        let claims = Claims::new("user123", 1000, 500).with_role(Role::user());
        assert!(claims.has_any_role(&[Role::admin(), Role::user()]));
        assert!(!claims.has_any_role(&[Role::admin()]));
    }

    #[test]
    fn test_claims_has_all_roles() {
        let claims = Claims::new("user123", 1000, 500).with_roles([Role::admin(), Role::user()]);
        assert!(claims.has_all_roles(&[Role::admin(), Role::user()]));
        assert!(!claims.has_all_roles(&[Role::admin(), Role::new("moderator")]));
    }

    #[test]
    fn test_claims_refresh_token() {
        let claims = Claims::new("user123", 1000, 500).as_refresh_token();
        assert!(claims.is_refresh_token());
    }

    #[test]
    fn test_claims_with_email_and_name() {
        let claims = Claims::new("user123", 1000, 500)
            .with_email("user@example.com")
            .with_name("John Doe");
        assert_eq!(claims.email, Some("user@example.com".to_string()));
        assert_eq!(claims.name, Some("John Doe".to_string()));
    }

    #[test]
    fn test_claims_is_expired() {
        let past = Claims::new("user123", 0, 0);
        assert!(past.is_expired());

        let future = Claims::new("user123", u64::MAX, 0);
        assert!(!future.is_expired());
    }
}
