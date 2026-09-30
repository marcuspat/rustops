//! # RustOps
//!
//! Intelligent AIOps platform for automated monitoring, anomaly detection, and
//! incident remediation.
//!
//! This crate is the top-level package published to crates.io for the
//! `rustops` Cargo workspace. It does not itself contain application logic;
//! the functionality lives in the workspace member crates:
//!
//! - `rustops-common` — shared IDs, error types, domain events
//! - `rustops-telemetry` — Prometheus log/metric normalizers and collectors
//! - `rustops-anomaly` — Z-score, IQR, and CUSUM anomaly detectors
//! - `rustops-incident` — event-sourced incident store and alert correlation
//! - `rustops-integration` — Prometheus scrape client, circuit breaker, retry
//! - `rustops-topology` — service topology graph
//! - `rustops-api` — Axum-based HTTP API server (binary: `rustops-api`)
//! - `rustops-agent` — telemetry collection agent (binary)
//! - `rustops-pipeline` — anomaly-to-incident processing pipeline (binary)
//!
//! See the [project README](https://github.com/marcuspat/rustops) for usage
//! and architecture details.
