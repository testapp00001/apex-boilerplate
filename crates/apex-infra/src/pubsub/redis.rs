//! Redis PubSub implementation.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};
use tokio::sync::RwLock;

use apex_core::ports::{PubSub, PubSubError, PubSubMessage};

use crate::cache::RedisConfig;

/// Redis-backed PubSub implementation.
pub struct RedisPubSub {
    conn: ConnectionManager,
    client: Client,
    subscriptions: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    #[allow(dead_code)]
    config: RedisConfig,
}

impl RedisPubSub {
    pub async fn new(config: RedisConfig) -> Result<Self, PubSubError> {
        let client = Client::open(config.url.as_str())
            .map_err(|e| PubSubError::Connection(e.to_string()))?;

        // Use timeout to prevent hanging if Redis is unreachable
        let conn_manager_fut = ConnectionManager::new(client.clone());
        let conn = tokio::time::timeout(config.connect_timeout, conn_manager_fut)
            .await
            .map_err(|_| PubSubError::Connection("Connection timed out".to_string()))?
            .map_err(|e| PubSubError::Connection(e.to_string()))?;

        tracing::info!(url = %config.url, "Connected to Redis PubSub");

        Ok(Self {
            conn,
            client,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Create from environment configuration.
    pub async fn from_env() -> Result<Self, PubSubError> {
        Self::new(RedisConfig::from_env()).await
    }
}

#[async_trait]
impl PubSub for RedisPubSub {
    async fn publish(&self, channel: &str, message: &str) -> Result<(), PubSubError> {
        let mut conn = self.conn.clone();
        conn.publish::<_, _, ()>(channel, message)
            .await
            .map_err(|e| PubSubError::PublishError(e.to_string()))?;
        Ok(())
    }

    async fn subscribe<F>(&self, channel: &str, handler: F) -> Result<(), PubSubError>
    where
        F: Fn(PubSubMessage) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static,
    {
        let client = self.client.clone();
        let channel_name = channel.to_string();
        let handler = Arc::new(handler);

        let handle = tokio::spawn(async move {
            let conn = match client.get_async_pubsub().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get pubsub connection");
                    return;
                }
            };

            let mut pubsub = conn;
            if let Err(e) = pubsub.subscribe(&channel_name).await {
                tracing::error!(channel = %channel_name, error = %e, "Failed to subscribe");
                return;
            }

            tracing::debug!(channel = %channel_name, "Subscribed to Redis channel");

            let mut stream = pubsub.on_message();
            while let Some(msg) = stream.next().await {
                let payload: String = match msg.get_payload() {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to get message payload");
                        continue;
                    }
                };

                let channel: String = msg.get_channel_name().to_string();
                let pubsub_msg = PubSubMessage { channel, payload };
                handler(pubsub_msg).await;
            }

            tracing::info!(channel = %channel_name, "PubSub connection closed");
        });

        self.subscriptions
            .write()
            .await
            .insert(channel.to_string(), handle);

        Ok(())
    }

    async fn unsubscribe(&self, channel: &str) -> Result<(), PubSubError> {
        if let Some(handle) = self.subscriptions.write().await.remove(channel) {
            handle.abort();
            tracing::debug!(channel = %channel, "Unsubscribed from Redis channel");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::mpsc;

    async fn get_test_pubsub() -> Option<RedisPubSub> {
        let config = RedisConfig {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6389".to_string()),
            connect_timeout: Duration::from_secs(1),
            fallback_to_memory: false,
        };

        RedisPubSub::new(config).await.ok()
    }

    #[tokio::test]
    async fn test_redis_pubsub() {
        let pubsub = match get_test_pubsub().await {
            Some(p) => p,
            None => return,
        };

        let channel = "test_channel";
        let message = "test_message";
        let (tx, mut rx) = mpsc::channel(1);

        pubsub
            .subscribe(channel, move |msg| {
                let tx = tx.clone();
                Box::pin(async move {
                    tx.send(msg.payload).await.unwrap();
                })
            })
            .await
            .unwrap();

        // Give some time for subscription to stabilize
        tokio::time::sleep(Duration::from_millis(100)).await;

        pubsub.publish(channel, message).await.unwrap();

        let received = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .unwrap();
        assert_eq!(received.unwrap(), message);

        pubsub.unsubscribe(channel).await.unwrap();
    }
}
