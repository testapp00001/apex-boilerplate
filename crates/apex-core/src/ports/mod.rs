//! Ports - trait definitions for external dependencies.
//! These are the "interfaces" that infrastructure must implement.

mod auth;
mod cache;
mod job_queue;
mod pubsub;
mod rate_limit;
mod repository;

pub use auth::{AuthError, PasswordService, TokenClaims, TokenService};
pub use cache::{Cache, CacheError};
pub use job_queue::{Job, JobQueue, JobQueueError, JobResult, QueueStats};
pub use pubsub::{PubSub, PubSubError, PubSubMessage};
pub use rate_limit::{RateLimitError, RateLimitResult, RateLimiter};
pub use repository::UserRepository;
