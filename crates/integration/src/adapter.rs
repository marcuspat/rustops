// Adapter pattern for unified integration interface
//
// Provides a consistent interface across all external system integrations

use crate::resilience::{HealthStatus, IntegrationResult};
use crate::{CircuitBreaker, CircuitBreakerConfig, RateLimiter, RateLimiterConfig, RetryConfig};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Base trait for all integration adapters
#[async_trait]
pub trait IntegrationAdapter: Send + Sync {
    /// Integration identifier
    fn id(&self) -> &str;

    /// Integration type classification
    fn kind(&self) -> IntegrationKind;

    /// Health check for external system
    async fn health_check(&self) -> IntegrationResult<HealthStatus>;

    /// Initialize connection (with reconnection support)
    async fn initialize(&mut self) -> IntegrationResult<()>;

    /// Shutdown gracefully
    async fn shutdown(&mut self) -> IntegrationResult<()>;
}

/// Telemetry collector interface
#[async_trait]
pub trait TelemetryCollector: IntegrationAdapter {
    /// Metric query
    async fn collect_metrics(&self, query: MetricQuery) -> IntegrationResult<Vec<Metric>>;

    /// Collect logs from external system
    async fn collect_logs(&self, query: LogQuery) -> IntegrationResult<LogStream>;

    /// Collect traces from external system
    async fn collect_traces(&self, query: TraceQuery) -> IntegrationResult<Vec<Trace>>;

    /// Subscribe to real-time telemetry updates
    async fn subscribe(&self) -> IntegrationResult<mpsc::Receiver<TelemetryEvent>>;
}

/// ITSM notifier interface
#[async_trait]
pub trait ITSMNotifier: IntegrationAdapter {
    /// Create or update incident
    async fn create_incident(&self, incident: Incident) -> IntegrationResult<String>;

    /// Update incident status
    async fn update_incident(&self, id: &str, update: IncidentUpdate) -> IntegrationResult<()>;

    /// Query incident details
    async fn get_incident(&self, id: &str) -> IntegrationResult<Incident>;

    /// Sync with CMDB
    async fn sync_cmdb(&self) -> IntegrationResult<CMDBSyncResult>;
}

/// Infrastructure monitor interface
#[async_trait]
pub trait InfrastructureMonitor: IntegrationAdapter {
    /// List monitored resources
    async fn list_resources(&self, filters: ResourceFilter) -> IntegrationResult<Vec<Resource>>;

    /// Get resource metrics
    async fn get_resource_metrics(&self, id: &str) -> IntegrationResult<ResourceMetrics>;

    /// Watch for resource changes (streaming)
    async fn watch_resources(&self) -> IntegrationResult<mpsc::Receiver<ResourceEvent>>;

    /// Execute infrastructure action
    async fn execute_action(&self, action: InfraAction) -> IntegrationResult<ActionResult>;
}

// IntegrationKind is now defined in lib.rs to avoid duplication
// Re-export here for convenience
pub use crate::IntegrationKind;

// =============================================================================
// Data Types
// =============================================================================

/// Metric query
#[derive(Debug, Clone)]
pub struct MetricQuery {
    /// Pub.
    pub metric_name: String,
    /// Pub.
    pub labels: HashMap<String, String>,
    /// Pub.
    pub start_time: DateTime<Utc>,
    /// Pub.
    pub end_time: DateTime<Utc>,
    /// Pub.
    pub step: Option<u64>, // Step in seconds
}

/// Metric data point
#[derive(Debug, Clone)]
pub struct Metric {
    /// Pub.
    pub name: String,
    /// Pub.
    pub labels: HashMap<String, String>,
    /// Pub.
    pub value: f64,
    /// Pub.
    pub timestamp: DateTime<Utc>,
}

/// Log query
#[derive(Debug, Clone)]
pub struct LogQuery {
    /// Pub.
    pub query: String,
    /// Pub.
    pub start_time: DateTime<Utc>,
    /// Pub.
    pub end_time: DateTime<Utc>,
    /// Pub.
    pub limit: usize,
}

/// Log stream
#[derive(Debug, Clone)]
pub struct LogStream {
    /// Pub.
    pub entries: Vec<LogEntry>,
    /// Pub.
    pub has_more: bool,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Pub.
    pub timestamp: DateTime<Utc>,
    /// Pub.
    pub level: String,
    /// Pub.
    pub message: String,
    /// Pub.
    pub metadata: HashMap<String, String>,
}

/// Trace query
#[derive(Debug, Clone)]
pub struct TraceQuery {
    /// Pub.
    pub trace_id: Option<String>,
    /// Pub.
    pub start_time: DateTime<Utc>,
    /// Pub.
    pub end_time: DateTime<Utc>,
    /// Pub.
    pub min_duration: Option<u64>,
    /// Pub.
    pub limit: usize,
}

/// Trace
#[derive(Debug, Clone)]
pub struct Trace {
    /// Pub.
    pub id: String,
    /// Pub.
    pub root_span_name: String,
    /// Pub.
    pub duration_ms: u64,
    /// Pub.
    pub start_time: DateTime<Utc>,
    /// Pub.
    pub spans: Vec<Span>,
}

/// Span
#[derive(Debug, Clone)]
pub struct Span {
    /// Pub.
    pub span_id: String,
    /// Pub.
    pub parent_span_id: Option<String>,
    /// Pub.
    pub operation: String,
    /// Pub.
    pub start_time: DateTime<Utc>,
    /// Pub.
    pub duration_ms: u64,
    /// Pub.
    pub tags: HashMap<String, String>,
}

/// Telemetry event
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    /// Metric.
    Metric(Metric),
    /// Log.
    Log(LogEntry),
    /// Trace.
    Trace(Trace),
}

/// Incident
#[derive(Debug, Clone)]
pub struct Incident {
    /// Pub.
    pub id: Option<String>,
    /// Pub.
    pub title: String,
    /// Pub.
    pub description: String,
    /// Pub.
    pub severity: IncidentSeverity,
    /// Pub.
    pub status: IncidentStatus,
    /// Pub.
    pub assigned_to: Option<String>,
    /// Pub.
    pub created_at: DateTime<Utc>,
    /// Pub.
    pub updated_at: Option<DateTime<Utc>>,
    /// Pub.
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Incident severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentSeverity {
    /// P1.
    P1, // Critical
    /// P2.
    P2, // High
    /// P3.
    P3, // Medium
    /// P4.
    P4, // Low
}

/// Incident status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentStatus {
    /// New.
    New,
    /// Assigned.
    Assigned,
    /// InProgress.
    InProgress,
    /// Resolved.
    Resolved,
    /// Closed.
    Closed,
}

/// Incident update
#[derive(Debug, Clone)]
pub struct IncidentUpdate {
    /// Pub.
    pub status: Option<IncidentStatus>,
    /// Pub.
    pub severity: Option<IncidentSeverity>,
    /// Pub.
    pub description: Option<String>,
    /// Pub.
    pub assigned_to: Option<String>,
    /// Pub.
    pub resolution: Option<String>,
}

/// CMDB sync result
#[derive(Debug, Clone)]
pub struct CMDBSyncResult {
    /// Pub.
    pub items_synced: usize,
    /// Pub.
    pub items_updated: usize,
    /// Pub.
    pub items_created: usize,
    /// Pub.
    pub items_failed: usize,
    /// Pub.
    pub errors: Vec<String>,
}

/// Resource filter
#[derive(Debug, Clone)]
pub struct ResourceFilter {
    /// Pub.
    pub resource_type: Option<String>,
    /// Pub.
    pub labels: HashMap<String, String>,
    /// Pub.
    pub namespace: Option<String>,
}

/// Resource
#[derive(Debug, Clone)]
pub struct Resource {
    /// Pub.
    pub id: String,
    /// Pub.
    pub name: String,
    /// Pub.
    pub resource_type: String,
    /// Pub.
    pub namespace: Option<String>,
    /// Pub.
    pub labels: HashMap<String, String>,
    /// Pub.
    pub status: String,
}

/// Resource metrics
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    /// Pub.
    pub resource_id: String,
    /// Pub.
    pub cpu_percent: f64,
    /// Pub.
    pub memory_percent: f64,
    /// Pub.
    pub custom_metrics: HashMap<String, f64>,
    /// Pub.
    pub timestamp: DateTime<Utc>,
}

/// Resource event
#[derive(Debug, Clone)]
pub struct ResourceEvent {
    /// Pub.
    pub event_type: ResourceEventType,
    /// Pub.
    pub resource: Resource,
    /// Pub.
    pub timestamp: DateTime<Utc>,
}

/// Resource event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceEventType {
    /// Added.
    Added,
    /// Modified.
    Modified,
    /// Deleted.
    Deleted,
}

/// Infrastructure action
#[derive(Debug, Clone)]
pub struct InfraAction {
    /// Pub.
    pub action_type: String,
    /// Pub.
    pub resource_id: String,
    /// Pub.
    pub parameters: HashMap<String, String>,
}

/// Action result
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// Pub.
    pub success: bool,
    /// Pub.
    pub message: String,
    /// Pub.
    pub output: Option<String>,
    /// Pub.
    pub error: Option<String>,
}

// =============================================================================
// Base Adapter Implementation
// =============================================================================

/// Base adapter with common functionality
#[derive(Clone)]
pub struct BaseAdapter {
    id: String,
    kind: IntegrationKind,
    circuit_breaker: Arc<CircuitBreaker>,
    rate_limiter: Arc<RateLimiter>,
    retry_config: RetryConfig,
}

impl BaseAdapter {
    /// Create new base adapter
    pub fn new(
        id: impl Into<String>,
        kind: IntegrationKind,
        circuit_breaker_config: CircuitBreakerConfig,
        rate_limiter_config: RateLimiterConfig,
        retry_config: RetryConfig,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            circuit_breaker: Arc::new(CircuitBreaker::new(circuit_breaker_config)),
            rate_limiter: Arc::new(RateLimiter::new(rate_limiter_config)),
            retry_config,
        }
    }

    /// Execute with resilience (circuit breaker + rate limit + retry)
    pub async fn execute_with_resilience<F, Fut, T, E>(&self, operation: F) -> IntegrationResult<T>
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, E>> + Send,
        E: std::fmt::Display + Send + 'static,
    {
        // Check rate limit
        self.rate_limiter.acquire().await?;

        // Execute with circuit breaker and retry
        self.circuit_breaker
            .call(crate::retry::retry_with_backoff(
                self.retry_config.clone(),
                operation,
            ))
            .await
    }

    /// Get circuit breaker reference
    pub fn circuit_breaker(&self) -> &CircuitBreaker {
        &self.circuit_breaker
    }

    /// Get rate limiter reference
    pub fn rate_limiter(&self) -> &RateLimiter {
        &self.rate_limiter
    }
}

#[async_trait]
impl IntegrationAdapter for BaseAdapter {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> IntegrationKind {
        self.kind
    }

    async fn health_check(&self) -> IntegrationResult<HealthStatus> {
        if self.circuit_breaker.is_open().await {
            return Ok(HealthStatus::Unhealthy);
        }
        Ok(HealthStatus::Healthy)
    }

    async fn initialize(&mut self) -> IntegrationResult<()> {
        Ok(())
    }

    async fn shutdown(&mut self) -> IntegrationResult<()> {
        Ok(())
    }
}
