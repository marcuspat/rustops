# Changelog

All notable changes to RustOps will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] — rescue pass

Workspace-wide honesty and correctness pass: every crate now compiles, all
test suites pass with no external services, and `cargo clippy --workspace
--all-targets -- -D warnings` is clean.

### Added
- `knowledge` and `remediation` crates joined the workspace as **experimental**
  members (compiled and tested, not wired into the pipeline).
- `knowledge`: HNSW indexer rewritten against the real `hnsw_rs` 0.3 API
  (cosine distance, caller-supplied vectors); honest crate docs stating there
  is no embedding model or persistence.
- `remediation`: `SimulatedActivityExecutor` replaces the fake K8s/AWS
  executors; workflow engine now enforces `max_concurrent_actions` and bounds
  every workflow with `default_workflow_timeout_secs`.
- Pipeline binary drives the real normalize → detect → topology path from a
  synthetic in-process Prometheus text-format source (documented: no Kafka
  consumer yet); sliding-window baseline so the Z-score detector actually
  fires on spikes.
- Prometheus adapter: real `/api/v1/query_range` wire types, response status
  validation, hermetic wiremock integration tests.
- Criterion benchmark target properly wired for `rustops-common`
  (`event_bench`).

### Changed
- Z-score detector uses a leave-one-out baseline (an outlier can no longer
  mask itself) with an explicit `MIN_SAMPLES = 8` floor.
- Circuit breaker implements the full Closed → Open → HalfOpen cycle with
  reset-timeout-driven half-open transitions; failure streak resets on
  success while closed; a half-open failure re-opens immediately.
- Topology blast radius traverses **incoming** edges (things that depend on
  the failed service), upstream traversal uses `Reversed`, and impact paths
  use A*; `ImpactAnalyzer` refreshes its graph snapshot per analysis.
- `InMemoryEventStore` clones share storage via `Arc` instead of
  deep-copying.
- Agent binary ports to the real Prometheus adapter, emits real sampled
  values, and performs graceful shutdown (run loop stopped via oneshot, then
  adapter shutdown).
- README rewritten to describe what the code actually does; crate-level docs
  updated to state stubs explicitly (ONNX/ML disabled, Kafka producer stub,
  Neo4j store refuses to connect instead of pretending).
- Workspace manifest: `resolver = "2"` set correctly; bogus `[default-bin]`
  section removed.
- `serde_json` gains the `float_roundtrip` feature so metric values survive
  JSON round-trips bit-exactly.

### Removed
- Fiction files in `knowledge` (4 modules describing unimplemented ML
  features), dead `integration/src/prometheus.rs` module and 2 dead examples,
  `common/src/testing/` and the `metric_bench` benchmark that depended on it,
  topology CLI module and dead discovery helpers, unused `kafka` dependency
  in the pipeline crate.
- Cloud-provider dependencies from `remediation` (nothing used them).

### Fixed
- `UpsertResource`-style state bugs, retry test `Arc<AtomicU32>` race, rate
  limiter exposing its configuration, ServiceNow base64 `Engine` usage,
  incident dedup `Default` placement, ~240 missing doc comments, and the
  property-test suite rewritten against the real `rustops_common::Metric`
  API.

## [0.1.0] - TBD

### Added
- Initial release of RustOps AIOps platform
