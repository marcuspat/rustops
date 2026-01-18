//! Property-based tests for common types.

use proptest::prelude::*;
use rustops_common::{telemetry::Metric, config::Config};

/// Property: Metric value should always be finite when created.
proptest! {
    #[test]
    fn test_metric_value_always_finite(value in proptest::num::f64::ANY) {
        // Skip NaN and infinite values
        if value.is_finite() {
            let mut labels = std::collections::HashMap::new();
            labels.insert("test".to_string(), "value".to_string());

            let metric = Metric {
                name: "test_metric".to_string(),
                value,
                labels,
                timestamp: chrono::Utc::now().timestamp(),
            };

            prop_assert_eq!(metric.value, value);
        }
    }
}

/// Property: Metric serialization roundtrip preserves data.
proptest! {
    #[test]
    fn test_metric_serialization_roundtrip(
        name in "[a-z_]{1,50}",
        value in -100000.0..100000.0,
        timestamp in 0i64..2000000000i64
    ) {
        let mut labels = std::collections::HashMap::new();
        labels.insert("key".to_string(), "value".to_string());

        let metric = Metric {
            name: name.clone(),
            value,
            labels: labels.clone(),
            timestamp,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&metric).unwrap();

        // Deserialize back
        let restored: Metric = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(restored.name, name);
        prop_assert_eq!(restored.value, value);
        prop_assert_eq!(restored.timestamp, timestamp);
        prop_assert_eq!(restored.labels, labels);
    }
}

/// Property: Config default has reasonable values.
proptest! {
    #[test]
    fn test_config_default_values() {
        let config = Config::default();

        prop_assert!(config.agent.collection_interval_seconds > 0);
        prop_assert!(config.agent.collection_interval_seconds <= 3600); // Max 1 hour
        prop_assert!(!config.pipeline.kafka_brokers.is_empty());
    }
}

/// Property: Multiple metrics can have the same name but different values.
proptest! {
    #[test]
    fn test_metrics_with_same_name_different_values(
        values in prop::collection::vec(prop::num::f64::NORMAL, 1..100)
    ) {
        let metrics: Vec<_> = values
            .iter()
            .enumerate()
            .map(|(i, &value)| Metric {
                name: "cpu_usage".to_string(),
                value,
                labels: {
                    let mut labels = std::collections::HashMap::new();
                    labels.insert("instance".to_string(), format!("host-{}", i));
                    labels
                },
                timestamp: chrono::Utc::now().timestamp(),
            })
            .collect();

        prop_assert_eq!(metrics.len(), values.len());

        // All should have the same name
        prop_assert!(metrics.iter().all(|m| m.name == "cpu_usage"));

        // All should have unique instances
        let instances: Vec<_> = metrics
            .iter()
            .filter_map(|m| m.labels.get("instance"))
            .collect();
        prop_assert_eq!(instances.len(), metrics.len());
    }
}

/// Property: Labels can store arbitrary string key-value pairs.
proptest! {
    #[test]
    fn test_labels_arbitrary_strings(
        key in "[a-zA-Z0-9_]{1,30}",
        value in "\\PC{[^\\x00-\\x1F]}{0,100}" // Non-control characters
    ) {
        let mut labels = std::collections::HashMap::new();
        labels.insert(key.clone(), value.clone());

        let metric = Metric {
            name: "test".to_string(),
            value: 42.0,
            labels,
            timestamp: chrono::Utc::now().timestamp(),
        };

        prop_assert_eq!(metric.labels.get(&key), Some(&value));
    }
}

/// Property: Timestamps should be in reasonable ranges.
proptest! {
    #[test]
    fn test_timestamps_in_reasonable_range(
        offset in -86400i64..86400i64 // -1 day to +1 day
    ) {
        let now = chrono::Utc::now().timestamp();
        let timestamp = now + offset;

        let metric = Metric {
            name: "test".to_string(),
            value: 42.0,
            labels: std::collections::HashMap::new(),
            timestamp,
        };

        // Should be within reasonable range (year 2000-2030)
        prop_assert!(metric.timestamp > 946684800); // 2000-01-01
        prop_assert!(metric.timestamp < 1893456000); // 2030-01-01
    }
}

/// Property: Metric aggregation invariants.
proptest! {
    #[test]
    fn test_metric_aggregation_invariants(
        values in prop::collection::vec(prop::num::f64::NORMAL..0.0..100.0, 2..100)
    ) {
        prop_assert!(!values.is_empty());

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let sum: f64 = values.iter().sum();
        let avg = sum / values.len() as f64;

        // Min should be <= all values
        prop_assert!(values.iter().all(|&v| v >= min));

        // Max should be >= all values
        prop_assert!(values.iter().all(|&v| v <= max));

        // Average should be within bounds
        prop_assert!(avg >= min);
        prop_assert!(avg <= max);

        // Sum should equal len * avg (within floating point precision)
        prop_assert!((sum - avg * values.len() as f64).abs() < 1e-9);
    }
}

/// Property: Config serialization preserves structure.
proptest! {
    #[test]
    fn test_config_serialization_roundtrip(
        interval in 1u64..3600u64
    ) {
        let config = Config {
            agent: rustops_common::config::AgentConfig {
                collection_interval_seconds: interval,
            },
            pipeline: rustops_common::config::PipelineConfig {
                kafka_brokers: vec!["localhost:9092".to_string()],
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(restored.agent.collection_interval_seconds, interval);
        prop_assert_eq!(restored.pipeline.kafka_brokers.len(), 1);
    }
}
