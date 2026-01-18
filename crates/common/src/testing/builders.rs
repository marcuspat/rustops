//! Test builders for creating test data.

use crate::telemetry::{LogEntry, Metric};
use crate::config::{AgentConfig, Config, PipelineConfig};
use std::collections::HashMap;

/// Builder for creating [`Metric`] instances in tests.
#[derive(Debug, Clone)]
pub struct MetricBuilder {
    name: String,
    value: f64,
    labels: HashMap<String, String>,
    timestamp: Option<i64>,
}

impl Default for MetricBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricBuilder {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self {
            name: "test_metric".to_string(),
            value: 42.0,
            labels: HashMap::new(),
            timestamp: None,
        }
    }

    /// Sets the metric name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the metric value.
    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Adds a label to the metric.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Sets multiple labels at once.
    pub fn labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = labels;
        self
    }

    /// Sets the timestamp.
    pub fn timestamp(mut self, ts: i64) -> Self {
        self.timestamp = Some(ts);
        self
    }

    /// Builds the metric.
    pub fn build(self) -> Metric {
        Metric {
            name: self.name,
            value: self.value,
            labels: self.labels,
            timestamp: self.timestamp.unwrap_or_else(|| {
                chrono::Utc::now().timestamp()
            }),
        }
    }
}

/// Builder for creating [`LogEntry`] instances in tests.
#[derive(Debug, Clone)]
pub struct LogEntryBuilder {
    level: String,
    message: String,
    labels: HashMap<String, String>,
    timestamp: Option<i64>,
}

impl Default for LogEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LogEntryBuilder {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self {
            level: "INFO".to_string(),
            message: "test log message".to_string(),
            labels: HashMap::new(),
            timestamp: None,
        }
    }

    /// Sets the log level.
    pub fn level(mut self, level: impl Into<String>) -> Self {
        self.level = level.into();
        self
    }

    /// Sets the log message.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Adds a label to the log entry.
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Sets the timestamp.
    pub fn timestamp(mut self, ts: i64) -> Self {
        self.timestamp = Some(ts);
        self
    }

    /// Builds the log entry.
    pub fn build(self) -> LogEntry {
        LogEntry {
            level: self.level,
            message: self.message,
            labels: self.labels,
            timestamp: self.timestamp.unwrap_or_else(|| {
                chrono::Utc::now().timestamp_nanos_opt()
                    .unwrap_or_else(|| chrono::Utc::now().timestamp() * 1_000_000_000)
            }),
        }
    }
}

/// Builder for creating [`Config`] instances in tests.
#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    agent_interval: Option<u64>,
    kafka_brokers: Option<Vec<String>>,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigBuilder {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self {
            agent_interval: None,
            kafka_brokers: None,
        }
    }

    /// Sets the agent collection interval.
    pub fn agent_interval(mut self, seconds: u64) -> Self {
        self.agent_interval = Some(seconds);
        self
    }

    /// Sets the Kafka brokers.
    pub fn kafka_brokers(mut self, brokers: Vec<String>) -> Self {
        self.kafka_brokers = Some(brokers);
        self
    }

    /// Builds the config.
    pub fn build(self) -> Config {
        Config {
            agent: AgentConfig {
                collection_interval_seconds: self.agent_interval.unwrap_or(15),
            },
            pipeline: PipelineConfig {
                kafka_brokers: self.kafka_brokers
                    .unwrap_or_else(|| vec!["localhost:9092".to_string()]),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_builder_default() {
        let metric = MetricBuilder::new().build();

        assert_eq!(metric.name, "test_metric");
        assert_eq!(metric.value, 42.0);
        assert!(metric.labels.is_empty());
    }

    #[test]
    fn test_metric_builder_custom() {
        let mut labels = HashMap::new();
        labels.insert("host".to_string(), "server1".to_string());

        let metric = MetricBuilder::new()
            .name("cpu_usage".to_string())
            .value(75.5)
            .labels(labels.clone())
            .timestamp(1234567890)
            .build();

        assert_eq!(metric.name, "cpu_usage");
        assert_eq!(metric.value, 75.5);
        assert_eq!(metric.labels, labels);
        assert_eq!(metric.timestamp, 1234567890);
    }

    #[test]
    fn test_metric_builder_chain() {
        let metric = MetricBuilder::new()
            .name("memory_usage".to_string())
            .value(80.0)
            .label("host", "server2")
            .label("region", "us-west")
            .build();

        assert_eq!(metric.name, "memory_usage");
        assert_eq!(metric.value, 80.0);
        assert_eq!(metric.labels.len(), 2);
        assert_eq!(metric.labels.get("host"), Some(&"server2".to_string()));
        assert_eq!(metric.labels.get("region"), Some(&"us-west".to_string()));
    }

    #[test]
    fn test_log_entry_builder() {
        let log = LogEntryBuilder::new()
            .level("ERROR".to_string())
            .message("something went wrong".to_string())
            .label("service", "api")
            .build();

        assert_eq!(log.level, "ERROR");
        assert_eq!(log.message, "something went wrong");
        assert_eq!(log.labels.get("service"), Some(&"api".to_string()));
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .agent_interval(30)
            .kafka_brokers(vec!["kafka1:9092".to_string(), "kafka2:9092".to_string()])
            .build();

        assert_eq!(config.agent.collection_interval_seconds, 30);
        assert_eq!(config.pipeline.kafka_brokers.len(), 2);
        assert_eq!(config.pipeline.kafka_brokers[0], "kafka1:9092");
    }

    #[test]
    fn test_config_builder_default() {
        let config = ConfigBuilder::new().build();

        assert_eq!(config.agent.collection_interval_seconds, 15);
        assert_eq!(config.pipeline.kafka_brokers, vec!["localhost:9092"]);
    }
}
