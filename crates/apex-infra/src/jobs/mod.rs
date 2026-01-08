//! Job queue implementations.

mod memory;

pub use memory::InMemoryJobQueue;

#[cfg(feature = "redis")]
mod redis;
#[cfg(feature = "redis")]
pub use self::redis::{RedisJobQueue, RedisJobQueueConfig};
