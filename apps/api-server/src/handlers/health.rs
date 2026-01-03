//! Health check endpoint.

use actix_web::{HttpResponse, web};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub timestamp: String,
}

/// Health check endpoint - returns server status.
///
/// GET /api/health
pub async fn health_check(_state: web::Data<AppState>) -> HttpResponse {
    let response = HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    HttpResponse::Ok().json(response)
}
