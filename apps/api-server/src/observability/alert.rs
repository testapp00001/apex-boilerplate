//! Critical error alerting layer for tracing.
//!
//! This layer intercepts ERROR-level events and dispatches alerts
//! to configured channels (Slack, PagerDuty, email, etc.).

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{Event, Subscriber};
use tracing_subscriber::{Layer, layer::Context};

/// Alert message containing error details.
#[derive(Debug, Clone)]
pub struct AlertMessage {
    pub level: String,
    pub message: String,
    pub target: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub fields: Vec<(String, String)>,
}

/// Configuration for the alert layer.
#[derive(Debug, Clone)]
pub struct AlertConfig {
    /// Minimum level to trigger alerts (default: ERROR).
    pub min_level: tracing::Level,
    /// Channel buffer size.
    pub buffer_size: usize,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            min_level: tracing::Level::ERROR,
            buffer_size: 100,
        }
    }
}

/// Trait for alert senders - implement this for different backends.
#[async_trait::async_trait]
pub trait AlertSender: Send + Sync {
    async fn send(&self, alert: AlertMessage) -> Result<(), AlertError>;
}

#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error("Failed to send alert: {0}")]
    SendError(String),
}

/// Console alert sender - logs alerts to stdout (for development).
pub struct ConsoleAlertSender;

#[async_trait::async_trait]
impl AlertSender for ConsoleAlertSender {
    async fn send(&self, alert: AlertMessage) -> Result<(), AlertError> {
        eprintln!(
            "\n🚨 CRITICAL ALERT 🚨\n\
             Level: {}\n\
             Target: {}\n\
             Message: {}\n\
             Time: {}\n",
            alert.level, alert.target, alert.message, alert.timestamp
        );
        Ok(())
    }
}

/// Webhook alert sender - sends alerts to a webhook URL (Slack, Discord, etc.).
pub struct WebhookAlertSender {
    url: String,
    client: reqwest::Client,
}

impl WebhookAlertSender {
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl AlertSender for WebhookAlertSender {
    async fn send(&self, alert: AlertMessage) -> Result<(), AlertError> {
        let payload = serde_json::json!({
            "text": format!(
                "🚨 *CRITICAL ERROR*\n*Target:* {}\n*Message:* {}\n*Time:* {}",
                alert.target, alert.message, alert.timestamp
            )
        });

        self.client
            .post(&self.url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AlertError::SendError(e.to_string()))?;

        Ok(())
    }
}

/// Tracing layer that sends alerts on ERROR-level events.
pub struct AlertLayer {
    sender: mpsc::Sender<AlertMessage>,
}

impl AlertLayer {
    /// Create a new alert layer with the given sender.
    pub fn new(alert_sender: Arc<dyn AlertSender>) -> Self {
        let (tx, mut rx) = mpsc::channel::<AlertMessage>(100);

        // Spawn background task to process alerts
        tokio::spawn(async move {
            while let Some(alert) = rx.recv().await {
                if let Err(e) = alert_sender.send(alert).await {
                    eprintln!("Failed to send alert: {}", e);
                }
            }
        });

        Self { sender: tx }
    }

    /// Create an alert layer that logs to console.
    pub fn console() -> Self {
        Self::new(Arc::new(ConsoleAlertSender))
    }

    /// Create an alert layer that sends to a webhook.
    pub fn webhook(url: String) -> Self {
        Self::new(Arc::new(WebhookAlertSender::new(url)))
    }
}

/// Visitor to extract fields from events.
struct FieldVisitor {
    message: String,
    fields: Vec<(String, String)>,
}

impl FieldVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
            fields: Vec::new(),
        }
    }
}

impl tracing::field::Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else {
            self.fields
                .push((field.name().to_string(), format!("{:?}", value)));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }
}

impl<S> Layer<S> for AlertLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Only alert on ERROR level
        if *event.metadata().level() != tracing::Level::ERROR {
            return;
        }

        let mut visitor = FieldVisitor::new();
        event.record(&mut visitor);

        let alert = AlertMessage {
            level: event.metadata().level().to_string(),
            message: visitor.message,
            target: event.metadata().target().to_string(),
            timestamp: chrono::Utc::now(),
            fields: visitor.fields,
        };

        // Non-blocking send
        let _ = self.sender.try_send(alert);
    }
}
