//! # Detection router
//!
//! Routes telemetry to the optimal anomaly detector.

use crate::detector::{AnomalyDetector, DetectionResult, Result};
use crate::models::MLDetector;
use crate::statistical::{IQRDetector, ZScoreDetector};
use rustops_common::{Error, Metric};
use std::sync::Arc;
use tracing::debug;

/// Detection router - selects optimal detector based on metric characteristics
pub struct DetectionRouter {
    /// Fast statistical detector for simple metrics
    statistical: Arc<dyn AnomalyDetector>,
    /// IQR detector for outlier detection
    iqr: Arc<dyn AnomalyDetector>,
    /// Optional ML detector for complex patterns
    ml: Option<Arc<MLDetector>>,
}

impl DetectionRouter {
    /// Create a new detection router with default detectors
    pub fn new() -> Self {
        Self {
            statistical: Arc::new(ZScoreDetector::default()) as Arc<dyn AnomalyDetector>,
            iqr: Arc::new(IQRDetector::default()) as Arc<dyn AnomalyDetector>,
            ml: None,
        }
    }

    /// Create router with ML detector
    pub fn with_ml(mut self, ml_detector: Arc<MLDetector>) -> Self {
        self.ml = Some(ml_detector);
        self
    }

    /// Set the statistical detector
    pub fn with_statistical(mut self, detector: Arc<dyn AnomalyDetector>) -> Self {
        self.statistical = detector;
        self
    }

    /// Detect anomalies using the optimal detector for each metric
    pub async fn detect(&self, metrics: &[Metric]) -> Result<DetectionResult> {
        if metrics.is_empty() {
            return Ok(DetectionResult {
                anomalies: vec![],
                processing_time_ms: 0,
                metrics_analyzed: 0,
            });
        }

        // Route metrics to appropriate detector
        let routed = self.route_metrics(metrics);

        // Run detection in parallel
        let mut all_anomalies = Vec::new();
        let mut total_time = 0u64;

        if !routed.statistical.is_empty() {
            let result = self
                .statistical
                .detect(&routed.statistical)
                .await
                .map_err(|e| Error::AnomalyDetection {
                    message: format!("Statistical detection failed: {}", e),
                })?;
            all_anomalies.extend(result.anomalies);
            total_time = total_time.max(result.processing_time_ms);
        }

        if !routed.iqr.is_empty() {
            let result =
                self.iqr
                    .detect(&routed.iqr)
                    .await
                    .map_err(|e| Error::AnomalyDetection {
                        message: format!("IQR detection failed: {}", e),
                    })?;
            all_anomalies.extend(result.anomalies);
            total_time = total_time.max(result.processing_time_ms);
        }

        if let Some(ml) = &self.ml {
            if !routed.ml.is_empty() {
                let result = ml
                    .detect(&routed.ml)
                    .await
                    .map_err(|e| Error::AnomalyDetection {
                        message: format!("ML detection failed: {}", e),
                    })?;
                all_anomalies.extend(result.anomalies);
                total_time = total_time.max(result.processing_time_ms);
            }
        }

        Ok(DetectionResult {
            anomalies: all_anomalies,
            processing_time_ms: total_time,
            metrics_analyzed: metrics.len(),
        })
    }

    /// Route metrics to appropriate detectors
    fn route_metrics(&self, metrics: &[Metric]) -> RoutedMetrics {
        let mut routed = RoutedMetrics::default();

        for metric in metrics {
            // Simple routing rules:
            // 1. Use ML for latency/error metrics (complex patterns)
            // 2. Use IQR for metrics with high variance (outliers)
            // 3. Use Z-score for everything else (fast)

            if self.ml.is_some() {
                if self.should_use_ml(metric) {
                    routed.ml.push(metric.clone());
                    continue;
                }
            }

            if self.should_use_iqr(metric) {
                routed.iqr.push(metric.clone());
            } else {
                routed.statistical.push(metric.clone());
            }
        }

        debug!(
            "Routed {} metrics: {} statistical, {} IQR, {} ML",
            metrics.len(),
            routed.statistical.len(),
            routed.iqr.len(),
            routed.ml.len()
        );

        routed
    }

    /// Check if metric should use ML detector
    fn should_use_ml(&self, metric: &Metric) -> bool {
        // Use ML for complex patterns:
        // - Latency metrics
        // - Error rates
        // - Request rates
        // - Anything with time-series patterns

        let name_lower = metric.name.to_lowercase();

        name_lower.contains("latency")
            || name_lower.contains("duration")
            || name_lower.contains("error")
            || name_lower.contains("rate")
            || name_lower.contains("request")
    }

    /// Check if metric should use IQR detector
    fn should_use_iqr(&self, metric: &Metric) -> bool {
        // Use IQR for metrics prone to outliers:
        // - Memory usage (can have spikes)
        // - Disk usage (can have spikes)
        // - Network traffic (bursty)
        // - Queue sizes

        let name_lower = metric.name.to_lowercase();

        name_lower.contains("memory")
            || name_lower.contains("disk")
            || name_lower.contains("network")
            || name_lower.contains("queue")
            || name_lower.contains("buffer")
    }
}

impl Default for DetectionRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Routed metrics split by detector
#[derive(Default)]
struct RoutedMetrics {
    statistical: Vec<Metric>,
    iqr: Vec<Metric>,
    ml: Vec<Metric>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustops_common::ServiceId;
    use std::collections::HashMap;

    fn create_test_metric(name: &str, value: f64) -> Metric {
        Metric::gauge(name.to_string(), value, ServiceId::new(), HashMap::new())
    }

    #[tokio::test]
    async fn test_router_basic() {
        let router = DetectionRouter::new();

        let metrics = vec![
            create_test_metric("cpu_usage", 95.0),         // Statistical
            create_test_metric("memory_usage", 100.0),     // IQR
            create_test_metric("request_latency", 5000.0), // Would be ML if configured
        ];

        let result = router.detect(&metrics).await.unwrap();

        assert_eq!(result.metrics_analyzed, 3);
    }

    #[test]
    fn test_routing_rules() {
        let router = DetectionRouter::new();

        // ML-eligible metrics
        assert!(router.should_use_ml(&create_test_metric("request_latency", 100.0)));
        assert!(router.should_use_ml(&create_test_metric("error_rate", 0.5)));

        // IQR-eligible metrics
        assert!(router.should_use_iqr(&create_test_metric("memory_usage", 80.0)));
        assert!(router.should_use_iqr(&create_test_metric("network_in", 1000.0)));

        // Default to statistical
        assert!(!router.should_use_ml(&create_test_metric("cpu_usage", 50.0)));
        assert!(!router.should_use_iqr(&create_test_metric("cpu_usage", 50.0)));
    }
}
