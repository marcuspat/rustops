//! # Telemetry metrics
//!
//! Metrics for monitoring the telemetry pipeline itself.

use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram, CounterVec, GaugeVec,
    Histogram,
};
use std::sync::Arc;

/// Telemetry pipeline metrics
#[derive(Clone)]
pub struct TelemetryMetrics {
    /// Metrics collected counter
    pub metrics_collected: CounterVec,
    /// Logs collected counter
    pub logs_collected: CounterVec,
    /// Spans collected counter
    pub spans_collected: CounterVec,
    /// Kafka produce errors counter
    pub kafka_produce_errors: CounterVec,
    /// Current buffer size gauge
    pub buffer_size: GaugeVec,
    /// Processing latency histogram
    pub processing_latency: Histogram,
}

impl TelemetryMetrics {
    /// Create new telemetry metrics
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            metrics_collected: register_counter_vec!(
                "telemetry_metrics_collected_total",
                "Total metrics collected",
                &["service"]
            )
            .unwrap(),

            logs_collected: register_counter_vec!(
                "telemetry_logs_collected_total",
                "Total logs collected",
                &["service", "level"]
            )
            .unwrap(),

            spans_collected: register_counter_vec!(
                "telemetry_spans_collected_total",
                "Total spans collected",
                &["service"]
            )
            .unwrap(),

            kafka_produce_errors: register_counter_vec!(
                "telemetry_kafka_produce_errors_total",
                "Total Kafka produce errors",
                &["topic"]
            )
            .unwrap(),

            buffer_size: register_gauge_vec!(
                "telemetry_buffer_size",
                "Current buffer size",
                &["collector_type"]
            )
            .unwrap(),

            processing_latency: register_histogram!(
                "telemetry_processing_latency_seconds",
                "Telemetry processing latency"
            )
            .unwrap(),
        })
    }
}

impl Default for TelemetryMetrics {
    fn default() -> Self {
        Self::new().as_ref().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = TelemetryMetrics::new();

        metrics.metrics_collected.with_label_values(&["test"]).inc();
        assert_eq!(
            metrics
                .metrics_collected
                .with_label_values(&["test"])
                .get() as u64,
            1
        );
    }
}
