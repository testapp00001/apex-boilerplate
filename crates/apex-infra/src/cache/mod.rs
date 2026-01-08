//! Cache implementations - Redis and in-memory fallback.

mod memory;

pub use memory::InMemoryCache;

#[cfg(feature = "redis")]
mod redis;
#[cfg(feature = "redis")]
pub use self::redis::{RedisCache, RedisConfig};
