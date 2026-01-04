//! # Apex API Server
//!
//! The main entry point for the Actix-web HTTP server.
//!
//! ## Feature Flags
//!
//! - `full` (default) - All features enabled
//! - `minimal` - Bare HTTP server only
//! - `postgres` - PostgreSQL database support
//! - `auth` - JWT authentication
//! - `rate-limit` - Request rate limiting
//! - `scheduler` - Cron job scheduling
//! - `websocket` - WebSocket support
//! - `otel` - OpenTelemetry tracing

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

#[cfg(feature = "scheduler")]
mod background;

#[cfg(feature = "websocket")]
mod websocket;

use config::AppConfig;
use observability::RequestIdMiddleware;
use state::AppState;
use telemetry::TelemetryConfig;

#[cfg(feature = "auth")]
use apex_core::ports::TokenService;

#[cfg(feature = "rate-limit")]
use apex_core::ports::RateLimiter;

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

    // Create services based on features
    #[cfg(feature = "auth")]
    let token_service: Arc<dyn TokenService> = Arc::new(apex_infra::JwtTokenService::from_env());

    #[cfg(feature = "rate-limit")]
    let rate_limiter: Arc<dyn RateLimiter> = Arc::new(apex_infra::InMemoryRateLimiter::from_env());

    // Job queue (always available - in-memory fallback)
    let job_queue = Arc::new(apex_infra::InMemoryJobQueue::from_env());

    // Start job workers
    let jq = job_queue.clone();
    tokio::spawn(async move {
        use apex_core::ports::{JobQueue, JobResult};

        if let Err(e) = jq
            .start_worker(|job| {
                Box::pin(async move {
                    tracing::info!(job_id = %job.id, job_type = %job.job_type, "Processing job");
                    match job.job_type.as_str() {
                        "email" => {
                            tracing::info!("Sending email: {:?}", job.payload);
                            JobResult::Success
                        }
                        "cleanup" => {
                            tracing::info!("Running cleanup");
                            JobResult::Success
                        }
                        _ => {
                            tracing::warn!("Unknown job type: {}", job.job_type);
                            JobResult::Failed(format!("Unknown job type: {}", job.job_type))
                        }
                    }
                })
            })
            .await
        {
            tracing::error!("Failed to start job worker: {}", e);
        }
    });

    // Initialize scheduler if enabled
    #[cfg(feature = "scheduler")]
    {
        use background::{Scheduler, SchedulerConfig};

        let scheduler_config = SchedulerConfig::from_env();
        let scheduler = Scheduler::new(scheduler_config)
            .await
            .expect("Failed to create scheduler");

        // Add heartbeat cron job (runs every minute)
        scheduler
            .add_cron("0 * * * * *", || async {
                tracing::debug!("Heartbeat cron job running");
            })
            .await
            .ok();

        scheduler.start().await.expect("Failed to start scheduler");
    }

    // Initialize WebSocket layer if enabled
    #[cfg(feature = "websocket")]
    let (_socket_layer, _io) = {
        use websocket::WsState;
        let pubsub = Arc::new(apex_infra::InMemoryPubSub::default());
        let ws_state = WsState { pubsub };
        websocket::create_socketio_layer(ws_state)
    };

    // Start HTTP server with graceful shutdown
    let server = HttpServer::new(move || {
        #[cfg(feature = "rate-limit")]
        let rate_limiter_clone = rate_limiter.clone();

        #[cfg(feature = "auth")]
        let token_service_clone = token_service.clone();

        // Build app with all middleware upfront
        #[cfg(all(feature = "rate-limit"))]
        let app = App::new()
            .wrap(TracingLogger::default())
            .wrap(RequestIdMiddleware)
            .wrap(middleware::rate_limit::RateLimitMiddleware::new(
                rate_limiter_clone,
            ));

        #[cfg(not(feature = "rate-limit"))]
        let app = App::new()
            .wrap(TracingLogger::default())
            .wrap(RequestIdMiddleware);

        // Add data
        let app = app
            .app_data(web::Data::new(state.clone()))
            .app_data(web::Data::new(job_queue.clone()));

        #[cfg(feature = "auth")]
        let app = app.app_data(web::Data::new(token_service_clone));

        // Configure routes
        app.configure(handlers::configure_routes)
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
