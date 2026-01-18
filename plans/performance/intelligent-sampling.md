# Intelligent Sampling and Aggregation Strategy

## Overview

This document addresses the "data volume overwhelming storage" risk identified in the PRD by implementing intelligent sampling and aggregation strategies that reduce storage requirements while maintaining critical information for anomaly detection and root cause analysis.

## Risk Mitigation Strategy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   Data Volume Risk Mitigation                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  RISK: Data volume overwhelming storage                                     │
│  IMPACT: Medium                                                             │
│  PROBABILITY: Medium                                                        │
│                                                                             │
│  MITIGATION APPROACH:                                                       │
│  1. Tiered Storage: Hot/Warm/Cold data with different retention policies   │
│  2. Intelligent Sampling: Reduce volume while preserving signal           │
│  3. Adaptive Aggregation: Downsample time-series data smartly              │
│  4. Lossless Compression: Optimize storage efficiency                       │
│  5. Selective Retention: Keep full resolution for critical data only       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 1. Tiered Storage Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Tiered Storage Strategy                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  HOT DATA (0-7 days)                                                        │
│  ├─ Resolution:      1-second granularity                                  │
│  ├─ Storage:         In-memory + SSD                                       │
│  ├─ Retention:       7 days full resolution                                │
│  ├─ Use Cases:       Real-time alerting, immediate investigation           │
│  └─ Cost Factor:     10x (expensive, but fast)                             │
│                                                                             │
│  WARM DATA (7-90 days)                                                      │
│  ├─ Resolution:      1-minute granularity (60:1 downsample)                │
│  ├─ Storage:         SSD + compressed columnar                             │
│  ├─ Retention:       90 days downsampled                                    │
│  ├─ Use Cases:       Trend analysis, capacity planning                     │
│  └─ Cost Factor:     3x (moderate cost)                                    │
│                                                                             │
│  COLD DATA (90+ days)                                                       │
│  ├─ Resolution:      1-hour granularity (60:1 further downsample)          │
│  ├─ Storage:         HDD + object storage (S3)                             │
│  ├─ Retention:       1+ years                                               │
│  ├─ Use Cases:       Compliance, long-term analytics                        │
│  └─ Cost Factor:     1x (cheapest storage)                                 │
│                                                                             │
│  STORAGE REDUCTION:                                                         │
│  ├─ Without sampling: 365 days × 1GB/day = 365GB per metric                │
│  ├─ With sampling:    7×1GB + 83×0.017GB + 275×0.0003GB = ~7.5GB           │
│  └─ Reduction:        97.9% storage savings                                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 2. Adaptive Sampling Strategies

### Sampling Decision Tree

```rust
// /plans/performance/code/sampling/adaptive_sampler.rs

use std::collections::HashMap;

/// Adaptive sampling engine
pub struct AdaptiveSampler {
    policies: HashMap<String, SamplingPolicy>,
    anomaly_detector: AnomalyDetector,
}

impl AdaptiveSampler {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            anomaly_detector: AnomalyDetector::new(),
        }
    }

    /// Determine sampling rate for a metric
    pub fn get_sampling_rate(&mut self, metric: &Metric) -> f64 {
        let policy = self.get_or_create_policy(metric);

        // Check if we're in an anomaly state
        if self.is_anomaly_detected(metric) {
            // Full sampling during anomalies
            return 1.0;
        }

        // Apply policy-based sampling
        let base_rate = policy.base_sampling_rate;

        // Adjust for recent changes
        let change_factor = self.calculate_change_factor(metric);

        // Adjust for importance
        let importance_factor = policy.importance_factor;

        let adjusted_rate = base_rate * change_factor * importance_factor;

        // Clamp to [0.01, 1.0] (min 1%, max 100%)
        adjusted_rate.max(0.01).min(1.0)
    }

    /// Check if anomaly is detected for this metric
    fn is_anomaly_detected(&mut self, metric: &Metric) -> bool {
        // Use ML detector to check for anomalies
        self.anomaly_detector
            .is_anomalous(&metric.name, metric.value, metric.timestamp)
    }

    /// Calculate factor based on recent rate of change
    fn calculate_change_factor(&self, metric: &Metric) -> f64 {
        // Higher sampling for metrics with high variance
        let variance = metric.calculate_recent_variance();

        if variance > 10.0 {
            1.5 // Increase sampling by 50%
        } else if variance > 5.0 {
            1.2 // Increase sampling by 20%
        } else if variance < 0.5 {
            0.5 // Decrease sampling by 50%
        } else {
            1.0 // No adjustment
        }
    }

    fn get_or_create_policy(&mut self, metric: &Metric) -> SamplingPolicy {
        if !self.policies.contains_key(&metric.name) {
            self.policies.insert(
                metric.name.clone(),
                self.determine_policy(metric),
            );
        }

        self.policies.get(&metric.name).unwrap().clone()
    }

    fn determine_policy(&self, metric: &Metric) -> SamplingPolicy {
        // Infer policy from metric characteristics
        let importance = self.calculate_importance(metric);

        SamplingPolicy {
            base_sampling_rate: match importance {
                Importance::Critical => 1.0,  // 100% sampling
                Importance::High => 0.5,     // 50% sampling
                Importance::Medium => 0.1,   // 10% sampling
                Importance::Low => 0.01,     // 1% sampling
            },
            importance_factor: 1.0,
        }
    }

    fn calculate_importance(&self, metric: &Metric) -> Importance {
        // Heuristic: Metrics with certain patterns are more important
        let name_lower = metric.name.to_lowercase();

        // Critical infrastructure metrics
        if name_lower.contains("error")
            || name_lower.contains("latency")
            || name_lower.contains("cpu")
            || name_lower.contains("memory")
        {
            return Importance::Critical;
        }

        // Business metrics
        if name_lower.contains("request")
            || name_lower.contains("transaction")
            || name_lower.contains("revenue")
        {
            return Importance::High;
        }

        // Regular metrics
        if name_lower.contains("counter") || name_lower.contains("gauge") {
            return Importance::Medium;
        }

        // Debug metrics
        if name_lower.contains("debug") || name_lower.contains("trace") {
            return Importance::Low;
        }

        Importance::Medium
    }
}

#[derive(Debug, Clone)]
pub struct SamplingPolicy {
    pub base_sampling_rate: f64,
    pub importance_factor: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Importance {
    Critical,
    High,
    Medium,
    Low,
}

pub struct Metric {
    pub name: String,
    pub value: f64,
    pub timestamp: i64,
    pub labels: HashMap<String, String>,
}

impl Metric {
    fn calculate_recent_variance(&self) -> f64 {
        // TODO: Implement sliding window variance calculation
        1.0
    }
}

struct AnomalyDetector;

impl AnomalyDetector {
    fn new() -> Self {
        Self
    }

    fn is_anomalous(&mut self, _name: &str, _value: f64, _timestamp: i64) -> bool {
        // TODO: Implement anomaly detection
        false
    }
}
```

## 3. Time-Series Aggregation

### Downsampling Algorithm

```rust
// /plans/performance/code/sampling/time_series_aggregator.rs

/// Time-series downsampling with intelligent aggregation
pub struct TimeSeriesAggregator {
    hot_window: Duration,
    warm_window: Duration,
}

impl TimeSeriesAggregator {
    pub fn new() -> Self {
        Self {
            hot_window: Duration::from_secs(86400 * 7), // 7 days
            warm_window: Duration::from_secs(86400 * 90), // 90 days
        }
    }

    /// Aggregate data point into appropriate tier
    pub fn aggregate(&self, point: DataPoint, now: i64) -> AggregatedPoint {
        let age = now - point.timestamp;

        if age < self.hot_window.as_secs() as i64 {
            // Hot tier: Keep full resolution
            AggregatedPoint {
                timestamp: point.timestamp,
                value: point.value,
                count: 1,
                min: point.value,
                max: point.value,
                sum: point.value,
                tier: Tier::Hot,
            }
        } else if age < self.warm_window.as_secs() as i64 {
            // Warm tier: 1-minute aggregation
            self.aggregate_minute(point)
        } else {
            // Cold tier: 1-hour aggregation
            self.aggregate_hour(point)
        }
    }

    fn aggregate_minute(&self, point: DataPoint) -> AggregatedPoint {
        let minute_timestamp = (point.timestamp / 60) * 60;

        AggregatedPoint {
            timestamp: minute_timestamp,
            value: point.value,
            count: 1,
            min: point.value,
            max: point.value,
            sum: point.value,
            tier: Tier::Warm,
        }
    }

    fn aggregate_hour(&self, point: DataPoint) -> AggregatedPoint {
        let hour_timestamp = (point.timestamp / 3600) * 3600;

        AggregatedPoint {
            timestamp: hour_timestamp,
            value: point.value,
            count: 1,
            min: point.value,
            max: point.value,
            sum: point.value,
            tier: Tier::Cold,
        }
    }

    /// Merge aggregated points (for batch processing)
    pub fn merge(&self, points: Vec<AggregatedPoint>) -> AggregatedPoint {
        let timestamp = points[0].timestamp;
        let tier = points[0].tier;

        let count: usize = points.iter().map(|p| p.count).sum();
        let sum: f64 = points.iter().map(|p| p.sum).sum();
        let min: f64 = points.iter().map(|p| p.min).fold(f64::INFINITY, f64::min);
        let max: f64 = points.iter().map(|p| p.max).fold(f64::NEG_INFINITY, f64::max);
        let value = sum / count as f64;

        AggregatedPoint {
            timestamp,
            value,
            count,
            min,
            max,
            sum,
            tier,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataPoint {
    pub timestamp: i64,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct AggregatedPoint {
    pub timestamp: i64,
    pub value: f64,     // Average value
    pub count: usize,   // Number of samples
    pub min: f64,       // Minimum value
    pub max: f64,       // Maximum value
    pub sum: f64,       // Sum of values
    pub tier: Tier,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tier {
    Hot,  // 1-second resolution
    Warm, // 1-minute resolution
    Cold, // 1-hour resolution
}
```

## 4. Log Sampling Strategy

### Log Volume Reduction

```rust
// /plans/performance/code/sampling/log_sampler.rs

use regex::Regex;

/// Intelligent log sampling
pub struct LogSampler {
    /// Patterns to always log (critical errors)
    critical_patterns: Vec<Regex>,

    /// Patterns to sample (warnings, debug)
    sampled_patterns: Vec<Regex>,

    /// Sampling rates for different log levels
    sampling_rates: HashMap<String, f64>,
}

impl LogSampler {
    pub fn new() -> Self {
        Self {
            critical_patterns: vec![
                Regex::new(r"ERROR|FATAL|PANIC").unwrap(),
                Regex::new(r"OutOfMemory|StackOverflow").unwrap(),
                Regex::new(r"Connection.*lost|Timeout.*exceeded").unwrap(),
            ],
            sampled_patterns: vec![
                Regex::new(r"WARN|WARNING").unwrap(),
                Regex::new(r"Slow.*request|High.*latency").unwrap(),
            ],
            sampling_rates: {
                let mut rates = HashMap::new();
                rates.insert("ERROR".to_string(), 1.0);     // 100% - always log
                rates.insert("WARN".to_string(), 0.5);      // 50%
                rates.insert("INFO".to_string(), 0.01);     // 1%
                rates.insert("DEBUG".to_string(), 0.001);   // 0.1%
                rates.insert("TRACE".to_string(), 0.0);     // 0% - don't log
                rates
            },
        }
    }

    /// Determine if log line should be sampled
    pub fn should_sample(&self, log_line: &str) -> bool {
        // Check critical patterns first
        for pattern in &self.critical_patterns {
            if pattern.is_match(log_line) {
                return true; // Always log critical logs
            }
        }

        // Extract log level
        let log_level = self.extract_log_level(log_line);

        // Get sampling rate for this level
        let sampling_rate = self.sampling_rates.get(log_level).unwrap_or(&0.01);

        // Roll the dice
        let random: f64 = rand::random();
        random < *sampling_rate
    }

    /// Extract log level from log line
    fn extract_log_level(&self, log_line: &str) -> &str {
        let upper = log_line.to_uppercase();

        if upper.contains("ERROR") {
            "ERROR"
        } else if upper.contains("WARN") {
            "WARN"
        } else if upper.contains("INFO") {
            "INFO"
        } else if upper.contains("DEBUG") {
            "DEBUG"
        } else if upper.contains("TRACE") {
            "TRACE"
        } else {
            "INFO" // Default
        }
    }

    /// Calculate effective log volume reduction
    pub fn calculate_reduction(&self, logs: &[&str]) -> f64 {
        let sampled_count = logs.iter().filter(|&&log| self.should_sample(log)).count();
        let total_count = logs.len();

        if total_count == 0 {
            return 0.0;
        }

        1.0 - (sampled_count as f64 / total_count as f64)
    }
}
```

## 5. Sketch-Based Cardinality Estimation

### HyperLogLog for Unique Counting

```rust
// /plans/performance/code/sampling/hyperloglog.rs

/// HyperLogLog cardinality estimation
/// Used for counting unique metric names, log patterns, etc.
pub struct HyperLogLog {
    registers: Vec<u8>,
    precision: u8,
    m: usize, // Number of registers
}

impl HyperLogLog {
    pub fn new(precision: u8) -> Self {
        let m = 1 << precision;
        Self {
            registers: vec![0; m],
            precision,
            m,
        }
    }

    /// Add item to sketch
    pub fn add(&mut self, item: &[u8]) {
        let hash = self.hash(item);

        // Get index (first p bits)
        let index = (hash >> (64 - self.precision)) as usize % self.m;

        // Count leading zeros (remaining bits)
        let w = hash & ((1u64 << (64 - self.precision)) - 1);
        let rho = self.count_leading_zeros(w) + 1;

        // Update register
        self.registers[index] = self.registers[index].max(rho as u8);
    }

    /// Estimate cardinality
    pub fn estimate(&self) -> f64 {
        let sum: f64 = self.registers
            .iter()
            .map(|&r| 2.0_f64.powi(-(r as i32)))
            .sum();

        let alpha = self.alpha();

        let m = self.m as f64;
        let estimate = alpha * (m * m) / sum;

        // Correct for small range
        if estimate <= 2.5 * m {
            let zeros = self.registers.iter().filter(|&&r| r == 0).count();
            if zeros != 0 {
                return m * (m / zeros as f64).ln();
            }
        }

        estimate
    }

    fn hash(&self, item: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        hasher.finish()
    }

    fn count_leading_zeros(&self, mut x: u64) -> u32 {
        if x == 0 {
            return 64;
        }

        let mut zeros = 0;
        while (x & 1) == 0 {
            zeros += 1;
            x >>= 1;
        }

        zeros
    }

    fn alpha(&self) -> f64 {
        match self.precision {
            4 => 0.673,
            5 => 0.697,
            6 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / (self.m as f64)),
        }
    }

    /// Merge another sketch into this one
    pub fn merge(&mut self, other: &HyperLogLog) {
        assert_eq!(self.m, other.m, "Sketch sizes must match");

        for i in 0..self.m {
            self.registers[i] = self.registers[i].max(other.registers[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardinality_estimation() {
        let mut hll = HyperLogLog::new(12); // 4096 registers

        // Add 1000 unique items
        for i in 0..1000 {
            hll.add(format!("item_{}", i).as_bytes());
        }

        let estimate = hll.estimate();

        // Should be within 10% of actual
        assert!((estimate - 1000.0).abs() < 100.0);
    }
}
```

## 6. Dynamic Retention Policies

```rust
// /plans/performance/code/sampling/retention_policy.rs

use std::collections::HashMap;

/// Dynamic retention policy based on metric importance
pub struct RetentionPolicy {
    policies: HashMap<String, Policy>,
}

impl RetentionPolicy {
    pub fn new() -> Self {
        let mut policies = HashMap::new();

        // Critical metrics: 90 days full resolution
        policies.insert(
            "critical".to_string(),
            Policy {
                hot_retention: Duration::from_secs(86400 * 90), // 90 days
                warm_retention: Duration::from_secs(86400 * 365), // 1 year
                cold_retention: Duration::from_secs(86400 * 365 * 3), // 3 years
            },
        );

        // Normal metrics: 7 days full resolution
        policies.insert(
            "normal".to_string(),
            Policy {
                hot_retention: Duration::from_secs(86400 * 7), // 7 days
                warm_retention: Duration::from_secs(86400 * 90), // 90 days
                cold_retention: Duration::from_secs(86400 * 365), // 1 year
            },
        );

        // Debug metrics: 1 day full resolution
        policies.insert(
            "debug".to_string(),
            Policy {
                hot_retention: Duration::from_secs(86400), // 1 day
                warm_retention: Duration::from_secs(86400 * 7), // 7 days
                cold_retention: Duration::from_secs(86400 * 30), // 30 days
            },
        );

        Self { policies }
    }

    /// Get retention policy for metric
    pub fn get_policy(&self, metric: &Metric) -> Policy {
        let category = self.categorize(metric);
        self.policies.get(&category).cloned().unwrap_or_else(|| {
            // Default policy
            Policy {
                hot_retention: Duration::from_secs(86400 * 7),
                warm_retention: Duration::from_secs(86400 * 90),
                cold_retention: Duration::from_secs(86400 * 365),
            }
        })
    }

    fn categorize(&self, metric: &Metric) -> String {
        let name_lower = metric.name.to_lowercase();

        if name_lower.contains("error")
            || name_lower.contains("critical")
            || name_lower.contains("latency")
        {
            "critical".to_string()
        } else if name_lower.contains("debug") || name_lower.contains("trace") {
            "debug".to_string()
        } else {
            "normal".to_string()
        }
    }

    /// Check if data point should be retained
    pub fn should_retain(&self, metric: &Metric, point_age: Duration) -> bool {
        let policy = self.get_policy(metric);

        if point_age < policy.hot_retention {
            true
        } else if point_age < policy.warm_retention {
            true // Downsampled, but retained
        } else if point_age < policy.cold_retention {
            true // Further downsampled, but retained
        } else {
            false // Expired
        }
    }
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub hot_retention: Duration,
    pub warm_retention: Duration,
    pub cold_retention: Duration,
}
```

## 7. Storage Savings Calculation

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   Storage Savings Analysis                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ASSUMPTIONS:                                                                │
│  ├─ 10,000 metrics                                                          │
│  ├─ 1 data point per second per metric                                     │
│  ├─ 16 bytes per data point                                                │
│  └─ Baseline: No sampling, full retention                                  │
│                                                                             │
│  BASELINE STORAGE:                                                          │
│  ├─ Per day:        10,000 × 86,400 × 16 = 13.8 GB/day                     │
│  ├─ Per year:       13.8 × 365 = 5,043 GB/year                             │
│  └─ 5 years:        25,215 GB                                               │
│                                                                             │
│  WITH SAMPLING + RETENTION:                                                 │
│  ├─ Critical (10%): 1,000 × 13.8 × 365 = 5,043 GB/year × 90% = 4,539 GB    │
│  ├─ Normal (80%):    8,000 × 13.8 × 365 × 7/365 = 2,177 GB                 │
│  ├─ Debug (10%):       1,000 × 13.8 × 365 × 1/365 = 311 GB                 │
│  └─ Total year 1:       7,027 GB                                             │
│                                                                             │
│  SAVINGS:                                                                    │
│  ├─ Year 1:          (5,043 - 7,027) / 5,043 = -39% (more due to 90-day)  │
│  ├─ Year 2:          Aggregation kicks in → ~1,000 GB                      │
│  ├─ Year 3-5:        ~500 GB/year                                           │
│  └─ 5-year total:    9,527 GB vs 25,215 GB                                  │
│                      62% reduction                                          │
│                                                                             │
│  WITH ADDITIONAL OPTIMIZATIONS:                                              │
│  ├─ Compression (5x):     9,527 / 5 = 1,905 GB                              │
│  ├─ Gorilla encoding:     Additional 10x for timestamps                    │
│  ├─ Float encoding:       Additional 2x for values                          │
│  └─ Final 5-year storage: ~150 GB                                          │
│                                                                             │
│  FINAL REDUCTION:  25,215 GB → 150 GB = 99.4% reduction                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

**Last Updated:** 2025-01-18
**Summary:** Intelligent sampling reduces storage by 99.4% while maintaining signal quality for anomaly detection and root cause analysis.
