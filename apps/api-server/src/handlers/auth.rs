//! Authentication handlers.

use actix_web::{HttpResponse, web};
use std::sync::Arc;

use apex_core::domain::User;
use apex_core::ports::{PasswordService, TokenService};
use apex_shared::dto::{AuthResponse, LoginRequest, RegisterUserRequest, UserResponse};

use crate::middleware::auth::Identity;
use crate::middleware::error::{AppError, AppResult};
use crate::state::AppState;

/// POST /api/auth/register
pub async fn register(
    state: web::Data<AppState>,
    token_service: web::Data<Arc<dyn TokenService>>,
    password_service: web::Data<Arc<dyn PasswordService>>,
    body: web::Json<RegisterUserRequest>,
) -> AppResult<HttpResponse> {
    let req = body.into_inner();

    // Validate input
    if req.email.is_empty() || !req.email.contains('@') {
        return Err(AppError::BadRequest("Invalid email address".to_string()));
    }
    if req.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    // Check if user already exists
    if let Some(_) = state.users.find_by_email(&req.email).await? {
        return Err(AppError::Conflict("Email already registered".to_string()));
    }

    // Hash password
    let password_hash = password_service
        .hash(&req.password)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Create user
    let user = User::new(req.email.clone(), password_hash);
    let saved_user = state.users.save(user).await?;

    // Generate token
    let token = token_service
        .generate_token(saved_user.id, &saved_user.email, vec!["user".to_string()])
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(HttpResponse::Created().json(AuthResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: token_service.expiration_seconds() as u64,
    }))
}

/// POST /api/auth/login
pub async fn login(
    state: web::Data<AppState>,
    token_service: web::Data<Arc<dyn TokenService>>,
    password_service: web::Data<Arc<dyn PasswordService>>,
    body: web::Json<LoginRequest>,
) -> AppResult<HttpResponse> {
    let req = body.into_inner();

    // Find user by email
    let user = state
        .users
        .find_by_email(&req.email)
        .await?
        .ok_or_else(|| AppError::Unauthorized)?;

    // Verify password
    let valid = password_service
        .verify(&req.password, &user.password_hash)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !valid {
        return Err(AppError::Unauthorized);
    }

    // Generate token
    let token = token_service
        .generate_token(user.id, &user.email, vec!["user".to_string()])
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(HttpResponse::Ok().json(AuthResponse {
        access_token: token,
        token_type: "Bearer".to_string(),
        expires_in: token_service.expiration_seconds() as u64,
    }))
}

/// GET /api/auth/me - Protected route
pub async fn me(identity: Identity) -> AppResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(UserResponse {
        id: identity.user_id.to_string(),
        email: identity.email,
        created_at: chrono::Utc::now().to_rfc3339(), // Would normally come from DB
    }))
}
