//! # S3 TUI Library
//!
//! This library provides the core functionality for the S3 TUI application,
//! including S3 data fetching, transfer management, and model types.
//!
//! The library is primarily used by the s3tui binary, but can also be used
//! for integration testing with S3-compatible storage.

#![forbid(unsafe_code)]

pub mod model;
pub mod services;
pub mod settings;
pub mod utils;
