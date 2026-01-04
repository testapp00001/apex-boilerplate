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

/// Configure auth routes (only when auth feature is enabled).
#[cfg(feature = "auth")]
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
