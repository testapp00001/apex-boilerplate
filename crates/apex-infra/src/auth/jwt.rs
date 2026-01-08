//! JWT token service implementation.

use chrono::{TimeDelta, Utc};
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
        let secret =
            std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-in-production".to_string());

        // Warn if using default secret in production
        if secret == "change-me-in-production" {
            let is_production = std::env::var("RUST_ENV")
                .map(|v| v == "production" || v == "prod")
                .unwrap_or(false);

            if is_production {
                tracing::error!(
                    "SECURITY: Using default JWT secret in production! Set JWT_SECRET environment variable."
                );
            } else {
                tracing::warn!("Using default JWT secret. Set JWT_SECRET for production use.");
            }
        }

        let config = JwtConfig {
            secret,
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
        let exp = now + TimeDelta::hours(self.config.expiration_hours);

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

    fn expiration_seconds(&self) -> i64 {
        self.config.expiration_hours * 3600
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> JwtConfig {
        JwtConfig {
            secret: "test-secret-key".to_string(),
            expiration_hours: 1,
            issuer: "test-issuer".to_string(),
        }
    }

    #[test]
    fn test_generate_token_success() {
        let service = JwtTokenService::new(test_config());
        let user_id = Uuid::new_v4();

        let result = service.generate_token(user_id, "test@example.com", vec!["user".to_string()]);

        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_validate_token_success() {
        let service = JwtTokenService::new(test_config());
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = service
            .generate_token(user_id, email, vec!["admin".to_string()])
            .unwrap();

        let claims = service.validate_token(&token).unwrap();

        assert_eq!(claims.user_id, user_id);
        assert_eq!(claims.email, email);
        assert_eq!(claims.roles, vec!["admin".to_string()]);
    }

    #[test]
    fn test_validate_invalid_token() {
        let service = JwtTokenService::new(test_config());

        let result = service.validate_token("invalid-token");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidToken(_)));
    }

    #[test]
    fn test_validate_wrong_issuer_token() {
        let service1 = JwtTokenService::new(JwtConfig {
            secret: "same-secret".to_string(),
            expiration_hours: 1,
            issuer: "issuer1".to_string(),
        });
        let service2 = JwtTokenService::new(JwtConfig {
            secret: "same-secret".to_string(),
            expiration_hours: 1,
            issuer: "issuer2".to_string(),
        });

        let token = service1
            .generate_token(Uuid::new_v4(), "test@test.com", vec![])
            .unwrap();

        let result = service2.validate_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_expiration_seconds() {
        let service = JwtTokenService::new(JwtConfig {
            secret: "test".to_string(),
            expiration_hours: 24,
            issuer: "test".to_string(),
        });

        assert_eq!(service.expiration_seconds(), 86400);
    }
}
