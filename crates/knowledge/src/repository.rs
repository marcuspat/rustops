// Memory repository implementations
//
// Provides persistence layer for memory entries using SQLite

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite};
use tracing::{debug, error};

use super::memory::{MemoryEntry, MemoryQuery, MemoryNamespace, QueryFilter};

/// Memory repository trait
#[async_trait]
pub trait MemoryRepository: Send + Sync {
    /// Store memory entry
    async fn store(&self, entry: &MemoryEntry) -> Result<()>;

    /// Retrieve by ID
    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>>;

    /// Query memory entries
    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>>;

    /// Archive entry
    async fn archive(&self, id: &str) -> Result<()>;
}

/// SQLite repository implementation
pub struct SqliteRepository {
    pool: Pool<Sqlite>,
}

impl SqliteRepository {
    /// Create new SQLite repository
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = Pool::connect(database_url).await?;

        // Run migrations
        sqlx::query(include_str!("schema.sql"))
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    /// Parse row to memory entry
    fn parse_row(&self, row: sqlx::sqlite::SqliteRow) -> Result<MemoryEntry> {
        // Simplified parsing
        Ok(MemoryEntry {
            id: row.get("id"),
            namespace: serde_json::from_str(row.get("namespace"))?,
            entry_type: serde_json::from_str(row.get("entry_type"))?,
            content: row.get("content"),
            embedding: row.try_get::<_, Option<Vec<f32>>>("embedding").ok().flatten(),
            metadata: serde_json::from_str(row.get("metadata"))?,
            temporal: serde_json::from_str(row.get("temporal"))?,
            learning: row.try_get::<_, Option<String>>("learning").ok()
                .and_then(|s| serde_json::from_str(&s).ok()),
        })
    }
}

#[async_trait]
impl MemoryRepository for SqliteRepository {
    async fn store(&self, entry: &MemoryEntry) -> Result<()> {
        let query = r#"
            INSERT OR REPLACE INTO memory_entries (
                id, namespace, entry_type, content, embedding,
                metadata, temporal, learning, archived
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        let embedding_json = entry.embedding.as_ref()
            .map(|e| serde_json::to_string(e))
            .transpose()?;

        sqlx::query(query)
            .bind(&entry.id)
            .bind(serde_json::to_string(&entry.namespace)?)
            .bind(serde_json::to_string(&entry.entry_type)?)
            .bind(&entry.content)
            .bind(embedding_json)
            .bind(serde_json::to_string(&entry.metadata)?)
            .bind(serde_json::to_string(&entry.temporal)?)
            .bind(entry.learning.as_ref().map(|l| serde_json::to_string(l)).transpose()?)
            .bind(false)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let row = sqlx::query("SELECT * FROM memory_entries WHERE id = ? AND archived = FALSE")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(r) => Ok(Some(self.parse_row(r)?)),
            None => Ok(None),
        }
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let mut sql = "SELECT * FROM memory_entries WHERE archived = FALSE".to_string();
        let mut count = 0;

        // Namespace filter
        if let Some(ns) = &query.namespace {
            sql.push_str(&format!(" AND namespace = '{}'", serde_json::to_string(ns)?));
            count += 1;
        }

        // Apply filters
        for filter in &query.filters {
            match filter {
                QueryFilter::Service(s) => {
                    sql.push_str(&format!(" AND json_extract(metadata, '$.service') = '{}'", s));
                }
                QueryFilter::Environment(e) => {
                    sql.push_str(&format!(" AND json_extract(metadata, '$.environment') = '{}'", e));
                }
                _ => {}
            }
        }

        sql.push_str(&format!(" LIMIT {}", query.limit));

        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(self.parse_row(row)?);
        }

        Ok(entries)
    }

    async fn archive(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE memory_entries SET archived = TRUE WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

// =============================================================================
// Schema
// =============================================================================

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS memory_entries (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    entry_type TEXT NOT NULL,
    content TEXT NOT NULL,
    embedding TEXT,
    metadata TEXT NOT NULL,
    temporal TEXT NOT NULL,
    learning TEXT,
    archived BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_namespace ON memory_entries(namespace);
CREATE INDEX IF NOT EXISTS idx_archived ON memory_entries(archived);
"#;
