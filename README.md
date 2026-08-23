# RustOps

**An AIOps toolkit in Rust: statistical anomaly detection, incident correlation, service topology, and (experimental) automated remediation.**

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Main Branch](https://github.com/marcuspat/rustops/actions/workflows/main.yml/badge.svg)](https://github.com/marcuspat/rustops/actions/workflows/main.yml)
[![RustOps Test Suite](https://github.com/marcuspat/rustops/actions/workflows/test.yml/badge.svg)](https://github.com/marcuspat/rustops/actions/workflows/test.yml)

---

## What this is — and what it is not

RustOps is a working set of building blocks for AIOps, not a finished platform. This README describes what the code actually does today. Where a capability is stubbed or experimental, it says so.

**Implemented and tested:**

- **Statistical anomaly detection** — Z-score with a leave-one-out baseline (an outlier cannot mask itself by inflating its own baseline) and IQR outlier detection. No ML: the ONNX model path is an explicit stub that returns an error until the `ort` integration is finished.
- **Incident management** — alert correlation, similarity-based deduplication, and event-sourced incident records (in memory).
- **Service topology** — a petgraph-based dependency graph with upstream/downstream traversal, blast-radius calculation, shortest-path (A*) impact routes, and impact analysis with severity scoring. Storage is **in-memory**; the Neo4j store is an unimplemented stub that refuses to connect rather than pretending.
- **Integrations** — a real Prometheus adapter (`/api/v1/query_range` over HTTP, tested hermetically against a mock server), a Kubernetes adapter built on `kube` (its live tests are `#[ignore]`d without a cluster), and a ServiceNow adapter. Resilience primitives: circuit breaker (Closed → Open → HalfOpen with reset timeout), token-bucket rate limiter, retry with exponential backoff.
- **Telemetry** — Prometheus text-format normalization into typed metrics; collectors hand off to a Kafka producer **stub** (no real broker I/O yet).

**Experimental (compiled and tested, not wired into the pipeline):**

- **`knowledge`** — HNSW approximate nearest-neighbor search (`hnsw_rs`, cosine distance) over *caller-supplied* vectors, pattern extraction from resolved incidents, and in-memory runbook storage. There is **no embedding model** in this repo; bring your own vectors.
- **`remediation`** — workflow engine (concurrency limit, per-workflow timeouts), safety machinery (circuit breaker, blast-radius limits, approval gates, rollback strategies), and a **simulated** activity executor. Nothing here touches a real cluster or cloud account.

**Not implemented:** Kafka ingestion, Neo4j persistence, ML inference, CUSUM/seasonal detection, PostgreSQL/Redis/Temporal — none of these are behind the binaries today, whatever older docs claimed.

---

## Quick start

Prerequisites: Rust 1.85+.

```bash
git clone https://github.com/marcuspat/rustops.git
cd rustops

cargo build --workspace
cargo test --workspace          # all suites green, no external services needed
cargo clippy --workspace --all-targets -- -D warnings
```

### Binaries

```bash
# API server (axum skeleton: /health, /metrics placeholder, /api/v1)
RUST_LOG=info cargo run --bin rustops-api

# Telemetry agent: scrapes a real Prometheus at PROMETHEUS_URL on an interval
RUST_LOG=info cargo run --bin rustops-agent

# Pipeline: synthetic in-process source -> normalize -> detect -> topology
# (no Kafka consumer yet; the synthetic source exists so the real path runs)
RUST_LOG=info cargo run --bin rustops-pipeline
```

---

## Workspace layout

```
crates/
├── common/        # IDs, domain events, error types, Metric primitives
├── telemetry/     # Prometheus text-format normalization, collectors (Kafka stub)
├── anomaly/       # Z-score + IQR detectors, detector router (ONNX stubbed)
├── incident/      # Correlation, deduplication, event-sourced incidents
├── integration/   # Prometheus / Kubernetes / ServiceNow adapters + resilience
├── topology/      # In-memory service graph, blast radius, impact analysis
├── api/           # axum API skeleton
├── agent/         # Prometheus scrape loop -> collector
├── pipeline/      # Synthetic source -> normalize -> detect -> topology
├── knowledge/     # EXPERIMENTAL: HNSW vector search, patterns, runbooks
└── remediation/   # EXPERIMENTAL: workflow engine + simulated executor
```

| Crate | Status | Key exports |
|-------|--------|-------------|
| `common` | stable | typed IDs, `DomainEvent`, `Metric`, `Error` |
| `telemetry` | working; Kafka stubbed | `TelemetryNormalizer`, `MetricsCollector` |
| `anomaly` | working (statistical only) | `ZScoreDetector`, `IQRDetector`, `DetectionRouter` |
| `incident` | working (in-memory) | `AlertCorrelator`, `AlertDeduplicator` |
| `integration` | working | `PrometheusAdapter`, `KubernetesAdapter`, `CircuitBreaker`, `RateLimiter` |
| `topology` | working (in-memory; Neo4j stub errors) | `ServiceGraph`, `ImpactAnalyzer` |
| `api` | skeleton | health endpoint |
| `agent` | working against a live Prometheus | scrape loop |
| `pipeline` | working with synthetic source | normalize → detect → topology |
| `knowledge` | experimental | `HNSWIndexer`, `PatternExtractor`, runbooks |
| `remediation` | experimental, simulated | `WorkflowEngine`, `SafetyCheck`, rollback |

---

## Example

```rust
use rustops_anomaly::{AnomalyDetector, ZScoreDetector};

let detector = ZScoreDetector::new(2.0);
let result = detector.detect(&metrics).await?;   // needs >= 8 samples per metric name

for anomaly in result.anomalies {
    println!("{:?} score={:.2}", anomaly.anomaly_type, anomaly.score);
}
```

The detector requires at least 8 samples of a metric name in a batch before it scores anything — below that the baseline is too small to mean much.

---

## Configuration

The topology service takes its optional Prometheus endpoint from `TopologyConfig::prometheus_url` (empty = no Prometheus-backed discovery). The agent reads `PROMETHEUS_URL` and a scrape interval. The pipeline's Kafka settings are placeholders for the future consumer and are logged as such.

---

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings   # CI-enforced, zero warnings
cargo test --workspace
cargo bench -p rustops-common                            # criterion benches for domain events
```

CI runs build, tests, lint, and security scanning via the workflows in `.github/workflows/`.

---

## License

Apache-2.0 — see [LICENSE](LICENSE).
