//! # Apex Shared
//!
//! Shared types between frontend and backend.
//! In a full-stack Rust setup, this crate is compiled for both server and WASM.

pub mod dto;
pub mod response;

pub use response::{ApiResponse, ErrorResponse};
