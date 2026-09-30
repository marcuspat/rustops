// Telemetry collector implementations
//
// Implements Prometheus, Datadog, and other telemetry integrations

/// Prometheus telemetry collector implementation.
pub mod prometheus;

pub use prometheus::PrometheusAdapter;
