//! Integration tests for common types and utilities.

use rustops_common::{Config, Metric, LogEntry};
use rustops_common::testing::{MetricBuilder, LogEntryBuilder, ConfigBuilder};
use std::collections::HashMap;

/// Test metric builder produces valid metrics.
#[tokio::test]
async fn test_metric_builder_integration() {
    let metric = MetricBuilder::new()
        .name("test_metric".to_string())
        .value(99.9)
        .label("host", "test-server")
        .label("region", "us-west-2")
        .build();

    assert_eq!(metric.name, "test_metric");
    assert_eq!(metric.value, 99.9);
    assert_eq!(metric.labels.len(), 2);
    assert_eq!(metric.labels.get("host"), Some(&"test-server".to_string()));
    assert_eq!(metric.labels.get("region"), Some(&"us-west-2".to_string()));
}

/// Test log entry builder produces valid entries.
#[tokio::test]
async fn test_log_entry_builder_integration() {
    let log = LogEntryBuilder::new()
        .level("ERROR".to_string())
        .message("Database connection failed".to_string())
        .label("service", "api")
        .label("instance", "api-1")
        .build();

    assert_eq!(log.level, "ERROR");
    assert_eq!(log.message, "Database connection failed");
    assert_eq!(log.labels.len(), 2);
}

/// Test config builder produces valid configurations.
#[tokio::test]
async fn test_config_builder_integration() {
    let config = ConfigBuilder::new()
        .agent_interval(60)
        .kafka_brokers(vec![
            "kafka1:9092".to_string(),
            "kafka2:9092".to_string(),
            "kafka3:9092".to_string(),
        ])
        .build();

    assert_eq!(config.agent.collection_interval_seconds, 60);
    assert_eq!(config.pipeline.kafka_brokers.len(), 3);
}

/// Test metric serialization and deserialization.
#[tokio::test]
async fn test_metric_serialization_integration() {
    let mut labels = HashMap::new();
    labels.insert("host".to_string(), "server1".to_string());
    labels.insert("region".to_string(), "us-east".to_string());

    let metric = Metric {
        name: "cpu_usage_percent".to_string(),
        value: 75.5,
        labels,
        timestamp: 1705536000,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&metric).expect("Failed to serialize");

    // Deserialize back
    let deserialized: Metric =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.name, metric.name);
    assert_eq!(deserialized.value, metric.value);
    assert_eq!(deserialized.labels, metric.labels);
    assert_eq!(deserialized.timestamp, metric.timestamp);
}

/// Test log entry serialization and deserialization.
#[tokio::test]
async fn test_log_entry_serialization_integration() {
    let mut labels = HashMap::new();
    labels.insert("service".to_string(), "api".to_string());

    let log = LogEntry {
        level: "ERROR".to_string(),
        message: "Connection timeout".to_string(),
        labels,
        timestamp: 1705536000_000_000_000,
    };

    let json = serde_json::to_string(&log).expect("Failed to serialize");
    let deserialized: LogEntry =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.level, log.level);
    assert_eq!(deserialized.message, log.message);
    assert_eq!(deserialized.labels, log.labels);
    assert_eq!(deserialized.timestamp, log.timestamp);
}

/// Test config serialization and deserialization.
#[tokio::test]
async fn test_config_serialization_integration() {
    let config = Config {
        agent: rustops_common::config::AgentConfig {
            collection_interval_seconds: 30,
        },
        pipeline: rustops_common::config::PipelineConfig {
            kafka_brokers: vec!["broker1:9092".to_string(), "broker2:9092".to_string()],
        },
    };

    let json = serde_json::to_string(&config).expect("Failed to serialize");
    let deserialized: Config =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(
        deserialized.agent.collection_interval_seconds,
        30
    );
    assert_eq!(deserialized.pipeline.kafka_brokers.len(), 2);
}

/// Test error handling with invalid inputs.
#[tokio::test]
async fn test_error_handling_integration() {
    use rustops_common::Error;

    // Test various error types
    let config_err = Error::Config("invalid value".to_string());
    assert!(config_err.to_string().contains("configuration error"));

    let network_err = Error::Network("connection refused".to_string());
    assert!(network_err.to_string().contains("network error"));

    let not_found_err = Error::NotFound("resource".to_string());
    assert!(not_found_err.to_string().contains("not found"));

    let invalid_input_err = Error::InvalidInput("bad parameter".to_string());
    assert!(invalid_input_err.to_string().contains("invalid input"));

    let auth_err = Error::Auth;
    assert!(auth_err.to_string().contains("authentication"));

    let internal_err = Error::Internal("something broke".to_string());
    assert!(internal_err.to_string().contains("internal error"));
}

/// Test metric aggregation scenarios.
#[tokio::test]
async fn test_metric_aggregation_integration() {
    // Create multiple metrics with same name
    let metrics: Vec<Metric> = (0..10)
        .map(|i| {
            let mut labels = HashMap::new();
            labels.insert("instance".to_string(), format!("host-{}", i));

            MetricBuilder::new()
                .name("cpu_usage".to_string())
                .value(50.0 + i as f64 * 5.0)
                .labels(labels)
                .build()
        })
        .collect();

    // Verify all metrics have the same name
    assert!(metrics.iter().all(|m| m.name == "cpu_usage"));

    // Calculate average
    let sum: f64 = metrics.iter().map(|m| m.value).sum();
    let avg = sum / metrics.len() as f64;

    // Average should be 72.5 (middle of 50.0 to 95.0)
    assert!((avg - 72.5).abs() < 0.01);
}

/// Test label manipulation operations.
#[tokio::test]
async fn test_label_manipulation_integration() {
    let mut labels = HashMap::new();
    labels.insert("env".to_string(), "production".to_string());
    labels.insert("team".to_string(), "platform".to_string());

    let metric = MetricBuilder::new()
        .name("request_count".to_string())
        .value(1000.0)
        .labels(labels.clone())
        .build();

    // Verify labels are correctly set
    assert_eq!(metric.labels.get("env"), Some(&"production".to_string()));
    assert_eq!(metric.labels.get("team"), Some(&"platform".to_string()));

    // Add additional label via builder
    let metric2 = MetricBuilder::new()
        .name("request_count".to_string())
        .value(1000.0)
        .labels(labels)
        .label("version", "v1.2.3".to_string())
        .build();

    assert_eq!(metric2.labels.get("version"), Some(&"v1.2.3".to_string()));
    assert_eq!(metric2.labels.len(), 3);
}
