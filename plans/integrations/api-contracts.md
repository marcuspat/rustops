# API Contract Specifications

This document defines the API contracts and interfaces for all RustOps integrations, ensuring consistent behavior and enabling contract testing.

---

## Unified Adapter Interface

### Base Adapter Trait

All integrations must implement the `IntegrationAdapter` trait:

```rust
/// Base adapter interface for all integrations
#[async_trait]
pub trait IntegrationAdapter: Send + Sync {
    /// Integration identifier (e.g., "servicenow", "prometheus")
    fn id(&self) -> &str;

    /// Integration type classification
    fn kind(&self) -> IntegrationKind;

    /// Health check - verify external system connectivity
    async fn health_check(&self) -> Result<HealthStatus, AdapterError>;

    /// Initialize connection (with reconnection support)
    async fn initialize(&mut self) -> Result<(), AdapterError>;

    /// Shutdown gracefully
    async fn shutdown(&mut self) -> Result<(), AdapterError>;
}

/// Integration classification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntegrationKind {
    Metrics,
    Logs,
    Traces,
    Events,
    ITSM,
    Collaboration,
    Infrastructure,
    APM,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Base adapter error
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("API error: {status}: {message}")]
    Api { status: u16, message: String },

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Circuit breaker open")]
    CircuitBreakerOpen,

    #[error("Unknown error: {0}")]
    Unknown(String),
}
```

---

## Telemetry Collector Interface

### Metrics Collection

```rust
/// Telemetry collector for metrics, logs, and traces
#[async_trait]
pub trait TelemetryCollector: IntegrationAdapter {
    /// Collect metrics from external system
    async fn collect_metrics(
        &self,
        query: MetricQuery,
    ) -> Result<Vec<Metric>, AdapterError>;

    /// Collect logs from external system
    async fn collect_logs(
        &self,
        query: LogQuery,
    ) -> Result<LogStream, AdapterError>;

    /// Collect traces from external system
    async fn collect_traces(
        &self,
        query: TraceQuery,
    ) -> Result<Vec<Trace>, AdapterError>;

    /// Subscribe to real-time telemetry updates
    async fn subscribe(
        &self,
    ) -> Result<tokio::sync::mpsc::Receiver<TelemetryEvent>, AdapterError>;
}

/// Metrics query
#[derive(Debug, Clone)]
pub struct MetricQuery {
    pub metric_name: String,
    pub filters: Vec<MetricFilter>,
    pub aggregation: MetricAggregation,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub granularity: Duration,
}

#[derive(Debug, Clone)]
pub struct MetricFilter {
    pub label: String,
    pub operator: FilterOperator,
    pub value: String,
}

#[derive(Debug, Clone, Copy)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    Matches,
    NotMatches,
    GreaterThan,
    LessThan,
}

#[derive(Debug, Clone, Copy)]
pub enum MetricAggregation {
    Avg,
    Sum,
    Min,
    Max,
    Count,
}

/// Metric data point
#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub labels: Vec<(String, String)>,
    pub datapoints: Vec<MetricDataPoint>,
}

#[derive(Debug, Clone)]
pub struct MetricDataPoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: f64,
}

/// Log query
#[derive(Debug, Clone)]
pub struct LogQuery {
    pub query: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub limit: usize,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub message: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Log stream
pub struct LogStream {
    pub entries: tokio::sync::mpsc::Receiver<LogEntry>,
}

/// Trace query
#[derive(Debug, Clone)]
pub struct TraceQuery {
    pub trace_id: Option<String>,
    pub service: Option<String>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub limit: usize,
}

/// Distributed trace
#[derive(Debug, Clone)]
pub struct Trace {
    pub trace_id: String,
    pub root_span_id: String,
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation: String,
    pub service: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub duration: Duration,
    pub tags: Vec<(String, String)>,
    pub logs: Vec<SpanLog>,
}

#[derive(Debug, Clone)]
pub struct SpanLog {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub message: String,
    pub fields: Vec<(String, String)>,
}

/// Telemetry event
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    Metric(Metric),
    Log(LogEntry),
    Trace(Trace),
}
```

---

## ITSM Notifier Interface

### Incident Management

```rust
/// ITSM notifier for incident management
#[async_trait]
pub trait ITSMNotifier: IntegrationAdapter {
    /// Create or update incident
    async fn create_incident(
        &self,
        incident: Incident,
    ) -> Result<String, AdapterError>;

    /// Update incident status
    async fn update_incident(
        &self,
        id: &str,
        update: IncidentUpdate,
    ) -> Result<(), AdapterError>;

    /// Query incident details
    async fn get_incident(
        &self,
        id: &str,
    ) -> Result<Incident, AdapterError>;

    /// Search incidents
    async fn search_incidents(
        &self,
        query: IncidentQuery,
    ) -> Result<Vec<Incident>, AdapterError>;

    /// Sync with CMDB
    async fn sync_cmdb(
        &self,
    ) -> Result<CMDBSyncResult, AdapterError>;
}

/// Incident
#[derive(Debug, Clone)]
pub struct Incident {
    pub id: Option<String>,
    pub title: String,
    pub description: String,
    pub severity: IncidentSeverity,
    pub priority: IncidentPriority,
    pub impact: IncidentImpact,
    pub urgency: IncidentUrgency,
    pub state: IncidentState,
    pub assigned_to: Option<String>,
    pub assignment_group: Option<String>,
    pub configuration_item: Option<String>,
    pub correlation_id: String,
    pub metadata: IncidentMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IncidentSeverity {
    Critical = 1,
    High = 2,
    Moderate = 3,
    Low = 4,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IncidentPriority {
    P1 = 1,
    P2 = 2,
    P3 = 3,
    P4 = 4,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IncidentImpact {
    Critical = 1,
    High = 2,
    Medium = 3,
    Low = 4,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IncidentUrgency {
    Critical = 1,
    High = 2,
    Moderate = 3,
    Low = 4,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IncidentState {
    New,
    Active,
    InProgress,
    Pending,
    Resolved,
    Closed,
    Deleted,
}

/// Incident metadata
#[derive(Debug, Clone)]
pub struct IncidentMetadata {
    pub alert_id: String,
    pub service_name: String,
    pub affected_resources: Vec<String>,
    pub detection_time: chrono::DateTime<chrono::Utc>,
    pub first_seen: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub occurrences: u32,
    pub root_cause_hypothesis: Option<String>,
    pub auto_remediation_attempted: bool,
    pub tags: Vec<String>,
}

/// Incident update
#[derive(Debug, Clone)]
pub struct IncidentUpdate {
    pub state: Option<IncidentState>,
    pub priority: Option<IncidentPriority>,
    pub assigned_to: Option<String>,
    pub assignment_group: Option<String>,
    pub close_code: Option<String>,
    pub close_notes: Option<String>,
    pub work_notes: Option<String>,
    pub comments: Option<String>,
}

/// Incident query
#[derive(Debug, Clone)]
pub struct IncidentQuery {
    pub correlation_id: Option<String>,
    pub state: Option<IncidentState>,
    pub severity: Option<IncidentSeverity>,
    pub assigned_to: Option<String>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: usize,
}

/// CMDB sync result
#[derive(Debug, Clone)]
pub struct CMDBSyncResult {
    pub created: usize,
    pub updated: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}
```

---

## Infrastructure Monitor Interface

### Infrastructure Management

```rust
/// Infrastructure monitor for cloud/platform integrations
#[async_trait]
pub trait InfrastructureMonitor: IntegrationAdapter {
    /// List monitored resources
    async fn list_resources(
        &self,
        filters: ResourceFilter,
    ) -> Result<Vec<Resource>, AdapterError>;

    /// Get resource metrics
    async fn get_resource_metrics(
        &self,
        id: &str,
    ) -> Result<ResourceMetrics, AdapterError>;

    /// Watch for resource changes (streaming)
    async fn watch_resources(
        &self,
    ) -> Result<tokio::sync::mpsc::Receiver<ResourceEvent>, AdapterError>;

    /// Execute infrastructure action
    async fn execute_action(
        &self,
        action: InfraAction,
    ) -> Result<ActionResult, AdapterError>;
}

/// Resource filter
#[derive(Debug, Clone)]
pub struct ResourceFilter {
    pub resource_type: Option<String>,
    pub tags: Vec<(String, String)>,
    pub status: Option<String>,
    pub limit: usize,
}

/// Resource
#[derive(Debug, Clone)]
pub struct Resource {
    pub id: String,
    pub name: String,
    pub resource_type: String,
    pub status: ResourceStatus,
    pub tags: Vec<(String, String)>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceStatus {
    Running,
    Stopped,
    Pending,
    Failed,
    Unknown,
}

/// Resource metrics
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    pub resource_id: String,
    pub cpu_percent: Option<f64>,
    pub memory_percent: Option<f64>,
    pub network_in_bytes: Option<u64>,
    pub network_out_bytes: Option<u64>,
    pub disk_percent: Option<f64>,
    pub custom_metrics: Vec<(String, f64)>,
}

/// Resource event
#[derive(Debug, Clone)]
pub struct ResourceEvent {
    pub event_type: ResourceEventType,
    pub resource: Resource,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceEventType {
    Created,
    Updated,
    Deleted,
    StatusChanged,
}

/// Infrastructure action
#[derive(Debug, Clone)]
pub enum InfraAction {
    Restart { resource_id: String },
    Stop { resource_id: String },
    Start { resource_id: String },
    Scale { resource_id: String, replicas: i32 },
    Exec {
        resource_id: String,
        command: Vec<String>,
    },
    Rollback { resource_id: String },
}

/// Action result
#[derive(Debug, Clone)]
pub struct ActionResult {
    pub action_id: String,
    pub status: ActionStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}
```

---

## Collaboration Channel Interface

### Notifications and ChatOps

```rust
/// Collaboration channel for notifications and ChatOps
#[async_trait]
pub trait CollaborationChannel: IntegrationAdapter {
    /// Send notification to channel
    async fn send_notification(
        &self,
        channel: &str,
        notification: Notification,
    ) -> Result<String, AdapterError>;

    /// Create incident channel
    async fn create_incident_channel(
        &self,
        incident: Incident,
    ) -> Result<ChannelInfo, AdapterError>;

    /// Archive channel
    async fn archive_channel(
        &self,
        channel_id: &str,
    ) -> Result<(), AdapterError>;

    /// Handle slash command
    async fn handle_command(
        &self,
        command: ChatCommand,
    ) -> Result<CommandResponse, AdapterError>;

    /// Update message
    async fn update_message(
        &self,
        channel_id: &str,
        message_id: &str,
        update: MessageUpdate,
    ) -> Result<(), AdapterError>;
}

/// Notification
#[derive(Debug, Clone)]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub severity: NotificationSeverity,
    pub blocks: Vec<MessageBlock>,
    pub actions: Vec<Action>,
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotificationSeverity {
    Critical,
    High,
    Moderate,
    Low,
    Info,
}

/// Message block (Slack Block Kit style)
#[derive(Debug, Clone)]
pub enum MessageBlock {
    Section { text: Text, fields: Vec<Text> },
    Header { text: Text },
    Divider,
    Actions { elements: Vec<Action> },
    Context { elements: Vec<Text> },
}

#[derive(Debug, Clone)]
pub struct Text {
    pub text_type: TextType,
    pub content: String,
    pub emoji: Option<bool>,
}

#[derive(Debug, Clone)]
pub enum TextType {
    PlainText,
    Markdown,
}

/// Interactive action (button, etc.)
#[derive(Debug, Clone)]
pub struct Action {
    pub action_id: String,
    pub style: ActionStyle,
    pub text: Text,
    pub value: Option<String>,
    pub confirm: Option<Confirmation>,
}

#[derive(Debug, Clone)]
pub enum ActionStyle {
    Default,
    Primary,
    Danger,
}

#[derive(Debug, Clone)]
pub struct Confirmation {
    pub title: String,
    pub text: String,
    pub confirm_text: String,
    pub deny_text: String,
}

/// Channel info
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub channel_id: String,
    pub channel_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Chat command
#[derive(Debug, Clone)]
pub struct ChatCommand {
    pub command: String,
    pub text: String,
    pub user_id: String,
    pub user_name: String,
    pub channel_id: String,
    pub response_url: String,
}

/// Command response
#[derive(Debug, Clone)]
pub struct CommandResponse {
    pub response_type: ResponseType,
    pub text: String,
    pub blocks: Option<Vec<MessageBlock>>,
}

#[derive(Debug, Clone)]
pub enum ResponseType {
    Ephemeral,  // Only visible to user
    InChannel,  // Visible to everyone
}

/// Message update
#[derive(Debug, Clone)]
pub struct MessageUpdate {
    pub text: Option<String>,
    pub blocks: Option<Vec<MessageBlock>>,
    pub attachments: Option<Vec<Attachment>>,
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub color: String,
    pub title: Option<String>,
    pub text: Option<String>,
    pub fields: Vec<AttachmentField>,
}

#[derive(Debug, Clone)]
pub struct AttachmentField {
    pub title: String,
    pub value: String,
    pub short: bool,
}
```

---

## Rate Limiting Contracts

### Rate Limit Configuration

```rust
/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_concurrent: usize,
    pub requests_per_second: u32,
    pub burst_size: Option<usize>,
}

/// Rate limiter
pub struct RateLimiter {
    config: RateLimitConfig,
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl RateLimiter {
    pub async fn acquire(&self) -> Result<(), AdapterError> {
        tokio::select! {
            permit = self.semaphore.acquire() => {
                permit.forget();
                Ok(())
            }
            _ = tokio::time::sleep(Duration::from_secs(10)) => {
                Err(AdapterError::Timeout("Rate limit acquire timeout".into()))
            }
        }
    }
}
```

---

## Circuit Breaker Contracts

### Circuit Breaker Pattern

```rust
/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub error_threshold: usize,
    pub success_threshold: usize,
    pub timeout: Duration,
    pub max_calls: usize,
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
    error_count: Arc<AtomicUsize>,
    success_count: Arc<AtomicUsize>,
}

impl CircuitBreaker {
    pub async fn call<F, T, E>(
        &self,
        operation: F,
    ) -> Result<T, AdapterError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let state = *self.state.read().await;

        match state {
            CircuitBreakerState::Open => {
                Err(AdapterError::CircuitBreakerOpen)
            }
            CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen => {
                match operation.await {
                    Ok(result) => {
                        self.record_success().await;
                        Ok(result)
                    }
                    Err(err) => {
                        self.record_error().await;
                        Err(AdapterError::Unknown(err.to_string()))
                    }
                }
            }
        }
    }

    async fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::SeqCst);

        if self.success_count.load(Ordering::SeqCst) >= self.config.success_threshold {
            *self.state.write().await = CircuitBreakerState::Closed;
            self.error_count.store(0, Ordering::SeqCst);
            self.success_count.store(0, Ordering::SeqCst);
        }
    }

    async fn record_error(&self) {
        let count = self.error_count.fetch_add(1, Ordering::SeqCst) + 1;

        if count >= self.config.error_threshold {
            *self.state.write().await = CircuitBreakerState::Open;
            self.success_count.store(0, Ordering::SeqCst);

            // Reset after timeout
            let state = Arc::clone(&self.state);
            let timeout = self.config.timeout;
            tokio::spawn(async move {
                tokio::time::sleep(timeout).await;
                *state.write().await = CircuitBreakerState::HalfOpen;
            });
        }
    }
}
```

---

## Retry Contracts

### Retry with Backoff

```rust
/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub multiplier: f64,
    pub jitter: bool,
}

/// Retry logic
pub async fn retry_with_backoff<F, Fut, T, E>(
    config: RetryConfig,
    operation: F,
) -> Result<T, AdapterError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut backoff = config.initial_backoff;

    for attempt in 0..config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) if attempt < config.max_attempts - 1 => {
                tracing::warn!(
                    error = %err,
                    attempt = attempt + 1,
                    max_attempts = config.max_attempts,
                    retry_after = ?backoff,
                    "Operation failed, retrying..."
                );

                tokio::time::sleep(backoff).await;

                // Exponential backoff
                backoff = std::cmp::min(
                    Duration::from_secs_f64(backoff.as_secs_f64() * config.multiplier),
                    config.max_backoff,
                );
            }
            Err(err) => {
                return Err(AdapterError::Unknown(err.to_string()));
            }
        }
    }

    Err(AdapterError::Unknown("Max retry attempts exceeded".into()))
}
```

---

## Contract Testing

### Test Helpers

```rust
/// Contract test for adapter
#[async_trait]
pub trait ContractTest {
    async fn test_health_check(&self) -> Result<(), AdapterError>;
    async fn test_authentication(&self) -> Result<(), AdapterError>;
    async fn test_basic_operation(&self) -> Result<(), AdapterError>;
}

/// Example contract test
#[tokio::test]
async fn test_servicenow_contract() {
    let adapter = create_test_adapter().await;

    // Test health check
    let health = adapter.health_check().await.unwrap();
    assert_eq!(health, HealthStatus::Healthy);

    // Test incident creation
    let incident = create_test_incident();
    let incident_id = adapter.create_incident(incident).await.unwrap();
    assert!(!incident_id.is_empty());

    // Test incident retrieval
    let retrieved = adapter.get_incident(&incident_id).await.unwrap();
    assert_eq!(retrieved.id, Some(incident_id));
}
```

---

**Version**: 1.0
**Last Updated**: 2026-01-18
