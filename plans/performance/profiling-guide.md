# Profiling Guide for RustOps

## Overview

This guide provides comprehensive instructions for profiling RustOps components to identify performance bottlenecks and optimization opportunities.

## Profiling Tools

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      Profiling Tools Ecosystem                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  CPU PROFILING                                                               │
│  ├─ Flamegraph:     Visualize call stacks and hot paths                    │
│  ├─ perf:           Linux perf events for low-overhead sampling            │
│  ├─ Instruments:    macOS profiling (time profiler)                       │
│  └─ VTune:          Intel VTune Amplifier (advanced CPU analysis)         │
│                                                                             │
│  MEMORY PROFILING                                                           │
│  ├─ valgrind:       Memory leak detection and heap analysis                │
│  ├─ dhprof:         Dynamic heap profiling for Rust                        │
│  ├─ heaptrack:      Heap memory usage tracking                             │
│  └─ mmap:           Memory mapping and allocation tracking                 │
│                                                                             │
│  ALLOCATION PROFILING                                                       │
│  ├─ dhat:           Heap usage analysis                                    │
│  ├─ tally:          Allocation counter for Rust                            │
│  └─ custom:         Custom allocation tracking                             │
│                                                                             │
│  I/O PROFILING                                                              │
│  ├─ eBPF:           Kernel-level I/O tracing                               │
│  ├─ strace:         System call tracing                                    │
│  ├─ iostat:         Disk I/O statistics                                    │
│  └─ tcpdump:        Network packet capture                                 │
│                                                                             │
│  APPLICATION PROFILING                                                      │
│  ├─ tracing:        Structured logging and spans                           │
│  ├─ tokio-console:  Async runtime debugging                                │
│  ├─ metrics:        Prometheus metrics export                              │
│  └─ custom:         Application-level instrumentation                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 1. Flamegraph Profiling

### Installation

```bash
# Install Flamegraph tool
cargo install flamegraph

# Or install from source
git clone https://github.com/flamegraph-rs/flamegraph.git
cd flamegraph
cargo install --path .
```

### Basic Usage

```bash
# Generate flamegraph for entire application
cargo flamegraph --bin rustops-core

# Generate flamegraph for specific benchmark
cargo flamegraph --bench metric_ingestion

# With custom options
cargo flamegraph \
  --bin rustops-core \
  --frequency 999 \  # Sampling frequency (Hz)
  --duration 60 \    # Sampling duration (seconds)
  --output flamegraph.svg

# Generate flamegraph with specific command-line args
cargo flamegraph --bin rustops-core -- --config production.toml
```

### Example: Profiling Metric Ingestion

```bash
#!/bin/bash
# /plans/performance/scripts/profile-ingestion.sh

set -e

echo "Profiling metric ingestion pipeline..."

# Build with profiling symbols
cargo build --release --bins

# Profile with Flamegraph
cargo flamegraph \
  --bin rustops-ingestion \
  --frequency 9997 \
  --duration 120 \
  --output ingestion-flamegraph.svg \
  -- \
  --rate 200000 \
  --duration 60

echo "Flamegraph generated: ingestion-flamegraph.svg"

# Open in browser (Linux)
xdg-open ingestion-flamegraph.svg

# Open in browser (macOS)
open ingestion-flamegraph.svg
```

### Analyzing Flamegraphs

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Flamegraph Interpretation Guide                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  VISUAL STRUCTURE                                                           │
│  ├─ Width:         Time spent in function (wider = more time)              │
│  ├─ Height:        Stack depth (deeper = more nested calls)                │
│  ├─ Color:         Random (helps distinguish adjacent frames)              │
│  └─ Order:         Alphabetical within each level                          │
│                                                                             │
│  IDENTIFYING BOTTLENECKS                                                    │
│  ├─ Wide frames:    Functions consuming most CPU time                       │
│  ├─ Wide paths:     Hot call paths to optimize                             │
│  ├─ Self time:      Time in function excluding children (hover in SVG)    │
│  └─ Flat profile:   List of functions sorted by time                       │
│                                                                             │
│  COMMON PATTERNS                                                           │
│  ├─ "icache miss":  Frequent small function calls (consider inlining)      │
│  ├─ "memcpy":       Excessive copying (consider zero-copy)                 │
│  ├─ "alloc":        Frequent allocations (consider arena/ reuse)           │
│  ├─ "lock":         Lock contention (consider lock-free structures)        │
│  └─ "sys":          System call overhead (consider batching)               │
│                                                                             │
│  OPTIMIZATION PRIORITY                                                      │
│  1. Widest frames at top of hot paths                                     │
│  2. Functions with high self time                                         │
│  3. Functions called frequently from hot paths                            │
│  4. Functions in critical execution paths (ingestion, correlation)        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Differential Flamegraphs

Compare two flamegraphs to identify performance changes:

```bash
# Generate baseline flamegraph
cargo flamegraph --bin rustops-core --output baseline.svg

# Make code changes, then generate new flamegraph
cargo flamegraph --bin rustops-core --output optimized.svg

# Use Flamegraph differential tool
cargo install flamegraph-diff
flamegraph-diff baseline.svg optimized.svg > diff.svg

# Red = regression (slower), Blue = improvement (faster)
```

## 2. perf Profiling (Linux)

### Installation

```bash
# Linux perf tool (usually pre-installed)
sudo apt-get install linux-tools-common linux-tools-generic

# For specific kernel version
sudo apt-get install linux-tools-$(uname -r)
```

### Basic Usage

```bash
# Record CPU profile
perf record -F 997 -g -- cargo run --release --bin rustops-core

# Analyze recording
perf report

# Generate annotated call graph
perf report --graph --stdio

# Generate flamegraph from perf data
perf script | stackcollapse-perf.pl | flamegraph.pl > perf-flamegraph.svg
```

### Advanced perf Usage

```bash
# Profile specific PID
perf record -F 997 -g -p $(pidof rustops-core)

# Profile with sleep duration
perf record -F 997 -g -- sleep 60

# Show CPU cycles
perf stat -e cycles,instructions,cache-misses cargo run --release

# Show cache misses
perf stat -e L1-dcache-load-misses,L1-dcache-loads cargo run --release

# Show context switches
perf stat -e context-switches,cpu-migrations cargo run --release

# Profile memory access
perf mem record cargo run --release
perf mem report
```

### perf Examples

```bash
#!/bin/bash
# /plans/performance/scripts/profile-perf.sh

echo "Profiling with perf..."

# Build release binary
cargo build --release --bins

# Profile with detailed events
perf record \
  --call-graph dwarf \
  --event cycles,instructions,cache-references,cache-misses,branches,branch-misses \
  --freq 997 \
  --output perf.data \
  ./target/release/rustops-core --config production.toml &
PERF_PID=$!

# Let it run for 60 seconds
sleep 60

# Stop recording
kill -INT $PERF_PID
wait $PERF_PID

# Generate reports
echo "=== CPU Cycles ===" > perf-report.txt
perf report --input perf.data --sort overhead --stdio >> perf-report.txt

echo "=== Annotated Source ===" >> perf-report.txt
perf annotate --input perf.data --stdio >> perf-report.txt

echo "Report generated: perf-report.txt"
```

## 3. Memory Profiling

### valgrind / massif

```bash
# Install valgrind
sudo apt-get install valgrind

# Profile heap memory usage
valgrind \
  --tool=massif \
  --massif-out-file=massif.out \
  ./target/release/rustops-core --config test.toml

# Analyze results
ms_print massif.out > massif-report.txt

# Visualize massif output
massif-visualizer massif.out
```

### dhprof (Dynamic Heap Profiler)

```rust
// Cargo.toml
[dependencies]
dhprof = "0.1"

// src/main.rs
#[cfg(feature = "profile")]
use dhprof::Profile;

#[tokio::main]
async fn main() {
    #[cfg(feature = "profile")]
    let _profiler = Profile::new("my-profiling-session");

    // Application code
}
```

### Custom Allocation Tracking

```rust
// /plans/performance/code/profiling/allocation_tracker.rs

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Global allocation tracker
pub struct AllocationTracker {
    total_allocated: AtomicUsize,
    total_deallocated: AtomicUsize,
    allocation_count: AtomicUsize,
    deallocation_count: AtomicUsize,
    peak_usage: AtomicUsize,
}

impl AllocationTracker {
    pub const fn new() -> Self {
        Self {
            total_allocated: AtomicUsize::new(0),
            total_deallocated: AtomicUsize::new(0),
            allocation_count: AtomicUsize::new(0),
            deallocation_count: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
        }
    }

    pub fn snapshot(&self) -> AllocationSnapshot {
        let allocated = self.total_allocated.load(Ordering::Acquire);
        let deallocated = self.total_deallocated.load(Ordering::Acquire);
        let current_usage = allocated - deallocated;

        AllocationSnapshot {
            total_allocated: allocated,
            total_deallocated: deallocated,
            current_usage,
            peak_usage: self.peak_usage.load(Ordering::Acquire),
            allocation_count: self.allocation_count.load(Ordering::Acquire),
            deallocation_count: self.deallocation_count.load(Ordering::Acquire),
        }
    }
}

unsafe impl GlobalAlloc for AllocationTracker {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let ptr = System.alloc(layout);

        if !ptr.is_null() {
            self.total_allocated.fetch_add(size, Ordering::Release);
            self.allocation_count.fetch_add(1, Ordering::Release);

            // Update peak usage
            let allocated = self.total_allocated.load(Ordering::Acquire);
            let deallocated = self.total_deallocated.load(Ordering::Acquire);
            let current = allocated - deallocated;

            let mut peak = self.peak_usage.load(Ordering::Acquire);
            while current > peak {
                match self.peak_usage.compare_exchange_weak(
                    peak,
                    current,
                    Ordering::Release,
                    Ordering::Acquire,
                ) {
                    Ok(_) => break,
                    Err(new_peak) => peak = new_peak,
                }
            }
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size();
        System.dealloc(ptr, layout);

        self.total_deallocated.fetch_add(size, Ordering::Release);
        self.deallocation_count.fetch_add(1, Ordering::Release);
    }
}

#[derive(Debug, Clone)]
pub struct AllocationSnapshot {
    pub total_allocated: usize,
    pub total_deallocated: usize,
    pub current_usage: usize,
    pub peak_usage: usize,
    pub allocation_count: usize,
    pub deallocation_count: usize,
}

// Example usage
#[global_allocator]
static GLOBAL: AllocationTracker = AllocationTracker::new();

pub fn print_allocation_stats() {
    let snapshot = GLOBAL.snapshot();
    println!("=== Allocation Stats ===");
    println!("Current usage:     {} MB", snapshot.current_usage / 1_048_576);
    println!("Peak usage:        {} MB", snapshot.peak_usage / 1_048_576);
    println!("Total allocated:   {} MB", snapshot.total_allocated / 1_048_576);
    println!("Allocations:       {}", snapshot.allocation_count);
    println!("Deallocations:     {}", snapshot.deallocation_count);
}
```

## 4. Tracing and Instrumentation

### Structured Logging with tracing

```rust
// /plans/performance/code/profiling/tracing_instrumentation.rs

use tracing::{info, info_span, instrument, span, Level};
use tracing_subscriber;

/// Initialize tracing subscriber
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .init();
}

/// Instrument function for automatic tracing
#[instrument(skip(data))]
pub fn process_metrics(data: &[Metric]) -> ProcessResult {
    let span = span!(Level::INFO, "process_metrics", count = data.len());
    let _enter = span.enter();

    info!("Starting metric processing");

    // Process metrics...
    let result = ProcessResult { processed: data.len() };

    info!(processed = result.processed, "Completed metric processing");

    result
}

/// Create nested spans for detailed tracking
#[instrument]
pub async fn correlate_alert(alert: &Alert) -> CorrelationResult {
    let _span = info_span!("correlate_alert", alert_id = %alert.id).entered();

    info!("Starting correlation");

    // Step 1: Embed alert
    let embedding = embed_alert(alert).await;
    info!(embedding_dim = embedding.len(), "Alert embedded");

    // Step 2: Search patterns
    let _search_span = info_span!("search_patterns").entered();
    let patterns = search_patterns(&embedding).await?;
    info!(patterns_found = patterns.len(), "Patterns found");
    drop(_search_span);

    // Step 3: Correlate
    let result = if let Some(pattern) = patterns.first() {
        CorrelationResult::Correlated {
            pattern_id: pattern.id,
            confidence: pattern.confidence,
        }
    } else {
        CorrelationResult::NoCorrelation
    };

    info!(result = ?result, "Correlation complete");

    Ok(result)
}
```

### Metrics Export

```rust
// /plans/performance/code/profiling/metrics_export.rs

use prometheus::{Counter, Histogram, IntGauge, Registry};
use lazy_static::lazy_static;

lazy_static! {
    // Metric ingestion metrics
    static ref METRICS_INGESTED: Counter = register_counter!(
        "rustops_metrics_ingested_total",
        "Total number of metrics ingested"
    ).unwrap();

    static ref INGESTION_LATENCY: Histogram = register_histogram!(
        "rustops_ingestion_latency_seconds",
        "Metric ingestion latency"
    ).unwrap();

    // Alert correlation metrics
    static ref ALERTS_CORRELATED: Counter = register_counter!(
        "rustops_alerts_correlated_total",
        "Total number of alerts correlated"
    ).unwrap();

    static ref CORRELATION_LATENCY: Histogram = register_histogram!(
        "rustops_correlation_latency_seconds",
        "Alert correlation latency"
    ).unwrap();

    // ML inference metrics
    static ref INFERENCE_LATENCY: Histogram = register_histogram!(
        "rustops_inference_latency_seconds",
        "ML model inference latency"
    ).unwrap();

    static ref ANOMALIES_DETECTED: Counter = register_counter!(
        "rustops_anomalies_detected_total",
        "Total number of anomalies detected"
    ).unwrap();

    // Resource usage metrics
    static ref MEMORY_USAGE: IntGauge = register_int_gauge!(
        "rustops_memory_usage_bytes",
        "Current memory usage"
    ).unwrap();

    static ref CPU_USAGE: IntGauge = register_int_gauge!(
        "rustops_cpu_usage_percent",
        "Current CPU usage percentage"
    ).unwrap();
}

/// Record metric ingestion
pub fn record_metric_ingestion(count: u64, duration: Duration) {
    METRICS_INGESTED.inc_by(count);
    INGESTION_LATENCY.observe(duration.as_secs_f64());
}

/// Record alert correlation
pub fn record_alert_correlation(duration: Duration) {
    ALERTS_CORRELATED.inc();
    CORRELATION_LATENCY.observe(duration.as_secs_f64());
}

/// Record ML inference
pub fn record_inference(duration: Duration) {
    INFERENCE_LATENCY.observe(duration.as_secs_f64());
}

/// Record anomaly detection
pub fn record_anomaly_detected() {
    ANOMALIES_DETECTED.inc();
}

/// Update resource usage metrics
pub fn update_resource_metrics() {
    let memory = get_current_memory_usage();
    MEMORY_USAGE.set(memory as i64);

    let cpu = get_current_cpu_usage();
    CPU_USAGE.set(cpu as i64);
}

/// Export metrics for Prometheus scraping
pub fn export_metrics() -> String {
    use prometheus::Encoder;

    let encoder = TextEncoder::new();
    let metric_families = Registry::gather();
    let mut buffer = Vec::new();

    encoder.encode(&metric_families, &mut buffer).unwrap();

    String::from_utf8(buffer).unwrap()
}
```

## 5. tokio-console Profiling

### Installation

```bash
# Install tokio-console
cargo install tokio-console

# Add dependencies
cargo add console-subscriber
```

### Usage

```rust
// src/main.rs

#[tokio::main]
async fn main() {
    // Initialize console subscriber
    console_subscriber::init();

    // Your application code
    run_application().await;
}
```

```bash
# Run application with console instrumentation
RUSTFLAGS="--cfg tokio_unstable" cargo run --release

# In another terminal, run tokio-console
tokio-console
```

### What tokio-console Shows

- Task lifecycle (spawn, wake, poll)
- Task timing breakdown
- Resource contention
- Async operation latencies
- Poll frequencies

## 6. Custom Profiling Macros

```rust
// /plans/performance/code/profiling/custom_macros.rs

#[macro_export]
macro_rules! profile {
    ($name:expr, $block:block) => {{
        let start = Instant::now();
        let result = $block;
        let duration = start.elapsed();

        info!(
            target: "profile",
            name = $name,
            duration_ms = duration.as_millis(),
            duration_us = duration.as_micros(),
            "Profiled block"
        );

        result
    }};
}

#[macro_export]
macro_rules! profile_span {
    ($name:expr, $block:block) => {{
        let span = span!(Level::INFO, "profile", name = $name);
        let _enter = span.enter();

        let start = Instant::now();
        let result = $block;
        let duration = start.elapsed();

        info!(
            name = $name,
            duration_ms = duration.as_millis(),
            "Completed"
        );

        result
    }};
}

// Usage
#[tokio::main]
async fn main() {
    init_tracing();

    let result = profile!("metric_processing", {
        process_metrics(&metrics).await
    });

    let correlated = profile_span!("alert_correlation", {
        correlate_alert(&alert).await
    })?;
}
```

## 7. Continuous Profiling

### Setup for Production

```bash
#!/bin/bash
# /plans/performance/scripts/continuous-profiling.sh

PROFILE_DIR="/var/log/rustops/profiles"
RETENTION_DAYS=7

# Create profile directory
mkdir -p "$PROFILE_DIR"

echo "Starting continuous profiling..."

# Start perf recording in background
perf record \
  --freq 997 \
  --call-graph dwarf \
  -o "$PROFILE_DIR/perf.data" \
  -- sleep 3600 &

PERF_PID=$!

# Rotate profiles every hour
while true; do
    sleep 3600

    # Save current profile
    timestamp=$(date +%Y%m%d_%H%M%S)
    mv "$PROFILE_DIR/perf.data" "$PROFILE_DIR/perf_$timestamp.data"

    # Generate flamegraph
    perf script -i "$PROFILE_DIR/perf_$timestamp.data" | \
      stackcollapse-perf.pl | \
      flamegraph.pl > "$PROFILE_DIR/flamegraph_$timestamp.svg"

    # Clean up old profiles
    find "$PROFILE_DIR" -name "perf_*.data" -mtime +$RETENTION_DAYS -delete
    find "$PROFILE_DIR" -name "flamegraph_*.svg" -mtime +$RETENTION_DAYS -delete

    # Start new recording
    perf record \
      --freq 997 \
      --call-graph dwarf \
      -o "$PROFILE_DIR/perf.data" \
      -- sleep 3600 &

    PERF_PID=$!
done
```

## 8. Profiling Checklist

### Before Profiling

- [ ] Build with `--release` flag for optimized binary
- [ ] Ensure debug symbols are included (`-g` or no strip)
- [ ] Disable CPU frequency scaling (`cpupower frequency-set -g performance`)
- [ ] Close unnecessary applications
- [ ] Use representative workload

### During Profiling

- [ ] Profile for sufficient duration (60+ seconds)
- [ ] Use appropriate sampling frequency (997-9997 Hz)
- [ ] Capture multiple runs to account for variance
- [ ] Profile realistic workloads, not synthetic benchmarks

### After Profiling

- [ ] Identify top 5 hot functions by self time
- [ ] Look for unexpected functions in hot paths
- [ ] Check for excessive allocations/deallocations
- [ ] Verify lock contention is minimal
- [ ] Document findings and optimization priorities

---

**Last Updated:** 2025-01-18
**Next:** Implement regression prevention strategy
