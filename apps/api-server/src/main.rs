//! # Apex API Server
//!
//! The main entry point for the Actix-web HTTP server.

use actix_web::{App, HttpServer, web};
use tracing_actix_web::TracingLogger;

mod config;
mod handlers;
mod state;

use config::AppConfig;
use state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    init_tracing();

    // Load configuration
    let config = AppConfig::from_env();

    tracing::info!(
        "Starting Apex API Server on {}:{}",
        config.host,
        config.port
    );

    // Build application state
    let state = AppState::new(config.database.as_ref()).await;

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(state.clone()))
            .configure(handlers::configure_routes)
    })
    .bind((config.host.as_str(), config.port))?
    .run()
    .await
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,api_server=debug,apex_infra=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
}
