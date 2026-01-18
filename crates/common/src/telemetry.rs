//! Telemetry types for RustOps.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A time-series metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    /// Metric name (e.g., "cpu_usage_percent").
    pub name: String,
    /// Metric value.
    pub value: f64,
    /// Dimensional labels.
    pub labels: HashMap<String, String>,
    /// Collection timestamp (Unix seconds).
    pub timestamp: i64,
}

impl Metric {
    /// Create a new metric.
    pub fn new(name: String, value: f64, labels: HashMap<String, String>) -> Self {
        Self {
            name,
            value,
            labels,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// A log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Log level.
    pub level: String,
    /// Log message.
    pub message: String,
    /// Log labels.
    pub labels: HashMap<String, String>,
    /// Timestamp (Unix nanoseconds).
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_creation() {
        let mut labels = HashMap::new();
        labels.insert("host".to_string(), "server1".to_string());

        let metric = Metric::new("cpu_usage".to_string(), 75.5, labels.clone());

        assert_eq!(metric.name, "cpu_usage");
        assert_eq!(metric.value, 75.5);
        assert_eq!(metric.labels, labels);
    }
}
