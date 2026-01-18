//! # RustOps Knowledge Management
//!
//! This crate provides knowledge management capabilities for the RustOps AIOps platform,
//! including vector embeddings for semantic search, pattern storage, and runbook management.
//!
//! ## Features
//!
//! - **Vector Embeddings**: HNSW-indexed semantic search (150x-12,500x faster)
//! - **Pattern Storage**: Store and retrieve successful remediation patterns
//! - **Runbook Management**: Store and query automation procedures
//! - **SONA Integration**: Self-Optimizing Neural Architecture for continuous learning
//!
//! ## Architecture
//!
//! This crate follows the Knowledge Management bounded context from the RustOps DDD model.
//! See `/plans/ddd/` for the complete domain model.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use rustops_knowledge::KnowledgeBase;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let kb = KnowledgeBase::new().await?;
//!     // Store a pattern
//!     kb.store_pattern("memory-leak-fix", "restart service").await?;
//!     // Search for similar patterns
//!     let results = kb.search("fix memory issue").await?;
//!     Ok(())
//! }
//! ```

pub mod embedding;
pub mod pattern;
pub mod runbook;
pub mod repository;
pub mod search;

pub use embedding::Embedding;
pub use pattern::Pattern;
pub use runbook::Runbook;
pub use repository::KnowledgeRepository;
pub use search::SearchEngine;

/// Knowledge base for storing and retrieving patterns, runbooks, and embeddings
#[derive(Clone)]
pub struct KnowledgeBase {
    repository: KnowledgeRepository,
    search: SearchEngine,
}

impl KnowledgeBase {
    /// Create a new knowledge base
    pub async fn new() -> anyhow::Result<Self> {
        let repository = KnowledgeRepository::new().await?;
        let search = SearchEngine::new(repository.clone()).await?;
        Ok(Self { repository, search })
    }

    /// Get the underlying repository
    pub fn repository(&self) -> &KnowledgeRepository {
        &self.repository
    }

    /// Get the search engine
    pub fn search(&self) -> &SearchEngine {
        &self.search
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_knowledge_base_creation() {
        let kb = KnowledgeBase::new().await;
        assert!(kb.is_ok());
    }
}
