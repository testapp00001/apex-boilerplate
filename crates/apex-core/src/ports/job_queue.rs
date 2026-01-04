//! Job queue port - abstraction over job queue backends.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// A job that can be queued and processed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier.
    pub id: String,
    /// Job type/name for routing to handlers.
    pub job_type: String,
    /// Serialized payload.
    pub payload: serde_json::Value,
    /// Number of retry attempts.
    pub attempts: u32,
    /// Maximum retry attempts.
    pub max_attempts: u32,
    /// When the job was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When to execute the job (for delayed jobs).
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Job {
    pub fn new(job_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            job_type: job_type.into(),
            payload,
            attempts: 0,
            max_attempts: 3,
            created_at: chrono::Utc::now(),
            scheduled_at: None,
        }
    }

    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }

    pub fn delayed(mut self, delay: chrono::Duration) -> Self {
        self.scheduled_at = Some(chrono::Utc::now() + delay);
        self
    }
}

/// Result of job processing.
#[derive(Debug)]
pub enum JobResult {
    /// Job completed successfully.
    Success,
    /// Job failed, should be retried.
    Retry(String),
    /// Job failed permanently, should not be retried.
    Failed(String),
}

/// Job handler function type.
pub type JobHandler =
    Box<dyn Fn(Job) -> Pin<Box<dyn Future<Output = JobResult> + Send>> + Send + Sync>;

/// Job queue trait - abstraction over job queue backends.
#[async_trait]
pub trait JobQueue: Send + Sync {
    /// Enqueue a job for processing.
    async fn enqueue(&self, job: Job) -> Result<(), JobQueueError>;

    /// Start processing jobs with the given handler.
    async fn start_worker<F>(&self, handler: F) -> Result<(), JobQueueError>
    where
        F: Fn(Job) -> Pin<Box<dyn Future<Output = JobResult> + Send>> + Send + Sync + 'static;

    /// Get queue statistics.
    async fn stats(&self) -> Result<QueueStats, JobQueueError>;
}

/// Queue statistics.
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub pending: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
}

/// Job queue errors.
#[derive(Debug, thiserror::Error)]
pub enum JobQueueError {
    #[error("Failed to enqueue job: {0}")]
    EnqueueError(String),

    #[error("Queue is full")]
    QueueFull,

    #[error("Backend error: {0}")]
    Backend(String),
}
