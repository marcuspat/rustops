# Comprehensive Benchmarking Plan

## Overview

This document defines the comprehensive benchmarking strategy for validating RustOps performance targets and preventing performance regressions.

## Benchmark Categories

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      Benchmark Categories                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. INGESTION BENCHMARKS                                                    │
│     ├─ Metric ingestion rate (target: 10M metrics/minute)                  │
│     ├─ Log processing throughput (target: 1TB/day)                         │
│     ├─ Trace ingestion rate (target: 100K spans/second)                    │
│     └─ Event processing (target: 50K events/second)                        │
│                                                                             │
│  2. ML INFERENCE BENCHMARKS                                                 │
│     ├─ Anomaly detection latency (target: <100ms p99)                      │
│     ├─ Flash Attention speedup (target: 2.49x-7.47x)                       │
│     ├─ Model quantization accuracy (target: >95% with INT8)                │
│     └─ Batch inference throughput (target: 10K predictions/second)         │
│                                                                             │
│  3. CORRELATION BENCHMARKS                                                  │
│     ├─ Alert correlation latency (target: <500ms p99)                       │
│     ├─ HNSW search speedup (target: 150x-12,500x)                          │
│     ├─ Deduplication accuracy (target: >98%)                               │
│     └─ Topology graph traversal (target: <50ms for 1000 nodes)            │
│                                                                             │
│  4. QUERY BENCHMARKS                                                        │
│     ├─ Query response time (target: <200ms p95)                            │
│     ├─ Cache hit rates (target: >80% L1, >30% L2)                          │
│     ├─ Aggregation query performance (target: <100ms for 1M points)       │
│     └─ Full-text search (target: <500ms for 1TB logs)                      │
│                                                                             │
│  5. AGENT BENCHMARKS                                                        │
│     ├─ CPU overhead (target: <1%)                                          │
│     ├─ Memory usage (target: <150MB)                                       │
│     ├─ Network bandwidth (target: <10MB/s per agent)                       │
│     └─ Startup time (target: <2 seconds)                                   │
│                                                                             │
│  6. SCALING BENCHMARKS                                                      │
│     ├─ Horizontal scaling efficiency (target: >90%)                        │
│     ├─ Shard rebalancing time (target: <5 minutes)                         │
│     ├─ Backpressure handling (target: zero data loss)                       │
│     └─ Failover time (target: <30 seconds)                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Benchmark Implementation

### Using Criterion.rs

```rust
// /plans/performance/code/benches/metric_ingestion.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rustops_telemetry::MetricParser;

/// Benchmark metric parsing with zero-copy optimization
fn bench_metric_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("metric_parsing");

    // Test different batch sizes
    for size in [100, 1000, 10000, 100000].iter() {
        let input = generate_test_metrics(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &_size| {
            b.iter(|| {
                let parser = MetricParser::new(&input);
                parser.parse_batch()
            })
        });
    }

    group.finish();
}

/// Benchmark SIMD-accelerated metric normalization
fn bench_simd_normalization(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_normalization");

    for size in [256, 1024, 4096, 16384].iter() {
        let values: Vec<f32> = (0..*size).map(|i| i as f32).collect();
        let mean = values.iter().sum::<f32>() / values.len() as f32;
        let std_dev = (values.iter().map(|&v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32).sqrt();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &_size| {
            b.iter(|| {
                simd_normalize_metrics(black_box(&values), black_box(mean), black_box(std_dev))
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_metric_parsing, bench_simd_normalization);
criterion_main!(benches);
```

### Flash Attention Benchmark

```rust
// /plans/performance/code/benches/flash_attention.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rustops_ml::FlashAttentionEngine;

/// Benchmark Flash Attention vs Standard Attention
fn bench_flash_attention(c: &mut Criterion) {
    let mut group = c.benchmark_group("flash_attention");

    // Test different sequence lengths
    for seq_len in [128, 256, 512, 1024, 2048, 4096].iter() {
        let (q, k, v) = generate_attention_input(*seq_len);

        // Flash Attention
        group.bench_with_input(
            BenchmarkId::new("flash", seq_len),
            seq_len,
            |b, &_len| {
                let engine = FlashAttentionEngine::new(8, 512, AttentionBackend::CPU {
                    simd_enabled: true,
                    num_threads: 4,
                });

                b.iter(|| {
                    engine.forward(
                        black_box(&q),
                        black_box(&k),
                        black_box(&v),
                    )
                })
            },
        );

        // Standard Attention (baseline)
        group.bench_with_input(
            BenchmarkId::new("standard", seq_len),
            seq_len,
            |b, &_len| {
                b.iter(|| {
                    standard_attention_forward(
                        black_box(&q),
                        black_box(&k),
                        black_box(&v),
                    )
                })
            },
        );
    }

    group.finish();
}

fn generate_attention_input(seq_len: usize) -> (Array3<f32>, Array3<f32>, Array3<f32>) {
    let batch_size = 1;
    let num_heads = 8;
    let dim = 64;

    let q = Array3::random((batch_size, num_heads, seq_len, dim));
    let k = Array3::random((batch_size, num_heads, seq_len, dim));
    let v = Array3::random((batch_size, num_heads, seq_len, dim));

    (q, k, v)
}

fn standard_attention_forward(
    q: &Array3<f32>,
    k: &Array3<f32>,
    v: &Array3<f32>,
) -> Array3<f32> {
    // Standard attention implementation for comparison
    // O(N²) memory and time
    todo!("Implement standard attention")
}

criterion_group!(benches, bench_flash_attention);
criterion_main!(benches);
```

### HNSW Search Benchmark

```rust
// /plans/performance/code/benches/hnsw_search.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rustops_correlation::HNSWPatternIndex;

/// Benchmark HNSW search vs Linear search
fn bench_hnsw_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_search");

    // Test different numbers of patterns
    for num_patterns in [1_000, 10_000, 100_000, 1_000_000].iter() {
        let (index, query) = setup_hnsw_index(*num_patterns);

        // HNSW search
        group.bench_with_input(
            BenchmarkId::new("hnsw", num_patterns),
            num_patterns,
            |b, &_num| {
                b.iter(|| {
                    index.find_similar(black_box(&query), black_box(10))
                })
            },
        );

        // Linear search (baseline)
        let linear_patterns = index.get_all_patterns();
        group.bench_with_input(
            BenchmarkId::new("linear", num_patterns),
            num_patterns,
            |b, &_num| {
                b.iter(|| {
                    linear_search(black_box(&linear_patterns), black_box(&query), black_box(10))
                })
            },
        );
    }

    group.finish();
}

fn setup_hnsw_index(num_patterns: usize) -> (HNSWPatternIndex, Vec<f32>) {
    let mut index = HNSWPatternIndex::new(128, Default::default());

    for i in 0..num_patterns {
        let pattern = AlertPattern {
            id: i,
            embedding: generate_random_embedding(128),
            pattern_type: PatternType::MetricSpike {
                metric_name: format!("metric_{}", i),
                magnitude: rand::random(),
            },
            temporal_features: TemporalFeatures {
                hour_of_day: rand::random(),
                day_of_week: rand::random(),
                is_weekend: rand::random(),
                time_since_last_incident: rand::random(),
            },
        };

        index.add_pattern(pattern).unwrap();
    }

    let query = generate_random_embedding(128);

    (index, query)
}

fn linear_search(
    patterns: &[AlertPattern],
    query: &[f32],
    k: usize,
) -> Vec<usize> {
    // Naive linear search for comparison
    patterns
        .iter()
        .map(|p| cosine_similarity(&p.embedding, query))
        .enumerate()
        .sorted_by(|a, b| b.1.partial_cmp(&a.1).unwrap())
        .take(k)
        .map(|(i, _)| i)
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}

fn generate_random_embedding(dim: usize) -> Vec<f32> {
    (0..dim).map(|_| rand::random::<f32>() * 2.0 - 1.0).collect()
}

criterion_group!(benches, bench_hnsw_search);
criterion_main!(benches);
```

### Alert Correlation Benchmark

```rust
// /plans/performance/code/benches/alert_correlation.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustops_correlation::AlertCorrelator;

/// Benchmark alert correlation latency
fn bench_alert_correlation(c: &mut Criterion) {
    let mut group = c.benchmark_group("alert_correlation");

    let pattern_index = setup_pattern_index(100_000);
    let correlator = AlertCorrelator::new(pattern_index, 300); // 5-minute window

    // Single alert correlation
    group.bench_function("single_alert", |b| {
        let alert = generate_test_alert();

        b.iter(|| {
            correlator.correlate(black_box(alert.clone()))
        })
    });

    // Batch alert correlation
    group.bench_function("batch_100_alerts", |b| {
        let alerts: Vec<_> = (0..100).map(|_| generate_test_alert()).collect();

        b.iter(|| {
            for alert in &alerts {
                black_box(correlator.correlate(black_box(alert.clone())));
            }
        })
    });

    group.finish();
}

/// Benchmark with different alert rates
fn bench_alert_rate_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("alert_rate_scalability");

    for alerts_per_second in [100, 1000, 10000, 50000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(alerts_per_second),
            alerts_per_second,
            |b, &_rate| {
                let pattern_index = setup_pattern_index(100_000);
                let correlator = AlertCorrelator::new(pattern_index, 300);

                b.iter(|| {
                    // Simulate 1 second of alerts
                    for _ in 0..*alerts_per_second {
                        let alert = generate_test_alert();
                        black_box(correlator.correlate(alert));
                    }
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_alert_correlation, bench_alert_rate_scalability);
criterion_main!(benches);
```

### Memory Benchmark

```rust
// /plans/performance/code/benches/memory_usage.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rustops_agent::AgentRuntime;

/// Benchmark agent memory usage
fn bench_agent_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("agent_memory");

    // Test different numbers of monitored metrics
    for num_metrics in [100, 1000, 10000, 100000].iter() {
        group.bench_with_input(
            BenchmarkId::new("memory_usage", num_metrics),
            num_metrics,
            |b, &_num| {
                b.iter(|| {
                    let agent = AgentRuntime::new();
                    agent.monitor_metrics(black_box(*num_metrics));

                    // Measure memory after stabilization
                    let memory_mb = get_memory_usage_mb();
                    memory_mb
                })
            },
        );
    }

    group.finish();
}

/// Benchmark arena allocator vs standard allocator
fn bench_arena_vs_standard(c: &mut Criterion) {
    let mut group = c.benchmark_group("arena_allocator");

    for num_allocations in [1000, 10000, 100000].iter() {
        // Arena allocator
        group.bench_with_input(
            BenchmarkId::new("arena", num_allocations),
            num_allocations,
            |b, &_num| {
                b.iter(|| {
                    let arena = ArenaAllocator::new(1024 * 1024); // 1MB

                    for i in 0..*num_allocations {
                        let value = MetricData {
                            name: format!("metric_{}", i),
                            value: rand::random(),
                            timestamp: chrono::Utc::now().timestamp(),
                        };

                        black_box(arena.alloc(value));
                    }
                })
            },
        );

        // Standard allocator
        group.bench_with_input(
            BenchmarkId::new("standard", num_allocations),
            num_allocations,
            |b, &_num| {
                b.iter(|| {
                    let mut metrics: Vec<MetricData> = Vec::with_capacity(*num_allocations);

                    for i in 0..*num_allocations {
                        let value = MetricData {
                            name: format!("metric_{}", i),
                            value: rand::random(),
                            timestamp: chrono::Utc::now().timestamp(),
                        };

                        metrics.push(value);
                    }
                })
            },
        );
    }

    group.finish();
}

fn get_memory_usage_mb() -> f64 {
    // Use libc or similar to get actual memory usage
    // For benchmarking, we can track allocations manually
    0.0 // Placeholder
}

#[derive(Clone)]
struct MetricData {
    name: String,
    value: f64,
    timestamp: i64,
}

criterion_group!(benches, bench_agent_memory, bench_arena_vs_standard);
criterion_main!(benches);
```

## Load Testing Scenarios

### Gradual Ramp-Up Test

```rust
// /plans/performance/code/load_tests/gradual_rampup.rs

use tokio::time::{sleep, Duration};

/// Gradual ramp-up test for metric ingestion
pub async fn gradual_rampup_test() -> Result<TestReport, TestError> {
    let config = RampUpConfig {
        initial_rate: 10_000,   // Start at 10K metrics/second
        max_rate: 200_000,      // Ramp up to 200K metrics/second (12M/min)
        ramp_duration: Duration::from_secs(600), // 10 minutes
        measurement_interval: Duration::from_secs(10),
    };

    let mut current_rate = config.initial_rate;
    let mut measurements = Vec::new();

    let start_time = Instant::now();
    let mut elapsed = Duration::ZERO;

    while elapsed < config.ramp_duration {
        // Send metrics at current rate
        let send_start = Instant::now();
        let result = send_metrics(current_rate).await;
        let send_duration = send_start.elapsed();

        // Measure processing latency
        let processing_latency = measure_processing_latency().await;

        // Measure system resources
        let cpu_usage = measure_cpu_usage();
        let memory_usage = measure_memory_usage();

        measurements.push(Measurement {
            timestamp: start_time.elapsed(),
            rate: current_rate,
            send_duration,
            processing_latency_p50: processing_latency.p50,
            processing_latency_p99: processing_latency.p99,
            cpu_usage,
            memory_usage_mb: memory_usage.mb,
        });

        // Check if we're meeting SLA
        if processing_latency.p99 > 100.0 {
            return Err(TestError::SLAViolation {
                metric: "processing_latency_p99".to_string(),
                threshold: 100.0,
                actual: processing_latency.p99,
            });
        }

        if cpu_usage > 90.0 {
            return Err(TestError::ResourceExhausted {
                resource: "cpu".to_string(),
                usage: cpu_usage,
            });
        }

        // Ramp up rate
        elapsed = start_time.elapsed();
        let progress = elapsed.as_secs_f64() / config.ramp_duration.as_secs_f64();
        current_rate = (config.initial_rate as f64
            + (config.max_rate as f64 - config.initial_rate as f64) * progress)
            as usize;

        sleep(config.measurement_interval).await;
    }

    Ok(TestReport {
        test_name: "gradual_rampup".to_string(),
        duration: start_time.elapsed(),
        measurements,
        max_rate_achieved: current_rate,
        sla_met: true,
    })
}

struct RampUpConfig {
    initial_rate: usize,
    max_rate: usize,
    ramp_duration: Duration,
    measurement_interval: Duration,
}

struct Measurement {
    timestamp: Duration,
    rate: usize,
    send_duration: Duration,
    processing_latency_p50: f64,
    processing_latency_p99: f64,
    cpu_usage: f64,
    memory_usage_mb: f64,
}

struct TestReport {
    test_name: String,
    duration: Duration,
    measurements: Vec<Measurement>,
    max_rate_achieved: usize,
    sla_met: bool,
}

enum TestError {
    SLAViolation { metric: String, threshold: f64, actual: f64 },
    ResourceExhausted { resource: String, usage: f64 },
}
```

### Spike Test

```rust
// /plans/performance/code/load_tests/spike_test.rs

/// Spike test: Sudden 10x traffic increase
pub async fn spike_test() -> Result<TestReport, TestError> {
    let baseline_rate = 50_000; // 50K metrics/second (3M/min)
    let spike_multiplier = 10;
    let spike_duration = Duration::from_secs(60);

    println!("Starting spike test with {}x spike", spike_multiplier);
    println!("Baseline: {} metrics/second", baseline_rate);
    println!("Spike: {} metrics/second", baseline_rate * spike_multiplier);

    // Establish baseline
    println!("Establishing baseline (30 seconds)...");
    for _ in 0..3 {
        send_metrics(baseline_rate).await;
        sleep(Duration::from_secs(10)).await;
    }

    let baseline_latency = measure_processing_latency().await;

    // Apply spike
    println!("Applying spike...");
    let spike_start = Instant::now();
    let mut spike_measurements = Vec::new();

    while spike_start.elapsed() < spike_duration {
        let send_start = Instant::now();
        send_metrics(baseline_rate * spike_multiplier).await;
        let send_duration = send_start.elapsed();

        let latency = measure_processing_latency().await;

        spike_measurements.push(SpikeMeasurement {
            elapsed: spike_start.elapsed(),
            latency_p50: latency.p50,
            latency_p99: latency.p99,
            send_duration,
        });

        sleep(Duration::from_secs(1)).await;
    }

    // Analyze results
    let max_latency_p99 = spike_measurements
        .iter()
        .map(|m| m.latency_p99)
        .fold(f64::NEG_INFINITY, f64::max);

    let avg_latency_p99 = spike_measurements
        .iter()
        .map(|m| m.latency_p99)
        .sum::<f64>() / spike_measurements.len() as f64;

    // Check if system recovered
    println!("Spike ended. Measuring recovery...");
    sleep(Duration::from_secs(10)).await;
    let recovery_latency = measure_processing_latency().await;

    let recovered = recovery_latency.p99 < baseline_latency.p99 * 1.5;

    println!("Spike test results:");
    println!("  Max p99 latency during spike: {:.2}ms", max_latency_p99);
    println!("  Avg p99 latency during spike: {:.2}ms", avg_latency_p99);
    println!("  Baseline p99 latency: {:.2}ms", baseline_latency.p99);
    println!("  Recovery p99 latency: {:.2}ms", recovery_latency.p99);
    println!("  System recovered: {}", recovered);

    if max_latency_p99 > 500.0 {
        return Err(TestError::SLAViolation {
            metric: "latency_p99".to_string(),
            threshold: 500.0,
            actual: max_latency_p99,
        });
    }

    Ok(TestReport {
        test_name: "spike_test".to_string(),
        duration: spike_start.elapsed(),
        measurements: spike_measurements
            .into_iter()
            .map(|m| Measurement {
                timestamp: m.elapsed,
                rate: baseline_rate * spike_multiplier,
                send_duration: m.send_duration,
                processing_latency_p50: m.latency_p50,
                processing_latency_p99: m.latency_p99,
                cpu_usage: 0.0,
                memory_usage_mb: 0.0,
            })
            .collect(),
        max_rate_achieved: baseline_rate * spike_multiplier,
        sla_met: recovered,
    })
}

struct SpikeMeasurement {
    elapsed: Duration,
    latency_p50: f64,
    latency_p99: f64,
    send_duration: Duration,
}
```

### Sustained Load Test

```rust
// /plans/performance/code/load_tests/sustained_load.rs

/// Sustained load test: Run at target rate for extended period
pub async fn sustained_load_test(duration: Duration) -> Result<TestReport, TestError> {
    let target_rate = 166_667; // 10M metrics/minute

    println!("Starting sustained load test:");
    println!("  Target rate: {} metrics/second ({:.2}M/minute)", target_rate, target_rate * 60 / 1_000_000);
    println!("  Duration: {:?}", duration);

    let start_time = Instant::now();
    let mut measurements = Vec::new();
    let mut consecutive_sla_violations = 0;
    let max_consecutive_violations = 5;

    while start_time.elapsed() < duration {
        let measurement_start = Instant::now();

        // Send metrics
        let send_result = tokio::time::timeout(
            Duration::from_secs(10),
            send_metrics(target_rate),
        ).await;

        let send_duration = measurement_start.elapsed();

        match send_result {
            Ok(_) => {
                // Measure processing latency
                let latency = measure_processing_latency().await;

                // Check SLA
                let sla_met = latency.p99 < 500.0;

                if !sla_met {
                    consecutive_sla_violations += 1;
                    eprintln!(
                        "SLA violation: p99 latency {:.2}ms (threshold: 500ms), consecutive: {}",
                        latency.p99,
                        consecutive_sla_violations
                    );

                    if consecutive_sla_violations >= max_consecutive_violations {
                        return Err(TestError::SLAViolation {
                            metric: "latency_p99".to_string(),
                            threshold: 500.0,
                            actual: latency.p99,
                        });
                    }
                } else {
                    consecutive_sla_violations = 0;
                }

                measurements.push(Measurement {
                    timestamp: start_time.elapsed(),
                    rate: target_rate,
                    send_duration,
                    processing_latency_p50: latency.p50,
                    processing_latency_p99: latency.p99,
                    cpu_usage: measure_cpu_usage(),
                    memory_usage_mb: measure_memory_usage().mb,
                });
            }
            Err(_) => {
                return Err(TestError::Timeout {
                    operation: "send_metrics".to_string(),
                    timeout: Duration::from_secs(10),
                });
            }
        }

        // Log progress every minute
        let elapsed = start_time.elapsed();
        if elapsed.as_secs() % 60 == 0 && elapsed.as_secs() > 0 {
            let latest = &measurements[measurements.len() - 1];
            println!(
                "[{:02}:{:02}:{:02}] Rate: {}/s, p50: {:.1}ms, p99: {:.1}ms, CPU: {:.1}%, Mem: {:.0}MB",
                elapsed.as_hours() as u8,
                (elapsed.as_minutes() % 60) as u8,
                (elapsed.as_secs() % 60) as u8,
                latest.rate,
                latest.processing_latency_p50,
                latest.processing_latency_p99,
                latest.cpu_usage,
                latest.memory_usage_mb
            );
        }

        sleep(Duration::from_secs(1)).await;
    }

    // Compute statistics
    let avg_p50 = measurements.iter().map(|m| m.processing_latency_p50).sum::<f64>() / measurements.len() as f64;
    let avg_p99 = measurements.iter().map(|m| m.processing_latency_p99).sum::<f64>() / measurements.len() as f64;
    let max_p99 = measurements.iter().map(|m| m.processing_latency_p99).fold(f64::NEG_INFINITY, f64::max);

    println!("\nSustained load test completed:");
    println!("  Duration: {:?}", start_time.elapsed());
    println!("  Total measurements: {}", measurements.len());
    println!("  Avg p50 latency: {:.2}ms", avg_p50);
    println!("  Avg p99 latency: {:.2}ms", avg_p99);
    println!("  Max p99 latency: {:.2}ms", max_p99);
    println!("  SLA met: {}", avg_p99 < 500.0);

    Ok(TestReport {
        test_name: "sustained_load".to_string(),
        duration: start_time.elapsed(),
        measurements,
        max_rate_achieved: target_rate,
        sla_met: avg_p99 < 500.0,
    })
}
```

## Continuous Benchmarking

### CI/CD Integration

```yaml
# .github/workflows/benchmark.yml

name: Performance Benchmarks

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]
  schedule:
    # Run daily at midnight UTC
    - cron: '0 0 * * *'

jobs:
  benchmark:
    runs-on: [self-hosted, performance]
    timeout-minutes: 120

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust toolchain
        uses: actions-rs/toolchain@v1
        with:
          profile: release
          toolchain: stable

      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run benchmarks
        run: |
          cargo bench --bench metric_ingestion -- --save-baseline main
          cargo bench --bench flash_attention -- --save-baseline main
          cargo bench --bench hnsw_search -- --save-baseline main
          cargo bench --bench alert_correlation -- --save-baseline main
          cargo bench --bench memory_usage -- --save-baseline main

      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: benchmark/results.json
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true
          alert-threshold: '150%'
          comment-on-alert: true
          fail-on-alert: true
          alert-comment-cc: '@platform-team'

      - name: Upload benchmark results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: benchmark/results.json
```

### Performance Regression Detection

```rust
// /plans/performance/code/benches/regression_detection.rs

use std::collections::HashMap;

/// Detect performance regressions by comparing to baseline
pub struct RegressionDetector {
    baseline: HashMap<String, BenchmarkBaseline>,
    threshold: f64, // Percentage change considered regression
}

impl RegressionDetector {
    pub fn new(threshold: f64) -> Self {
        Self {
            baseline: HashMap::new(),
            threshold,
        }
    }

    /// Load baseline from previous benchmark run
    pub fn load_baseline(&mut self, path: &str) -> Result<(), RegressionError> {
        let data = std::fs::read_to_string(path)?;
        let baseline: BenchmarkBaseline = serde_json::from_str(&data)?;

        self.baseline.insert(baseline.name.clone(), baseline);

        Ok(())
    }

    /// Check if current results show regression
    pub fn check_regression(&self, current: &BenchmarkResult) -> Vec<Regression> {
        let mut regressions = Vec::new();

        if let Some(baseline) = self.baseline.get(&current.name) {
            for (metric_name, current_value) in &current.metrics {
                if let Some(baseline_value) = baseline.metrics.get(metric_name) {
                    // For latency, higher is worse
                    if metric_name.contains("latency") || metric_name.contains("time") {
                        let change = (current_value - baseline_value) / baseline_value * 100.0;

                        if change > self.threshold {
                            regressions.push(Regression {
                                benchmark: current.name.clone(),
                                metric: metric_name.clone(),
                                baseline_value: *baseline_value,
                                current_value: *current_value,
                                change_percent: change,
                                severity: if change > self.threshold * 2.0 {
                                    RegressionSeverity::Critical
                                } else {
                                    RegressionSeverity::Warning
                                },
                            });
                        }
                    }
                    // For throughput, lower is worse
                    else if metric_name.contains("throughput") || metric_name.contains("rate") {
                        let change = (baseline_value - current_value) / baseline_value * 100.0;

                        if change > self.threshold {
                            regressions.push(Regression {
                                benchmark: current.name.clone(),
                                metric: metric_name.clone(),
                                baseline_value: *baseline_value,
                                current_value: *current_value,
                                change_percent: change,
                                severity: if change > self.threshold * 2.0 {
                                    RegressionSeverity::Critical
                                } else {
                                    RegressionSeverity::Warning
                                },
                            });
                        }
                    }
                }
            }
        }

        regressions
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct BenchmarkBaseline {
    name: String,
    metrics: HashMap<String, f64>,
}

#[derive(Debug)]
pub struct BenchmarkResult {
    pub name: String,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug)]
pub struct Regression {
    pub benchmark: String,
    pub metric: String,
    pub baseline_value: f64,
    pub current_value: f64,
    pub change_percent: f64,
    pub severity: RegressionSeverity,
}

#[derive(Debug, PartialEq)]
pub enum RegressionSeverity {
    Warning,   // 5-10% regression
    Critical,  // >10% regression
}

#[derive(Debug)]
pub enum RegressionError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
}
```

---

**Last Updated:** 2025-01-18
**Next:** Implement profiling guide and code optimization samples
