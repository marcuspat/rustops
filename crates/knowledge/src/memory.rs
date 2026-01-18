// Unified memory service implementing ADR-006
//
// Provides a unified interface for storing and retrieving knowledge
// across all bounded contexts with semantic and structured search

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::hnsw::HNSWIndexer;
use super::repository::MemoryRepository;
use super::embeddings::EmbeddingModel;

/// Unified memory service
pub struct UnifiedMemoryService {
    /// Repository for persistence
    repository: Arc<dyn MemoryRepository>,

    /// HNSW indexer for semantic search
    hnsw: Arc<RwLock<HNSWIndexer>>,

    /// Embedding model
    embedding_model: Arc<EmbeddingModel>,

    /// LRU cache for hot data
    cache: Arc<moka::future::Cache<String, MemoryEntry>>,

    /// Configuration
    config: MemoryConfig,
}

impl UnifiedMemoryService {
    /// Create new memory service
    pub async fn new(config: MemoryConfig) -> Result<Self> {
        // Initialize repository
        let repository = config.repository_factory.create().await?;

        // Initialize HNSW indexer
        let hnsw = Arc::new(RwLock::new(
            HNSWIndexer::new(config.embedding_dimensions).await?
        ));

        // Initialize embedding model
        let embedding_model = Arc::new(
            EmbeddingModel::new(config.embedding_config).await?
        );

        // Initialize cache
        let cache = Arc::new(
            moka::future::CacheBuilder::new(config.cache_max_items)
                .time_to_live(config.cache_ttl)
                .build()
        );

        Ok(Self {
            repository: Arc::new(repository),
            hnsw,
            embedding_model,
            cache,
            config,
        })
    }

    /// Store a memory entry
    pub async fn store(&self, mut entry: MemoryEntry) -> Result<String> {
        // Generate embedding if not present
        if entry.embedding.is_none() {
            let embedding = self.embedding_model.embed(&entry.content).await?;
            entry.embedding = Some(embedding);
        }

        // Store in repository
        self.repository.store(&entry).await?;

        // Index in HNSW
        if let Some(embedding) = &entry.embedding {
            let mut hnsw = self.hnsw.write().await;
            hnsw.index(&entry.id, embedding).await?;
        }

        // Cache if frequently accessed
        if entry.temporal.access_count > 10 {
            self.cache.insert(entry.id.clone(), entry).await;
        }

        Ok(entry.id)
    }

    /// Query memory
    pub async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let results = match query.query_type {
            QueryType::Semantic => self.semantic_search(query).await?,
            QueryType::Structured => self.structured_query(query).await?,
            QueryType::Hybrid => self.hybrid_query(query).await?,
        };

        Ok(results)
    }

    /// Semantic search using HNSW
    async fn semantic_search(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let content = query.content.ok_or_else(|| anyhow::anyhow!("Missing content for semantic search"))?;

        // Generate query embedding
        let query_embedding = self.embedding_model.embed(&content).await?;

        // Search HNSW index
        let hnsw = self.hnsw.read().await;
        let search_results = hnsw.search(
            &query_embedding,
            query.limit,
            query.threshold.unwrap_or(0.7),
        ).await?;

        drop(hnsw);

        // Retrieve full entries
        let mut results = Vec::new();
        for result in search_results {
            if let Some(entry) = self.repository.retrieve(&result.id).await? {
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Structured query using repository
    async fn structured_query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        self.repository.query(query).await
    }

    /// Hybrid query combining semantic and structured
    async fn hybrid_query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        // Run both in parallel
        let (semantic_results, structured_results) = tokio::try_join!(
            self.semantic_search(query.clone()),
            self.structured_query(query.clone())
        )?;

        // Combine and deduplicate
        let mut combined = semantic_results;
        for entry in structured_results {
            if !combined.iter().any(|e| e.id == entry.id) {
                combined.push(entry);
            }
        }

        // Re-rank by combined score
        combined.sort_by(|a, b| {
            let score_a = self.calculate_combined_score(a, &query);
            let score_b = self.calculate_combined_score(b, &query);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        combined.truncate(query.limit);

        Ok(combined)
    }

    /// Retrieve by ID
    pub async fn retrieve_by_id(&self, id: &str) -> Result<Option<MemoryEntry>> {
        // Check cache first
        if let Some(entry) = self.cache.get(id).await {
            return Ok(Some(entry));
        }

        // Retrieve from repository
        let entry = self.repository.retrieve(id).await?;

        // Cache if found
        if let Some(ref entry) = entry {
            self.cache.insert(id.to_string(), entry.clone()).await;
        }

        Ok(entry)
    }

    /// Calculate combined score for hybrid query
    fn calculate_combined_score(&self, entry: &MemoryEntry, query: &MemoryQuery) -> f32 {
        let mut score = 0.0;

        // Metadata matching
        if let Some(namespace) = &query.namespace {
            if &entry.namespace == namespace {
                score += 0.3;
            }
        }

        // Recency boost
        let age_days = (Utc::now() - entry.temporal.created_at).num_days();
        if age_days < 7 {
            score += 0.2;
        }

        // Access frequency boost
        if entry.temporal.access_count > 10 {
            score += 0.2;
        }

        // Verdict boost
        if let Some(learning) = &entry.learning {
            if matches!(learning.verdict, Verdict::Success) {
                score += 0.3;
            }
        }

        score
    }

    /// Decay access for memory consolidation
    pub async fn decay_access(&self, id: &str, decay_rate: f32) -> Result<()> {
        let mut entry = self.retrieve_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Entry not found: {}", id))?;

        // Apply decay
        entry.temporal.access_count = (entry.temporal.access_count as f32 * (1.0 - decay_rate)) as u64;

        // Store back
        self.store(entry).await?;

        Ok(())
    }

    /// Archive old entry
    pub async fn archive(&self, id: &str) -> Result<()> {
        // Remove from cache
        self.cache.invalidate(id).await;

        // Archive in repository
        self.repository.archive(id).await?;

        Ok(())
    }
}

/// Memory configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Embedding dimensions
    pub embedding_dimensions: usize,

    /// Cache max items
    pub cache_max_items: u64,

    /// Cache TTL
    pub cache_ttl: std::time::Duration,

    /// Repository factory
    pub repository_factory: Arc<dyn RepositoryFactory>,

    /// Embedding configuration
    pub embedding_config: EmbeddingConfig,
}

/// Repository factory trait
#[async_trait]
pub trait RepositoryFactory: Send + Sync {
    async fn create(&self) -> Result<Box<dyn MemoryRepository>>;
}

/// Memory query types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    Semantic,
    Structured,
    Hybrid,
}

/// Memory query
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    pub query_type: QueryType,
    pub namespace: Option<MemoryNamespace>,
    pub content: Option<String>,
    pub filters: Vec<QueryFilter>,
    pub limit: usize,
    pub threshold: Option<f32>,
}

/// Query filters
#[derive(Debug, Clone)]
pub enum QueryFilter {
    Service(String),
    Environment(String),
    TimeRange(TimeRange),
    MinSimilarity(f32),
    OlderThan(std::time::Duration),
}

/// Time range filter
#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

// =============================================================================
// Memory Entry Types
// =============================================================================

/// Memory namespaces for data isolation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryNamespace {
    Incidents,
    Patterns,
    Topology,
    Anomalies,
    Runbooks,
    Feedback,
    ConfigChanges,
}

/// Memory entry types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryEntryType {
    IncidentRecord,
    Pattern,
    Anomaly,
    TopologySnapshot,
    Runbook,
    Feedback,
    ConfigChange,
}

/// Unified memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub namespace: MemoryNamespace,
    pub entry_type: MemoryEntryType,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: MemoryMetadata,
    pub temporal: TemporalData,
    pub learning: Option<LearningData>,
}

/// Structured metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub service: Option<String>,
    pub environment: Option<String>,
    pub severity: Option<SeverityLevel>,
    pub related_incidents: Vec<String>,
    pub tags: Vec<String>,
    pub source: DataSource,
}

/// Temporal data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalData {
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u64,
    pub ttl: Option<u64>,
    pub decay_rate: f32,
}

/// Learning data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningData {
    pub sona_mode: SonaMode,
    pub reward: f32,
    pub trajectory_id: Option<String>,
    pub verdict: Verdict,
    pub consolidated: bool,
}

/// Severity level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd)]
pub enum SeverityLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Data source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSource {
    Agent { agent_type: String, agent_id: String },
    Human { user_id: String },
    System { component: String },
}

/// SONA modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SonaMode {
    RealTime,
    Balanced,
    Research,
    Edge,
    Batch,
}

/// Verdict
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Verdict {
    Success,
    Partial,
    Failure,
    Unknown,
}

/// Embedding configuration
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub model_name: String,
    pub model_path: Option<String>,
    pub device: ort::ExecutionProviderDispatch,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_name: "all-MiniLM-L6-v2".to_string(),
            model_path: None,
            device: ort::ExecutionProviderDispatch::CPU,
        }
    }
}
