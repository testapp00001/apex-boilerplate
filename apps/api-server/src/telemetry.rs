//! Telemetry initialization - tracing and alerting setup.

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::observability::AlertLayer;

/// Telemetry configuration.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Enable JSON logging (for production).
    pub json_logs: bool,
    /// Service name for tracing.
    pub service_name: String,
    /// Enable critical error alerting.
    pub alerts_enabled: bool,
    /// Webhook URL for alerts (Slack, Discord, etc.).
    pub alert_webhook_url: Option<String>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            json_logs: false,
            service_name: "apex-api".to_string(),
            alerts_enabled: true,
            alert_webhook_url: None,
        }
    }
}

impl TelemetryConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        Self {
            json_logs: std::env::var("LOG_FORMAT")
                .map(|v| v.to_lowercase() == "json")
                .unwrap_or(false),
            service_name: std::env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "apex-api".to_string()),
            alerts_enabled: std::env::var("ALERTS_ENABLED")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
            alert_webhook_url: std::env::var("ALERT_WEBHOOK_URL").ok(),
        }
    }
}

/// Initialize telemetry (tracing and alerting).
pub fn init_telemetry(config: &TelemetryConfig) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,api_server=debug,apex_infra=debug"));

    // Create alert layer if enabled
    let alert_layer = if config.alerts_enabled {
        let layer = if let Some(webhook_url) = &config.alert_webhook_url {
            tracing::info!("Alert webhook configured");
            AlertLayer::webhook(webhook_url.clone())
        } else {
            AlertLayer::console()
        };
        Some(layer)
    } else {
        None
    };

    // Build and init subscriber based on log format
    if config.json_logs {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .with(alert_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().pretty())
            .with(alert_layer)
            .init();
    }

    tracing::info!(
        service = %config.service_name,
        json_logs = config.json_logs,
        alerts_enabled = config.alerts_enabled,
        "Telemetry initialized"
    );
}
