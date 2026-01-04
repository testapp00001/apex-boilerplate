//! Ports - trait definitions for external dependencies.
//! These are the "interfaces" that infrastructure must implement.

mod auth;
mod cache;
mod rate_limit;
mod repository;

pub use auth::{AuthError, PasswordService, TokenClaims, TokenService};
pub use cache::{Cache, CacheError};
pub use rate_limit::{RateLimitError, RateLimitResult, RateLimiter};
pub use repository::UserRepository;
