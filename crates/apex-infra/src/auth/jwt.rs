//! JWT token service implementation.

use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use apex_core::ports::{AuthError, TokenClaims, TokenService};

/// JWT token service configuration.
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
    pub issuer: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "change-me-in-production".to_string(),
            expiration_hours: 24,
            issuer: "apex-api".to_string(),
        }
    }
}

/// Internal JWT claims structure for serialization.
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // user_id
    email: String,
    roles: Vec<String>,
    exp: i64,    // expiration timestamp
    iat: i64,    // issued at
    iss: String, // issuer
}

/// JWT-based token service.
pub struct JwtTokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    config: JwtConfig,
}

impl JwtTokenService {
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            encoding_key,
            decoding_key,
            config,
        }
    }

    pub fn from_env() -> Self {
        let config = JwtConfig {
            secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(24),
            issuer: std::env::var("JWT_ISSUER").unwrap_or_else(|_| "apex-api".to_string()),
        };
        Self::new(config)
    }
}

impl TokenService for JwtTokenService {
    fn generate_token(
        &self,
        user_id: Uuid,
        email: &str,
        roles: Vec<String>,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.config.expiration_hours);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            roles,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            iss: self.config.issuer.clone(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))
    }

    fn validate_token(&self, token: &str) -> Result<TokenClaims, AuthError> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation).map_err(|e| {
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                _ => AuthError::InvalidToken(e.to_string()),
            }
        })?;

        let user_id = Uuid::parse_str(&token_data.claims.sub)
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(TokenClaims {
            user_id,
            email: token_data.claims.email,
            roles: token_data.claims.roles,
            exp: token_data.claims.exp,
        })
    }
}
