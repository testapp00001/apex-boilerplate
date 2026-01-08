//! Redis job queue implementation using LIST operations.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};
use tokio::sync::RwLock;

use apex_core::ports::{Job, JobQueue, JobQueueError, JobResult, QueueStats};

use crate::cache::RedisConfig;

/// Redis job queue configuration.
#[derive(Debug, Clone)]
pub struct RedisJobQueueConfig {
    /// Redis connection config
    pub redis: RedisConfig,
    /// Queue name/key prefix
    pub queue_name: String,
    /// Number of worker consumers
    pub workers: usize,
    /// Timeout for blocking pop (seconds)
    pub pop_timeout: u64,
}

impl Default for RedisJobQueueConfig {
    fn default() -> Self {
        Self {
            redis: RedisConfig::default(),
            queue_name: "jobs".to_string(),
            workers: 4,
            pop_timeout: 5,
        }
    }
}

impl RedisJobQueueConfig {
    pub fn from_env() -> Self {
        Self {
            redis: RedisConfig::from_env(),
            queue_name: std::env::var("JOB_QUEUE_NAME").unwrap_or_else(|_| "jobs".to_string()),
            workers: std::env::var("JOB_QUEUE_WORKERS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4),
            pop_timeout: std::env::var("JOB_QUEUE_POP_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
        }
    }
}

/// Redis-backed job queue using LIST operations.
pub struct RedisJobQueue {
    conn: ConnectionManager,
    config: RedisJobQueueConfig,
    stats: Arc<JobStats>,
    running: Arc<RwLock<bool>>,
}

#[derive(Debug, Default)]
struct JobStats {
    pending: AtomicUsize,
    processing: AtomicUsize,
    completed: AtomicUsize,
    failed: AtomicUsize,
}

impl RedisJobQueue {
    pub async fn new(config: RedisJobQueueConfig) -> Result<Self, JobQueueError> {
        let client = Client::open(config.redis.url.as_str())
            .map_err(|e| JobQueueError::Backend(e.to_string()))?;

        // Use timeout to prevent hanging if Redis is unreachable
        let conn_manager_fut = ConnectionManager::new(client);
        let conn = tokio::time::timeout(config.redis.connect_timeout, conn_manager_fut)
            .await
            .map_err(|_| JobQueueError::Backend("Connection timed out".to_string()))?
            .map_err(|e| JobQueueError::Backend(e.to_string()))?;

        tracing::info!(
            url = %config.redis.url,
            queue = %config.queue_name,
            workers = config.workers,
            "Connected to Redis job queue"
        );

        Ok(Self {
            conn,
            config,
            stats: Arc::new(JobStats::default()),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Create from environment configuration.
    pub async fn from_env() -> Result<Self, JobQueueError> {
        Self::new(RedisJobQueueConfig::from_env()).await
    }

    fn pending_key(&self) -> String {
        format!("{}:pending", self.config.queue_name)
    }
}

#[async_trait]
impl JobQueue for RedisJobQueue {
    async fn enqueue(&self, job: Job) -> Result<(), JobQueueError> {
        let mut conn = self.conn.clone();
        let job_json =
            serde_json::to_string(&job).map_err(|e| JobQueueError::EnqueueError(e.to_string()))?;

        conn.rpush::<_, _, ()>(&self.pending_key(), &job_json)
            .await
            .map_err(|e| JobQueueError::Backend(e.to_string()))?;

        self.stats.pending.fetch_add(1, Ordering::Relaxed);
        tracing::debug!(job_id = %job.id, job_type = %job.job_type, "Job enqueued");

        Ok(())
    }

    async fn start_worker<F>(&self, handler: F) -> Result<(), JobQueueError>
    where
        F: Fn(Job) -> Pin<Box<dyn Future<Output = JobResult> + Send>> + Send + Sync + 'static,
    {
        *self.running.write().await = true;
        let handler = Arc::new(handler);

        for worker_id in 0..self.config.workers {
            let conn = self.conn.clone();
            let pending_key = self.pending_key();
            let stats = self.stats.clone();
            let running = self.running.clone();
            let handler = handler.clone();
            let pop_timeout = self.config.pop_timeout;
            let queue_name = self.config.queue_name.clone();

            tokio::spawn(async move {
                tracing::info!(
                    worker_id = worker_id,
                    queue = %queue_name,
                    "Job queue worker started"
                );

                let mut conn = conn;

                loop {
                    if !*running.read().await {
                        tracing::info!(worker_id = worker_id, "Worker stopping");
                        break;
                    }

                    // Blocking pop with timeout
                    let result: Result<Option<(String, String)>, _> =
                        conn.blpop(&pending_key, pop_timeout as f64).await;

                    let job_json = match result {
                        Ok(Some((_, json))) => json,
                        Ok(None) => continue, // Timeout, loop again
                        Err(e) => {
                            tracing::error!(error = %e, "Redis BLPOP error");
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    };

                    let mut job: Job = match serde_json::from_str(&job_json) {
                        Ok(j) => j,
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to deserialize job");
                            stats.failed.fetch_add(1, Ordering::Relaxed);
                            continue;
                        }
                    };

                    stats.pending.fetch_sub(1, Ordering::Relaxed);
                    stats.processing.fetch_add(1, Ordering::Relaxed);

                    job.attempts += 1;
                    let job_id = job.id.clone();
                    let job_type = job.job_type.clone();

                    tracing::debug!(
                        worker_id = worker_id,
                        job_id = %job_id,
                        job_type = %job_type,
                        attempt = job.attempts,
                        "Processing job"
                    );

                    match handler(job.clone()).await {
                        JobResult::Success => {
                            stats.processing.fetch_sub(1, Ordering::Relaxed);
                            stats.completed.fetch_add(1, Ordering::Relaxed);
                            tracing::debug!(job_id = %job_id, "Job completed successfully");
                        }
                        JobResult::Retry(reason) => {
                            stats.processing.fetch_sub(1, Ordering::Relaxed);
                            if job.attempts < job.max_attempts {
                                // Re-enqueue for retry
                                let job_json = serde_json::to_string(&job).unwrap();
                                if let Err(e) =
                                    conn.rpush::<_, _, ()>(&pending_key, &job_json).await
                                {
                                    tracing::error!(error = %e, "Failed to re-enqueue job for retry");
                                    stats.failed.fetch_add(1, Ordering::Relaxed);
                                } else {
                                    stats.pending.fetch_add(1, Ordering::Relaxed);
                                    tracing::warn!(
                                        job_id = %job_id,
                                        attempt = job.attempts,
                                        reason = %reason,
                                        "Job queued for retry"
                                    );
                                }
                            } else {
                                stats.failed.fetch_add(1, Ordering::Relaxed);
                                tracing::error!(
                                    job_id = %job_id,
                                    reason = %reason,
                                    "Job failed after max retries"
                                );
                            }
                        }
                        JobResult::Failed(reason) => {
                            stats.processing.fetch_sub(1, Ordering::Relaxed);
                            stats.failed.fetch_add(1, Ordering::Relaxed);
                            tracing::error!(job_id = %job_id, reason = %reason, "Job failed");
                        }
                    }
                }
            });
        }

        Ok(())
    }

    async fn stats(&self) -> Result<QueueStats, JobQueueError> {
        Ok(QueueStats {
            pending: self.stats.pending.load(Ordering::Relaxed),
            processing: self.stats.processing.load(Ordering::Relaxed),
            completed: self.stats.completed.load(Ordering::Relaxed),
            failed: self.stats.failed.load(Ordering::Relaxed),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::mpsc;

    async fn get_test_job_queue() -> Option<RedisJobQueue> {
        let config = RedisJobQueueConfig {
            redis: RedisConfig {
                url: std::env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://localhost:6389".to_string()),
                connect_timeout: Duration::from_secs(1),
                fallback_to_memory: false,
            },
            queue_name: "test_jobs".to_string(),
            workers: 1,
            pop_timeout: 1,
        };

        RedisJobQueue::new(config).await.ok()
    }

    #[tokio::test]
    async fn test_redis_job_queue() {
        let queue = match get_test_job_queue().await {
            Some(q) => q,
            None => return,
        };

        let (tx, mut rx) = mpsc::channel(1);
        let job_type = "test_job";
        let payload = serde_json::json!({"foo": "bar"});
        let job = Job::new(job_type, payload.clone());

        queue
            .start_worker(move |job| {
                let tx = tx.clone();
                Box::pin(async move {
                    tx.send(job.payload).await.unwrap();
                    JobResult::Success
                })
            })
            .await
            .unwrap();

        queue.enqueue(job).await.unwrap();

        let received = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .unwrap();
        assert_eq!(received.unwrap(), payload);

        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.completed, 1);

        *queue.running.write().await = false;
    }
}
