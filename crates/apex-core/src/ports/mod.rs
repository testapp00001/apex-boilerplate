//! Ports - trait definitions for external dependencies.
//! These are the "interfaces" that infrastructure must implement.

mod cache;
mod repository;

pub use cache::{Cache, CacheError};
pub use repository::UserRepository;
