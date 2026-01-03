//! HTTP handlers and route configuration.

mod health;

use actix_web::web;

pub use health::health_check;

/// Configure all application routes.
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/api").route("/health", web::get().to(health::health_check)));
}
