//! # Apex API Server
//!
//! The main entry point for the Actix-web HTTP server.

use actix_web::{App, HttpServer, web};
use std::sync::Arc;
use tokio::signal;
use tracing_actix_web::TracingLogger;

mod background;
mod config;
mod handlers;
mod middleware;
mod observability;
mod state;
mod telemetry;
mod websocket;

use apex_core::ports::{RateLimiter, TokenService};
use apex_infra::{InMemoryJobQueue, InMemoryPubSub, InMemoryRateLimiter, JwtTokenService};
use background::{Scheduler, SchedulerConfig};
use config::AppConfig;
use observability::RequestIdMiddleware;
use state::AppState;
use telemetry::TelemetryConfig;
use websocket::WsState;

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
    let pubsub = Arc::new(InMemoryPubSub::default());

    // Initialize job queue
    let job_queue = Arc::new(InMemoryJobQueue::from_env());

    // Start job worker with example handler
    let jq = job_queue.clone();
    tokio::spawn(async move {
        use apex_core::ports::{JobQueue, JobResult};

        if let Err(e) = jq
            .start_worker(|job| {
                Box::pin(async move {
                    tracing::info!(job_id = %job.id, job_type = %job.job_type, "Processing job");
                    // Process based on job type
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

    // Initialize scheduler
    let scheduler_config = SchedulerConfig::from_env();
    let scheduler = Scheduler::new(scheduler_config)
        .await
        .expect("Failed to create scheduler");

    // Add example cron job (runs every minute)
    scheduler
        .add_cron("0 * * * * *", || async {
            tracing::debug!("Heartbeat cron job running");
        })
        .await
        .ok();

    scheduler.start().await.expect("Failed to start scheduler");

    // Create WebSocket state
    let ws_state = WsState {
        pubsub: pubsub.clone(),
    };
    let (_socket_layer, _io) = websocket::create_socketio_layer(ws_state);

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
            .app_data(web::Data::new(job_queue.clone()))
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
