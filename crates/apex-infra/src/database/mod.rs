//! Database connection management.

mod connections;

#[cfg(feature = "postgres")]
mod postgres_repo;

#[cfg(feature = "postgres")]
pub mod entity;

pub use connections::{DatabaseConfig, DatabaseConnections, NamedConnection, SecondaryDbConfig};

#[cfg(feature = "postgres")]
pub use postgres_repo::PostgresUserRepository;
