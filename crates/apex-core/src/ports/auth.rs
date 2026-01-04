//! Authentication and authorization ports.

use async_trait::async_trait;
use uuid::Uuid;

/// Claims stored in JWT tokens.
#[derive(Debug, Clone)]
pub struct TokenClaims {
    pub user_id: Uuid,
    pub email: String,
    pub roles: Vec<String>,
    pub exp: i64,
}

/// Token service trait for JWT operations.
#[async_trait]
pub trait TokenService: Send + Sync {
    /// Generate access token for a user.
    fn generate_token(
        &self,
        user_id: Uuid,
        email: &str,
        roles: Vec<String>,
    ) -> Result<String, AuthError>;

    /// Validate and decode a token.
    fn validate_token(&self, token: &str) -> Result<TokenClaims, AuthError>;
}

/// Password hashing service.
pub trait PasswordService: Send + Sync {
    /// Hash a plain text password.
    fn hash(&self, password: &str) -> Result<String, AuthError>;

    /// Verify a password against a hash.
    fn verify(&self, password: &str, hash: &str) -> Result<bool, AuthError>;
}

/// Authentication errors.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Missing authorization header")]
    MissingAuth,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Hashing error: {0}")]
    HashingError(String),
}
