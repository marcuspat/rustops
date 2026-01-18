//! # RustOps Telemetry Collection
//!
//! Bounded context for collecting, normalizing, and processing telemetry data.
//!
//! ## Architecture
//!
//! This crate implements the telemetry pipeline from ADR-0005:
//! - Metrics collection (Prometheus format)
//! - Log collection (fluentd compatible)
//! - Trace collection (OpenTelemetry OTLP)
//!
//! ## Key Components
//!
//! - **Collectors**: Ingest telemetry from various sources
//! - **Normalizers**: Standardize telemetry formats
//! - **Metrics**: Internal monitoring

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod collector;
pub mod metrics;
pub mod normalizer;

pub use collector::{LogCollector, MetricsCollector, TraceCollector};
pub use metrics::TelemetryMetrics;
pub use normalizer::{TelemetryNormalizer, TelemetryType};

use rustops_common::{Error, Result};
