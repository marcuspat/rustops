//! Property-based tests for the core telemetry types, written against the
//! real `rustops_common` API (Metric carries id/type/service_id and a
//! `DateTime<Utc>` timestamp).

use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use proptest::prelude::*;
use rustops_common::{Metric, MetricType, ServiceId};

fn metric_type_strategy() -> impl Strategy<Value = MetricType> {
    prop_oneof![
        Just(MetricType::Gauge),
        Just(MetricType::Counter),
        Just(MetricType::Histogram),
        Just(MetricType::Summary),
    ]
}

proptest! {
    /// Constructing a metric preserves the finite value it was given.
    #[test]
    fn metric_preserves_finite_value(
        value in proptest::num::f64::NORMAL,
        metric_type in metric_type_strategy(),
    ) {
        let metric = Metric::new(
            "test_metric",
            metric_type,
            value,
            ServiceId::new(),
            HashMap::new(),
        );
        prop_assert_eq!(metric.value, value);
        prop_assert!(metric.value.is_finite());
    }

    /// JSON serialization round-trips every field.
    #[test]
    fn metric_serde_roundtrip(
        name in "[a-z_]{1,50}",
        value in -100_000.0f64..100_000.0,
        secs in 0i64..2_000_000_000,
        metric_type in metric_type_strategy(),
        label_val in "[a-zA-Z0-9_-]{0,32}",
    ) {
        let mut labels = HashMap::new();
        labels.insert("key".to_string(), label_val);

        let mut metric = Metric::new(
            name,
            metric_type,
            value,
            ServiceId::new(),
            labels,
        );
        metric.timestamp = Utc.timestamp_opt(secs, 0).unwrap();

        let json = serde_json::to_string(&metric).unwrap();
        let restored: Metric = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(restored.id, metric.id);
        prop_assert_eq!(restored.name, metric.name);
        prop_assert_eq!(restored.metric_type, metric.metric_type);
        prop_assert_eq!(restored.value, metric.value);
        prop_assert_eq!(restored.service_id, metric.service_id);
        prop_assert_eq!(restored.labels, metric.labels);
        prop_assert_eq!(restored.timestamp, metric.timestamp);
    }

    /// Metric names survive arbitrary label maps.
    #[test]
    fn metric_labels_roundtrip(
        entries in prop::collection::hash_map("[a-z]{1,16}", "[a-zA-Z0-9]{0,16}", 0..8),
    ) {
        let metric = Metric::new(
            "labeled_metric",
            MetricType::Gauge,
            1.0,
            ServiceId::new(),
            entries.clone(),
        );
        let json = serde_json::to_string(&metric).unwrap();
        let restored: Metric = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(restored.labels, entries);
    }
}
