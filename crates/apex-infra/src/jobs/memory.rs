//! In-memory job queue implementation.
//!
//! This is a fallback when Redis is not available.
//! Jobs are stored in memory and processed by local workers.
//! Note: Jobs are lost on server restart.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc};

use apex_core::ports::{Job, JobQueue, JobQueueError, JobResult, QueueStats};

/// In-memory job queue configuration.
#[derive(Debug, Clone)]
pub struct InMemoryJobQueueConfig {
    /// Maximum queue size (0 = unlimited).
    pub max_size: usize,
    /// Number of worker tasks.
    pub workers: usize,
}

impl Default for InMemoryJobQueueConfig {
    fn default() -> Self {
        Self {
            max_size: 10000,
            workers: 4,
        }
    }
}

/// In-memory job queue.
pub struct InMemoryJobQueue {
    stats: Arc<JobStats>,
    config: InMemoryJobQueueConfig,
    job_sender: mpsc::Sender<Job>,
    job_receiver: Arc<Mutex<mpsc::Receiver<Job>>>,
}

struct JobStats {
    pending: AtomicUsize,
    processing: AtomicUsize,
    completed: AtomicUsize,
    failed: AtomicUsize,
}

impl InMemoryJobQueue {
    pub fn new(config: InMemoryJobQueueConfig) -> Self {
        let (tx, rx) = mpsc::channel(config.max_size.max(100));

        Self {
            stats: Arc::new(JobStats {
                pending: AtomicUsize::new(0),
                processing: AtomicUsize::new(0),
                completed: AtomicUsize::new(0),
                failed: AtomicUsize::new(0),
            }),
            config,
            job_sender: tx,
            job_receiver: Arc::new(Mutex::new(rx)),
        }
    }

    pub fn from_env() -> Self {
        let config = InMemoryJobQueueConfig {
            max_size: std::env::var("JOB_QUEUE_MAX_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10000),
            workers: std::env::var("JOB_QUEUE_WORKERS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4),
        };
        Self::new(config)
    }
}

#[async_trait]
impl JobQueue for InMemoryJobQueue {
    async fn enqueue(&self, job: Job) -> Result<(), JobQueueError> {
        // Check queue size
        if self.config.max_size > 0 {
            let current_size = self.stats.pending.load(Ordering::Relaxed);
            if current_size >= self.config.max_size {
                return Err(JobQueueError::QueueFull);
            }
        }

        self.stats.pending.fetch_add(1, Ordering::Relaxed);

        self.job_sender
            .send(job)
            .await
            .map_err(|e| JobQueueError::EnqueueError(e.to_string()))?;

        tracing::debug!(
            "Job enqueued. Queue size: {}",
            self.stats.pending.load(Ordering::Relaxed)
        );

        Ok(())
    }

    async fn start_worker<F>(&self, handler: F) -> Result<(), JobQueueError>
    where
        F: Fn(Job) -> Pin<Box<dyn Future<Output = JobResult> + Send>> + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        let receiver = self.job_receiver.clone();
        let stats = self.stats.clone();
        let sender = self.job_sender.clone();

        for worker_id in 0..self.config.workers {
            let handler = handler.clone();
            let receiver = receiver.clone();
            let stats = stats.clone();
            let sender = sender.clone();

            tokio::spawn(async move {
                tracing::info!("Job worker {} started", worker_id);

                loop {
                    let job = {
                        let mut rx = receiver.lock().await;
                        rx.recv().await
                    };

                    match job {
                        Some(mut job) => {
                            stats.pending.fetch_sub(1, Ordering::Relaxed);
                            stats.processing.fetch_add(1, Ordering::Relaxed);

                            tracing::debug!(
                                worker = worker_id,
                                job_id = %job.id,
                                job_type = %job.job_type,
                                "Processing job"
                            );

                            job.attempts += 1;
                            let result = handler(job.clone()).await;

                            stats.processing.fetch_sub(1, Ordering::Relaxed);

                            match result {
                                JobResult::Success => {
                                    stats.completed.fetch_add(1, Ordering::Relaxed);
                                    tracing::debug!(job_id = %job.id, "Job completed successfully");
                                }
                                JobResult::Retry(reason) => {
                                    if job.attempts < job.max_attempts {
                                        tracing::warn!(
                                            job_id = %job.id,
                                            attempt = job.attempts,
                                            max_attempts = job.max_attempts,
                                            reason = %reason,
                                            "Job failed, will retry"
                                        );
                                        // Actually re-enqueue the job for retry
                                        // Small delay before retry to prevent tight loops
                                        let sender = sender.clone();
                                        tokio::spawn(async move {
                                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                                100 * job.attempts as u64,
                                            ))
                                            .await;
                                            if let Err(e) = sender.send(job).await {
                                                tracing::error!(
                                                    "Failed to re-enqueue job for retry: {}",
                                                    e
                                                );
                                            }
                                        });
                                        stats.pending.fetch_add(1, Ordering::Relaxed);
                                    } else {
                                        stats.failed.fetch_add(1, Ordering::Relaxed);
                                        tracing::error!(
                                            job_id = %job.id,
                                            reason = %reason,
                                            "Job failed after max retries"
                                        );
                                    }
                                }
                                JobResult::Failed(reason) => {
                                    stats.failed.fetch_add(1, Ordering::Relaxed);
                                    tracing::error!(job_id = %job.id, reason = %reason, "Job failed permanently");
                                }
                            }
                        }
                        None => {
                            tracing::info!("Job worker {} shutting down", worker_id);
                            break;
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
