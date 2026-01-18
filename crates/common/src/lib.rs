//! # RustOps Common
//!
//! Common utilities and types for the RustOps AIOps platform.

pub mod config;
pub mod error;
pub mod telemetry;

pub use config::Config;
pub use error::{Error, Result};
pub use telemetry::{Metric, LogEntry};
