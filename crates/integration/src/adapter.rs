// Adapter pattern for unified integration interface
//
// Provides a consistent interface across all external system integrations

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crate::resilience::{IntegrationError, IntegrationResult, HealthStatus};
use crate::{CircuitBreaker, CircuitBreakerConfig, RateLimiter, RateLimiterConfig, RetryConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

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

/// Integration kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegrationKind {
    TelemetryCollector,
    ITSMNotifier,
    InfrastructureMonitor,
    CollaborationChannel,
}

// =============================================================================
// Data Types
// =============================================================================

/// Metric query
#[derive(Debug, Clone)]
pub struct MetricQuery {
    pub metric_name: String,
    pub labels: HashMap<String, String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub step: Option<u64>, // Step in seconds
}

/// Metric data point
#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
}

/// Log query
#[derive(Debug, Clone)]
pub struct LogQuery {
    pub query: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub limit: usize,
}

/// Log stream
#[derive(Debug, Clone)]
pub struct LogStream {
    pub entries: Vec<LogEntry>,
    pub has_more: bool,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub metadata: HashMap<String, String>,
}

/// Trace query
#[derive(Debug, Clone)]
pub struct TraceQuery {
    pub trace_id: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub min_duration: Option<u64>,
    pub limit: usize,
}

/// Trace
#[derive(Debug, Clone)]
pub struct Trace {
    pub id: String,
    pub root_span_name: String,
    pub duration_ms: u64,
    pub start_time: DateTime<Utc>,
    pub spans: Vec<Span>,
}

/// Span
#[derive(Debug, Clone)]
pub struct Span {
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation: String,
    pub start_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub tags: HashMap<String, String>,
}

/// Telemetry event
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    Metric(Metric),
    Log(LogEntry),
    Trace(Trace),
}

/// Incident
#[derive(Debug, Clone)]
pub struct Incident {
    pub id: Option<String>,
    pub title: String,
    pub description: String,
    pub severity: IncidentSeverity,
    pub status: IncidentStatus,
    pub assigned_to: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Incident severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentSeverity {
    P1, // Critical
    P2, // High
    P3, // Medium
    P4, // Low
}

/// Incident status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentStatus {
    New,
    Assigned,
    InProgress,
    Resolved,
    Closed,
}

/// Incident update
#[derive(Debug, Clone)]
pub struct IncidentUpdate {
    pub status: Option<IncidentStatus>,
    pub severity: Option<IncidentSeverity>,
    pub description: Option<String>,
    pub assigned_to: Option<String>,
    pub resolution: Option<String>,
}

/// CMDB sync result
#[derive(Debug, Clone)]
pub struct CMDBSyncResult {
    pub items_synced: usize,
    pub items_updated: usize,
    pub items_created: usize,
    pub items_failed: usize,
    pub errors: Vec<String>,
}

/// Resource filter
#[derive(Debug, Clone)]
pub struct ResourceFilter {
    pub resource_type: Option<String>,
    pub labels: HashMap<String, String>,
    pub namespace: Option<String>,
}

/// Resource
#[derive(Debug, Clone)]
pub struct Resource {
    pub id: String,
    pub name: String,
    pub resource_type: String,
    pub namespace: Option<String>,
    pub labels: HashMap<String, String>,
    pub status: String,
}

/// Resource metrics
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    pub resource_id: String,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub custom_metrics: HashMap<String, f64>,
    pub timestamp: DateTime<Utc>,
}

/// Resource event
#[derive(Debug, Clone)]
pub struct ResourceEvent {
    pub event_type: ResourceEventType,
    pub resource: Resource,
    pub timestamp: DateTime<Utc>,
}

/// Resource event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceEventType {
    Added,
    Modified,
    Deleted,
}

/// Infrastructure action
#[derive(Debug, Clone)]
pub struct InfraAction {
    pub action_type: String,
    pub resource_id: String,
    pub parameters: HashMap<String, String>,
}

/// Action result
#[derive(Debug, Clone)]
pub struct ActionResult {
    pub success: bool,
    pub message: String,
    pub output: Option<String>,
    pub error: Option<String>,
}

// =============================================================================
// Base Adapter Implementation
// =============================================================================

/// Base adapter with common functionality
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
