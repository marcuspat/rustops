//! # Statistical anomaly detectors
//!
//! Fast, rule-based anomaly detection algorithms.

use crate::detector::{Anomaly, AnomalyDetector, AnomalyType, DetectionResult, Result};
use async_trait::async_trait;
use rustops_common::Metric;
use std::collections::HashMap;

/// Z-score detector - detects spikes using standard deviation
///
/// Formula: z = (x - μ) / σ
/// Anomaly if |z| > threshold
#[derive(Clone)]
pub struct ZScoreDetector {
    /// Z-score threshold (typically 2-3)
    threshold: f64,
}

/// Minimum samples of a metric in a batch before z-scores are computed.
/// Below this, the leave-one-out baseline is too small to be meaningful.
const MIN_SAMPLES: usize = 8;

impl ZScoreDetector {
    /// Create a new Z-score detector
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }
}

impl Default for ZScoreDetector {
    /// Default threshold of 3.0.
    fn default() -> Self {
        Self::new(3.0)
    }
}

#[async_trait]
impl AnomalyDetector for ZScoreDetector {
    async fn detect(&self, metrics: &[Metric]) -> Result<DetectionResult> {
        let start = std::time::Instant::now();
        let mut anomalies = Vec::new();

        // Group values by metric name once (instead of re-filtering the
        // batch for every point).
        let mut by_name: HashMap<&str, Vec<f64>> = HashMap::new();
        for metric in metrics {
            by_name
                .entry(metric.name.as_str())
                .or_default()
                .push(metric.value);
        }

        for metric in metrics {
            let values = &by_name[metric.name.as_str()];
            if values.len() < MIN_SAMPLES {
                continue; // Not enough data for a meaningful baseline
            }

            // Leave-one-out baseline: exclude the candidate point from its
            // own mean/stddev, so a large outlier cannot mask itself by
            // inflating the baseline it is judged against.
            let n = (values.len() - 1) as f64;
            let sum: f64 = values.iter().sum();
            let mean = (sum - metric.value) / n;
            let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                - (metric.value - mean).powi(2);
            let variance = variance / (n - 1.0);
            let stddev = variance.sqrt();

            if stddev == 0.0 || !stddev.is_finite() {
                continue;
            }

            let z_score = (metric.value - mean) / stddev;

            if z_score.abs() > self.threshold {
                let anomaly_type = if z_score > 0.0 {
                    AnomalyType::Spike
                } else {
                    AnomalyType::Drop
                };

                let anomaly = Anomaly::new(
                    metric.id,
                    metric.service_id,
                    anomaly_type,
                    (z_score.abs() / self.threshold).min(1.0),
                    0.95,
                    format!(
                        "Z-score of {:.2} exceeds threshold {:.2}",
                        z_score, self.threshold
                    ),
                    metric.value,
                    mean,
                )
                .with_context("z_score", format!("{:.2}", z_score))
                .with_context("mean", format!("{:.2}", mean))
                .with_context("stddev", format!("{:.2}", stddev));

                anomalies.push(anomaly);
            }
        }

        Ok(DetectionResult {
            anomalies,
            processing_time_ms: start.elapsed().as_millis() as u64,
            metrics_analyzed: metrics.len(),
        })
    }

    fn name(&self) -> &str {
        "z_score"
    }

    fn expected_latency(&self) -> std::time::Duration {
        std::time::Duration::from_micros(100)
    }
}

/// IQR (Interquartile Range) detector - detects outliers
///
/// Anomaly if:
/// - value > Q3 + 1.5 * IQR
/// - value < Q1 - 1.5 * IQR
#[derive(Clone)]
pub struct IQRDetector {
    /// IQR multiplier (default 1.5)
    multiplier: f64,
}

impl IQRDetector {
    /// Create a new IQR detector
    pub fn new(multiplier: f64) -> Self {
        Self { multiplier }
    }

    /// Calculate quartiles from sorted values
    fn quartiles(values: &mut [f64]) -> (f64, f64, f64) {
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = values.len();
        let q1 = values[n / 4];
        let q2 = values[n / 2];
        let q3 = values[(3 * n) / 4];

        (q1, q2, q3)
    }
}

impl Default for IQRDetector {
    /// Default multiplier of 1.5.
    fn default() -> Self {
        Self::new(1.5)
    }
}

#[async_trait]
impl AnomalyDetector for IQRDetector {
    async fn detect(&self, metrics: &[Metric]) -> Result<DetectionResult> {
        let start = std::time::Instant::now();
        let mut anomalies = Vec::new();

        // Group metrics by name
        let mut metric_groups: HashMap<String, Vec<&Metric>> = HashMap::new();
        for metric in metrics {
            metric_groups
                .entry(metric.name.clone())
                .or_default()
                .push(metric);
        }

        for (_name, group) in metric_groups {
            if group.len() < 4 {
                continue; // Need at least 4 points for IQR
            }

            let mut values: Vec<f64> = group.iter().map(|m| m.value).collect();
            let (q1, _q2, q3) = Self::quartiles(&mut values);
            let iqr = q3 - q1;

            if iqr == 0.0 {
                continue;
            }

            let upper_bound = q3 + self.multiplier * iqr;
            let lower_bound = q1 - self.multiplier * iqr;

            for metric in group {
                if metric.value > upper_bound || metric.value < lower_bound {
                    let anomaly = Anomaly::new(
                        metric.id,
                        metric.service_id,
                        AnomalyType::Outlier,
                        if metric.value > upper_bound {
                            ((metric.value - upper_bound) / iqr).min(1.0)
                        } else {
                            ((lower_bound - metric.value) / iqr).min(1.0)
                        },
                        0.85,
                        format!(
                            "Value {:.2} outside IQR bounds [{:.2}, {:.2}]",
                            metric.value, lower_bound, upper_bound
                        ),
                        metric.value,
                        (q1 + q3) / 2.0,
                    )
                    .with_context("q1", format!("{:.2}", q1))
                    .with_context("q3", format!("{:.2}", q3))
                    .with_context("iqr", format!("{:.2}", iqr));

                    anomalies.push(anomaly);
                }
            }
        }

        Ok(DetectionResult {
            anomalies,
            processing_time_ms: start.elapsed().as_millis() as u64,
            metrics_analyzed: metrics.len(),
        })
    }

    fn name(&self) -> &str {
        "iqr"
    }

    fn expected_latency(&self) -> std::time::Duration {
        std::time::Duration::from_micros(200)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustops_common::ServiceId;
    use std::collections::HashMap;

    fn create_test_metric(name: &str, value: f64) -> Metric {
        Metric::gauge(name.to_string(), value, ServiceId::new(), HashMap::new())
    }

    #[test]
    fn test_z_score_detector() {
        let detector = ZScoreDetector::new(2.0);

        // Create metrics with a clear outlier
        let metrics = vec![
            create_test_metric("cpu", 50.0),
            create_test_metric("cpu", 51.0),
            create_test_metric("cpu", 49.0),
            create_test_metric("cpu", 50.0),
            create_test_metric("cpu", 52.0),
            create_test_metric("cpu", 48.0),
            create_test_metric("cpu", 50.0),
            create_test_metric("cpu", 100.0), // Outlier
        ];

        // Use blocking detection
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(detector.detect(&metrics)).unwrap();

        assert!(!result.anomalies.is_empty());
        assert_eq!(result.anomalies[0].anomaly_type, AnomalyType::Spike);
    }

    #[test]
    fn test_iqr_detector() {
        let detector = IQRDetector::new(1.5);

        // Create metrics with outliers
        let metrics = vec![
            create_test_metric("memory", 40.0),
            create_test_metric("memory", 42.0),
            create_test_metric("memory", 41.0),
            create_test_metric("memory", 43.0),
            create_test_metric("memory", 39.0),
            create_test_metric("memory", 100.0), // Outlier
        ];

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(detector.detect(&metrics)).unwrap();

        assert!(!result.anomalies.is_empty());
        assert_eq!(result.anomalies[0].anomaly_type, AnomalyType::Outlier);
    }
}
