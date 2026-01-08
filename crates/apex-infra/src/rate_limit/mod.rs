//! Rate limiting implementations.

mod memory;

pub use memory::{InMemoryRateLimiter, RateLimitConfig};

#[cfg(feature = "redis")]
mod redis;
#[cfg(feature = "redis")]
pub use self::redis::{RedisRateLimitConfig, RedisRateLimiter};
