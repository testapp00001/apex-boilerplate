//! # Apex Infrastructure
//!
//! Concrete implementations of the ports defined in `apex-core`.
//! This crate contains database, cache, and external service integrations.
//!
//! ## Feature Flags
//!
//! - `full` (default) - All features enabled
//! - `minimal` - No external dependencies, in-memory only
//! - `postgres` - PostgreSQL database support via SeaORM
//! - `auth` - JWT + Argon2 authentication
//! - `rate-limit` - Rate limiting via governor

pub mod cache;
pub mod database;
pub mod jobs;
pub mod pubsub;

#[cfg(feature = "auth")]
pub mod auth;

#[cfg(feature = "rate-limit")]
pub mod rate_limit;

// Re-exports
pub use database::DatabaseConnections;
pub use jobs::InMemoryJobQueue;
pub use pubsub::InMemoryPubSub;

#[cfg(feature = "auth")]
pub use auth::{Argon2PasswordService, JwtTokenService};

#[cfg(feature = "rate-limit")]
pub use rate_limit::{InMemoryRateLimiter, RateLimitConfig};
