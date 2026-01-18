# Performance Regression Prevention Strategy

## Overview

This document defines the strategy for preventing performance regressions in RustOps through automated testing, continuous monitoring, and policy enforcement.

## Regression Prevention Layers

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Regression Prevention Layers                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  LAYER 1: PRE-COMMIT (Developer Machine)                                    │
│  ├─ Cargo clippy:    Lint checks for performance anti-patterns             │
│  ├─ Cargo fmt:       Enforced code formatting                               │
│  ├─ Quick smoke test: 30-second sanity check                                │
│  └─ Git pre-commit:  Automated block on failure                            │
│                                                                             │
│  LAYER 2: PRE-MERGE (CI/CD Pipeline)                                        │
│  ├─ Full benchmarks:  All performance tests                                │
│  ├─ Regression check:  Compare to baseline                                 │
│  ├─ Flamegraph diff:   Visualize changes                                   │
│  └─ PR blocking:      Require approval for regressions                     │
│                                                                             │
│  LAYER 3: POST-MERGE (Main Branch)                                         │
│  ├─ Nightly benchmarks: Track performance over time                        │
│  ├─ Trend analysis:   Detect gradual degradation                           │
│  ├─ Alert on drift:    Notify team of significant changes                 │
│  └─ Auto-revert:       Automatic rollback on severe regression             │
│                                                                             │
│  LAYER 4: PRODUCTION (Deployed Systems)                                    │
│  ├─ Real-time SLO monitoring: Check SLI compliance                         │
│  ├─ Canary deployments: Gradual rollout with monitoring                    │
│  ├─ A/B testing:       Compare old vs new versions                         │
│  └─ Instant rollback:  Automated rollback on SLA violation                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Pre-Commit Hooks

### Installation

```bash
# Install pre-commit framework
cargo install pre-commit

# Setup hooks in repository
pre-commit install
```

### Configuration

```yaml
# .pre-commit-config.yaml

repos:
  - repo: local
    hooks:
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt -- --check
        language: system
        files: \.rs$
        pass_filenames: false

      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy --all-targets --all-features -- -D warnings
        language: system
        files: \.rs$
        pass_filenames: false

      - id: cargo-quick-bench
        name: quick performance smoke test
        entry: bash -c 'cargo test --release --bench quick_smoke --test-threads=1'
        language: system
        files: \.rs$
        pass_filenames: false

      - id: check-allocations
        name: check for excessive allocations
        entry: bash -c 'cargo clippy --all-targets --warn "unused_allocations"'
        language: system
        files: \.rs$
        pass_filenames: false
```

## CI/CD Integration

### Performance Gate Configuration

```yaml
# .github/workflows/performance-gate.yml

name: Performance Gate

on:
  pull_request:
    branches: [main]
    paths:
      - 'src/**'
      - 'Cargo.toml'
      - 'Cargo.lock'

jobs:
  performance-check:
    name: Performance Regression Check
    runs-on: [self-hosted, performance]
    timeout-minutes: 60

    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: release
          toolchain: stable
          override: true

      - name: Cache Baseline
        uses: actions/cache@v3
        with:
          path: benchmark/baseline
          key: benchmark-baseline-${{ github.base_ref }}

      - name: Download Baseline
        run: |
          if [ -f "benchmark/baseline/main.json" ]; then
            echo "Baseline found"
          else
            echo "Baseline not found, creating from main branch"
            git fetch origin main
            git checkout origin/main
            cargo bench --bench metric_ingestion -- --save-baseline main
            cargo bench --bench flash_attention -- --save-baseline main
            cargo bench --bench hnsw_search -- --save-baseline main
            cargo bench --bench alert_correlation -- --save-baseline main
            mkdir -p benchmark/baseline
            cp criterion/*.json benchmark/baseline/
            git checkout ${{ github.head_ref }}
          fi

      - name: Run Benchmarks
        run: |
          cargo bench --bench metric_ingestion -- --save-baseline pr
          cargo bench --bench flash_attention -- --save-baseline pr
          cargo bench --bench hnsw_search -- --save-baseline pr
          cargo bench --bench alert_correlation -- --save-baseline pr

      - name: Compare to Baseline
        run: |
          cargo run --bin compare-benchmarks \
            --baseline benchmark/baseline/main.json \
            --current criterion/metric_ingestion/pr/ \
            --threshold 5.0

      - name: Generate Report
        run: |
          cargo run --bin benchmark-report \
            --output benchmark-report.md \
            --format markdown

      - name: Comment PR
        uses: actions/github-script@v6
        with:
          github-token: ${{ secrets.GITHUB_TOKEN }}
          script: |
            const fs = require('fs');
            const report = fs.readFileSync('benchmark-report.md', 'utf8');

            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: report
            });

      - name: Block on Regression
        if: contains(steps.compare.outputs.result, 'REGRESSION')
        run: exit 1
```

### Benchmark Comparison Tool

```rust
// /plans/performance/code/tools/compare-benchmarks.rs

use std::collections::HashMap;
use serde_json::Value;

/// Compare benchmark results to baseline
pub struct BenchmarkComparator {
    baseline: HashMap<String, BenchmarkMetrics>,
    threshold: f64, // Percentage change considered regression
}

impl BenchmarkComparator {
    pub fn new(threshold: f64) -> Self {
        Self {
            baseline: HashMap::new(),
            threshold,
        }
    }

    /// Load baseline from JSON file
    pub fn load_baseline(&mut self, path: &str) -> Result<(), ComparatorError> {
        let data = std::fs::read_to_string(path)?;
        let json: Value = serde_json::from_str(&data)?;

        // Parse Criterion JSON format
        if let Some(benchmarks) = json.as_array() {
            for benchmark in benchmarks {
                let name = benchmark["name"].as_str().unwrap();
                let metrics = self.parse_metrics(&benchmark["value"]);

                self.baseline.insert(name.to_string(), metrics);
            }
        }

        Ok(())
    }

    /// Compare current results to baseline
    pub fn compare(&self, current: &BenchmarkResult) -> ComparisonReport {
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();
        let mut unchanged = Vec::new();

        if let Some(baseline) = self.baseline.get(&current.name) {
            // Compare each metric
            for (metric_name, current_value) in &current.metrics {
                if let Some(baseline_value) = baseline.metrics.get(metric_name) {
                    let change = self.calculate_change(metric_name, *baseline_value, *current_value);

                    match change {
                        ChangeType::Regression(percent) => {
                            regressions.push(MetricChange {
                                metric: metric_name.clone(),
                                baseline: *baseline_value,
                                current: *current_value,
                                percent_change: percent,
                            });
                        }
                        ChangeType::Improvement(percent) => {
                            improvements.push(MetricChange {
                                metric: metric_name.clone(),
                                baseline: *baseline_value,
                                current: *current_value,
                                percent_change: percent,
                            });
                        }
                        ChangeType::Unchanged => {
                            unchanged.push(metric_name.clone());
                        }
                    }
                }
            }
        }

        ComparisonReport {
            benchmark: current.name.clone(),
            regressions,
            improvements,
            unchanged,
            has_regression: !regressions.is_empty(),
        }
    }

    fn calculate_change(&self, metric: &str, baseline: f64, current: f64) -> ChangeType {
        // For latency: higher is worse
        if metric.contains("latency") || metric.contains("time") {
            let change = ((current - baseline) / baseline) * 100.0;

            if change > self.threshold {
                ChangeType::Regression(change)
            } else if change < -self.threshold {
                ChangeType::Improvement(-change)
            } else {
                ChangeType::Unchanged
            }
        }
        // For throughput: lower is worse
        else if metric.contains("throughput") || metric.contains("rate") {
            let change = ((baseline - current) / baseline) * 100.0;

            if change > self.threshold {
                ChangeType::Regression(change)
            } else if change < -self.threshold {
                ChangeType::Improvement(-change)
            } else {
                ChangeType::Unchanged
            }
        }
        // For memory: higher is worse
        else if metric.contains("memory") || metric.contains("bytes") {
            let change = ((current - baseline) / baseline) * 100.0;

            if change > self.threshold {
                ChangeType::Regression(change)
            } else if change < -self.threshold {
                ChangeType::Improvement(-change)
            } else {
                ChangeType::Unchanged
            }
        } else {
            ChangeType::Unchanged
        }
    }

    fn parse_metrics(&self, value: &Value) -> BenchmarkMetrics {
        let mut metrics = HashMap::new();

        // Parse Criterion's typical output format
        if let Some(avg) = value["avg"].as_f64() {
            metrics.insert("avg".to_string(), avg);
        }

        if let Some(mean) = value["mean"].as_object() {
            if let Some(point_estimate) = mean["point_estimate"].as_f64() {
                metrics.insert("mean".to_string(), point_estimate);
            }
        }

        // Parse percentiles
        if let Some(mut slopes) = value["slope"].as_object() {
            // None
        }

        BenchmarkMetrics { metrics }
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkMetrics {
    metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug)]
pub struct ComparisonReport {
    pub benchmark: String,
    pub regressions: Vec<MetricChange>,
    pub improvements: Vec<MetricChange>,
    pub unchanged: Vec<String>,
    pub has_regression: bool,
}

#[derive(Debug)]
pub struct MetricChange {
    pub metric: String,
    pub baseline: f64,
    pub current: f64,
    pub percent_change: f64,
}

#[derive(Debug)]
enum ChangeType {
    Regression(f64),
    Improvement(f64),
    Unchanged,
}

#[derive(Debug)]
pub enum ComparatorError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
}

impl std::fmt::Display for ComparisonReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "## Benchmark: {}", self.benchmark)?;

        if self.regressions.is_empty() && self.improvements.is_empty() {
            writeln!(f, "✅ No significant performance changes")?;
        } else {
            if !self.regressions.is_empty() {
                writeln!(f, "\n### ⚠️ Regressions")?;
                for reg in &self.regressions {
                    writeln!(
                        f,
                        "- **{}**: {:.2} → {:.2} ({:.1}% {})",
                        reg.metric,
                        reg.baseline,
                        reg.current,
                        reg.percent_change.abs(),
                        if reg.percent_change > 0.0 { "slower" } else { "faster" }
                    )?;
                }
            }

            if !self.improvements.is_empty() {
                writeln!(f, "\n### ✅ Improvements")?;
                for imp in &self.improvements {
                    writeln!(
                        f,
                        "- **{}**: {:.2} → {:.2} ({:.1}% {})",
                        imp.metric,
                        imp.baseline,
                        imp.current,
                        imp.percent_change.abs(),
                        if imp.percent_change > 0.0 { "faster" } else { "slower" }
                    )?;
                }
            }
        }

        Ok(())
    }
}
```

## Nightly Benchmarks

### GitHub Actions Workflow

```yaml
# .github/workflows/nightly-benchmarks.yml

name: Nightly Benchmarks

on:
  schedule:
    - cron: '0 2 * * *'  # Run at 2 AM UTC
  workflow_dispatch:

jobs:
  nightly-benchmarks:
    name: Run Nightly Benchmarks
    runs-on: [self-hosted, performance]
    timeout-minutes: 180

    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          profile: release
          toolchain: stable

      - name: Run Full Benchmark Suite
        run: |
          cargo bench --all \
            -- --save-baseline nightly \
            --plotting-data \
            --noplot

      - name: Generate Report
        run: |
          cargo run --bin benchmark-report \
            --baseline main \
            --current nightly \
            --output nightly-report.md

      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: nightly-benchmark-results
          path: |
            criterion/
            nightly-report.md

      - name: Store in Database
        env:
          BENCHMARK_DB_URL: ${{ secrets.BENCHMARK_DB_URL }}
        run: |
          cargo run --bin store-benchmarks \
            --db-url "$BENCHMARK_DB_URL" \
            --source nightly \
            --commit ${{ github.sha }}

      - name: Check for Regression
        run: |
          cargo run --bin check-regression \
            --threshold 10.0 \
            --fail-on-regression

      - name: Create Issue on Regression
        if: failure()
        uses: actions/github-script@v6
        with:
          github-token: ${{ secrets.GITHUB_TOKEN }}
          script: |
            github.rest.issues.create({
              owner: context.repo.owner,
              repo: context.repo.repo,
              title: '⚠️ Performance Regression Detected',
              body: 'Performance regression detected in nightly benchmarks.\n\nSee [workflow run](${context.payload.repository.html_url}/actions/runs/${context.runId})',
              labels: ['performance', 'regression', 'priority:high']
            });
```

## Trend Analysis

### Performance Dashboard

```rust
// /plans/performance/code/monitoring/performance_dashboard.rs

use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

/// Performance trend over time
#[derive(Debug, Clone)]
pub struct PerformanceTrend {
    pub benchmark: String,
    pub metric: String,
    pub data_points: Vec<DataPoint>,
}

#[derive(Debug, Clone)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub commit: String,
    pub value: f64,
}

/// Analyze performance trends for gradual degradation
pub struct TrendAnalyzer {
    data: BTreeMap<DateTime<Utc>, BenchmarkRun>,
}

impl TrendAnalyzer {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    /// Add benchmark run data
    pub fn add_run(&mut self, run: BenchmarkRun) {
        self.data.insert(run.timestamp, run);
    }

    /// Detect gradual performance degradation
    pub fn detect_degradation(
        &self,
        benchmark: &str,
        metric: &str,
        window_days: i64,
        degradation_threshold: f64,
    ) -> Option<DegradationAlert> {
        let now = Utc::now();
        let window_start = now - chrono::Duration::days(window_days);

        // Collect data points within window
        let mut points = Vec::new();
        for (_timestamp, run) in self.data.range(window_start..) {
            if let Some(value) = run.get_metric(benchmark, metric) {
                points.push(DataPoint {
                    timestamp: run.timestamp,
                    commit: run.commit.clone(),
                    value,
                });
            }
        }

        if points.len() < 3 {
            return None; // Not enough data
        }

        // Calculate linear regression
        let (slope, correlation) = self.linear_regression(&points);

        // Check if slope indicates degradation
        // Positive slope for latency = degradation
        // Negative slope for throughput = degradation
        let is_degradation = if metric.contains("latency") || metric.contains("time") {
            slope > degradation_threshold
        } else if metric.contains("throughput") || metric.contains("rate") {
            slope < -degradation_threshold
        } else {
            false
        };

        if is_degradation && correlation.abs() > 0.7 {
            // Strong correlation + significant slope
            Some(DegradationAlert {
                benchmark: benchmark.to_string(),
                metric: metric.to_string(),
                slope,
                correlation,
                window_days,
                data_points: points,
            })
        } else {
            None
        }
    }

    /// Calculate linear regression (y = mx + b)
    fn linear_regression(&self, points: &[DataPoint]) -> (f64, f64) {
        if points.len() < 2 {
            return (0.0, 0.0);
        }

        let n = points.len() as f64;

        // Convert timestamps to numeric values
        let min_time = points.iter().map(|p| p.timestamp.timestamp()).min().unwrap();

        let sum_x: f64 = points
            .iter()
            .map(|p| (p.timestamp.timestamp() - min_time) as f64)
            .sum();

        let sum_y: f64 = points.iter().map(|p| p.value).sum();

        let sum_xy: f64 = points
            .iter()
            .map(|p| {
                let x = (p.timestamp.timestamp() - min_time) as f64;
                x * p.value
            })
            .sum();

        let sum_x2: f64 = points
            .iter()
            .map(|p| {
                let x = (p.timestamp.timestamp() - min_time) as f64;
                x * x
            })
            .sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);

        // Calculate correlation coefficient
        let mean_x = sum_x / n;
        let mean_y = sum_y / n;

        let mut numerator = 0.0;
        let mut sum_xx = 0.0;
        let mut sum_yy = 0.0;

        for point in points {
            let x = (point.timestamp.timestamp() - min_time) as f64;
            let y = point.value;

            numerator += (x - mean_x) * (y - mean_y);
            sum_xx += (x - mean_x).powi(2);
            sum_yy += (y - mean_y).powi(2);
        }

        let correlation = numerator / (sum_xx * sum_yy).sqrt();

        (slope, correlation)
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkRun {
    pub timestamp: DateTime<Utc>,
    pub commit: String,
    pub benchmarks: HashMap<String, BenchmarkMetrics>,
}

impl BenchmarkRun {
    pub fn get_metric(&self, benchmark: &str, metric: &str) -> Option<f64> {
        self.benchmarks.get(benchmark)?.metrics.get(metric).copied()
    }
}

#[derive(Debug)]
pub struct DegradationAlert {
    pub benchmark: String,
    pub metric: String,
    pub slope: f64,
    pub correlation: f64,
    pub window_days: i64,
    pub data_points: Vec<DataPoint>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkMetrics {
    pub metrics: HashMap<String, f64>,
}
```

## Canary Deployments

### Canary Strategy

```rust
// /plans/performance/code/deployment/canary.rs

use std::time::Duration;

/// Canary deployment configuration
pub struct CanaryConfig {
    /// Percentage of traffic to send to canary
    pub traffic_percentage: u8,

    /// Duration of canary phase
    pub duration: Duration,

    /// SLO compliance threshold
    pub slo_threshold: f64,

    /// Auto-rollback on SLO violation
    pub auto_rollback: bool,

    /// Minimum success rate before promotion
    pub min_success_rate: f64,
}

impl Default for CanaryConfig {
    fn default() -> Self {
        Self {
            traffic_percentage: 5, // Start with 5% traffic
            duration: Duration::from_secs(1800), // 30 minutes
            slo_threshold: 0.95, // 95% of requests must meet SLO
            auto_rollback: true,
            min_success_rate: 0.99, // 99% success rate
        }
    }
}

/// Canary deployment manager
pub struct CanaryManager {
    config: CanaryConfig,
    metrics: MetricsCollector,
}

impl CanaryManager {
    pub fn new(config: CanaryConfig) -> Self {
        Self {
            config,
            metrics: MetricsCollector::new(),
        }
    }

    /// Execute canary deployment
    pub async fn run_canary(&self, new_version: &str) -> Result<CanaryResult, CanaryError> {
        println!("Starting canary deployment for version: {}", new_version);
        println!("Traffic: {}%", self.config.traffic_percentage);
        println!("Duration: {:?}", self.config.duration);

        let start_time = Instant::now();
        let mut success_count = 0;
        let mut total_count = 0;

        // Monitor canary for duration
        while start_time.elapsed() < self.config.duration {
            // Collect metrics for canary version
            let canary_metrics = self.metrics.collect_version(new_version).await?;

            // Collect metrics for stable version
            let stable_metrics = self.metrics.collect_version("stable").await?;

            // Compare SLO compliance
            let slo_comparison = self.compare_slo(&stable_metrics, &canary_metrics);

            println!(
                "[{:?}] SLO Compliance: {:.1}%",
                start_time.elapsed(),
                slo_comparison.compliance * 100.0
            );

            // Check for SLO violation
            if slo_comparison.compliance < self.config.slo_threshold {
                if self.config.auto_rollback {
                    eprintln!("SLO violation detected! Auto-rolling back.");
                    self.rollback(new_version).await?;

                    return Ok(CanaryResult {
                        success: false,
                        reason: Some("SLO violation".to_string()),
                        duration: start_time.elapsed(),
                        slo_compliance: slo_comparison.compliance,
                    });
                }
            }

            // Track success rate
            total_count += 1;
            if slo_comparison.compliance >= self.config.slo_threshold {
                success_count += 1;
            }

            sleep(Duration::from_secs(60)).await;
        }

        // Calculate final success rate
        let success_rate = success_count as f64 / total_count as f64;

        if success_rate >= self.config.min_success_rate {
            println!("Canary successful! Promoting to full rollout.");
            self.promote(new_version).await?;

            Ok(CanaryResult {
                success: true,
                reason: None,
                duration: start_time.elapsed(),
                slo_compliance: success_rate,
            })
        } else {
            eprintln!("Canary failed! Success rate: {:.1}%", success_rate * 100.0);
            self.rollback(new_version).await?;

            Ok(CanaryResult {
                success: false,
                reason: Some(format!("Success rate below threshold: {:.1}%", success_rate * 100.0)),
                duration: start_time.elapsed(),
                slo_compliance: success_rate,
            })
        }
    }

    fn compare_slo(&self, stable: &VersionMetrics, canary: &VersionMetrics) -> SLOComparison {
        // Compare latency
        let latency_compliant = canary.latency_p99 <= stable.latency_p99 * 1.1; // Allow 10% degradation

        // Compare error rate
        let error_rate_compliant = canary.error_rate <= stable.error_rate * 1.05; // Allow 5% increase

        // Compare throughput
        let throughput_compliant = canary.throughput >= stable.throughput * 0.95; // Allow 5% decrease

        let compliant_metrics = [
            latency_compliant,
            error_rate_compliant,
            throughput_compliant,
        ]
        .iter()
        .filter(|&&x| x)
        .count();

        let compliance = compliant_metrics as f64 / 3.0;

        SLOComparison { compliance }
    }

    async fn rollback(&self, version: &str) -> Result<(), CanaryError> {
        // Execute rollback
        println!("Rolling back version: {}", version);
        // TODO: Implement rollback logic
        Ok(())
    }

    async fn promote(&self, version: &str) -> Result<(), CanaryError> {
        // Promote canary to stable
        println!("Promoting version: {}", version);
        // TODO: Implement promotion logic
        Ok(())
    }
}

#[derive(Debug)]
pub struct CanaryResult {
    pub success: bool,
    pub reason: Option<String>,
    pub duration: Duration,
    pub slo_compliance: f64,
}

#[derive(Debug, Clone)]
pub struct VersionMetrics {
    pub latency_p99: f64,
    pub error_rate: f64,
    pub throughput: f64,
}

#[derive(Debug, Clone)]
pub struct SLOComparison {
    pub compliance: f64,
}

#[derive(Debug)]
pub enum CanaryError {
    MetricsError(String),
    RollbackError(String),
}

struct MetricsCollector;

impl MetricsCollector {
    fn new() -> Self {
        Self
    }

    async fn collect_version(&self, _version: &str) -> Result<VersionMetrics, CanaryError> {
        // TODO: Implement metrics collection
        Ok(VersionMetrics {
            latency_p99: 100.0,
            error_rate: 0.01,
            throughput: 1000.0,
        })
    }
}
```

---

**Last Updated:** 2025-01-18
**Next:** Complete code optimization samples and intelligent sampling strategy
