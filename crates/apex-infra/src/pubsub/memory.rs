//! In-memory pub/sub implementation.
//!
//! This is a fallback when Redis is not available.
//! Works within a single process only.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{RwLock, broadcast};

use apex_core::ports::{PubSub, PubSubError, PubSubMessage};

/// In-memory pub/sub system.
pub struct InMemoryPubSub {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
    buffer_size: usize,
}

impl InMemoryPubSub {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            buffer_size,
        }
    }
}

impl Default for InMemoryPubSub {
    fn default() -> Self {
        Self::new(100)
    }
}

#[async_trait]
impl PubSub for InMemoryPubSub {
    async fn publish(&self, channel: &str, message: &str) -> Result<(), PubSubError> {
        let channels = self.channels.read().await;

        if let Some(sender) = channels.get(channel) {
            // Ignore send errors (no subscribers)
            let _ = sender.send(message.to_string());
            tracing::debug!(channel = %channel, "Message published");
        } else {
            tracing::debug!(channel = %channel, "No subscribers for channel");
        }

        Ok(())
    }

    async fn subscribe<F>(&self, channel: &str, handler: F) -> Result<(), PubSubError>
    where
        F: Fn(PubSubMessage) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static,
    {
        let mut channels = self.channels.write().await;

        // Create channel if it doesn't exist
        let sender = channels
            .entry(channel.to_string())
            .or_insert_with(|| broadcast::channel(self.buffer_size).0);

        let mut receiver = sender.subscribe();
        let channel_name = channel.to_string();
        let handler = Arc::new(handler);

        tokio::spawn(async move {
            tracing::info!(channel = %channel_name, "Subscribed to channel");

            loop {
                match receiver.recv().await {
                    Ok(payload) => {
                        let msg = PubSubMessage {
                            channel: channel_name.clone(),
                            payload,
                        };
                        handler(msg).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        tracing::warn!(
                            channel = %channel_name,
                            lagged = count,
                            "Subscriber lagged behind"
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!(channel = %channel_name, "Channel closed");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    async fn unsubscribe(&self, channel: &str) -> Result<(), PubSubError> {
        let mut channels = self.channels.write().await;
        channels.remove(channel);
        tracing::info!(channel = %channel, "Unsubscribed from channel");
        Ok(())
    }
}
