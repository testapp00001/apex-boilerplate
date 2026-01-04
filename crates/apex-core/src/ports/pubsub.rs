//! Pub/Sub port - abstraction over pub/sub backends.

use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;

/// Message received from a channel.
#[derive(Debug, Clone)]
pub struct PubSubMessage {
    pub channel: String,
    pub payload: String,
}

/// Handler for incoming messages.
pub type MessageHandler =
    Box<dyn Fn(PubSubMessage) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Pub/Sub trait - abstraction over pub/sub backends.
#[async_trait]
pub trait PubSub: Send + Sync {
    /// Publish a message to a channel.
    async fn publish(&self, channel: &str, message: &str) -> Result<(), PubSubError>;

    /// Subscribe to a channel with a handler.
    async fn subscribe<F>(&self, channel: &str, handler: F) -> Result<(), PubSubError>
    where
        F: Fn(PubSubMessage) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static;

    /// Unsubscribe from a channel.
    async fn unsubscribe(&self, channel: &str) -> Result<(), PubSubError>;
}

/// Pub/Sub errors.
#[derive(Debug, thiserror::Error)]
pub enum PubSubError {
    #[error("Failed to publish: {0}")]
    PublishError(String),

    #[error("Failed to subscribe: {0}")]
    SubscribeError(String),

    #[error("Connection error: {0}")]
    Connection(String),
}
