// Runbook storage and retrieval
//
// Manages storage and retrieval of runbook procedures

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Runbook storage trait
#[async_trait]
pub trait RunbookStorage: Send + Sync {
    /// Store runbook
    async fn store(&self, runbook: &Runbook) -> Result<()>;

    /// Retrieve by ID
    async fn retrieve(&self, id: &str) -> Result<Option<Runbook>>;

    /// Query runbooks
    async fn query(&self, query: RunbookQuery) -> Result<Vec<Runbook>>;
}

/// Runbook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runbook {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: RunbookCategory,
    pub severity: SeverityLevel,
    pub steps: Vec<RunbookStep>,
    pub estimated_duration: std::time::Duration,
    pub prerequisites: Vec<String>,
    pub tags: Vec<String>,
    pub success_rate: Option<f32>,
    pub last_used: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u32,
}

/// Runbook step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookStep {
    pub id: String,
    pub step_number: usize,
    pub title: String,
    pub description: String,
    pub action_type: RunbookActionType,
    pub parameters: HashMap<String, String>,
    pub expected_outcome: String,
    pub rollback_action: Option<String>,
}

/// Runbook categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunbookCategory {
    IncidentResponse,
    Maintenance,
    Deployment,
    Scaling,
    Debugging,
    Recovery,
}

/// Runbook action types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunbookActionType {
    RestartService,
    ScaleDeployment,
    ClearCache,
    ExecuteScript,
    UpdateConfig,
    RollbackDeployment,
    NotifyTeam,
    CustomCommand,
}

/// Runbook query
#[derive(Debug, Clone)]
pub struct RunbookQuery {
    pub category: Option<RunbookCategory>,
    pub severity: Option<SeverityLevel>,
    pub min_success_rate: Option<f32>,
    pub tags: Vec<String>,
    pub search_text: Option<String>,
    pub limit: usize,
}

/// Severity level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd)]
pub enum SeverityLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// In-memory runbook storage (for testing)
pub struct InMemoryRunbookStorage {
    runbooks: tokio::sync::RwLock<HashMap<String, Runbook>>,
}

impl InMemoryRunbookStorage {
    /// Create new in-memory storage
    pub fn new() -> Self {
        Self {
            runbooks: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl RunbookStorage for InMemoryRunbookStorage {
    async fn store(&self, runbook: &Runbook) -> Result<()> {
        let mut runbooks = self.runbooks.write().await;
        runbooks.insert(runbook.id.clone(), runbook.clone());
        Ok(())
    }

    async fn retrieve(&self, id: &str) -> Result<Option<Runbook>> {
        let runbooks = self.runbooks.read().await;
        Ok(runbooks.get(id).cloned())
    }

    async fn query(&self, query: RunbookQuery) -> Result<Vec<Runbook>> {
        let runbooks = self.runbooks.read().await;
        let mut results: Vec<Runbook> = runbooks.values().cloned().collect();

        // Apply filters
        if let Some(category) = &query.category {
            results.retain(|r| &r.category == category);
        }
        if let Some(severity) = &query.severity {
            results.retain(|r| r.severity == *severity);
        }
        if let Some(min_rate) = query.min_success_rate {
            results.retain(|r| r.success_rate.map_or(false, |sr| sr >= min_rate));
        }
        if !query.tags.is_empty() {
            results.retain(|r| query.tags.iter().all(|t| r.tags.contains(t)));
        }
        if let Some(text) = &query.search_text {
            let text_lower = text.to_lowercase();
            results.retain(|r| {
                r.title.to_lowercase().contains(&text_lower)
                    || r.description.to_lowercase().contains(&text_lower)
            });
        }

        // Apply limit
        results.truncate(query.limit);

        Ok(results)
    }
}

impl Default for InMemoryRunbookStorage {
    fn default() -> Self {
        Self::new()
    }
}
