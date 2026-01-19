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

use chrono::{DateTime, Utc};
use rustops_common::{Error, LogEntry, Metric, Result, ServiceId, TraceSpan};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod collector;
pub mod metrics;
pub mod normalizer;

pub use collector::{LogCollector, MetricsCollector, TraceCollector};
pub use metrics::TelemetryMetrics;
pub use normalizer::{TelemetryFormat, TelemetryNormalizer, TelemetryType};

/// Telemetry data wrapper for transport
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryEnvelope {
    /// Type of telemetry data
    pub telemetry_type: TelemetryType,
    /// Timestamp when the telemetry was collected
    pub timestamp: DateTime<Utc>,
    /// The actual telemetry payload
    pub payload: TelemetryPayload,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Telemetry payload variants
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TelemetryPayload {
    /// Metric data
    Metric(Metric),
    /// Log entry
    Log(LogEntry),
    /// Trace span
    Trace(TraceSpan),
}

impl From<Metric> for TelemetryPayload {
    fn from(metric: Metric) -> Self {
        Self::Metric(metric)
    }
}

impl From<LogEntry> for TelemetryPayload {
    fn from(entry: LogEntry) -> Self {
        Self::Log(entry)
    }
}

impl From<TraceSpan> for TelemetryPayload {
    fn from(span: TraceSpan) -> Self {
        Self::Trace(span)
    }
}

/// Kafka producer stub (for future implementation)
///
/// Note: This is a placeholder for when Kafka integration is added.
#[derive(Clone)]
pub struct KafkaProducer {
    _service_id: ServiceId,
}

impl KafkaProducer {
    /// Create a new Kafka producer (stub)
    pub fn new(_service_id: ServiceId) -> Result<Self> {
        // TODO: Implement actual Kafka producer
        Ok(Self {
            _service_id: ServiceId::new(),
        })
    }

    /// Produce a telemetry envelope (stub)
    pub async fn produce(&self, _envelope: TelemetryEnvelope) -> Result<()> {
        // TODO: Implement actual Kafka production
        Ok(())
    }
}
