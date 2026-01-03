//! Data Transfer Objects - request/response types for the API.

use serde::{Deserialize, Serialize};

/// Request to register a new user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUserRequest {
    pub email: String,
    pub password: String,
}

/// Request to login.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Response containing a user's public information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub created_at: String,
}

/// Response containing authentication tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}
