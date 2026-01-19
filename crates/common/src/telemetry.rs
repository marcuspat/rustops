//! # Telemetry primitives
//!
//! Core types for metrics, logs, and traces in the RustOps platform.

use crate::{MetricId, ServiceId, TraceId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of metrics
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    /// Gauge - can go up or down (e.g., CPU usage, memory)
    Gauge,
    /// Counter - only increases (e.g., request count)
    Counter,
    /// Histogram - counted observations (e.g., request latency)
    Histogram,
    /// Summary - sampled observations (e.g., latency quantiles)
    Summary,
}

/// A time-series metric
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metric {
    /// Unique metric ID
    pub id: MetricId,
    /// Metric name (e.g., "cpu_usage_percent")
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Metric value
    pub value: f64,
    /// Service that produced this metric
    pub service_id: ServiceId,
    /// Dimensional labels
    pub labels: HashMap<String, String>,
    /// Collection timestamp
    pub timestamp: DateTime<Utc>,
}

impl Metric {
    /// Create a new metric
    pub fn new(
        name: impl Into<String>,
        metric_type: MetricType,
        value: f64,
        service_id: ServiceId,
        labels: HashMap<String, String>,
    ) -> Self {
        Self {
            id: MetricId::new(),
            name: name.into(),
            metric_type,
            value,
            service_id,
            labels,
            timestamp: Utc::now(),
        }
    }

    /// Create a gauge metric
    pub fn gauge(
        name: impl Into<String>,
        value: f64,
        service_id: ServiceId,
        labels: HashMap<String, String>,
    ) -> Self {
        Self::new(name, MetricType::Gauge, value, service_id, labels)
    }

    /// Create a counter metric
    pub fn counter(
        name: impl Into<String>,
        value: f64,
        service_id: ServiceId,
        labels: HashMap<String, String>,
    ) -> Self {
        Self::new(name, MetricType::Counter, value, service_id, labels)
    }
}

/// A log entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    /// Log level (trace, debug, info, warn, error)
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Service that produced this log
    pub service_id: ServiceId,
    /// Log labels/metadata
    pub labels: HashMap<String, String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Log level
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Trace - most verbose
    Trace,
    /// Debug
    Debug,
    /// Info
    Info,
    /// Warning
    Warn,
    /// Error
    Error,
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("invalid log level: {}", s)),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(
        level: LogLevel,
        message: impl Into<String>,
        service_id: ServiceId,
        labels: HashMap<String, String>,
    ) -> Self {
        Self {
            level,
            message: message.into(),
            service_id,
            labels,
            timestamp: Utc::now(),
        }
    }
}

/// A trace span representing a segment of work
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceSpan {
    /// Trace ID (all spans in a trace share this)
    pub trace_id: TraceId,
    /// Span ID (unique within trace)
    pub span_id: uuid::Uuid,
    /// Parent span ID (if any)
    pub parent_span_id: Option<uuid::Uuid>,
    /// Span name
    pub name: String,
    /// Service that produced this span
    pub service_id: ServiceId,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// Duration in microseconds
    pub duration_us: i64,
    /// Span attributes
    pub attributes: HashMap<String, String>,
    /// Events within this span
    pub events: Vec<SpanEvent>,
    /// Span status
    pub status: SpanStatus,
}

/// An event that occurred within a span
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpanEvent {
    /// Event name
    pub name: String,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event attributes
    pub attributes: HashMap<String, String>,
}

/// Span status
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    /// OK
    Ok,
    /// Unset
    Unset,
    /// Error
    Error {
        /// Error description
        description: String,
    },
}

impl TraceSpan {
    /// Create a new root span (no parent)
    pub fn root(
        trace_id: TraceId,
        name: impl Into<String>,
        service_id: ServiceId,
    ) -> Self {
        Self {
            trace_id,
            span_id: uuid::Uuid::new_v4(),
            parent_span_id: None,
            name: name.into(),
            service_id,
            start_time: Utc::now(),
            duration_us: 0,
            attributes: HashMap::new(),
            events: Vec::new(),
            status: SpanStatus::Unset,
        }
    }

    /// Create a child span
    pub fn child(&self, name: impl Into<String>) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: uuid::Uuid::new_v4(),
            parent_span_id: Some(self.span_id),
            name: name.into(),
            service_id: self.service_id,
            start_time: Utc::now(),
            duration_us: 0,
            attributes: HashMap::new(),
            events: Vec::new(),
            status: SpanStatus::Unset,
        }
    }

    /// Mark span as completed
    pub fn complete(&mut self, duration_us: i64) {
        self.duration_us = duration_us;
        self.status = SpanStatus::Ok;
    }

    /// Mark span as error
    pub fn error(&mut self, description: impl Into<String>, duration_us: i64) {
        self.duration_us = duration_us;
        self.status = SpanStatus::Error {
            description: description.into(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_creation() {
        let service_id = ServiceId::new();
        let mut labels = HashMap::new();
        labels.insert("host".to_string(), "server1".to_string());

        let metric = Metric::gauge("cpu_usage".to_string(), 75.5, service_id, labels);

        assert_eq!(metric.name, "cpu_usage");
        assert_eq!(metric.value, 75.5);
        assert_eq!(metric.metric_type, MetricType::Gauge);
    }

    #[test]
    fn test_log_entry() {
        let service_id = ServiceId::new();
        let log = LogEntry::new(LogLevel::Error, "Database connection failed", service_id, HashMap::new());

        assert_eq!(log.level, LogLevel::Error);
        assert_eq!(log.message, "Database connection failed");
    }

    #[test]
    fn test_trace_span_hierarchy() {
        let trace_id = TraceId::new();
        let service_id = ServiceId::new();

        let root = TraceSpan::root(trace_id, "http_request", service_id);
        assert!(root.parent_span_id.is_none());

        let child = root.child("db_query");
        assert_eq!(child.trace_id, trace_id);
        assert_eq!(child.parent_span_id, Some(root.span_id));
    }

    #[test]
    fn test_span_completion() {
        let trace_id = TraceId::new();
        let service_id = ServiceId::new();
        let mut span = TraceSpan::root(trace_id, "operation", service_id);

        span.complete(1000);
        assert_eq!(span.duration_us, 1000);
        assert_eq!(span.status, SpanStatus::Ok);
    }

    #[test]
    fn test_metric_serialization() {
        let service_id = ServiceId::new();
        let metric = Metric::counter("requests_total".to_string(), 42.0, service_id, HashMap::new());

        let json = serde_json::to_string(&metric).unwrap();
        let deserialized: Metric = serde_json::from_str(&json).unwrap();
        assert_eq!(metric.id, deserialized.id);
        assert_eq!(metric.value, deserialized.value);
    }
}
