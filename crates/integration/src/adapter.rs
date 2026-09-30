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
    /// Metric name to query
    pub metric_name: String,
    /// Label selectors for the query
    pub labels: HashMap<String, String>,
    /// Query start time
    pub start_time: DateTime<Utc>,
    /// Query end time
    pub end_time: DateTime<Utc>,
    /// Step in seconds
    pub step: Option<u64>,
}

/// Metric data point
#[derive(Debug, Clone)]
pub struct Metric {
    /// Metric name
    pub name: String,
    /// Metric labels
    pub labels: HashMap<String, String>,
    /// Metric value
    pub value: f64,
    /// Sample timestamp
    pub timestamp: DateTime<Utc>,
}

/// Log query
#[derive(Debug, Clone)]
pub struct LogQuery {
    /// Query expression
    pub query: String,
    /// Query start time
    pub start_time: DateTime<Utc>,
    /// Query end time
    pub end_time: DateTime<Utc>,
    /// Maximum number of entries to return
    pub limit: usize,
}

/// Log stream
#[derive(Debug, Clone)]
pub struct LogStream {
    /// Log entries matching the query
    pub entries: Vec<LogEntry>,
    /// Whether more entries are available
    pub has_more: bool,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Entry timestamp
    pub timestamp: DateTime<Utc>,
    /// Severity level
    pub level: String,
    /// Log message
    pub message: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Trace query
#[derive(Debug, Clone)]
pub struct TraceQuery {
    /// Trace identifier
    pub trace_id: Option<String>,
    /// Query start time
    pub start_time: DateTime<Utc>,
    /// Query end time
    pub end_time: DateTime<Utc>,
    /// Minimum trace duration in milliseconds
    pub min_duration: Option<u64>,
    /// Maximum number of traces to return
    pub limit: usize,
}

/// Trace
#[derive(Debug, Clone)]
pub struct Trace {
    /// Trace identifier
    pub id: String,
    /// Name of the root span
    pub root_span_name: String,
    /// Total trace duration in milliseconds
    pub duration_ms: u64,
    /// Trace start time
    pub start_time: DateTime<Utc>,
    /// Spans composing the trace
    pub spans: Vec<Span>,
}

/// Span
#[derive(Debug, Clone)]
pub struct Span {
    /// Span identifier
    pub span_id: String,
    /// Parent span identifier
    pub parent_span_id: Option<String>,
    /// Span operation name
    pub operation: String,
    /// Span start time
    pub start_time: DateTime<Utc>,
    /// Span duration in milliseconds
    pub duration_ms: u64,
    /// Span tags
    pub tags: HashMap<String, String>,
}

/// Telemetry event
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    /// Metric data point event
    Metric(Metric),
    /// Log entry event
    Log(LogEntry),
    /// Trace event
    Trace(Trace),
}

/// Incident
#[derive(Debug, Clone)]
pub struct Incident {
    /// Incident identifier
    pub id: Option<String>,
    /// Incident title
    pub title: String,
    /// Incident description
    pub description: String,
    /// Incident severity
    pub severity: IncidentSeverity,
    /// Incident status
    pub status: IncidentStatus,
    /// Current assignee
    pub assigned_to: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: Option<DateTime<Utc>>,
    /// Resolution timestamp
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Incident severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentSeverity {
    /// Critical
    P1,
    /// High
    P2,
    /// Medium
    P3,
    /// Low
    P4,
}

/// Incident status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentStatus {
    /// Newly created
    New,
    /// Assigned to an owner
    Assigned,
    /// Actively being worked on
    InProgress,
    /// Resolved
    Resolved,
    /// Closed
    Closed,
}

/// Incident update
#[derive(Debug, Clone)]
pub struct IncidentUpdate {
    /// New status
    pub status: Option<IncidentStatus>,
    /// New severity
    pub severity: Option<IncidentSeverity>,
    /// New description
    pub description: Option<String>,
    /// New assignee
    pub assigned_to: Option<String>,
    /// Resolution notes
    pub resolution: Option<String>,
}

/// CMDB sync result
#[derive(Debug, Clone)]
pub struct CMDBSyncResult {
    /// Total items synced
    pub items_synced: usize,
    /// Items updated
    pub items_updated: usize,
    /// Items created
    pub items_created: usize,
    /// Items failed
    pub items_failed: usize,
    /// Errors encountered during sync
    pub errors: Vec<String>,
}

/// Resource filter
#[derive(Debug, Clone)]
pub struct ResourceFilter {
    /// Resource type to filter by
    pub resource_type: Option<String>,
    /// Label selectors
    pub labels: HashMap<String, String>,
    /// Namespace to filter by
    pub namespace: Option<String>,
}

/// Resource
#[derive(Debug, Clone)]
pub struct Resource {
    /// Resource identifier
    pub id: String,
    /// Resource name
    pub name: String,
    /// Resource type
    pub resource_type: String,
    /// Resource namespace
    pub namespace: Option<String>,
    /// Resource labels
    pub labels: HashMap<String, String>,
    /// Resource status
    pub status: String,
}

/// Resource metrics
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    /// Resource identifier
    pub resource_id: String,
    /// CPU utilization percentage
    pub cpu_percent: f64,
    /// Memory utilization percentage
    pub memory_percent: f64,
    /// Custom metric values
    pub custom_metrics: HashMap<String, f64>,
    /// Sample timestamp
    pub timestamp: DateTime<Utc>,
}

/// Resource event
#[derive(Debug, Clone)]
pub struct ResourceEvent {
    /// Event type
    pub event_type: ResourceEventType,
    /// Resource the event relates to
    pub resource: Resource,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
}

/// Resource event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceEventType {
    /// Resource added
    Added,
    /// Resource modified
    Modified,
    /// Resource deleted
    Deleted,
}

/// Infrastructure action
#[derive(Debug, Clone)]
pub struct InfraAction {
    /// Action type
    pub action_type: String,
    /// Target resource identifier
    pub resource_id: String,
    /// Action parameters
    pub parameters: HashMap<String, String>,
}

/// Action result
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,
    /// Result message
    pub message: String,
    /// Action output
    pub output: Option<String>,
    /// Error details on failure
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
