//! Domain-level error types.

use thiserror::Error;
use uuid::Uuid;

/// Domain errors - business logic failures.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Entity not found: {entity_type} with id {id}")]
    NotFound { entity_type: &'static str, id: Uuid },

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Duplicate entity: {0}")]
    Duplicate(String),

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Repository-level errors.
#[derive(Debug, Error)]
pub enum RepoError {
    #[error("Database connection failed: {0}")]
    Connection(String),

    #[error("Query execution failed: {0}")]
    Query(String),

    #[error("Entity not found")]
    NotFound,

    #[error("Constraint violation: {0}")]
    Constraint(String),
}
