//! Cron-style job scheduler using tokio-cron-scheduler.

use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};

/// Scheduler configuration.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Enable scheduler.
    pub enabled: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl SchedulerConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("SCHEDULER_ENABLED")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
        }
    }
}

/// Cron job scheduler wrapper.
pub struct Scheduler {
    inner: JobScheduler,
    config: SchedulerConfig,
}

impl Scheduler {
    /// Create a new scheduler.
    pub async fn new(config: SchedulerConfig) -> Result<Self, JobSchedulerError> {
        let inner = JobScheduler::new().await?;
        Ok(Self { inner, config })
    }

    /// Add a cron job.
    ///
    /// # Example
    /// ```ignore
    /// scheduler.add_cron("0 0 * * * *", || async {
    ///     tracing::info!("Running hourly job");
    /// }).await?;
    /// ```
    pub async fn add_cron<F, Fut>(
        &self,
        schedule: &str,
        task: F,
    ) -> Result<uuid::Uuid, JobSchedulerError>
    where
        F: Fn() -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let job = Job::new_async(schedule, move |_uuid, _lock| {
            let task = task.clone();
            Box::pin(async move {
                task().await;
            })
        })?;

        let id = self.inner.add(job).await?;
        tracing::info!(schedule = %schedule, job_id = %id, "Cron job registered");
        Ok(id)
    }

    /// Add a one-shot delayed job.
    pub async fn add_one_shot<F, Fut>(
        &self,
        delay: std::time::Duration,
        task: F,
    ) -> Result<uuid::Uuid, JobSchedulerError>
    where
        F: FnOnce() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let task = Arc::new(tokio::sync::Mutex::new(Some(task)));

        let job = Job::new_one_shot_async(delay, move |_uuid, _lock| {
            let task = task.clone();
            Box::pin(async move {
                if let Some(t) = task.lock().await.take() {
                    t().await;
                }
            })
        })?;

        let id = self.inner.add(job).await?;
        tracing::info!(delay_secs = delay.as_secs(), job_id = %id, "One-shot job scheduled");
        Ok(id)
    }

    /// Start the scheduler.
    pub async fn start(&self) -> Result<(), JobSchedulerError> {
        if !self.config.enabled {
            tracing::info!("Scheduler disabled");
            return Ok(());
        }

        self.inner.start().await?;
        tracing::info!("Scheduler started");
        Ok(())
    }

    /// Stop the scheduler.
    pub async fn shutdown(&mut self) -> Result<(), JobSchedulerError> {
        self.inner.shutdown().await?;
        tracing::info!("Scheduler stopped");
        Ok(())
    }
}
