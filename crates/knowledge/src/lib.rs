//! # RustOps Knowledge (experimental)
//!
//! Building blocks for storing and searching operational knowledge:
//!
//! - **HNSW vector index** ([`hnsw::HNSWIndexer`]) — approximate
//!   nearest-neighbor search (cosine distance, via `hnsw_rs`) over
//!   **caller-supplied** embedding vectors
//! - **Pattern extraction** ([`patterns::PatternExtractor`]) — derive
//!   reusable remediation patterns from resolved incidents
//! - **Runbook storage** ([`runbooks::InMemoryRunbookStorage`]) — store and
//!   query automation procedures (in-memory)
//!
//! ## What this crate is not (yet)
//!
//! There is **no embedding model** here: nothing in this crate turns text
//! into vectors. Bring your own embeddings (from an external model or
//! service) and this crate will index and search them. There is also no
//! persistence — indexes and runbooks live in memory.
//!
//! This crate is **experimental** and not wired into the RustOps pipeline.

pub mod hnsw;
pub mod patterns;
pub mod runbooks;

pub use hnsw::{HNSWIndexer, HNSWStats, SearchResult};
pub use patterns::PatternExtractor;
pub use runbooks::{InMemoryRunbookStorage, Runbook, RunbookQuery, RunbookStep};
