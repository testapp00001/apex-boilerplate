//! API route handlers.

mod health;

#[cfg(feature = "auth")]
mod auth;

use actix_web::web;

/// Configure all API routes.
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/health", web::get().to(health::health_check))
            .configure(configure_auth_routes),
    );
}

/// Configure auth routes with stricter rate limiting.
#[cfg(all(feature = "auth", feature = "rate-limit"))]
fn configure_auth_routes(cfg: &mut web::ServiceConfig) {
    use crate::middleware::rate_limit::RateLimitMiddleware;
    use apex_infra::{InMemoryRateLimiter, RateLimitConfig};
    use std::sync::Arc;
    use std::time::Duration;

    // Stricter rate limit for auth: 10 requests per 60 seconds
    // This helps prevent brute-force login attempts
    let auth_limiter = Arc::new(InMemoryRateLimiter::new(RateLimitConfig {
        max_requests: 10,
        window: Duration::from_secs(60),
    }));

    cfg.service(
        web::scope("/auth")
            .wrap(RateLimitMiddleware::new(auth_limiter))
            .route("/register", web::post().to(auth::register))
            .route("/login", web::post().to(auth::login))
            .route("/me", web::get().to(auth::me)),
    );
}

/// Configure auth routes without rate limiting (rate-limit feature disabled).
#[cfg(all(feature = "auth", not(feature = "rate-limit")))]
fn configure_auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/register", web::post().to(auth::register))
            .route("/login", web::post().to(auth::login))
            .route("/me", web::get().to(auth::me)),
    );
}

#[cfg(not(feature = "auth"))]
fn configure_auth_routes(_cfg: &mut web::ServiceConfig) {
    // No auth routes when feature is disabled
}
