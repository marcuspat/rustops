//! Test data generation utilities.

use crate::telemetry::{LogEntry, Metric};
use crate::config::Config;
use rand::Rng;
use rand::seq::SliceRandom;
use std::collections::HashMap;

/// Generates random test data for tests.
#[derive(Debug, Clone)]
pub struct TestDataGenerator {
    rng: rand::rngs::StdRng,
}

impl Default for TestDataGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl TestDataGenerator {
    /// Creates a new generator with a random seed.
    pub fn new() -> Self {
        Self {
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }

    /// Creates a new generator with a fixed seed for reproducibility.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    /// Generates a random metric.
    pub fn metric(&mut self) -> Metric {
        Metric {
            name: self.random_metric_name(),
            value: self.rng.gen::<f64>() * 100.0,
            labels: self.random_labels(),
            timestamp: self.random_timestamp(),
        }
    }

    /// Generates a random log entry.
    pub fn log_entry(&mut self) -> LogEntry {
        LogEntry {
            level: self.random_log_level(),
            message: self.random_string(20..100),
            timestamp: self.random_timestamp_nanos(),
            labels: self.random_labels(),
        }
    }

    /// Generates a vector of random metrics.
    pub fn metrics(&mut self, count: usize) -> Vec<Metric> {
        (0..count).map(|_| self.metric()).collect()
    }

    /// Generates a vector of random log entries.
    pub fn log_entries(&mut self, count: usize) -> Vec<LogEntry> {
        (0..count).map(|_| self.log_entry()).collect()
    }

    /// Generates a default test config.
    pub fn config(&self) -> Config {
        Config::default()
    }

    // Private helper methods

    fn random_metric_name(&mut self) -> String {
        let names = [
            "cpu_usage_percent",
            "memory_usage_bytes",
            "disk_io_percent",
            "network_in_bytes",
            "network_out_bytes",
            "request_latency_ms",
            "error_rate",
            "throughput_rps",
        ];
        names.choose(&mut self.rng).unwrap().to_string()
    }

    fn random_log_level(&mut self) -> String {
        let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
        levels.choose(&mut self.rng).unwrap().to_string()
    }

    fn random_labels(&mut self) -> HashMap<String, String> {
        let mut labels = HashMap::new();
        labels.insert(
            "host".to_string(),
            format!("host-{}", self.rng.gen_range(0..100)),
        );
        labels.insert(
            "region".to_string(),
            format!("region-{}", self.rng.gen_range(0..5)),
        );
        if self.rng.gen_bool(0.5) {
            labels.insert(
                "service".to_string(),
                format!("service-{}", self.rng.gen_range(0..10)),
            );
        }
        labels
    }

    fn random_timestamp(&mut self) -> chrono::DateTime<chrono::Utc> {
        let now = chrono::Utc::now();
        let secs_ago = self.rng.gen_range(0..86400); // Within last 24 hours
        now - chrono::Duration::seconds(secs_ago)
    }

    fn random_timestamp_nanos(&mut self) -> chrono::DateTime<chrono::Utc> {
        let now = chrono::Utc::now();
        let nanos_ago = self.rng.gen_range(0..86400_000_000_000); // Within last 24 hours in nanos
        now - chrono::Duration::nanoseconds(nanos_ago)
    }

    fn random_string(&mut self, range: std::ops::Range<usize>) -> String {
        let len = self.rng.gen_range(range);
        (0..len)
            .map(|_| {
                let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ";
                chars[self.rng.gen_range(0..chars.len())] as char
            })
            .collect()
    }
}

/// Loads test fixtures from files.
pub struct FixtureLoader;

impl FixtureLoader {
    /// Loads a JSON fixture file.
    pub fn load_json<T, P>(path: P) -> std::io::Result<T>
    where
        T: serde::de::DeserializeOwned,
        P: AsRef<std::path::Path>,
    {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_deterministic_with_seed() {
        let mut gen1 = TestDataGenerator::with_seed(42);
        let mut gen2 = TestDataGenerator::with_seed(42);

        let metric1 = gen1.metric();
        let metric2 = gen2.metric();

        assert_eq!(metric1.name, metric2.name);
        assert_eq!(metric1.value, metric2.value);
        assert_eq!(metric1.timestamp, metric2.timestamp);
    }

    #[test]
    fn test_generator_different_metrics() {
        let mut gen = TestDataGenerator::new();
        let metrics = gen.metrics(100);

        assert_eq!(metrics.len(), 100);

        // At least some variety in metric names
        let unique_names: std::collections::HashSet<_> =
            metrics.iter().map(|m| &m.name).collect();
        assert!(unique_names.len() > 1);
    }

    #[test]
    fn test_generator_metric_range() {
        let mut gen = TestDataGenerator::new();
        let metrics = gen.metrics(1000);

        for metric in &metrics {
            assert!(metric.value >= 0.0);
            assert!(metric.value < 100.0);
        }
    }

    #[test]
    fn test_generator_log_levels() {
        let mut gen = TestDataGenerator::new();
        let logs = gen.log_entries(100);

        for log in &logs {
            assert!(matches!(
                log.level.as_str(),
                "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR"
            ));
        }
    }

    #[test]
    fn test_generator_timestamps_recent() {
        let mut gen = TestDataGenerator::new();
        let metric = gen.metric();

        let now = chrono::Utc::now().timestamp();
        let age_seconds = now - metric.timestamp;

        assert!(age_seconds >= 0);
        assert!(age_seconds <= 86400, "Timestamp should be within last 24 hours");
    }

    #[test]
    fn test_generator_labels_consistent() {
        let mut gen = TestDataGenerator::new();
        let metric = gen.metric();

        assert!(metric.labels.contains_key("host"));
        assert!(metric.labels.contains_key("region"));
    }
}
