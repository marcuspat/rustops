# Memory Implementation Guide

**Code Examples and Integration Patterns for RustOps Memory System**

## Overview

This guide provides concrete code examples for implementing the RustOps memory architecture using AgentDB, HNSW indexing, and SONA integration in Rust.

---

## Project Structure

```
rustops/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── memory/
│   │   ├── mod.rs
│   │   ├── service.rs           # UnifiedMemoryService
│   │   ├── agentdb.rs           # AgentDB adapter
│   │   ├── hnsw.rs              # HNSW indexer
│   │   ├── sona.rs              # SONA integration
│   │   ├── hooks.rs             # Hooks integration
│   │   ├── workers.rs           # Background workers
│   │   └── namespaces/
│   │       ├── mod.rs
│   │       ├── incidents.rs
│   │       ├── patterns.rs
│   │       ├── topology.rs
│   │       ├── anomalies.rs
│   │       └── runbooks.rs
│   ├── learning/
│   │   ├── mod.rs
│   │   ├── patterns.rs          # Pattern extraction
│   │   ├── anomaly.rs           # Anomaly learning
│   │   ├── topology.rs          # Topology learning
│   │   └── feedback.rs          # Feedback integration
│   └── api/
│       ├── memory.rs            # Memory API endpoints
│       └── hooks.rs             # Hooks API endpoints
└── plans/
    └── memory/
        ├── ARCHITECTURE.md
        ├── NAMESPACES.md
        ├── LEARNING.md
        ├── INTEGRATION.md
        └── IMPLEMENTATION.md
```

---

## Cargo.toml Dependencies

```toml
[package]
name = "rustops"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Time
chrono = { version = "0.4", features = ["serde"] }

# UUID/ULID
ulid = "1.1"
uuid = { version = "1.6", features = ["v4", "serde"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "chrono", "json"] }

# Vector operations
ndarray = "0.15"

# HNSW indexing
hnsw-rs = "0.1"

# ONNX Runtime for embeddings
ort = { version = "2.0", features = ["fetch-models"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# HTTP server
axum = "0.7"

# Configuration
config = "0.13"

# Caching
moka = { version = "0.12", features = ["future"] }

# Metrics
prometheus = "0.13"

# Redis for distributed caching
fred = "8.0"

[dev-dependencies]
tokio-test = "0.4"
mockall = "0.12"
```

---

## Core Memory Service

### Unified Memory Service

```rust
// src/memory/service.rs

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::agentdb::AgentDBAdapter;
use super::hnsw::HNSWIndexer;
use super::namespaces::*;
use super::sona::SONAIntegration;

/// Unified memory service implementing ADR-006
pub struct UnifiedMemoryService {
    /// AgentDB adapter
    agentdb: Arc<AgentDBAdapter>,

    /// HNSW indexer for semantic search
    hnsw: Arc<RwLock<HNSWIndexer>>,

    /// SONA integration
    sona: Arc<SONAIntegration>,

    /// LRU cache for hot data
    cache: Arc<moka::future::Cache<String, MemoryEntry>>,

    /// Configuration
    config: MemoryConfig,
}

impl UnifiedMemoryService {
    /// Create new memory service
    pub async fn new(config: MemoryConfig) -> Result<Self> {
        // Initialize AgentDB adapter
        let agentdb = Arc::new(AgentDBAdapter::new(&config.agentdb_url).await?);

        // Initialize HNSW indexer
        let hnsw = Arc::new(RwLock::new(
            HNSWIndexer::new(config.embedding_dimensions).await?
        ));

        // Initialize SONA
        let sona = Arc::new(SONAIntegration::new(&config.sona_config).await?);

        // Initialize cache
        let cache = Arc::new(
            moka::future::CacheBuilder::new(config.cache_max_items)
                .time_to_live(config.cache_ttl)
                .build()
        );

        Ok(Self {
            agentdb,
            hnsw,
            sona,
            cache,
            config,
        })
    }

    /// Store a memory entry
    pub async fn store(&self, mut entry: MemoryEntry) -> Result<String> {
        // Generate embedding if not present
        if entry.embedding.is_none() {
            let hnsw = self.hnsw.read().await;
            entry.embedding = Some(hnsw.embed_content(&entry.content).await?);
            drop(hnsw);
        }

        // Store in AgentDB
        self.agentdb.store(&entry).await?;

        // Index in HNSW
        if let Some(embedding) = &entry.embedding {
            let mut hnsw = self.hnsw.write().await;
            hnsw.index(&entry.id, embedding).await?;
        }

        // Cache if frequently accessed
        if entry.temporal.access_count > 10 {
            self.cache.insert(entry.id.clone(), entry).await;
        }

        // Track metric
        metrics::counter!("memory_store_total", "namespace" => entry.namespace.as_str());
        metrics::histogram!("memory_store_size_bytes", entry.content.len() as f64);

        Ok(entry.id)
    }

    /// Query memory
    pub async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let start = std::time::Instant::now();

        // Check cache first
        if let Some(cache_key) = query.cache_key() {
            if let Some(cached) = self.cache.get(&cache_key).await {
                return Ok(vec![cached]);
            }
        }

        let results = match query.query_type {
            QueryType::Semantic => self.semantic_search(query).await?,
            QueryType::Structured => self.structured_query(query).await?,
            QueryType::Hybrid => self.hybrid_query(query).await?,
        };

        // Update cache for frequently accessed
        for entry in &results {
            self.cache.increment_access(&entry.id).await;
        }

        // Track metrics
        let elapsed = start.elapsed();
        metrics::histogram!("memory_query_duration_ms", elapsed.as_millis() as f64,
            "query_type" => query.query_type.as_str());
        metrics::counter!("memory_query_total");

        Ok(results)
    }

    /// Semantic search using HNSW
    async fn semantic_search(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let content = query.content.ok_or_else(|| anyhow!("Missing content for semantic search"))?;

        let hnsw = self.hnsw.read().await;

        // Generate query embedding
        let query_embedding = hnsw.embed_content(&content).await?;

        // Search HNSW index
        let search_results = hnsw.search(
            &query_embedding,
            query.limit,
            query.threshold.unwrap_or(0.7),
        ).await?;

        drop(hnsw);

        // Retrieve full entries
        let mut results = Vec::new();
        for result in search_results {
            if let Some(entry) = self.retrieve_by_id(&result.id).await? {
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Structured query using SQL
    async fn structured_query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        self.agentdb.query(query).await
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

        // Retrieve from AgentDB
        let entry = self.agentdb.retrieve(id).await?;

        // Cache if found
        if let Some(ref entry) = entry {
            self.cache.insert(id.to_string(), entry.clone()).await;
        }

        Ok(entry)
    }

    /// Calculate combined score for hybrid query
    fn calculate_combined_score(&self, entry: &MemoryEntry, query: &MemoryQuery) -> f32 {
        let mut score = 0.0;

        // Semantic similarity (if available)
        if let (Some(query_content), Some(entry_embedding)) = (&query.content, &entry.embedding) {
            // This would be computed during search
            score += 0.6; // Placeholder
        }

        // Metadata matching
        if let Some(namespace) = &query.namespace {
            if &entry.namespace == namespace {
                score += 0.2;
            }
        }

        // Recency boost
        let age_days = (Utc::now() - entry.temporal.created_at).num_days();
        if age_days < 7 {
            score += 0.1;
        }

        // Access frequency boost
        if entry.temporal.access_count > 10 {
            score += 0.1;
        }

        score
    }

    /// Decay access for memory consolidation
    pub async fn decay_access(&self, id: &str, decay_rate: f32) -> Result<()> {
        let mut entry = self.retrieve_by_id(id)
            .await?
            .ok_or_else(|| anyhow!("Entry not found: {}", id))?;

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

        // Move to archive storage
        self.agentdb.archive(id).await?;

        metrics::counter!("memory_archive_total");

        Ok(())
    }
}

/// Memory configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub agentdb_url: String,
    pub embedding_dimensions: usize,
    pub cache_max_items: u64,
    pub cache_ttl: std::time::Duration,
    pub sona_config: SonaConfig,
}

/// Memory query types
#[derive(Debug, Clone, Copy)]
pub enum QueryType {
    Semantic,
    Structured,
    Hybrid,
}

impl QueryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Semantic => "semantic",
            Self::Structured => "structured",
            Self::Hybrid => "hybrid",
        }
    }
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

impl MemoryQuery {
    pub fn cache_key(&self) -> Option<String> {
        // Only cache structured queries with specific filters
        if self.query_type == QueryType::Structured && self.content.is_none() {
            Some(format!("{:?}", self))
        } else {
            None
        }
    }
}
```

---

## AgentDB Adapter

```rust
// src/memory/agentdb.rs

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;

use super::service::*;

/// AgentDB adapter
pub struct AgentDBAdapter {
    pool: Pool<Sqlite>,
}

impl AgentDBAdapter {
    /// Create new AgentDB adapter
    pub async fn new(url: &str) -> Result<Self> {
        let pool = Pool::connect(url).await?;

        // Run migrations
        sqlx::query(include_str!("schema.sql"))
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    /// Store memory entry
    pub async fn store(&self, entry: &MemoryEntry) -> Result<()> {
        let query = r#"
            INSERT INTO memory_entries (
                id, namespace, entry_type, content, embedding,
                service, environment, severity, related_incidents, tags, source,
                created_at, last_accessed, access_count, ttl, decay_rate,
                sona_mode, reward, verdict, consolidated
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let embedding_json = entry.embedding.as_ref()
            .map(|e| serde_json::to_string(e))
            .transpose()?;

        sqlx::query(query)
            .bind(&entry.id)
            .bind(entry.namespace.as_str())
            .bind(format!("{:?}", entry.entry_type))
            .bind(&entry.content)
            .bind(embedding_json)
            .bind(entry.metadata.service.as_deref())
            .bind(entry.metadata.environment.as_ref().map(|e| e.as_str()))
            .bind(entry.metadata.severity.as_ref().map(|s| format!("{:?}", s)))
            .bind(serde_json::to_string(&entry.metadata.related_incidents)?)
            .bind(serde_json::to_string(&entry.metadata.tags)?)
            .bind(format!("{:?}", entry.metadata.source))
            .bind(entry.temporal.created_at)
            .bind(entry.temporal.last_accessed)
            .bind(entry.temporal.access_count as i64)
            .bind(entry.temporal.ttl.map(|t| t as i64))
            .bind(entry.temporal.decay_rate)
            .bind(entry.learning.as_ref().map(|l| format!("{:?}", l.sona_mode)))
            .bind(entry.learning.as_ref().map(|l| l.reward as f64))
            .bind(entry.learning.as_ref().map(|l| format!("{:?}", l.verdict)))
            .bind(entry.learning.as_ref().map(|l| l.consolidated))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Query memory entries
    pub async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let mut sql = "SELECT * FROM memory_entries WHERE 1=1".to_string();
        let mut params = Vec::new();

        // Namespace filter
        if let Some(ns) = &query.namespace {
            sql.push_str(" AND namespace = ?");
            params.push(ns.as_str().to_string());
        }

        // Apply filters
        for filter in &query.filters {
            match filter {
                QueryFilter::Service(s) => {
                    sql.push_str(" AND service = ?");
                    params.push(s.clone());
                }
                QueryFilter::TimeRange(range) => {
                    sql.push_str(" AND created_at >= ? AND created_at <= ?");
                    params.push(range.start.to_rfc3339());
                    params.push(range.end.to_rfc3339());
                }
                // ... other filters
                _ => {}
            }
        }

        sql.push_str(" LIMIT ?");
        params.push(query.limit.to_string());

        // Execute query
        let mut query_builder = sqlx::query(&sql);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        // Parse results
        let mut entries = Vec::new();
        for row in rows {
            entries.push(self.parse_row(row)?);
        }

        Ok(entries)
    }

    /// Retrieve by ID
    pub async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let row = sqlx::query("SELECT * FROM memory_entries WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(r) => Ok(Some(self.parse_row(r)?)),
            None => Ok(None),
        }
    }

    /// Archive entry
    pub async fn archive(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE memory_entries SET archived = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Parse database row to MemoryEntry
    fn parse_row(&self, row: sqlx::sqlite::SqliteRow) -> Result<MemoryEntry> {
        // Implementation would parse all fields from row
        // This is simplified
        Ok(MemoryEntry {
            id: row.get("id"),
            namespace: MemoryNamespace::from_str(row.get("namespace"))?,
            entry_type: MemoryEntryType::from_str(row.get("entry_type"))?,
            content: row.get("content"),
            embedding: None, // Would parse JSON
            metadata: MemoryMetadata {
                service: row.try_get("service").ok(),
                environment: None, // Would parse
                severity: None,   // Would parse
                related_incidents: vec![],
                tags: vec![],
                source: DataSource::System { component: "system".to_string() },
            },
            temporal: TemporalData {
                created_at: row.get("created_at"),
                last_accessed: row.get("last_accessed"),
                access_count: row.get("access_count"),
                ttl: row.try_get("ttl").ok(),
                decay_rate: row.get("decay_rate"),
            },
            learning: None, // Would parse
        })
    }
}
```

---

## HNSW Indexer

```rust
// src/memory/hnsw.rs

use anyhow::Result;
use hnsw_rs::Hnsw;
use ndarray::Array1;
use ort::{Environment, ExecutionProvider, Session};

/// HNSW indexer for fast semantic search
pub struct HNSWIndexer {
    /// HNSW index
    index: Hnsw<f32,DistCosine>,

    /// ONNX session for embeddings
    session: Session,

    /// Dimensions
    dimensions: usize,
}

/// Cosine distance for HNSW
#[derive(Debug, Clone)]
struct DistCosine;

impl hnsw_rs::Distance<f32> for DistCosine {
    fn distance(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        1.0 - (dot_product / (norm_a * norm_b))
    }
}

impl HNSWIndexer {
    /// Create new HNSW indexer
    pub async fn new(dimensions: usize) -> Result<Self> {
        // Create ONNX environment
        let environment = Environment::builder()
            .with_execution_providers([ExecutionProvider::CPU])
            .build()?;

        // Download/load model
        let session = Session::builder(&environment)?
            .with_model_downloaded("https://github.com/nmslib/hnswlib/raw/master/model.onnx")
            .await?;

        // Create HNSW index
        let hnsw = Hnsw::new(dimensions, 16, 32, DistCosine, false);

        Ok(Self {
            index: hnsw,
            session,
            dimensions,
        })
    }

    /// Index a memory entry
    pub async fn index(&mut self, id: &str, embedding: &[f32]) -> Result<()> {
        self.index.insert(embedding.to_vec(), id);
        Ok(())
    }

    /// Search for similar entries
    pub async fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<SearchResult>> {
        let results = self
            .index
            .search(query_embedding, limit, std::usize::MAX)
            .into_iter()
            .filter_map(|(id, distance)| {
                let similarity = 1.0 - distance;
                if similarity >= threshold {
                    Some(SearchResult { id, similarity })
                } else {
                    None
                }
            })
            .collect();

        Ok(results)
    }

    /// Generate embedding for content
    pub async fn embed_content(&self, content: &str) -> Result<Vec<f32>> {
        // Tokenize content (simplified)
        let tokens = self.tokenize(content);

        // Run ONNX model
        let input = ort::Value::from_array(
            self.session.allocator(),
            &tokens.into(),
        )?;

        let outputs = self.session.run(vec![input]).await?;
        let embedding = outputs[0].try_extract::<Array2<f32>>()?;

        // Average pooling
        let averaged = embedding.mean_axis(ndarray::Axis(1))?;

        Ok(avgregated.to_vec())
    }

    /// Tokenize content
    fn tokenize(&self, content: &str) -> Vec<i64> {
        // Simplified tokenization
        // In production, use proper tokenizer
        content
            .split_whitespace()
            .take(512)
            .map(|s| s.chars().take(4).count() as i64)
            .collect()
    }
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub similarity: f32,
}
```

---

## SONA Integration

```rust
// src/memory/sona.rs

use anyhow::Result;
use std::collections::HashMap;

use super::service::*;

/// SONA integration
pub struct SONAIntegration {
    /// LoRA fine-tuning model
    lora_model: Option<LoRAModel>,

    /// EWC++ consolidation
    ewc: EWCPlus,

    /// Trajectory tracker
    tracker: TrajectoryTracker,
}

impl SONAIntegration {
    /// Create new SONA integration
    pub async fn new(config: &SonaConfig) -> Result<Self> {
        Ok(Self {
            lora_model: if config.enable_lora {
                Some(LoRAModel::new(config).await?)
            } else {
                None
            },
            ewc: EWCPlus::new(config.lambda),
            tracker: TrajectoryTracker::new(),
        })
    }

    /// Train on trajectory
    pub async fn train_trajectory(&self, trajectory: &ResolutionTrajectory) -> Result<TrainingResult> {
        // Start tracking
        let traj_id = self.tracker.start(trajectory.clone()).await?;

        // Record steps
        for step in &trajectory.steps {
            self.tracker.record_step(
                &traj_id,
                &step.action,
                &step.result,
                step.quality,
            ).await?;
        }

        // Judge verdict
        let verdict = self.judge_verdict(trajectory)?;

        // Apply LoRA if available
        if let Some(lora) = &self.lora_model {
            lora.fine_tune(trajectory).await?;
        }

        // Consolidate if successful
        if matches!(verdict, Verdict::Success) {
            self.ewc.consolidate(trajectory, traj_id).await?;
        }

        Ok(TrainingResult {
            trajectory_id: traj_id,
            verdict,
            reward: self.calculate_reward(trajectory),
        })
    }

    /// Judge verdict for trajectory
    fn judge_verdict(&self, trajectory: &ResolutionTrajectory) -> Result<Verdict> {
        let time_to_resolve = trajectory.duration().as_secs() as f32;
        let success_rate = trajectory.success_rate();
        let user_feedback = trajectory.user_feedback.unwrap_or(0.5);

        if success_rate > 0.9 && time_to_resolve < 300.0 && user_feedback > 0.8 {
            Ok(Verdict::Success)
        } else if success_rate > 0.6 {
            Ok(Verdict::Partial)
        } else {
            Ok(Verdict::Failure)
        }
    }

    /// Calculate reward for trajectory
    fn calculate_reward(&self, trajectory: &ResolutionTrajectory) -> f32 {
        let time_reward = if trajectory.duration().as_secs() < 300 { 0.3 } else { 0.1 };
        let success_reward = trajectory.success_rate() * 0.5;
        let feedback_reward = trajectory.user_feedback.unwrap_or(0.5) * 0.2;

        time_reward + success_reward + feedback_reward
    }

    /// Consolidate recent trajectories
    pub async fn consolidate_recent_trajectories(&self, days: u64) -> Result<()> {
        let threshold = Utc::now() - chrono::Duration::days(days as i64);
        self.ewc.consolidate_recent(threshold).await
    }
}

/// LoRA model for fine-tuning
pub struct LoRAModel {
    // Implementation
}

impl LoRAModel {
    pub async fn new(config: &SonaConfig) -> Result<Self> {
        // Initialize LoRA model
        Ok(Self {})
    }

    pub async fn fine_tune(&self, trajectory: &ResolutionTrajectory) -> Result<()> {
        // Fine-tune on trajectory
        Ok(())
    }
}

/// EWC++ consolidation
pub struct EWCPlus {
    lambda: f32,
    fisher_matrix: HashMap<String, ndarray::Array2<f32>>,
    consolidated_params: HashMap<String, ndarray::Array1<f32>>,
}

impl EWCPlus {
    pub fn new(lambda: f32) -> Self {
        Self {
            lambda,
            fisher_matrix: HashMap::new(),
            consolidated_params: HashMap::new(),
        }
    }

    pub async fn consolidate(&mut self, trajectory: &ResolutionTrajectory, traj_id: String) -> Result<()> {
        // Compute Fisher information and consolidate
        Ok(())
    }

    pub async fn consolidate_recent(&mut self, threshold: DateTime<Utc>) -> Result<()> {
        // Consolidate recent successful trajectories
        Ok(())
    }
}

/// Trajectory tracker
pub struct TrajectoryTracker {
    trajectories: HashMap<String, ResolutionTrajectory>,
}

impl TrajectoryTracker {
    pub fn new() -> Self {
        Self {
            trajectories: HashMap::new(),
        }
    }

    pub async fn start(&mut self, trajectory: ResolutionTrajectory) -> Result<String> {
        let id = ulid::new().to_string();
        self.trajectories.insert(id.clone(), trajectory);
        Ok(id)
    }

    pub async fn record_step(&mut self, traj_id: &str, action: &str, result: &str, quality: f32) -> Result<()> {
        if let Some(traj) = self.trajectories.get_mut(traj_id) {
            traj.steps.push(TrajectoryStep {
                action: action.to_string(),
                context: String::new(),
                result: result.to_string(),
                quality,
            });
        }
        Ok(())
    }
}

/// Training result
pub struct TrainingResult {
    pub trajectory_id: String,
    pub verdict: Verdict,
    pub reward: f32,
}

/// SONA configuration
#[derive(Debug, Clone)]
pub struct SonaConfig {
    pub enable_lora: bool,
    pub lambda: f32,
}
```

---

## API Endpoints

```rust
// src/api/memory.rs

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::memory::service::*;

/// Memory API state
#[derive(Clone)]
pub struct MemoryState {
    pub memory: Arc<UnifiedMemoryService>,
}

/// Store memory entry
#[axum::debug_handler]
pub async fn store_entry(
    State(state): State<MemoryState>,
    Json(entry): Json<MemoryEntry>,
) -> Result<impl IntoResponse, ApiError> {
    let id = state.memory.store(entry).await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

/// Query memory
#[axum::debug_handler]
pub async fn query_memory(
    State(state): State<MemoryState>,
    Query(params): Query<QueryParams>,
) -> Result<impl IntoResponse, ApiError> {
    let query = MemoryQuery {
        query_type: params.query_type.unwrap_or(QueryType::Semantic),
        namespace: params.namespace.and_then(|n| MemoryNamespace::from_str(&n).ok()),
        content: params.content,
        filters: vec![],
        limit: params.limit.unwrap_or(10),
        threshold: params.threshold,
    };

    let results = state.memory.query(query).await?;
    Ok(Json(results))
}

/// Get entry by ID
pub async fn get_entry(
    State(state): State<MemoryState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let entry = state.memory.retrieve_by_id(&id).await?
        .ok_or_else(|| ApiError::NotFound)?;

    Ok(Json(entry))
}

/// Query params
#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub query_type: Option<QueryType>,
    pub namespace: Option<String>,
    pub content: Option<String>,
    pub limit: Option<usize>,
    pub threshold: Option<f32>,
}

/// API error
#[derive(Debug)]
pub enum ApiError {
    NotFound,
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "Not found"),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, &msg),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
```

---

## Example Usage

### Main Application

```rust
// src/main.rs

use anyhow::Result;
use rustops_memory::UnifiedMemoryService;
use rustops_memory::namespaces::Incident;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("rustops=debug,info")
        .init();

    // Initialize memory service
    let memory_config = rustops_memory::MemoryConfig {
        agentdb_url: "sqlite:./data/memory.db".to_string(),
        embedding_dimensions: 384,
        cache_max_items: 10_000,
        cache_ttl: std::time::Duration::from_secs(3600),
        sona_config: rustops_memory::SonaConfig {
            enable_lora: true,
            lambda: 5000.0,
        },
    };

    let memory = UnifiedMemoryService::new(memory_config).await?;

    // Example: Store an incident
    let incident = Incident {
        id: ulid::new().to_string(),
        title: "High error rate on payment service".to_string(),
        description: "Payment service experiencing 50% error rate".to_string(),
        service: "payment-service".to_string(),
        environment: Environment::Production,
        severity: SeverityLevel::High,
        detected_at: Utc::now(),
        resolved_at: None,
        resolution_steps: vec![],
        // ... other fields
    };

    // Store incident in memory
    memory.store(incident.to_memory_entry()?).await?;

    // Example: Query for similar incidents
    let similar = memory.query(MemoryQuery {
        query_type: QueryType::Semantic,
        namespace: Some(MemoryNamespace::Incidents),
        content: Some("High error rate".to_string()),
        filters: vec![],
        limit: 5,
        threshold: Some(0.8),
    }).await?;

    println!("Found {} similar incidents", similar.len());

    Ok(())
}
```

---

## Summary

| Component | Lines of Code | Key Features |
|-----------|---------------|--------------|
| UnifiedMemoryService | ~400 | Store, query, cache, HNSW integration |
| AgentDBAdapter | ~300 | SQLite storage, query execution |
| HNSWIndexer | ~200 | Vector indexing, semantic search |
| SONAIntegration | ~400 | LoRA, EWC++, trajectory tracking |
| Namespaces | ~2000 | Data models for each namespace |
| API Endpoints | ~300 | REST API for memory operations |
| **Total** | ~3600 | Complete memory system |

---

**Document Version**: 1.0
**Last Updated**: 2026-01-18
**Author**: Implementation Team
