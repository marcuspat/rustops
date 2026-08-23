// Telemetry collector implementations
//
// Implements Prometheus, Datadog, and other telemetry integrations

/// Prometheus.
pub mod prometheus;

pub use prometheus::PrometheusAdapter;
