//! # Apex Core
//!
//! The domain layer of the Apex boilerplate.
//! This crate contains pure business logic with zero infrastructure dependencies.

pub mod domain;
pub mod error;
pub mod ports;

pub use error::DomainError;
