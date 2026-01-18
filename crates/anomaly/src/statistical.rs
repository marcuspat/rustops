//! # Statistical anomaly detectors
//!
//! Fast, rule-based anomaly detection algorithms.

use crate::detector::{Anomaly, AnomalyDetector, AnomalyType, DetectionResult, Result};
use async_trait::async_trait;
use rustops_common::{Metric, MetricId};
use std::collections::HashMap;

/// Window size for statistical calculations
const DEFAULT_WINDOW_SIZE: usize = 100;

/// Z-score detector - detects spikes using standard deviation
///
/// Formula: z = (x - μ) / σ
/// Anomaly if |z| > threshold
#[derive(Clone)]
pub struct ZScoreDetector {
    /// Z-score threshold (typically 2-3)
    threshold: f64,
    /// Metric history for calculating mean/stddev
    history: HashMap<MetricId, MetricHistory>,
}

/// History of metric values for statistical calculations
#[derive(Clone, Default)]
struct MetricHistory {
    values: Vec<f64>,
    window_size: usize,
}

impl MetricHistory {
    fn new(window_size: usize) -> Self {
        Self {
            values: Vec::with_capacity(window_size),
            window_size,
        }
    }

    fn add(&mut self, value: f64) {
        self.values.push(value);
        if self.values.len() > self.window_size {
            self.values.remove(0);
        }
    }

    fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    fn stddev(&self) -> f64 {
        if self.values.len() < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let variance = self
            .values
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>()
            / (self.values.len() - 1) as f64;
        variance.sqrt()
    }

    fn z_score(&self, value: f64) -> f64 {
        let stddev = self.stddev();
        if stddev == 0.0 {
            return 0.0;
        }
        (value - self.mean()) / stddev
    }
}

impl ZScoreDetector {
    /// Create a new Z-score detector
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            history: HashMap::new(),
        }
    }

    /// Create with default threshold (3.0)
    pub fn default() -> Self {
        Self::new(3.0)
    }
}

#[async_trait]
impl AnomalyDetector for ZScoreDetector {
    async fn detect(&self, metrics: &[Metric]) -> Result<DetectionResult> {
        let start = std::time::Instant::now();
        let mut anomalies = Vec::new();

        // Note: In production, we'd use Arc<Mutex<HashMap>> for thread-safe history
        // For this implementation, we'll compute z-scores from the batch itself

        for metric in metrics {
            // Get or create history for this metric
            // In a real implementation, history would be shared across calls

            let values: Vec<f64> = metrics
                .iter()
                .filter(|m| m.name == metric.name)
                .map(|m| m.value)
                .collect();

            if values.len() < 10 {
                continue; // Not enough data
            }

            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance = values
                .iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>()
                / (values.len() - 1) as f64;
            let stddev = variance.sqrt();

            if stddev == 0.0 {
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

    /// Create with default multiplier (1.5)
    pub fn default() -> Self {
        Self::new(1.5)
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

        for (name, group) in metric_groups {
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
        Metric::gauge(
            name.to_string(),
            value,
            ServiceId::new(),
            HashMap::new(),
        )
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

    #[test]
    fn test_metric_history() {
        let mut history = MetricHistory::new(5);

        history.add(10.0);
        history.add(20.0);
        history.add(30.0);

        assert!((history.mean() - 20.0).abs() < 0.01);

        history.add(40.0);
        history.add(50.0);

        // Window is full, adding more should evict oldest
        history.add(60.0);

        assert_eq!(history.values.len(), 5);
        assert!(!history.values.contains(&10.0));
    }
}
