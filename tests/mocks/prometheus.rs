//! Mock Prometheus server for testing.

use std::collections::HashMap;
use serde_json::json;

/// Mock Prometheus server.
pub struct MockPrometheus {
    base_url: String,
    metrics: HashMap<String, Vec<f64>>,
}

impl MockPrometheus {
    /// Create a new mock Prometheus server.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            metrics: HashMap::new(),
        }
    }

    /// Add a metric value.
    pub fn add_metric(&mut self, name: impl Into<String>, value: f64) {
        let name = name.into();
        self.metrics.entry(name).or_default().push(value);
    }

    /// Generate query response.
    pub fn query_response(&self, query: &str) -> serde_json::Value {
        json!({
            "status": "success",
            "data": {
                "resultType": "vector",
                "result": [{
                    "metric": {
                        "__name__": query,
                        "job": "test"
                    },
                    "value": [1705536000, "42.0"]
                }]
            }
        })
    }

    /// Generate query_range response for time series data.
    pub fn query_range_response(&self, query: &str) -> serde_json::Value {
        let values: Vec<Vec<serde_json::Value>> = (0..10)
            .map(|i| {
                vec![
                    json!(1705536000 + i * 60),
                    json!(50.0 + i as f64 * 2.5)
                ]
            })
            .collect();

        json!({
            "status": "success",
            "data": {
                "resultType": "matrix",
                "result": [{
                    "metric": {
                        "__name__": query,
                        "instance": "localhost:9090"
                    },
                    "values": values
                }]
            }
        })
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_prometheus_creation() {
        let prom = MockPrometheus::new("http://localhost:9090");
        assert_eq!(prom.base_url(), "http://localhost:9090");
    }

    #[test]
    fn test_mock_prometheus_add_metric() {
        let mut prom = MockPrometheus::new("http://localhost:9090");
        prom.add_metric("cpu_usage", 75.5);
        prom.add_metric("cpu_usage", 80.0);

        assert_eq!(prom.metrics.get("cpu_usage").unwrap().len(), 2);
    }

    #[test]
    fn test_mock_prometheus_query_response() {
        let prom = MockPrometheus::new("http://localhost:9090");
        let response = prom.query_response("up");

        assert_eq!(response["status"], "success");
        assert!(response["data"]["result"].is_array());
    }
}
