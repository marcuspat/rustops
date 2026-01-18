// Telemetry collector implementations
//
// Implements Prometheus, Datadog, and other telemetry integrations

pub mod prometheus;

pub use prometheus::PrometheusAdapter;
