//! HTTP handlers and route configuration.

mod auth;
mod health;

use actix_web::web;

/// Configure all application routes.
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            // Public routes
            .route("/health", web::get().to(health::health_check))
            // Auth routes
            .service(
                web::scope("/auth")
                    .route("/register", web::post().to(auth::register))
                    .route("/login", web::post().to(auth::login))
                    .route("/me", web::get().to(auth::me)),
            ),
    );
}
