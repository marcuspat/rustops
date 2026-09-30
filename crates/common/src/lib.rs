//! # RustOps Common
//!
//! Common utilities, types, and abstractions for the RustOps AIOps platform.
//!
//! ## Architecture
//!
//! This crate provides:
//! - Type-safe IDs using the newtype pattern
//! - Domain events for event sourcing
//! - Error types with proper context
//! - Configuration management
//! - Telemetry primitives

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod config;
pub mod error;
pub mod events;
pub mod ids;
pub mod telemetry;

#[cfg(test)]
pub mod testing;

// Re-export common types
pub use config::Config;
pub use error::{Error, Result};
pub use events::{DomainEvent, EventType, Severity};
pub use ids::*;
pub use telemetry::{LogEntry, Metric, MetricType, TraceSpan};
