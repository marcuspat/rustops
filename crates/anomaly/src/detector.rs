//! # Anomaly detector trait and types
//!
//! Defines the common interface for all anomaly detection algorithms.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rustops_common::{AnomalyId, Metric, MetricId, ServiceId};
use std::collections::HashMap;

/// An anomaly detected in telemetry data
#[derive(Clone, Debug, PartialEq)]
pub struct Anomaly {
    /// Unique anomaly ID
    pub id: AnomalyId,
    /// Metric that triggered the anomaly
    pub metric_id: MetricId,
    /// Service that produced the metric
    pub service_id: ServiceId,
    /// Type of anomaly
    pub anomaly_type: AnomalyType,
    /// Anomaly score (0-1, higher = more anomalous)
    pub score: f64,
    /// Confidence in detection (0-1)
    pub confidence: f64,
    /// Human-readable explanation
    pub explanation: String,
    /// When the anomaly was detected
    pub timestamp: DateTime<Utc>,
    /// Metric value that triggered anomaly
    pub metric_value: f64,
    /// Expected/baseline value
    pub expected_value: f64,
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Type of anomaly
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnomalyType {
    /// Spike - sudden increase
    Spike,
    /// Drop - sudden decrease
    Drop,
    /// Outlier - value outside normal range
    Outlier,
    /// Trend change - gradual shift in behavior
    TrendChange,
    /// Pattern violation - deviation from expected pattern
    PatternViolation,
    /// ML detected - identified by ML model
    MLDetected,
}

/// Result of anomaly detection on a batch of metrics
#[derive(Clone, Debug)]
pub struct DetectionResult {
    /// Anomalies found
    pub anomalies: Vec<Anomaly>,
    /// Processing time
    pub processing_time_ms: u64,
    /// Metrics analyzed
    pub metrics_analyzed: usize,
}

/// Anomaly detector - analyzes metrics for anomalies
#[async_trait]
pub trait AnomalyDetector: Send + Sync {
    /// Detect anomalies in a batch of metrics
    async fn detect(&self, metrics: &[Metric]) -> Result<DetectionResult>;

    /// Get the detector name
    fn name(&self) -> &str;

    /// Get expected latency for this detector
    fn expected_latency(&self) -> std::time::Duration {
        std::time::Duration::from_millis(10)
    }
}

/// Result type for anomaly detection
pub type Result<T> = std::result::Result<T, rustops_common::Error>;

impl Anomaly {
    /// Create a new anomaly
    pub fn new(
        metric_id: MetricId,
        service_id: ServiceId,
        anomaly_type: AnomalyType,
        score: f64,
        confidence: f64,
        explanation: impl Into<String>,
        metric_value: f64,
        expected_value: f64,
    ) -> Self {
        Self {
            id: AnomalyId::new(),
            metric_id,
            service_id,
            anomaly_type,
            score,
            confidence,
            explanation: explanation.into(),
            timestamp: Utc::now(),
            metric_value,
            expected_value,
            context: HashMap::new(),
        }
    }

    /// Add context to the anomaly
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Check if anomaly is high severity
    pub fn is_high_severity(&self) -> bool {
        self.score > 0.8 && self.confidence > 0.7
    }

    /// Get severity as a string
    pub fn severity(&self) -> &str {
        match self.score {
            s if s >= 0.9 => "critical",
            s if s >= 0.7 => "high",
            s if s >= 0.5 => "medium",
            _ => "low",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_anomaly_creation() {
        let metric_id = MetricId::new();
        let service_id = ServiceId::new();

        let anomaly = Anomaly::new(
            metric_id,
            service_id,
            AnomalyType::Spike,
            0.9,
            0.95,
            "CPU usage spiked",
            95.0,
            50.0,
        );

        assert_eq!(anomaly.anomaly_type, AnomalyType::Spike);
        assert_eq!(anomaly.score, 0.9);
        assert!(anomaly.is_high_severity());
        assert_eq!(anomaly.severity(), "critical");
    }

    #[test]
    fn test_anomaly_with_context() {
        let metric_id = MetricId::new();
        let service_id = ServiceId::new();

        let anomaly = Anomaly::new(
            metric_id,
            service_id,
            AnomalyType::Spike,
            0.9,
            0.95,
            "CPU usage spiked",
            95.0,
            50.0,
        )
        .with_context("host", "server1")
        .with_context("region", "us-east");

        assert_eq!(anomaly.context.len(), 2);
        assert_eq!(anomaly.context.get("host"), Some(&"server1".to_string()));
    }
}
