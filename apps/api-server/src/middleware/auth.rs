//! Authentication middleware and extractors.

use actix_web::{FromRequest, HttpRequest, dev::Payload, http::header};
use std::future::{Ready, ready};
use std::sync::Arc;

use apex_core::ports::{AuthError, TokenClaims, TokenService};

/// Authenticated user identity extractor.
///
/// Use this in handlers to require authentication:
/// ```ignore
/// async fn protected_route(identity: Identity) -> impl Responder {
///     format!("Hello, user {}!", identity.user_id)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Identity {
    pub user_id: uuid::Uuid,
    pub email: String,
    pub roles: Vec<String>,
}

impl Identity {
    /// Check if the user has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

impl From<TokenClaims> for Identity {
    fn from(claims: TokenClaims) -> Self {
        Self {
            user_id: claims.user_id,
            email: claims.email,
            roles: claims.roles,
        }
    }
}

/// Error type for authentication failures.
#[derive(Debug)]
pub struct AuthenticationError(pub AuthError);

impl std::fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl actix_web::ResponseError for AuthenticationError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match &self.0 {
            AuthError::TokenExpired => actix_web::http::StatusCode::UNAUTHORIZED,
            AuthError::InvalidToken(_) => actix_web::http::StatusCode::UNAUTHORIZED,
            AuthError::MissingAuth => actix_web::http::StatusCode::UNAUTHORIZED,
            AuthError::InsufficientPermissions => actix_web::http::StatusCode::FORBIDDEN,
            _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        use apex_shared::ErrorResponse;

        let error = match &self.0 {
            AuthError::TokenExpired => ErrorResponse::new(401, "Token Expired")
                .with_detail("Your authentication token has expired. Please login again."),
            AuthError::InvalidToken(msg) => {
                ErrorResponse::new(401, "Invalid Token").with_detail(msg.clone())
            }
            AuthError::MissingAuth => ErrorResponse::new(401, "Authentication Required")
                .with_detail("Please provide a valid Bearer token in the Authorization header."),
            AuthError::InsufficientPermissions => ErrorResponse::forbidden(),
            _ => ErrorResponse::internal_error(),
        };

        actix_web::HttpResponse::build(self.status_code()).json(error)
    }
}

impl FromRequest for Identity {
    type Error = AuthenticationError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Get token service from app data
        let token_service = match req.app_data::<actix_web::web::Data<Arc<dyn TokenService>>>() {
            Some(service) => service,
            None => {
                tracing::error!("TokenService not found in app data");
                return ready(Err(AuthenticationError(AuthError::InvalidToken(
                    "Server configuration error".to_string(),
                ))));
            }
        };

        // Extract Bearer token from Authorization header
        let auth_header = match req.headers().get(header::AUTHORIZATION) {
            Some(value) => value,
            None => return ready(Err(AuthenticationError(AuthError::MissingAuth))),
        };

        let auth_str = match auth_header.to_str() {
            Ok(s) => s,
            Err(_) => {
                return ready(Err(AuthenticationError(AuthError::InvalidToken(
                    "Invalid authorization header".to_string(),
                ))));
            }
        };

        // Parse "Bearer <token>"
        let token = match auth_str.strip_prefix("Bearer ") {
            Some(t) => t,
            None => {
                return ready(Err(AuthenticationError(AuthError::InvalidToken(
                    "Expected Bearer token".to_string(),
                ))));
            }
        };

        // Validate token
        match token_service.validate_token(token) {
            Ok(claims) => ready(Ok(Identity::from(claims))),
            Err(e) => ready(Err(AuthenticationError(e))),
        }
    }
}

/// Optional identity extractor - doesn't fail if not authenticated.
pub struct OptionalIdentity(pub Option<Identity>);

impl FromRequest for OptionalIdentity {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        match Identity::from_request(req, payload).into_inner() {
            Ok(identity) => ready(Ok(OptionalIdentity(Some(identity)))),
            Err(_) => ready(Ok(OptionalIdentity(None))),
        }
    }
}
