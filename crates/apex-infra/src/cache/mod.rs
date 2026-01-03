//! Cache implementations - Redis and in-memory fallback.

mod memory;

pub use memory::InMemoryCache;

// Redis implementation will be added when redis feature is enabled
// #[cfg(feature = "redis")]
// mod redis_cache;
// #[cfg(feature = "redis")]
// pub use redis_cache::RedisCache;
