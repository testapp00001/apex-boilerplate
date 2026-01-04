//! # Apex API Server
//!
//! The main entry point for the Actix-web HTTP server.

use actix_web::{App, HttpServer, web};
use std::sync::Arc;
use tokio::signal;
use tracing_actix_web::TracingLogger;

mod config;
mod handlers;
mod middleware;
mod observability;
mod state;
mod telemetry;

use apex_core::ports::{RateLimiter, TokenService};
use apex_infra::{InMemoryRateLimiter, JwtTokenService};
use config::AppConfig;
use observability::RequestIdMiddleware;
use state::AppState;
use telemetry::TelemetryConfig;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize telemetry (tracing, alerts)
    let telemetry_config = TelemetryConfig::from_env();
    telemetry::init_telemetry(&telemetry_config);

    // Load configuration
    let config = AppConfig::from_env();

    tracing::info!(
        host = %config.host,
        port = %config.port,
        "Starting Apex API Server"
    );

    // Build application state
    let state = AppState::new(config.database.as_ref()).await;

    // Create services
    let token_service: Arc<dyn TokenService> = Arc::new(JwtTokenService::from_env());
    let rate_limiter: Arc<dyn RateLimiter> = Arc::new(InMemoryRateLimiter::from_env());

    // Start HTTP server with graceful shutdown
    let server = HttpServer::new(move || {
        App::new()
            // Middleware (order matters - first added = outermost)
            .wrap(TracingLogger::default())
            .wrap(RequestIdMiddleware)
            .wrap(middleware::rate_limit::RateLimitMiddleware::new(
                rate_limiter.clone(),
            ))
            // App data
            .app_data(web::Data::new(state.clone()))
            .app_data(web::Data::new(token_service.clone()))
            // Routes
            .configure(handlers::configure_routes)
    })
    .bind((config.host.as_str(), config.port))?
    .run();

    // Graceful shutdown handling
    let server_handle = server.handle();

    tokio::spawn(async move {
        shutdown_signal().await;
        tracing::info!("Shutdown signal received, starting graceful shutdown...");
        server_handle.stop(true).await;
    });

    server.await
}

/// Wait for shutdown signals (Ctrl+C or SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
