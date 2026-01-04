//! Error handling middleware - RFC 7807 compliant responses.

use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use apex_shared::ErrorResponse;
use std::fmt;

/// Application-level error type that converts to RFC 7807 responses.
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Unauthorized,
    Forbidden,
    Conflict(String),
    Internal(String),
    Validation(Vec<String>),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::Unauthorized => write!(f, "Unauthorized"),
            AppError::Forbidden => write!(f, "Forbidden"),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
            AppError::Validation(errors) => write!(f, "Validation errors: {:?}", errors),
        }
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let error = match self {
            AppError::NotFound(detail) => ErrorResponse::not_found(detail),
            AppError::BadRequest(detail) => ErrorResponse::bad_request(detail),
            AppError::Unauthorized => ErrorResponse::unauthorized(),
            AppError::Forbidden => ErrorResponse::forbidden(),
            AppError::Conflict(detail) => ErrorResponse::new(409, "Conflict").with_detail(detail),
            AppError::Internal(detail) => {
                // Log internal errors
                tracing::error!("Internal error: {}", detail);
                ErrorResponse::internal_error()
            }
            AppError::Validation(errors) => {
                ErrorResponse::new(422, "Validation Failed").with_detail(errors.join(", "))
            }
        };

        HttpResponse::build(self.status_code()).json(error)
    }
}

// Conversion from domain errors
impl From<apex_core::error::DomainError> for AppError {
    fn from(err: apex_core::error::DomainError) -> Self {
        match err {
            apex_core::error::DomainError::NotFound { entity_type, id } => {
                AppError::NotFound(format!("{} with id {} not found", entity_type, id))
            }
            apex_core::error::DomainError::Validation(msg) => AppError::BadRequest(msg),
            apex_core::error::DomainError::Duplicate(msg) => AppError::Conflict(msg),
            apex_core::error::DomainError::Unauthorized => AppError::Unauthorized,
            apex_core::error::DomainError::Internal(msg) => AppError::Internal(msg),
        }
    }
}

impl From<apex_core::error::RepoError> for AppError {
    fn from(err: apex_core::error::RepoError) -> Self {
        match err {
            apex_core::error::RepoError::NotFound => {
                AppError::NotFound("Resource not found".to_string())
            }
            apex_core::error::RepoError::Constraint(msg) => AppError::Conflict(msg),
            apex_core::error::RepoError::Connection(msg) => {
                tracing::error!("Database connection error: {}", msg);
                AppError::Internal("Database error".to_string())
            }
            apex_core::error::RepoError::Query(msg) => {
                tracing::error!("Database query error: {}", msg);
                AppError::Internal("Database error".to_string())
            }
        }
    }
}

/// Result type alias for handlers.
pub type AppResult<T> = Result<T, AppError>;
