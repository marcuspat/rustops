// Domain-Driven Design Reference Implementation
// RustOps AIOps Platform
//
// This module contains type-safe domain model definitions
// following DDD principles with bounded contexts, aggregates,
// and domain events.

#![allow(dead_code)]
#![warn(missing_docs)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::time::Duration;
use uuid::Uuid;

// =============================================================================
// Shared Kernel
// =============================================================================

/// Shared domain identifier types
pub mod id {
    use super::*;

    /// Telemetry source identifier
    pub type TelemetrySourceId = Uuid;

    /// Metric name (newtype for type safety)
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct MetricName(pub String);

    impl MetricName {
        pub fn new(name: impl Into<String>) -> Self {
            Self(name.into())
        }

        pub fn as_str(&self) -> &str {
            &self.0
        }
    }

    /// Anomaly identifier
    pub type AnomalyId = Uuid;

    /// Detector identifier
    pub type DetectorId = Uuid;

    /// Incident identifier
    pub type IncidentId = Uuid;

    /// Incident number (human-readable)
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct IncidentNumber(pub String);

    impl IncidentNumber {
        pub fn new(number: impl Into<String>) -> Self {
            Self(number.into())
        }
    }

    /// Workflow identifier
    pub type WorkflowId = Uuid;

    /// Service identifier
    pub type ServiceId = Uuid;

    /// Knowledge entry identifier
    pub type KnowledgeEntryId = Uuid;
}

/// Shared value objects
pub mod value {
    use super::*;

    /// Health status of a service or component
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum HealthStatus {
        Healthy,
        Degraded,
        Unhealthy,
        Unknown,
    }

    /// Service identity - shared across contexts
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct ServiceIdentity {
        pub id: Uuid,
        pub name: String,
        pub namespace: String,
        pub cluster: String,
    }

    impl ServiceIdentity {
        pub fn new(
            id: Uuid,
            name: impl Into<String>,
            namespace: impl Into<String>,
            cluster: impl Into<String>,
        ) -> Self {
            Self {
                id,
                name: name.into(),
                namespace: namespace.into(),
                cluster: cluster.into(),
            }
        }

        pub fn qualified_name(&self) -> String {
            format!("{}/{}", self.namespace, self.name)
        }
    }

    /// Metric labels (key-value pairs)
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Labels(pub BTreeMap<String, String>);

    impl Labels {
        pub fn new() -> Self {
            Self(BTreeMap::new())
        }

        pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
            self.0.insert(key.into(), value.into());
        }

        pub fn get(&self, key: &str) -> Option<&String> {
            self.0.get(key)
        }
    }

    impl Default for Labels {
        fn default() -> Self {
            Self::new()
        }
    }
}

/// Domain errors
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Invariant violated: {0}")]
    InvariantViolation(String),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Concurrency conflict: {0}")]
    Conflict(String),

    #[error("Transition invalid: {from} -> {to}")]
    InvalidTransition {
        from: String,
        to: String,
    },
}

/// Domain result type
pub type DomainResult<T> = Result<T, DomainError>;

// =============================================================================
// Domain Events (Shared)
// =============================================================================

/// Base trait for all domain events
pub trait DomainEvent: Send + Sync + Clone {
    fn event_id(&self) -> Uuid;
    fn event_type(&self) -> &'static str;
    fn occurred_at(&self) -> DateTime<Utc>;
    fn causation_id(&self) -> Option<Uuid>;
    fn correlation_id(&self) -> Option<Uuid>;
}

/// Domain event wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEventData {
    pub event_id: Uuid,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub causation_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
    pub payload: serde_json::Value,
}

impl DomainEventData {
    pub fn new(
        event_type: impl Into<String>,
        payload: serde_json::Value,
        causation_id: Option<Uuid>,
        correlation_id: Option<Uuid>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type: event_type.into(),
            occurred_at: Utc::now(),
            causation_id,
            correlation_id,
            payload,
        }
    }
}

// =============================================================================
// Aggregate Trait
// =============================================================================

/// Base trait for all aggregates
pub trait Aggregate {
    type Id;
    type Event: DomainEvent;

    fn id(&self) -> &Self::Id;
    fn version(&self) -> u64;
    fn apply_event(&mut self, event: Self::Event) -> DomainResult<()>;
}

// =============================================================================
// Repository Trait
// =============================================================================

/// Repository trait for aggregate persistence
#[async_trait::async_trait]
pub trait Repository<A: Aggregate>: Send + Sync {
    async fn get(&self, id: &A::Id) -> DomainResult<Option<A>>;
    async fn save(&self, aggregate: &A) -> DomainResult<()>;
    async fn delete(&self, id: &A::Id) -> DomainResult<()>;
}

// =============================================================================
// Telemetry Collection Context
// =============================================================================

pub mod telemetry {
    use super::*;

    /// Telemetry source types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum SourceType {
        Prometheus,
        CloudWatch,
        Datadog,
        OpenTelemetry,
        Fluentd,
        Custom,
    }

    /// Connection status
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum ConnectionStatus {
        Disconnected,
        Connecting,
        Connected,
        Error,
    }

    /// Log levels
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
    pub enum LogLevel {
        Trace,
        Debug,
        Info,
        Warn,
        Error,
    }

    /// Metric value types
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum MetricValue {
        Counter(f64),
        Gauge(f64),
        Histogram(HistogramData),
        Summary(SummaryData),
    }

    /// Histogram data
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HistogramData {
        pub count: u64,
        pub sum: f64,
        pub buckets: Vec<(f64, u64)>,
    }

    /// Summary data
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SummaryData {
        pub count: u64,
        pub sum: f64,
        pub quantiles: Vec<(f64, f64)>,
    }

    /// Telemetry source configuration
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SourceConfig {
        pub endpoint: String,
        pub credentials: Option<String>,
        pub collection_interval: Duration,
        pub labels: Labels,
    }

    /// Aggregate root: TelemetrySource
    #[derive(Debug, Clone)]
    pub struct TelemetrySource {
        pub id: id::TelemetrySourceId,
        pub source_type: SourceType,
        pub config: SourceConfig,
        pub status: ConnectionStatus,
        pub metrics_collected: u64,
        pub last_collected: Option<DateTime<Utc>>,
        pub version: u64,
    }

    impl TelemetrySource {
        pub fn new(
            id: id::TelemetrySourceId,
            source_type: SourceType,
            config: SourceConfig,
        ) -> Self {
            Self {
                id,
                source_type,
                config,
                status: ConnectionStatus::Disconnected,
                metrics_collected: 0,
                last_collected: None,
                version: 0,
            }
        }

        pub fn connect(&mut self) -> DomainResult<()> {
            if self.status == ConnectionStatus::Connected {
                return Err(DomainError::InvalidTransition {
                    from: format!("{:?}", self.status),
                    to: "Connected".to_string(),
                });
            }
            self.status = ConnectionStatus::Connecting;
            Ok(())
        }

        pub fn mark_connected(&mut self) -> DomainResult<()> {
            if self.status != ConnectionStatus::Connecting {
                return Err(DomainError::InvalidTransition {
                    from: format!("{:?}", self.status),
                    to: "Connected".to_string(),
                });
            }
            self.status = ConnectionStatus::Connected;
            self.version += 1;
            Ok(())
        }

        pub fn disconnect(&mut self) -> DomainResult<()> {
            self.status = ConnectionStatus::Disconnected;
            self.version += 1;
            Ok(())
        }

        pub fn record_metrics(&mut self, count: u64) -> DomainResult<()> {
            if self.status != ConnectionStatus::Connected {
                return Err(DomainError::InvariantViolation(
                    "Cannot collect metrics from disconnected source".to_string(),
                ));
            }
            self.metrics_collected += count;
            self.last_collected = Some(Utc::now());
            Ok(())
        }
    }

    /// Metric data point
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MetricData {
        pub metric_name: id::MetricName,
        pub value: MetricValue,
        pub timestamp: DateTime<Utc>,
        pub labels: Labels,
        pub source_id: id::TelemetrySourceId,
    }

    impl MetricData {
        pub fn new(
            metric_name: id::MetricName,
            value: MetricValue,
            labels: Labels,
            source_id: id::TelemetrySourceId,
        ) -> Self {
            Self {
                metric_name,
                value,
                timestamp: Utc::now(),
                labels,
                source_id,
            }
        }

        pub fn validate(&self) -> DomainResult<()> {
            if self.timestamp > Utc::now() {
                return Err(DomainError::ValidationError(
                    "Timestamp cannot be in the future".to_string(),
                ));
            }
            Ok(())
        }
    }

    /// Log entry
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LogEntry {
        pub id: Uuid,
        pub timestamp: DateTime<Utc>,
        pub level: LogLevel,
        pub message: String,
        pub structured_data: BTreeMap<String, serde_json::Value>,
        pub source_service: value::ServiceIdentity,
    }

    /// Telemetry context events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum TelemetryContextEvent {
        MetricsCollected {
            source_id: id::TelemetrySourceId,
            metric_count: u64,
            timestamp: DateTime<Utc>,
        },
        SourceConnected {
            source_id: id::TelemetrySourceId,
            connected_at: DateTime<Utc>,
        },
        SourceDisconnected {
            source_id: id::TelemetrySourceId,
            reason: Option<String>,
        },
    }
}

// =============================================================================
// Anomaly Detection Context
// =============================================================================

pub mod anomaly {
    use super::*;

    /// Detector types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum DetectorType {
        StatisticalThreshold,
        MovingAverage,
        ExponentialSmoothing,
        LSTM,
        IsolationForest,
        DBSCAN,
    }

    /// Anomaly types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum AnomalyType {
        Spike,
        Drop,
        TrendChange,
        PatternBreak,
        SeasonalityViolation,
        NewErrorPattern,
    }

    /// Anomaly severity
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
    pub enum AnomalySeverity {
        Low,
        Medium,
        High,
        Critical,
    }

    /// Detector state
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum DetectorState {
        Learning,
        Active,
        Paused,
    }

    /// Baseline period
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum BaselinePeriod {
        Minutes30,
        Hour1,
        Hours6,
        Day1,
        Week1,
    }

    /// Baseline statistics
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BaselineStatistics {
        pub mean: f64,
        pub std_dev: f64,
        pub p50: f64,
        pub p95: f64,
        pub p99: f64,
        pub min: f64,
        pub max: f64,
    }

    impl BaselineStatistics {
        pub fn is_anomaly(&self, value: f64, threshold: f64) -> bool {
            let z_score = (value - self.mean) / self.std_dev;
            z_score.abs() > threshold
        }
    }

    /// Seasonality information
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Seasonality {
        pub period: Duration,
        pub amplitude: f64,
    }

    /// Anomaly features
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AnomalyFeatures {
        pub magnitude: f64,
        pub duration: Duration,
        pub affected_services: Vec<value::ServiceIdentity>,
        pub correlation_score: f64,
    }

    /// Detector configuration
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DetectorConfig {
        pub sensitivity: f64, // 0.0 to 1.0
        pub threshold: f64,
        pub min_baseline_points: u32,
        pub alert_cooldown: Duration,
    }

    impl Default for DetectorConfig {
        fn default() -> Self {
            Self {
                sensitivity: 0.8,
                threshold: 3.0, // 3 sigma
                min_baseline_points: 30,
                alert_cooldown: Duration::from_secs(300),
            }
        }
    }

    /// Statistical baseline
    #[derive(Debug, Clone)]
    pub struct Baseline {
        pub metric_name: id::MetricName,
        pub period: BaselinePeriod,
        pub statistics: BaselineStatistics,
        pub seasonality: Option<Seasonality>,
        pub last_updated: DateTime<Utc>,
    }

    impl Baseline {
        pub fn new(
            metric_name: id::MetricName,
            period: BaselinePeriod,
            statistics: BaselineStatistics,
        ) -> Self {
            Self {
                metric_name,
                period,
                statistics,
                seasonality: None,
                last_updated: Utc::now(),
            }
        }

        pub fn update(&mut self, statistics: BaselineStatistics) -> DomainResult<()> {
            self.statistics = statistics;
            self.last_updated = Utc::now();
            Ok(())
        }
    }

    /// Aggregate root: AnomalyDetector
    #[derive(Debug, Clone)]
    pub struct AnomalyDetector {
        pub id: id::DetectorId,
        pub detector_type: DetectorType,
        pub target_metric: id::MetricName,
        pub config: DetectorConfig,
        pub baseline: Baseline,
        pub state: DetectorState,
        pub detection_count: u64,
        pub last_detection: Option<DateTime<Utc>>,
        pub version: u64,
    }

    impl AnomalyDetector {
        pub fn new(
            id: id::DetectorId,
            detector_type: DetectorType,
            target_metric: id::MetricName,
            baseline: Baseline,
        ) -> Self {
            Self {
                id,
                detector_type,
                target_metric,
                config: DetectorConfig::default(),
                baseline,
                state: DetectorState::Learning,
                detection_count: 0,
                last_detection: None,
                version: 0,
            }
        }

        pub fn activate(&mut self) -> DomainResult<()> {
            if self.state != DetectorState::Learning {
                return Err(DomainError::InvalidTransition {
                    from: format!("{:?}", self.state),
                    to: "Active".to_string(),
                });
            }
            self.state = DetectorState::Active;
            self.version += 1;
            Ok(())
        }

        pub fn detect(&mut self, value: f64) -> DomainResult<bool> {
            if self.state != DetectorState::Active {
                return Ok(false);
            }

            let is_anomaly = self.baseline.statistics.is_anomaly(value, self.config.threshold);
            if is_anomaly {
                self.detection_count += 1;
                self.last_detection = Some(Utc::now());
            }
            Ok(is_anomaly)
        }
    }

    /// Anomaly aggregate
    #[derive(Debug, Clone)]
    pub struct Anomaly {
        pub id: id::AnomalyId,
        pub detector_id: id::DetectorId,
        pub anomaly_type: AnomalyType,
        pub severity: AnomalySeverity,
        pub detected_at: DateTime<Utc>,
        pub description: String,
        pub affected_metrics: Vec<id::MetricName>,
        pub confidence_score: f64,
        pub features: AnomalyFeatures,
        pub status: AnomalyStatus,
        pub version: u64,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum AnomalyStatus {
        Active,
        Confirming,
        Confirmed,
        Recovered,
        FalsePositive,
    }

    impl Anomaly {
        pub fn new(
            id: id::AnomalyId,
            detector_id: id::DetectorId,
            anomaly_type: AnomalyType,
            severity: AnomalySeverity,
            confidence_score: f64,
        ) -> DomainResult<Self> {
            if !(0.0..=1.0).contains(&confidence_score) {
                return Err(DomainError::ValidationError(
                    "Confidence score must be between 0.0 and 1.0".to_string(),
                ));
            }

            Ok(Self {
                id,
                detector_id,
                anomaly_type,
                severity,
                detected_at: Utc::now(),
                description: String::new(),
                affected_metrics: Vec::new(),
                confidence_score,
                features: AnomalyFeatures {
                    magnitude: 0.0,
                    duration: Duration::ZERO,
                    affected_services: Vec::new(),
                    correlation_score: 0.0,
                },
                status: AnomalyStatus::Active,
                version: 0,
            })
        }

        pub fn confirm(&mut self) -> DomainResult<()> {
            self.status = AnomalyStatus::Confirmed;
            self.version += 1;
            Ok(())
        }

        pub fn mark_recovered(&mut self) -> DomainResult<()> {
            self.status = AnomalyStatus::Recovered;
            self.version += 1;
            Ok(())
        }
    }

    /// Anomaly detection events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum AnomalyDetectionEvent {
        AnomalyDetected {
            anomaly_id: id::AnomalyId,
            detector_id: id::DetectorId,
            severity: AnomalySeverity,
            confidence: f64,
            detected_at: DateTime<Utc>,
        },
        AnomalyConfirmed {
            anomaly_id: id::AnomalyId,
            confirmed_at: DateTime<Utc>,
        },
        BaselineUpdated {
            detector_id: id::DetectorId,
            metric_name: id::MetricName,
            updated_at: DateTime<Utc>,
        },
    }
}

// =============================================================================
// Incident Management Context
// =============================================================================

pub mod incident {
    use super::*;

    /// Incident status
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum IncidentStatus {
        New,
        Detected,
        Analyzing,
        Remediating,
        Resolved,
        Closed,
    }

    /// Incident severity (ITSM standard)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
    pub enum IncidentSeverity {
        P1, // Critical - immediate action required
        P2, // High - service degraded
        P3, // Medium - minor impact
        P4, // Low - no user impact
    }

    /// Correlation strategies
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum CorrelationStrategy {
        TimeWindow,
        Similarity,
        TopologyBased,
        CausalGraph,
    }

    /// Aggregate root: Incident
    #[derive(Debug, Clone)]
    pub struct Incident {
        pub id: id::IncidentId,
        pub incident_number: id::IncidentNumber,
        pub title: String,
        pub description: String,
        pub status: IncidentStatus,
        pub severity: IncidentSeverity,
        pub detected_at: DateTime<Utc>,
        pub assigned_to: Option<String>,
        pub resolved_at: Option<DateTime<Utc>>,
        pub related_anomalies: Vec<id::AnomalyId>,
        pub correlation_group: Option<id::CorrelationGroupId>,
        pub remediation_plan: Option<id::WorkflowId>,
        pub version: u64,
    }

    /// Correlation group ID (newtype)
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct CorrelationGroupId(pub Uuid);

    impl Incident {
        pub fn new(
            id: id::IncidentId,
            incident_number: id::IncidentNumber,
            title: impl Into<String>,
            severity: IncidentSeverity,
        ) -> Self {
            Self {
                id,
                incident_number,
                title: title.into(),
                description: String::new(),
                status: IncidentStatus::New,
                severity,
                detected_at: Utc::now(),
                assigned_to: None,
                resolved_at: None,
                related_anomalies: Vec::new(),
                correlation_group: None,
                remediation_plan: None,
                version: 0,
            }
        }

        pub fn transition_to(&mut self, new_status: IncidentStatus) -> DomainResult<()> {
            self.validate_transition(self.status, new_status)?;
            self.status = new_status;
            self.version += 1;

            if new_status == IncidentStatus::Resolved {
                self.resolved_at = Some(Utc::now());
            }

            Ok(())
        }

        pub fn assign(&mut self, user: impl Into<String>) -> DomainResult<()> {
            self.assigned_to = Some(user.into());
            self.version += 1;
            Ok(())
        }

        pub fn add_anomaly(&mut self, anomaly_id: id::AnomalyId) -> DomainResult<()> {
            if !self.related_anomalies.contains(&anomaly_id) {
                self.related_anomalies.push(anomaly_id);
            }
            Ok(())
        }

        pub fn set_remediation_plan(&mut self, workflow_id: id::WorkflowId) -> DomainResult<()> {
            self.remediation_plan = Some(workflow_id);
            self.version += 1;
            Ok(())
        }

        fn validate_transition(
            &self,
            from: IncidentStatus,
            to: IncidentStatus,
        ) -> DomainResult<()> {
            match (from, to) {
                (IncidentStatus::New, IncidentStatus::Detected) => Ok(()),
                (IncidentStatus::Detected, IncidentStatus::Analyzing) => Ok(()),
                (IncidentStatus::Analyzing, IncidentStatus::Remediating) => Ok(()),
                (IncidentStatus::Remediating, IncidentStatus::Resolved) => Ok(()),
                (IncidentStatus::Resolved, IncidentStatus::Closed) => Ok(()),
                _ => Err(DomainError::InvalidTransition {
                    from: format!("{:?}", from),
                    to: format!("{:?}", to),
                }),
            }
        }
    }

    /// Alert deduplicator
    #[derive(Debug, Clone)]
    pub struct AlertDeduplicator {
        pub fingerprint: AlertFingerprint,
        pub last_seen: DateTime<Utc>,
        pub count: u64,
        pub status: DeduplicationStatus,
        pub window: Duration,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct AlertFingerprint(String);

    impl AlertFingerprint {
        pub fn new(fingerprint: impl Into<String>) -> Self {
            Self(fingerprint.into())
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum DeduplicationStatus {
        Active,
        Suppressed,
        Resolved,
    }

    impl AlertDeduplicator {
        pub fn new(fingerprint: AlertFingerprint, window: Duration) -> Self {
            Self {
                fingerprint,
                last_seen: Utc::now(),
                count: 1,
                status: DeduplicationStatus::Active,
                window,
            }
        }

        pub fn check(&self, now: DateTime<Utc>) -> bool {
            now.signed_duration_since(self.last_seen).num_milliseconds() < self.window.as_millis() as i64
        }

        pub fn increment(&mut self) -> DomainResult<()> {
            self.count += 1;
            self.last_seen = Utc::now();
            Ok(())
        }

        pub fn suppress(&mut self) -> DomainResult<()> {
            self.status = DeduplicationStatus::Suppressed;
            Ok(())
        }
    }

    /// Incident management events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum IncidentManagementEvent {
        IncidentCreated {
            incident_id: id::IncidentId,
            incident_number: id::IncidentNumber,
            severity: IncidentSeverity,
            created_at: DateTime<Utc>,
        },
        IncidentStatusChanged {
            incident_id: id::IncidentId,
            old_status: IncidentStatus,
            new_status: IncidentStatus,
            changed_at: DateTime<Utc>,
        },
        IncidentsCorrelated {
            correlation_group_id: CorrelationGroupId,
            incident_ids: Vec<id::IncidentId>,
        },
        AlertDeduplicated {
            fingerprint: AlertFingerprint,
            suppressed_count: u64,
        },
    }
}

// =============================================================================
// Remediation Context
// =============================================================================

pub mod remediation {
    use super::*;

    /// Workflow types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum WorkflowType {
        RestartService,
        ScaleHorizontal,
        ScaleVertical,
        FailoverTraffic,
        RollbackDeployment,
        ClearCache,
        ResetConnectionPool,
        CustomRunbook,
    }

    /// Workflow status
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum WorkflowStatus {
        PendingApproval,
        Approved,
        InProgress,
        Completed,
        Failed,
        RolledBack,
        Cancelled,
    }

    /// Approval status
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum ApprovalStatus {
        Pending,
        Approved,
        Rejected,
    }

    /// Action types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum ActionType {
        RestartPod,
        ScaleDeployment,
        UpdateConfig,
        ExecuteScript,
        SendNotification,
        CreateTicket,
    }

    /// Resource types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum ResourceType {
        Pod,
        Deployment,
        StatefulSet,
        Service,
        ConfigMap,
        Secret,
    }

    /// Workflow result
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum WorkflowResult {
        Success,
        PartialSuccess,
        Failure(String),
    }

    /// Aggregate root: RemediationWorkflow
    #[derive(Debug, Clone)]
    pub struct RemediationWorkflow {
        pub id: id::WorkflowId,
        pub incident_id: id::IncidentId,
        pub workflow_type: WorkflowType,
        pub status: WorkflowStatus,
        pub steps: Vec<WorkflowStep>,
        pub current_step: Option<usize>,
        pub approval_state: ApprovalState,
        pub started_at: DateTime<Utc>,
        pub completed_at: Option<DateTime<Utc>>,
        pub result: Option<WorkflowResult>,
        pub version: u64,
    }

    /// Workflow step
    #[derive(Debug, Clone)]
    pub struct WorkflowStep {
        pub id: Uuid,
        pub step_number: usize,
        pub action: RemediationAction,
        pub status: StepStatus,
        pub started_at: Option<DateTime<Utc>>,
        pub completed_at: Option<DateTime<Utc>>,
        pub output: Option<String>,
        pub error: Option<String>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum StepStatus {
        Pending,
        InProgress,
        Completed,
        Failed,
        Skipped,
    }

    /// Remediation action
    #[derive(Debug, Clone)]
    pub struct RemediationAction {
        pub id: Uuid,
        pub action_type: ActionType,
        pub target: Target,
        pub parameters: ActionParameters,
        pub timeout: Duration,
        pub rollback_action: Option<Box<RemediationAction>>,
    }

    /// Action target
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Target {
        pub service: value::ServiceIdentity,
        pub resource_type: ResourceType,
        pub resource_id: String,
    }

    /// Action parameters
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum ActionParameters {
        RestartParams {
            pod_name: String,
            namespace: String,
        },
        ScaleParams {
            deployment: String,
            replicas: u32,
        },
        ConfigParams {
            config_map: String,
            data: BTreeMap<String, String>,
        },
    }

    /// Approval state
    #[derive(Debug, Clone)]
    pub struct ApprovalState {
        pub required: bool,
        pub required_approvers: u8,
        pub current_approvals: u8,
        pub approvers: HashSet<String>,
        pub status: ApprovalStatus,
        pub deadline: Option<DateTime<Utc>>,
    }

    impl ApprovalState {
        pub fn new(required: bool, required_approvers: u8) -> Self {
            Self {
                required,
                required_approvers,
                current_approvals: 0,
                approvers: HashSet::new(),
                status: ApprovalStatus::Pending,
                deadline: None,
            }
        }

        pub fn add_approval(&mut self, approver: impl Into<String>) -> DomainResult<()> {
            if !self.required {
                return Err(DomainError::InvariantViolation(
                    "Approval not required for this workflow".to_string(),
                ));
            }

            let approver = approver.into();
            if self.approvers.contains(&approver) {
                return Err(DomainError::Conflict(
                    "User already approved this workflow".to_string(),
                ));
            }

            self.approvers.insert(approver);
            self.current_approvals += 1;

            if self.current_approvals >= self.required_approvers {
                self.status = ApprovalStatus::Approved;
            }

            Ok(())
        }

        pub fn is_approved(&self) -> bool {
            !self.required || self.status == ApprovalStatus::Approved
        }
    }

    impl RemediationWorkflow {
        pub fn new(
            id: id::WorkflowId,
            incident_id: id::IncidentId,
            workflow_type: WorkflowType,
            steps: Vec<WorkflowStep>,
        ) -> Self {
            Self {
                id,
                incident_id,
                workflow_type,
                status: WorkflowStatus::PendingApproval,
                steps,
                current_step: None,
                approval_state: ApprovalState::new(true, 1),
                started_at: Utc::now(),
                completed_at: None,
                result: None,
                version: 0,
            }
        }

        pub fn approve(&mut self, approver: impl Into<String>) -> DomainResult<()> {
            self.approval_state.add_approval(approver)?;

            if self.approval_state.is_approved() {
                self.status = WorkflowStatus::Approved;
            }

            self.version += 1;
            Ok(())
        }

        pub fn start(&mut self) -> DomainResult<()> {
            if !self.approval_state.is_approved() {
                return Err(DomainError::InvariantViolation(
                    "Cannot start workflow without approval".to_string(),
                ));
            }

            self.status = WorkflowStatus::InProgress;
            self.current_step = Some(0);
            self.version += 1;
            Ok(())
        }

        pub fn execute_step(&mut self, step_index: usize) -> DomainResult<&WorkflowStep> {
            if self.status != WorkflowStatus::InProgress {
                return Err(DomainError::InvariantViolation(
                    "Workflow is not in progress".to_string(),
                ));
            }

            if self.current_step != Some(step_index) {
                return Err(DomainError::InvariantViolation(
                    "Step is not the current step".to_string(),
                ));
            }

            let step = self.steps.get_mut(step_index)
                .ok_or_else(|| DomainError::NotFound("Step not found".to_string()))?;

            step.status = StepStatus::InProgress;
            step.started_at = Some(Utc::now());

            Ok(step)
        }

        pub fn complete_step(
            &mut self,
            step_index: usize,
            result: Result<String, String>,
        ) -> DomainResult<()> {
            let step = self.steps.get_mut(step_index)
                .ok_or_else(|| DomainError::NotFound("Step not found".to_string()))?;

            match result {
                Ok(output) => {
                    step.status = StepStatus::Completed;
                    step.output = Some(output);
                    step.completed_at = Some(Utc::now());

                    // Move to next step
                    if step_index + 1 < self.steps.len() {
                        self.current_step = Some(step_index + 1);
                    } else {
                        self.complete(WorkflowResult::Success)?;
                    }
                }
                Err(error) => {
                    step.status = StepStatus::Failed;
                    step.error = Some(error);
                    step.completed_at = Some(Utc::now());
                    self.status = WorkflowStatus::Failed;
                }
            }

            self.version += 1;
            Ok(())
        }

        pub fn complete(&mut self, result: WorkflowResult) -> DomainResult<()> {
            self.status = WorkflowStatus::Completed;
            self.result = Some(result);
            self.completed_at = Some(Utc::now());
            self.version += 1;
            Ok(())
        }

        pub fn requires_approval(&self) -> bool {
            self.approval_state.required
        }
    }

    /// Remediation events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum RemediationEvent {
        WorkflowCreated {
            workflow_id: id::WorkflowId,
            incident_id: id::IncidentId,
            workflow_type: WorkflowType,
        },
        WorkflowApproved {
            workflow_id: id::WorkflowId,
            approver: String,
        },
        WorkflowCompleted {
            workflow_id: id::WorkflowId,
            result: WorkflowResult,
        },
        WorkflowFailed {
            workflow_id: id::WorkflowId,
            error: String,
        },
    }
}

// =============================================================================
// Service Topology Context
// =============================================================================

pub mod topology {
    use super::*;

    /// Service types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum ServiceType {
        Microservice,
        Database,
        Cache,
        MessageQueue,
        APIGateway,
        Frontend,
        BatchJob,
        ExternalService,
    }

    /// Dependency types
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum DependencyType {
        Synchronous,
        Asynchronous,
        DataDependency,
        WeakDependency,
    }

    /// Dependency strength
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
    pub enum DependencyStrength {
        Weak,
        Moderate,
        Strong,
    }

    /// Blast radius
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
    pub enum BlastRadius {
        SingleService,
        LocalCluster,
        MultiCluster,
        GlobalOutage,
    }

    /// Latency profile
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LatencyProfile {
        pub p50: Duration,
        pub p95: Duration,
        pub p99: Duration,
    }

    /// Call frequency
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub enum CallFrequency {
        PerSecond(f64),
        PerMinute(f64),
        PerHour(f64),
    }

    /// Service node
    #[derive(Debug, Clone)]
    pub struct ServiceNode {
        pub id: id::ServiceId,
        pub identity: value::ServiceIdentity,
        pub service_type: ServiceType,
        pub health_status: value::HealthStatus,
        pub capabilities: Vec<String>,
        pub metadata: BTreeMap<String, String>,
    }

    /// Dependency edge
    #[derive(Debug, Clone)]
    pub struct DependencyEdge {
        pub id: Uuid,
        pub from_service: id::ServiceId,
        pub to_service: id::ServiceId,
        pub dependency_type: DependencyType,
        pub latency: Option<LatencyProfile>,
        pub call_frequency: Option<CallFrequency>,
        pub strength: DependencyStrength,
    }

    /// Aggregate root: ServiceGraph
    #[derive(Debug, Clone)]
    pub struct ServiceGraph {
        pub id: Uuid,
        pub cluster: String,
        pub services: Vec<ServiceNode>,
        pub dependencies: Vec<DependencyEdge>,
        pub last_updated: DateTime<Utc>,
        pub version: u64,
    }

    impl ServiceGraph {
        pub fn new(id: Uuid, cluster: impl Into<String>) -> Self {
            Self {
                id,
                cluster: cluster.into(),
                services: Vec::new(),
                dependencies: Vec::new(),
                last_updated: Utc::now(),
                version: 0,
            }
        }

        pub fn add_service(&mut self, service: ServiceNode) -> DomainResult<()> {
            if self.services.iter().any(|s| s.id == service.id) {
                return Err(DomainError::Conflict(
                    "Service already exists in graph".to_string(),
                ));
            }
            self.services.push(service);
            self.version += 1;
            Ok(())
        }

        pub fn add_dependency(&mut self, edge: DependencyEdge) -> DomainResult<()> {
            // Check for cycles
            if self.would_create_cycle(&edge) {
                return Err(DomainError::InvariantViolation(
                    "Cannot add dependency: would create a cycle".to_string(),
                ));
            }

            // Validate endpoints exist
            let from_exists = self.services.iter().any(|s| s.id == edge.from_service);
            let to_exists = self.services.iter().any(|s| s.id == edge.to_service);

            if !from_exists || !to_exists {
                return Err(DomainError::NotFound(
                    "One or more services not found in graph".to_string(),
                ));
            }

            self.dependencies.push(edge);
            self.version += 1;
            Ok(())
        }

        pub fn get_downstream_services(&self, service_id: id::ServiceId) -> Vec<&ServiceNode> {
            let mut downstream = Vec::new();
            let mut visited = std::collections::HashSet::new();

            self.traverse_downstream(service_id, &mut downstream, &mut visited);
            downstream
        }

        pub fn get_upstream_services(&self, service_id: id::ServiceId) -> Vec<&ServiceNode> {
            let mut upstream = Vec::new();
            let mut visited = std::collections::HashSet::new();

            self.traverse_upstream(service_id, &mut upstream, &mut visited);
            upstream
        }

        fn traverse_downstream<'a>(
            &'a self,
            service_id: id::ServiceId,
            result: &mut Vec<&'a ServiceNode>,
            visited: &mut std::collections::HashSet<id::ServiceId>,
        ) {
            if visited.contains(&service_id) {
                return;
            }
            visited.insert(service_id);

            if let Some(service) = self.services.iter().find(|s| s.id == service_id) {
                for edge in self.dependencies.iter().filter(|e| e.from_service == service_id) {
                    if let Some(downstream) = self.services.iter().find(|s| s.id == edge.to_service) {
                        result.push(downstream);
                        self.traverse_downstream(edge.to_service, result, visited);
                    }
                }
            }
        }

        fn traverse_upstream<'a>(
            &'a self,
            service_id: id::ServiceId,
            result: &mut Vec<&'a ServiceNode>,
            visited: &mut std::collections::HashSet<id::ServiceId>,
        ) {
            if visited.contains(&service_id) {
                return;
            }
            visited.insert(service_id);

            if let Some(service) = self.services.iter().find(|s| s.id == service_id) {
                for edge in self.dependencies.iter().filter(|e| e.to_service == service_id) {
                    if let Some(upstream) = self.services.iter().find(|s| s.id == edge.from_service) {
                        result.push(upstream);
                        self.traverse_upstream(edge.from_service, result, visited);
                    }
                }
            }
        }

        fn would_create_cycle(&self, new_edge: &DependencyEdge) -> bool {
            // Check if there's already a path from to_service to from_service
            let downstream = self.get_downstream_services(new_edge.to_service);
            downstream.iter().any(|s| s.id == new_edge.from_service)
        }
    }

    /// Impact analysis result
    #[derive(Debug, Clone)]
    pub struct ImpactAnalysis {
        pub id: Uuid,
        pub failed_service: id::ServiceId,
        pub impacted_services: Vec<ImpactedService>,
        pub blast_radius: BlastRadius,
        pub user_impact: UserImpact,
        pub generated_at: DateTime<Utc>,
    }

    #[derive(Debug, Clone)]
    pub struct ImpactedService {
        pub service_id: id::ServiceId,
        pub impact_level: ImpactLevel,
        pub reason: String,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
    pub enum ImpactLevel {
        None,
        Low,
        Medium,
        High,
        Critical,
    }

    #[derive(Debug, Clone)]
    pub struct UserImpact {
        pub affected_users: u64,
        pub affected_regions: Vec<String>,
        pub estimated_duration: Option<Duration>,
    }

    /// Topology events
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum TopologyEvent {
        ServiceDiscovered {
            service_id: id::ServiceId,
            service_name: String,
        },
        ServiceRemoved {
            service_id: id::ServiceId,
        },
        DependencyAdded {
            from_service: id::ServiceId,
            to_service: id::ServiceId,
        },
        TopologyUpdated {
            graph_version: u64,
        },
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_status_transitions() {
        let mut incident = incident::Incident::new(
            id::IncidentId::new_v4(),
            id::IncidentNumber::new("INC-001"),
            "Test incident",
            incident::IncidentSeverity::P2,
        );

        // Valid transitions
        incident.transition_to(incident::IncidentStatus::Detected).unwrap();
        incident.transition_to(incident::IncidentStatus::Analyzing).unwrap();
        incident.transition_to(incident::IncidentStatus::Remediating).unwrap();
        incident.transition_to(incident::IncidentStatus::Resolved).unwrap();
        incident.transition_to(incident::IncidentStatus::Closed).unwrap();

        assert_eq!(incident.status, incident::IncidentStatus::Closed);
    }

    #[test]
    fn test_invalid_incident_transition() {
        let mut incident = incident::Incident::new(
            id::IncidentId::new_v4(),
            id::IncidentNumber::new("INC-002"),
            "Test incident",
            incident::IncidentSeverity::P3,
        );

        let result = incident.transition_to(incident::IncidentStatus::Resolved);
        assert!(matches!(result, Err(DomainError::InvalidTransition { .. })));
    }

    #[test]
    fn test_anomaly_confidence_validation() {
        let result = anomaly::Anomaly::new(
            id::AnomalyId::new_v4(),
            id::DetectorId::new_v4(),
            anomaly::AnomalyType::Spike,
            anomaly::AnomalySeverity::High,
            1.5, // Invalid: > 1.0
        );

        assert!(matches!(result, Err(DomainError::ValidationError(_))));

        let valid_anomaly = anomaly::Anomaly::new(
            id::AnomalyId::new_v4(),
            id::DetectorId::new_v4(),
            anomaly::AnomalyType::Spike,
            anomaly::AnomalySeverity::High,
            0.85, // Valid
        );

        assert!(valid_anomaly.is_ok());
    }

    #[test]
    fn test_approval_state() {
        let mut approval = remediation::ApprovalState::new(true, 2);

        assert!(!approval.is_approved());
        assert_eq!(approval.required_approvers, 2);

        approval.add_approval("user1").unwrap();
        assert!(!approval.is_approved());

        approval.add_approval("user2").unwrap();
        assert!(approval.is_approved());
        assert_eq!(approval.status, remediation::ApprovalStatus::Approved);
    }

    #[test]
    fn test_service_graph_cycle_detection() {
        let mut graph = topology::ServiceGraph::new(Uuid::new_v4(), "test-cluster");

        let svc1 = topology::ServiceNode {
            id: id::ServiceId::new_v4(),
            identity: value::ServiceIdentity::new(
                Uuid::new_v4(),
                "service-a",
                "default",
                "test-cluster",
            ),
            service_type: topology::ServiceType::Microservice,
            health_status: value::HealthStatus::Healthy,
            capabilities: vec![],
            metadata: BTreeMap::new(),
        };

        let svc2 = topology::ServiceNode {
            id: id::ServiceId::new_v4(),
            identity: value::ServiceIdentity::new(
                Uuid::new_v4(),
                "service-b",
                "default",
                "test-cluster",
            ),
            service_type: topology::ServiceType::Microservice,
            health_status: value::HealthStatus::Healthy,
            capabilities: vec![],
            metadata: BTreeMap::new(),
        };

        graph.add_service(svc1.clone()).unwrap();
        graph.add_service(svc2.clone()).unwrap();

        let edge1 = topology::DependencyEdge {
            id: Uuid::new_v4(),
            from_service: svc1.id,
            to_service: svc2.id,
            dependency_type: topology::DependencyType::Synchronous,
            latency: None,
            call_frequency: None,
            strength: topology::DependencyStrength::Strong,
        };

        graph.add_dependency(edge1).unwrap();

        // Try to add reverse edge (would create cycle)
        let edge2 = topology::DependencyEdge {
            id: Uuid::new_v4(),
            from_service: svc2.id,
            to_service: svc1.id,
            dependency_type: topology::DependencyType::Synchronous,
            latency: None,
            call_frequency: None,
            strength: topology::DependencyStrength::Strong,
        };

        let result = graph.add_dependency(edge2);
        assert!(matches!(result, Err(DomainError::InvariantViolation(_))));
    }

    #[test]
    fn test_alert_deduplication() {
        let fingerprint = incident::AlertFingerprint::new("test-fingerprint");
        let mut deduplicator = incident::AlertDeduplicator::new(fingerprint.clone(), Duration::from_secs(300));

        // First check - should not suppress
        assert!(!deduplicator.check(Utc::now()));

        // Increment
        deduplicator.increment().unwrap();
        assert_eq!(deduplicator.count, 2);

        // Check within window - should suppress
        assert!(deduplicator.check(Utc::now()));

        // Suppress
        deduplicator.suppress().unwrap();
        assert_eq!(deduplicator.status, incident::DeduplicationStatus::Suppressed);
    }

    #[test]
    fn test_remediation_workflow_approval() {
        let steps = vec![];
        let mut workflow = remediation::RemediationWorkflow::new(
            id::WorkflowId::new_v4(),
            id::IncidentId::new_v4(),
            remediation::WorkflowType::RestartService,
            steps,
        );

        assert!(workflow.requires_approval());
        assert_eq!(workflow.status, remediation::WorkflowStatus::PendingApproval);

        workflow.approve("admin").unwrap();
        assert_eq!(workflow.status, remediation::WorkflowStatus::Approved);

        // Cannot start without completing approval
        workflow.approval_state.add_approval("admin2").unwrap();

        workflow.start().unwrap();
        assert_eq!(workflow.status, remediation::WorkflowStatus::InProgress);
    }

    #[test]
    fn test_service_identity_qualified_name() {
        let service = value::ServiceIdentity::new(
            Uuid::new_v4(),
            "payment-service",
            "production",
            "us-east-1",
        );

        assert_eq!(service.qualified_name(), "production/payment-service");
    }

    #[test]
    fn test_baseline_anomaly_detection() {
        let stats = anomaly::BaselineStatistics {
            mean: 100.0,
            std_dev: 10.0,
            p50: 98.0,
            p95: 115.0,
            p99: 125.0,
            min: 80.0,
            max: 120.0,
        };

        // Normal value
        assert!(!stats.is_anomaly(105.0, 3.0));

        // Anomaly (> 3 sigma)
        assert!(stats.is_anomaly(140.0, 3.0));
    }
}
