# RustOps

**AIOps toolkit for anomaly detection, incident management, and telemetry collection — built in Rust.**

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

---

## Overview

RustOps is a Cargo-workspace AIOps toolkit written in Rust. It provides real-time anomaly detection (Z-score, IQR, CUSUM), alert correlation with deduplication, an in-memory event-sourced incident store, and a Prometheus telemetry pipeline with log/metric normalizers.

This is an **early-stage project**. The core detection, incident, and telemetry crates are functional. The topology, knowledge, and remediation crates contain data models and interfaces but their primary integrations (Neo4j, ONNX, Temporal) are not yet wired up.

---

## What Works

### Anomaly Detection (`crates/anomaly`)
- Z-score spike/drop detector
- IQR outlier detector
- CUSUM cumulative-change detector
- Router that dispatches to multiple detectors by metric name
- Unit and property tests

### Incident Management (`crates/incident`)
- In-memory event-sourced incident repository
- Alert correlation (time-window + topology grouping)
- Deduplication via similarity scoring
- CQRS-style event store

### Telemetry (`crates/telemetry`)
- Prometheus log and metric normalizers (parses real Prometheus text formats)
- Collector registry with metric aggregation
- Batch processing pipeline

### Common (`crates/common`)
- Shared IDs, error types, domain events, telemetry primitives
- Criterion benchmarks for metric creation and serialization

### Integration (`crates/integration`)
- Prometheus scrape client (functional)
- Circuit breaker, retry with exponential backoff, rate limiter
- Kubernetes adapter scaffold (reads config, pod list stubbed)
- ServiceNow adapter scaffold (struct definitions only)

---

## What's Scaffolded (Not Yet Functional)

| Crate | Status | Notes |
|-------|--------|-------|
| `topology` | Data models + HNSW index | Neo4j store is stubbed (returns empty). Impact analysis returns hardcoded values. |
| `knowledge` | Data models + HNSW | Embeddings use a placeholder tokenizer. Vector search exists but isn't production-viable. |
| `remediation` | Policy engine + safety checks | Workflow executor is a skeleton. No real remediation actions implemented. |
| `api` | Axum health-check server | Single `/health` endpoint. No real API routes. |
| `pipeline` | Binary entry point | Heartbeat loop only. Does not wire detection to incident creation. |
| `agent` | Binary entry point | Prometheus scrape loop. Telemetry producer is a no-op stub. |

---

## Quick Start

```bash
git clone https://github.com/marcuspat/rustops.git
cd rustops

# Build the workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Start the API server (health checks only)
RUST_LOG=info cargo run --bin rustops-api
# Available at http://localhost:8080/health
```

---

## Project Structure

```
rustops/
├── Cargo.toml              # Workspace
├── crates/
│   ├── common/              # Shared types, IDs, events, errors
│   ├── telemetry/           # Prometheus normalizers, collector
│   ├── anomaly/             # ZScore, IQR, CUSUM detectors
│   ├── incident/            # Correlation, dedup, event store
│   ├── integration/         # Prometheus client, circuit breaker
│   ├── topology/            # Service graph (scaffolded)
│   ├── knowledge/           # HNSW, embeddings (scaffolded)
│   └── remediation/         # Policy engine (scaffolded)
├── crates/api/              # Axum server (health only)
├── crates/pipeline/         # Pipeline binary (heartbeat)
├── crates/agent/            # Agent binary (prometheus scrape)
└── docs/                     # Manual
```

---

## Example: Anomaly Detection

```rust
use rustops_anomaly::statistical::ZScoreDetector;
use rustops_anomaly::detector::AnomalyDetector;
use rustops_common::metrics::Metric;

let detector = ZScoreDetector::new(2.0);
let metrics = vec![
    Metric::new("cpu_usage", 45.0),
    Metric::new("cpu_usage", 47.0),
    Metric::new("cpu_usage", 98.0),  // spike
];
let result = detector.detect(&metrics).await?;

for anomaly in result.anomalies {
    println!("Anomaly: {:?} confidence={}", anomaly.anomaly_type, anomaly.confidence);
}
```

---

## Configuration

Copy `config.yaml.example` to `config.yaml` and adjust values. The default configuration runs in-memory with no external dependencies.

---

## Testing

```bash
# Unit tests
cargo test --workspace

# Property-based tests (proptest)
cargo test --workspace -p rustops-common -- property_tests
```

---

## License

Apache 2.0 — see [LICENSE](LICENSE).
