# Bounded Contexts Architecture Design - RustOps AIOps Platform

**Document Version**: 1.0
**Date**: 2026-01-19
**Status**: Architecture Design
**Authors**: System Architecture Team

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Context Map Overview](#context-map-overview)
3. [Common Context (Shared Kernel)](#1-common-context-shared-kernel)
4. [Telemetry Collection Context](#2-telemetry-collection-context)
5. [Anomaly Detection Context](#3-anomaly-detection-context)
6. [Incident Management Context](#4-incident-management-context)
7. [Service Topology Context](#5-service-topology-context)
8. [Integration Context](#6-integration-context)
9. [Knowledge Management Context](#7-knowledge-management-context)
10. [Remediation Context](#8-remediation-context)
11. [Cross-Context Event Matrix](#cross-context-event-matrix)
12. [Implementation Roadmap](#implementation-roadmap)

---

## Executive Summary

This document provides the complete bounded context architecture design for the RustOps AIOps platform. Each bounded context is designed as an independent module with:

- **Clear responsibilities** based on domain capabilities
- **Well-defined boundaries** enforcing separation of concerns
- **Event-driven communication** for loose coupling
- **Aggregate roots** managing consistency boundaries
- **Repository interfaces** for persistence abstraction

### Design Principles

| Principle | Application |
|-----------|-------------|
| **Bounded Context Isolation** | Each context owns its domain model exclusively |
| **Ubiquitous Language** | Shared terminology defined in Common context |
| **Event-Driven Communication** | Contexts communicate via domain events only |
| **Aggregate Consistency** | Transactions bounded to single aggregates |
| **Type-Safe IDs** | Newtype pattern prevents ID mix-ups |

---

## Context Map Overview

```
                    ┌─────────────────────────────────────┐
                    │      RustOps AIOps Platform          │
                    │         (Bounded Contexts)           │
                    └─────────────────────────────────────┘
                                      │
        ┌─────────────────────────────┼─────────────────────────────┐
        │                             │                             │
        ▼                             ▼                             ▼
┌───────────────────┐      ┌───────────────────┐      ┌───────────────────┐
│   Telemetry       │      │   Anomaly         │      │   Incident        │
│   Collection      │──────│   Detection       │──────│   Management      │
│   Context         │      │   Context         │      │   Context         │
└───────────────────┘      └───────────────────┘      └───────────────────┘
        │                             │                             │
        │                             ▼                             │
        │                    ┌───────────────────┐                    │
        │                    │   Service         │                    │
        └────────────────────│   Topology        │────────────────────┘
                             │   Context         │
                             └───────────────────┘
                                      │
        ┌─────────────────────────────┼─────────────────────────────┐
        │                             │                             │
        ▼                             ▼                             ▼
┌───────────────────┐      ┌───────────────────┐      ┌───────────────────┐
│   Remediation     │      │   Integration     │      │   Knowledge       │
│   Context         │      │   Context         │      │   Management      │
└───────────────────┘      └───────────────────┘      └───────────────────┘
```

---

## 1. Common Context (Shared Kernel)

**Status**: Existing (crates/common/)
**Responsibility**: Provides shared types, events, and utilities used across all bounded contexts.

### Core Components

#### Type-Safe IDs (ids.rs)

```rust
// Existing implementation - comprehensive ID types
pub trait IdType: Clone + Copy + PartialEq + Eq + PartialOrd + Ord + Send + Sync + 'static {
    fn new() -> Self;
    fn as_uuid(&self) -> Uuid;
    fn from_str(s: &str) -> Result<Self, uuid::Error> where Self: Sized;
    fn to_string(&self) -> String;
}

// Implemented ID types:
impl_id!(IncidentId, "inc_");
impl_id!(AlertId, "alt_");
impl_id!(ServiceId, "svc_");
impl_id!(MetricId, "mtr_");
impl_id!(TraceId, "trc_");
impl_id!(SpanId, "spn_");
impl_id!(AnomalyId, "anm_");
impl_id!(CorrelationId, "cor_");
impl_id!(UserId, "usr_");
impl_id!(ResourceId, "res_");

// Additional IDs needed for full DDD implementation:
impl_id!(DetectorId, "det_");
impl_id!(WorkflowId, "wrk_");
impl_id!(ActionId, "act_");
impl_id!(ApprovalId, "apr_");
impl_id!(GraphId, "grf_");
impl_id!(EdgeId, "edg_");
impl_id!(AnalysisId, "ana_");
impl_id!(SystemId, "sys_");
impl_id!(MappingId, "map_");
impl_id!(JobId, "job_");
impl_id!(EntryId, "ent_");
impl_id!(PatternId, "ptn_");
impl_id!(KnowledgeBaseId, "knw_");
```

#### Domain Events (events.rs)

```rust
// Existing events - expand with additional types
pub enum EventType {
    // Telemetry events
    TelemetryReceived,
    MetricsCollected,
    LogsReceived,
    TraceReceived,

    // Anomaly events
    AnomalyDetected,
    AnomalyConfirmed,
    AnomalyResolved,
    BaselineUpdated,
    RootCauseIdentified,

    // Alert/Incident events
    AlertCreated,
    AlertUpdated,
    AlertResolved,
    IncidentCreated,
    IncidentUpdated,
    IncidentResolved,
    IncidentsCorrelated,
    AlertDeduplicated,
    RemediationRequested,

    // Topology events
    ServiceAdded,
    ServiceRemoved,
    ServiceUpdated,
    DependencyDiscovered,
    DependencyRemoved,
    TopologyUpdated,
    ImpactAnalysisCompleted,

    // Remediation events
    WorkflowCreated,
    WorkflowApproved,
    WorkflowRejected,
    WorkflowStepStarted,
    WorkflowStepCompleted,
    WorkflowCompleted,
    WorkflowFailed,
    WorkflowRolledBack,

    // Integration events
    SystemConnected,
    SystemDisconnected,
    MappingCreated,
    SyncCompleted,
    SyncFailed,
    ConflictDetected,

    // Knowledge events
    EntryCreated,
    EntryUpdated,
    EntryAccessed,
    PatternLearned,
    PatternValidated,
    EmbeddingGenerated,
}
```

#### Common Value Objects

```rust
// Service Identity - shared across contexts
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceIdentity {
    pub id: ServiceId,
    pub name: String,
    pub namespace: String,
    pub cluster: String,
    pub environment: Environment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

// Health Status - shared across contexts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

// Time window for aggregations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeWindow {
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
}

impl TimeWindow {
    pub fn as_duration(&self) -> Duration {
        match self {
            TimeWindow::Seconds(n) => Duration::from_secs(*n),
            TimeWindow::Minutes(n) => Duration::from_secs(*n * 60),
            TimeWindow::Hours(n) => Duration::from_secs(*n * 3600),
            TimeWindow::Days(n) => Duration::from_secs(*n * 86400),
        }
    }
}
```

### Domain Events to Publish

None - Common context provides infrastructure, does not publish domain events.

### Domain Events to Subscribe

None - Common context provides infrastructure, does not subscribe to events.

### Repository Interfaces

```rust
// Repository trait - base for all aggregates
#[async_trait::async_trait]
pub trait Repository<A: Aggregate>: Send + Sync {
    async fn get(&self, id: &A::Id) -> Result<Option<A>, Error>;
    async fn save(&self, aggregate: &A) -> Result<(), Error>;
    async fn delete(&self, id: &A::Id) -> Result<(), Error>;
    async fn exists(&self, id: &A::Id) -> Result<bool, Error>;
}

// Aggregate trait - base for all aggregates
pub trait Aggregate {
    type Id: IdType;
    type Event: DomainEventTrait;

    fn id(&self) -> &Self::Id;
    fn version(&self) -> u64;
    fn apply_event(&mut self, event: Self::Event) -> Result<(), Error>;
}

// Domain event trait
pub trait DomainEventTrait: Send + Sync + Clone {
    fn event_id(&self) -> Uuid;
    fn event_type(&self) -> &'static str;
    fn occurred_at(&self) -> DateTime<Utc>;
    fn causation_id(&self) -> Option<Uuid>;
    fn correlation_id(&self) -> Option<Uuid>;
}
```

### Aggregate Roots

None - Common context provides infrastructure only.

---

## 2. Telemetry Collection Context

**Status**: Exists (crates/telemetry/) - needs completion
**Responsibility**: Ingest, normalize, and distribute telemetry data from all sources.

### Aggregate Roots

| Aggregate | Description | File |
|-----------|-------------|------|
| `TelemetrySource` | Manages connection lifecycle and collection state | `sources.rs` |
| `MetricBatch` | Groups metrics for efficient storage | `collectors.rs` |

### Domain Events to Publish

| Event | Payload | Frequency | Consumers |
|-------|---------|-----------|-----------|
| `MetricsCollected` | `source_id, metric_count, window` | Very High (1M/min) | Anomaly Detection |
| `LogsReceived` | `source_id, log_count, window` | High (100K/min) | Anomaly Detection |
| `TraceReceived` | `trace_id, span_count, service_id` | Medium (10K/min) | Anomaly Detection, Topology |
| `SourceConnected` | `source_id, source_type` | Low (on change) | All Contexts |
| `SourceDisconnected` | `source_id, reason` | Low (on error) | Integration |

### Domain Events to Subscribe

| Event | Producer | Handler |
|-------|----------|---------|
| `ServiceAdded` | Topology | Register new telemetry source |
| `ServiceRemoved` | Topology | Unregister telemetry source |

### Repository Interfaces

```rust
#[async_trait::async_trait]
pub trait TelemetrySourceRepository: Repository<TelemetrySource> {
    async fn find_by_type(&self, source_type: SourceType) -> Result<Vec<TelemetrySource>, Error>;
    async fn find_active(&self) -> Result<Vec<TelemetrySource>, Error>;
    async fn update_metrics_collected(&self, id: &TelemetrySourceId, count: u64) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait MetricDataRepository {
    async fn store_batch(&self, metrics: Vec<MetricData>) -> Result<(), Error>;
    async fn query(&self, query: MetricQuery) -> Result<Vec<MetricData>, Error>;
    async fn delete_before(&self, timestamp: DateTime<Utc>) -> Result<u64, Error>;
}

#[async_trait::async_trait]
pub trait LogEntryRepository {
    async fn store_batch(&self, logs: Vec<LogEntry>) -> Result<(), Error>;
    async fn query(&self, query: LogQuery) -> Result<Vec<LogEntry>, Error>;
}

#[async_trait::async_trait]
pub trait TraceSpanRepository {
    async fn store_batch(&self, spans: Vec<TraceSpan>) -> Result<(), Error>;
    async fn get_trace(&self, trace_id: &TraceId) -> Result<Vec<TraceSpan>, Error>;
}
```

### Value Objects

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricName(String);

impl MetricName {
    pub fn new(name: String) -> Result<Self, Error> {
        if name.is_empty() || name.len() > 255 {
            return Err(Error::Validation("Invalid metric name".to_string()));
        }
        // Validate metric name format (e.g., prometheus conventions)
        if name.chars().any(|c| !c.is_alphanumeric() && c != '_' && c != ':') {
            return Err(Error::Validation("Invalid metric name format".to_string()));
        }
        Ok(Self(name))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(f64),
    Gauge(f64),
    Histogram(HistogramData),
    Summary(SummaryData),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistogramData {
    pub count: u64,
    pub sum: f64,
    pub buckets: Vec<(f64, u64)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Labels(pub BTreeMap<String, String>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    Prometheus,
    CloudWatch,
    Datadog,
    OpenTelemetry,
    Fluentd,
    StatsD,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}
```

### Invariants

1. Metric timestamps must be monotonically increasing per source
2. Log levels must be valid enum values
3. Trace spans must have valid parent-child relationships
4. Source connections must be mutually exclusive (connected/disconnected)

---

## 3. Anomaly Detection Context

**Status**: Exists (crates/anomaly/) - needs ONNX integration
**Responsibility**: Detect anomalous behavior using statistical and ML techniques.

### Aggregate Roots

| Aggregate | Description | File |
|-----------|-------------|------|
| `AnomalyDetector` | Manages detection lifecycle and baseline | `detector.rs` |
| `Anomaly` | Represents a single detected anomaly | `models.rs` |
| `RootCauseAnalysis` | Manages hypothesis generation | `models.rs` (to be added) |

### Domain Events to Publish

| Event | Payload | Frequency | Consumers |
|-------|---------|-----------|-----------|
| `AnomalyDetected` | `anomaly_id, detector_id, severity, confidence` | High (1K/min) | Incident Management |
| `AnomalyConfirmed` | `anomaly_id, features, root_cause` | Medium (100/min) | Incident Management |
| `AnomalyResolved` | `anomaly_id, duration, resolved_by` | Low (10/min) | Incident Management |
| `BaselineUpdated` | `metric_name, period, statistics` | Low (1/min) | Telemetry |
| `RootCauseIdentified` | `anomaly_id, hypothesis, confidence` | Medium (50/min) | Incident Management, Knowledge |

### Domain Events to Subscribe

| Event | Producer | Handler |
|-------|----------|---------|
| `MetricsCollected` | Telemetry | Process metrics for detection |
| `LogsReceived` | Telemetry | Analyze log patterns |
| `TraceReceived` | Telemetry | Detect trace anomalies |
| `PatternLearned` | Knowledge | Update detection patterns |

### Repository Interfaces

```rust
#[async_trait::async_trait]
pub trait AnomalyDetectorRepository: Repository<AnomalyDetector> {
    async fn find_by_metric(&self, metric_name: &MetricName) -> Result<Vec<AnomalyDetector>, Error>;
    async fn find_active(&self) -> Result<Vec<AnomalyDetector>, Error>;
    async fn update_detection_count(&self, id: &DetectorId, count: u64) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait AnomalyRepository: Repository<Anomaly> {
    async fn find_by_detector(&self, detector_id: &DetectorId) -> Result<Vec<Anomaly>, Error>;
    async fn find_active(&self) -> Result<Vec<Anomaly>, Error>;
    async fn find_by_severity(&self, severity: AnomalySeverity) -> Result<Vec<Anomaly>, Error>;
    async fn find_by_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<Anomaly>, Error>;
}

#[async_trait::async_trait]
pub trait BaselineRepository {
    async fn get(&self, metric_name: &MetricName) -> Result<Option<Baseline>, Error>;
    async fn save(&self, baseline: &Baseline) -> Result<(), Error>;
    async fn update_statistics(&self, metric_name: &MetricName, stats: BaselineStatistics) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait RootCauseRepository: Repository<RootCauseHypothesis> {
    async fn find_by_anomaly(&self, anomaly_id: &AnomalyId) -> Result<Vec<RootCauseHypothesis>, Error>;
    async fn find_high_confidence(&self, threshold: f64) -> Result<Vec<RootCauseHypothesis>, Error>;
}
```

### Value Objects

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectorType {
    StatisticalThreshold,
    MovingAverage,
    ExponentialSmoothing,
    LSTM,          // ONNX
    IsolationForest, // ONNX
    DBSCAN,        // ONNX
    Autoencoder,   // ONNX
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    Spike,
    Drop,
    TrendChange,
    PatternBreak,
    SeasonalityViolation,
    NewErrorPattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineStatistics {
    pub mean: f64,
    pub std_dev: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub min: f64,
    pub max: f64,
    pub sample_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyFeatures {
    pub magnitude: f64,
    pub duration: Duration,
    pub affected_services: Vec<ServiceId>,
    pub correlation_score: f64,
    pub seasonality_deviation: Option<f64>,
}

// ONNX Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnnxModelConfig {
    pub model_path: String,
    pub input_name: String,
    pub output_name: String,
    pub model_type: DetectorType,
    pub inference_threads: usize,
}
```

### Invariants

1. Confidence scores must be between 0.0 and 1.0
2. Anomaly severity must be proportional to confidence
3. Baseline periods must be at least 30 minutes
4. Root cause hypotheses must have at least one causal factor

---

## 4. Incident Management Context

**Status**: Exists (crates/incident/) - working
**Responsibility**: Correlate anomalies, deduplicate alerts, and manage incident lifecycle.

### Aggregate Roots

| Aggregate | Description | File |
|-----------|-------------|------|
| `Incident` | Manages incident lifecycle and state transitions | `incident.rs` |
| `CorrelationGroup` | Groups related incidents for unified handling | `correlation.rs` |
| `AlertDeduplicator` | Tracks and suppresses duplicate alerts | `deduplication.rs` |

### Domain Events to Publish

| Event | Payload | Frequency | Consumers |
|-------|---------|-----------|-----------|
| `IncidentCreated` | `incident_id, incident_number, severity, anomalies` | Medium (100/min) | Remediation, Integration |
| `IncidentAssigned` | `incident_id, assigned_to` | Medium (100/min) | Integration |
| `IncidentStatusChanged` | `incident_id, old_status, new_status` | Medium (100/min) | Integration, Knowledge |
| `IncidentsCorrelated` | `correlation_group_id, incident_ids, root_cause` | Medium (50/min) | Remediation |
| `AlertDeduplicated` | `alert_id, deduplication_count` | High (1K/min) | Telemetry |
| `RemediationRequested` | `incident_id, severity, suggested_workflow` | Medium (100/min) | Remediation |

### Domain Events to Subscribe

| Event | Producer | Handler |
|-------|----------|---------|
| `AnomalyDetected` | Anomaly Detection | Create incident from anomaly |
| `AnomalyConfirmed` | Anomaly Detection | Add to existing incident |
| `AnomalyResolved` | Anomaly Detection | Update incident status |
| `RootCauseIdentified` | Anomaly Detection | Update incident with root cause |
| `ImpactAnalysisCompleted` | Topology | Correlate by impact |
| `WorkflowCompleted` | Remediation | Update incident status |
| `ServiceAdded` | Topology | Create incident context |

### Repository Interfaces

```rust
// Event-sourced repository for incidents
#[async_trait::async_trait]
pub trait IncidentRepository: Repository<Incident> {
    async fn get_by_number(&self, number: &IncidentNumber) -> Result<Option<Incident>, Error>;
    async fn find_by_severity(&self, severity: IncidentSeverity) -> Result<Vec<Incident>, Error>;
    async fn find_by_status(&self, status: IncidentStatus) -> Result<Vec<Incident>, Error>;
    async fn find_by_assigned_user(&self, user_id: &UserId) -> Result<Vec<Incident>, Error>;
    async fn find_active(&self) -> Result<Vec<Incident>, Error>;
    async fn get_events(&self, id: &IncidentId) -> Result<Vec<IncidentEvent>, Error>;
    async fn replay_from_events(&self, id: &IncidentId) -> Result<Incident, Error>;
}

#[async_trait::async_trait]
pub trait CorrelationGroupRepository: Repository<CorrelationGroup> {
    async fn find_by_incident(&self, incident_id: &IncidentId) -> Result<Option<CorrelationGroup>, Error>;
    async fn find_active(&self) -> Result<Vec<CorrelationGroup>, Error>;
    async fn add_incident(&self, group_id: &CorrelationGroupId, incident_id: &IncidentId) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait AlertDeduplicatorRepository {
    async fn get(&self, fingerprint: &AlertFingerprint) -> Result<Option<AlertDeduplicator>, Error>;
    async fn save(&self, deduplicator: &AlertDeduplicator) -> Result<(), Error>;
    async fn update_count(&self, fingerprint: &AlertFingerprint, count: u64) -> Result<(), Error>;
    async fn cleanup_old(&self, before: DateTime<Utc>) -> Result<u64, Error>;
}
```

### Value Objects

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncidentStatus {
    New,
    Detected,
    Analyzing,
    Remediating,
    Resolved,
    Closed,
}

// State transition validation
impl IncidentStatus {
    pub fn can_transition_to(&self, new: &IncidentStatus) -> bool {
        match (self, new) {
            (IncidentStatus::New, IncidentStatus::Detected) => true,
            (IncidentStatus::Detected, IncidentStatus::Analyzing) => true,
            (IncidentStatus::Analyzing, IncidentStatus::Remediating) => true,
            (IncidentStatus::Analyzing, IncidentStatus::Resolved) => true,
            (IncidentStatus::Remediating, IncidentStatus::Resolved) => true,
            (IncidentStatus::Remediating, IncidentStatus::Analyzing) => true,
            (IncidentStatus::Resolved, IncidentStatus::Closed) => true,
            (IncidentStatus::Resolved, IncidentStatus::Analyzing) => true, // Reopened
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncidentSeverity {
    P1, // Critical
    P2, // High
    P3, // Medium
    P4, // Low
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorrelationStrategy {
    ByTimeWindow { minutes: u64 },
    ByTopology { max_hops: usize },
    ByCausalGraph,
    ByServiceImpact,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AlertFingerprint(String);

impl AlertFingerprint {
    pub fn from_alert(alert: &Alert) -> Self {
        // Generate fingerprint from alert properties
        let fingerprint = format!(
            "{}:{}:{}:{}",
            alert.service_id,
            alert.alert_type,
            alert.severity,
            alert.fingerprint_key()
        );
        Self(md5_compute(fingerprint))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeduplicationStatus {
    Active,
    Suppressed,
    Resolved,
}
```

### Invariants

1. Incident numbers must be unique and monotonically increasing
2. Correlation groups must have at least 2 incidents
3. Alert deduplication window is 5 minutes
4. Incidents must follow valid status transitions
5. P1 incidents require immediate assignment

---

## 5. Service Topology Context

**Status**: Exists as skeleton (crates/topology/) - needs implementation
**Responsibility**: Discover, map, and analyze service dependencies.

### Aggregate Roots

| Aggregate | Description | File |
|-----------|-------------|------|
| `ServiceGraph` | Manages the complete dependency graph | `graph.rs` |
| `ServiceNode` | Represents individual service state | `graph.rs` |
| `ImpactAnalysis` | Computes downstream impact | `discovery.rs` |

### Domain Events to Publish

| Event | Payload | Frequency | Consumers |
|-------|---------|-----------|-----------|
| `ServiceDiscovered` | `service_id, identity, capabilities` | Low (on change) | Telemetry, All Contexts |
| `ServiceRemoved` | `service_id, reason` | Low (on change) | All Contexts |
| `ServiceUpdated` | `service_id, changes` | Low (on change) | All Contexts |
| `DependencyAdded` | `from_service, to_service, type` | Medium (1/min) | All Contexts |
| `DependencyRemoved` | `from_service, to_service` | Low (on change) | All Contexts |
| `TopologyUpdated` | `graph_id, version, changes` | Low (1/min) | All Contexts |
| `ImpactAnalysisCompleted` | `analysis_id, failed_service, blast_radius` | Medium (50/min) | Incident Management |

### Domain Events to Subscribe

| Event | Producer | Handler |
|-------|----------|---------|
| `TraceReceived` | Telemetry | Extract dependencies |
| `MetricsCollected` | Telemetry | Update service health |
| `IncidentCreated` | Incident Management | Trigger impact analysis |

### Repository Interfaces

```rust
#[async_trait::async_trait]
pub trait ServiceGraphRepository: Repository<ServiceGraph> {
    async fn get_current(&self, cluster: &str) -> Result<Option<ServiceGraph>, Error>;
    async fn get_version(&self, id: &GraphId, version: u64) -> Result<Option<ServiceGraph>, Error>;
    async fn save_version(&self, graph: &ServiceGraph) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait ServiceNodeRepository: Repository<ServiceNode> {
    async fn find_by_cluster(&self, cluster: &str) -> Result<Vec<ServiceNode>, Error>;
    async fn find_by_type(&self, service_type: ServiceType) -> Result<Vec<ServiceNode>, Error>;
    async fn find_by_health(&self, health: HealthStatus) -> Result<Vec<ServiceNode>, Error>;
    async fn update_health(&self, id: &ServiceId, health: HealthStatus) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait DependencyEdgeRepository {
    async fn find_from_service(&self, service_id: &ServiceId) -> Result<Vec<DependencyEdge>, Error>;
    async fn find_to_service(&self, service_id: &ServiceId) -> Result<Vec<DependencyEdge>, Error>;
    async fn find_path(&self, from: &ServiceId, to: &ServiceId, max_hops: usize) -> Result<Vec<ServiceId>, Error>;
    async fn add_edge(&self, edge: &DependencyEdge) -> Result<(), Error>;
    async fn remove_edge(&self, id: &EdgeId) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait ImpactAnalysisRepository: Repository<ImpactAnalysis> {
    async fn find_by_service(&self, service_id: &ServiceId) -> Result<Vec<ImpactAnalysis>, Error>;
    async fn find_recent(&self, since: DateTime<Utc>) -> Result<Vec<ImpactAnalysis>, Error>;
    async fn save(&self, analysis: &ImpactAnalysis) -> Result<(), Error>;
}
```

### Value Objects

```rust
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
    CronJob,
    DaemonSet,
    StatefulSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    Synchronous,   // HTTP, gRPC
    Asynchronous,  // Message queue
    DataDependency, // Database, cache
    WeakDependency, // Optional
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyStrength {
    Strong,   // Required for operation
    Moderate, // Degrades without it
    Weak,     // Nice to have
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LatencyProfile {
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub max: Duration,
    pub sample_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallFrequency {
    PerSecond(f64),
    PerMinute(f64),
    PerHour(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlastRadius {
    SingleService,
    LocalCluster,
    MultiCluster,
    GlobalOutage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserImpact {
    None,
    Low,      // <1% users affected
    Medium,   // 1-10% users affected
    High,     // 10-50% users affected
    Critical, // >50% users affected
}
```

### Invariants

1. Dependency edges must reference valid service nodes
2. Graph must remain acyclic (no circular dependencies detected)
3. Latency profiles must be updated every 5 minutes
4. Services without dependencies are "leaf nodes"
5. Impact analysis must complete within 10 seconds

---

## 6. Integration Context

**Status**: Exists as skeleton (crates/integration/) - needs adapters
**Responsibility**: Synchronize with external ITSM and monitoring systems.

### Aggregate Roots

| Aggregate | Description | File |
|-----------|-------------|------|
| `ExternalSystem` | Manages connection and sync state | `adapter.rs` |
| `EntityMapping` | Tracks bidirectional entity mappings | `mappings.rs` (to be added) |
| `SyncJob` | Orchestrates synchronization tasks | `sync.rs` (to be added) |

### Domain Events to Publish

| Event | Payload | Frequency | Consumers |
|-------|---------|-----------|-----------|
| `SystemConnected` | `system_id, system_type` | Low (on change) | All Contexts |
| `SystemDisconnected` | `system_id, reason` | Low (on error) | All Contexts |
| `MappingCreated` | `mapping_id, internal_id, external_id` | Medium (on creation) | All Contexts |
| `SyncCompleted` | `job_id, system_id, entities_processed` | Medium (30s) | All Contexts |
| `SyncFailed` | `job_id, system_id, errors` | Low (on error) | All Contexts |
| `ConflictDetected` | `mapping_id, internal_state, external_state` | Low (on conflict) | Incident Management |

### Domain Events to Subscribe

| Event | Producer | Handler |
|-------|----------|---------|
| `IncidentCreated` | Incident Management | Sync to ITSM |
| `IncidentStatusChanged` | Incident Management | Update ITSM ticket |
| `IncidentAssigned` | Incident Management | Update ITSM assignment |
| `IncidentResolved` | Incident Management | Close ITSM ticket |

### Repository Interfaces

```rust
#[async_trait::async_trait]
pub trait ExternalSystemRepository: Repository<ExternalSystem> {
    async fn find_by_type(&self, system_type: SystemType) -> Result<Vec<ExternalSystem>, Error>;
    async fn find_active(&self) -> Result<Vec<ExternalSystem>, Error>;
    async fn update_sync_state(&self, id: &SystemId, state: SyncState) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait EntityMappingRepository: Repository<EntityMapping> {
    async fn find_internal_to_external(&self, internal_id: &InternalEntityId, system_id: &SystemId) -> Result<Option<EntityMapping>, Error>;
    async fn find_external_to_internal(&self, external_id: &ExternalEntityId, system_id: &SystemId) -> Result<Option<EntityMapping>, Error>;
    async fn find_by_type(&self, mapping_type: MappingType) -> Result<Vec<EntityMapping>, Error>;
}

#[async_trait::async_trait]
pub trait SyncJobRepository: Repository<SyncJob> {
    async fn find_pending(&self) -> Result<Vec<SyncJob>, Error>;
    async fn find_by_system(&self, system_id: &SystemId) -> Result<Vec<SyncJob>, Error>;
    async fn update_status(&self, id: &JobId, status: JobStatus) -> Result<(), Error>;
}
```

### Value Objects

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemType {
    ServiceNow,
    Jira,
    PagerDuty,
    OpsGenie,
    Slack,
    MSTeams,
    Datadog,
    Dynatrace,
    NewRelic,
    Splunk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncState {
    InSync,
    PendingSync,
    Syncing,
    Conflict,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MappingType {
    IncidentToTicket,
    ServiceToCI,
    UserToContact,
    AlertToEvent,
    MetricToMetric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncDirection {
    Bidirectional,
    InboundOnly,
    OutboundOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncError {
    pub entity_id: String,
    pub error_type: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}
```

### Anti-Corruption Layer (ACL) Adapters

```rust
// ACL Trait - base for all external system adapters
#[async_trait::async_trait]
pub trait AntiCorruptionLayer<D, E>: Send + Sync {
    async fn domain_to_external(&self, domain: &D) -> Result<E, IntegrationError>;
    async fn external_to_domain(&self, external: &E) -> Result<D, IntegrationError>;
}

// ServiceNow Adapter
pub struct ServiceNowAdapter {
    client: ServiceNowClient,
    mapper: ServiceNowMapper,
}

#[async_trait::async_trait]
impl AntiCorruptionLayer<Incident, ServiceNowTicket> for ServiceNowAdapter {
    async fn domain_to_external(&self, incident: &Incident) -> Result<ServiceNowTicket, IntegrationError> {
        Ok(self.mapper.incident_to_ticket(incident))
    }

    async fn external_to_domain(&self, ticket: &ServiceNowTicket) -> Result<Incident, IntegrationError> {
        Ok(self.mapper.ticket_to_incident(ticket))
    }
}

// PagerDuty Adapter
pub struct PagerDutyAdapter {
    client: PagerDutyClient,
    mapper: PagerDutyMapper,
}

// Jira Adapter
pub struct JiraAdapter {
    client: JiraClient,
    mapper: JiraMapper,
}
```

### Invariants

1. Entity mappings must be unique per external system
2. Sync jobs must have a maximum timeout of 10 minutes
3. ACL transformations must preserve data semantics
4. Conflicts must be resolved within 1 hour
5. Connection failures trigger retry with exponential backoff

---

## 7. Knowledge Management Context

**Status**: Exists as skeleton (crates/knowledge/) - needs vector search
**Responsibility**: Store, retrieve, and learn from remediation patterns.

### Aggregate Roots

| Aggregate | Description | File |
|-----------|-------------|------|
| `KnowledgeBase` | Manages knowledge storage and retrieval | `memory.rs` |
| `KnowledgeEntry` | Represents a single knowledge entry | `runbooks.rs` |
| `LearnedPattern` | Represents extracted remediation patterns | `patterns.rs` |

### Domain Events to Publish

| Event | Payload | Frequency | Consumers |
|-------|---------|-----------|-----------|
| `EntryCreated` | `entry_id, entry_type, title` | Low (on creation) | All Contexts |
| `EntryUpdated` | `entry_id, changes` | Low (on update) | All Contexts |
| `EntryAccessed` | `entry_id, access_type` | Medium (on access) | None (metrics only) |
| `PatternLearned` | `pattern_id, pattern_type, source_incident` | Very Low (1/hour) | Anomaly Detection, Remediation |
| `PatternValidated` | `pattern_id, success_rate, confidence` | Low (on validation) | Remediation |
| `EmbeddingGenerated` | `entry_id, model, dimension` | Low (on creation) | None (internal) |

### Domain Events to Subscribe

| Event | Producer | Handler |
|-------|----------|---------|
| `WorkflowCompleted` | Remediation | Learn from successful remediation |
| `WorkflowFailed` | Remediation | Learn from failures |
| `IncidentResolved` | Incident Management | Extract resolution patterns |
| `RootCauseIdentified` | Anomaly Detection | Update detection patterns |
| `AnomalyDetected` | Anomaly Detection | Suggest similar patterns |

### Repository Interfaces

```rust
#[async_trait::async_trait]
pub trait KnowledgeBaseRepository: Repository<KnowledgeBase> {
    async fn find_by_name(&self, name: &str) -> Result<Option<KnowledgeBase>, Error>;
    async fn find_active(&self) -> Result<Vec<KnowledgeBase>, Error>;
    async fn update_indexing_state(&self, id: &KnowledgeBaseId, state: IndexingState) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait KnowledgeEntryRepository: Repository<KnowledgeEntry> {
    async fn find_by_type(&self, entry_type: EntryType) -> Result<Vec<KnowledgeEntry>, Error>;
    async fn find_by_tags(&self, tags: &[String]) -> Result<Vec<KnowledgeEntry>, Error>;
    async fn search_fulltext(&self, query: &str) -> Result<Vec<KnowledgeEntry>, Error>;
    async fn search_semantic(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<(EntryId, f64)>, Error>;
    async fn update_usage_stats(&self, id: &EntryId, access_type: AccessType) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait LearnedPatternRepository: Repository<LearnedPattern> {
    async fn find_by_type(&self, pattern_type: PatternType) -> Result<Vec<LearnedPattern>, Error>;
    async fn find_high_confidence(&self, threshold: f64) -> Result<Vec<LearnedPattern>, Error>;
    async fn find_by_trigger(&self, trigger: &TriggerCondition) -> Result<Vec<LearnedPattern>, Error>;
    async fn update_success_rate(&self, id: &PatternId, success: bool) -> Result<(), Error>;
}
```

### Vector Embedding Infrastructure

```rust
// Vector embedding store with HNSW indexing
pub struct VectorEmbeddingStore {
    index: HnswIndex,
    dimension: usize,
    model: EmbeddingModel,
}

impl VectorEmbeddingStore {
    pub async fn insert(&self, entry_id: EntryId, embedding: Vec<f32>) -> Result<(), Error> {
        // Validate embedding dimension
        if embedding.len() != self.dimension {
            return Err(Error::Validation("Embedding dimension mismatch".to_string()));
        }

        // Normalize to unit vector
        let normalized = Self::normalize(&embedding);

        // Insert into HNSW index
        self.index.insert(entry_id, normalized)?;

        Ok(())
    }

    pub async fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(EntryId, f64)>, Error> {
        let query_normalized = Self::normalize(query);
        self.index.search(&query_normalized, top_k)
    }

    fn normalize(vector: &[f32]) -> Vec<f32> {
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        vector.iter().map(|x| x / norm).collect()
    }
}

// Embedding model wrapper
pub enum EmbeddingModel {
    Onnx(OnnxEmbeddingModel),
    Api(ApiEmbeddingModel),
    Hybrid(HybridEmbeddingModel),
}

impl EmbeddingModel {
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>, Error> {
        match self {
            EmbeddingModel::Onnx(model) => model.embed(text).await,
            EmbeddingModel::Api(model) => model.embed(text).await,
            EmbeddingModel::Hybrid(model) => model.embed(text).await,
        }
    }
}
```

### Value Objects

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    Runbook,
    KnownIssue,
    Pattern,
    Playbook,
    TroubleshootingGuide,
    PostMortem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntryContent {
    Markdown(String),
    Structured(StructuredContent),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredContent {
    pub summary: String,
    pub symptoms: Vec<String>,
    pub diagnosis: String,
    pub steps: Vec<Step>,
    pub verification: VerificationCriteria,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    AnomalyResponse,
    IncidentResponse,
    ProactiveRemediation,
    ScalingPattern,
    FailoverPattern,
    RollbackPattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexingState {
    NotIndexed,
    Indexing,
    Indexed,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessType {
    View,
    Apply,
    Validate,
    Reference,
}
```

### SONA (Self-Optimizing Neural Architecture) Components

```rust
// SONA learning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonaConfig {
    pub learning_rate: f64,
    pub ewc_lambda: f64,    // Elastic Weight Consolidation
    pub replay_buffer_size: usize,
    pub consolidation_interval: Duration,
}

// Learned pattern with SONA metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    pub id: PatternId,
    pub pattern_type: PatternType,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub remediation_steps: Vec<RemediationStep>,
    pub success_rate: f64,
    pub confidence_score: f64,
    pub times_used: u64,
    pub last_success: Option<DateTime<Utc>>,

    // SONA-specific fields
    pub trajectory_id: Option<Uuid>,
    pub ewc_importance: Option<HashMap<String, f64>>,
    pub consolidation_count: u64,
}
```

### Invariants

1. Embeddings must be normalized (unit vectors)
2. Pattern confidence must be > 0.7 for auto-use
3. Entries must be indexed within 1 second of creation
4. Usage stats must update on each access
5. SONA consolidation occurs after every 100 successful applications

---

## 8. Remediation Context

**Status**: Exists as skeleton (crates/remediation/) - needs workflows
**Responsibility**: Orchestrate self-healing workflows and manual remediation actions.

### Aggregate Roots

| Aggregate | Description | File |
|-----------|-------------|------|
| `RemediationWorkflow` | Orchestrates multi-step remediation | `workflow.rs` |
| `ApprovalGate` | Manages approval process | `policy.rs` |
| `ExecutionLog` | Tracks action execution history | `activity.rs` |

### Domain Events to Publish

| Event | Payload | Frequency | Consumers |
|-------|---------|-----------|-----------|
| `WorkflowCreated` | `workflow_id, incident_id, type, requires_approval` | Medium (100/min) | Integration |
| `WorkflowApproved` | `workflow_id, approvers` | Medium (50/min) | Integration |
| `WorkflowRejected` | `workflow_id, reason` | Low (10/min) | Integration |
| `WorkflowStepStarted` | `workflow_id, step_number, action` | Medium (100/min) | Integration |
| `WorkflowStepCompleted` | `workflow_id, step_number, result` | Medium (100/min) | Integration |
| `WorkflowCompleted` | `workflow_id, incident_id, result, duration` | Medium (50/min) | Knowledge, Incident Management |
| `WorkflowFailed` | `workflow_id, error, failed_step` | Low (10/min) | Incident Management, Knowledge |
| `WorkflowRolledBack` | `workflow_id, rollback_actions` | Low (5/min) | Incident Management, Knowledge |

### Domain Events to Subscribe

| Event | Producer | Handler |
|-------|----------|---------|
| `RemediationRequested` | Incident Management | Create workflow |
| `IncidentCreated` | Incident Management | Suggest workflow |
| `PatternValidated` | Knowledge | Use validated pattern |
| `ImpactAnalysisCompleted` | Topology | Adjust workflow based on impact |
| `ApprovalGranted` | Integration | Start workflow |

### Repository Interfaces

```rust
#[async_trait::async_trait]
pub trait RemediationWorkflowRepository: Repository<RemediationWorkflow> {
    async fn find_by_incident(&self, incident_id: &IncidentId) -> Result<Vec<RemediationWorkflow>, Error>;
    async fn find_by_status(&self, status: WorkflowStatus) -> Result<Vec<RemediationWorkflow>, Error>;
    async fn find_pending_approval(&self) -> Result<Vec<RemediationWorkflow>, Error>;
    async fn update_status(&self, id: &WorkflowId, status: WorkflowStatus) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait ApprovalGateRepository: Repository<ApprovalGate> {
    async fn find_by_workflow(&self, workflow_id: &WorkflowId) -> Result<Vec<ApprovalGate>, Error>;
    async fn find_pending(&self, approver_id: &UserId) -> Result<Vec<ApprovalGate>, Error>;
    async fn add_approval(&self, id: &ApprovalId, approver_id: &UserId) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait RemediationActionRepository: Repository<RemediationAction> {
    async fn find_by_type(&self, action_type: ActionType) -> Result<Vec<RemediationAction>, Error>;
    async fn find_rollback_for(&self, action_id: &ActionId) -> Result<Option<RemediationAction>, Error>;
}
```

### Value Objects

```rust
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    RestartPod,
    ScaleDeployment,
    UpdateConfig,
    ExecuteScript,
    SendNotification,
    CreateTicket,
    RollbackRelease,
    SwitchTraffic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub service: ServiceIdentity,
    pub resource_type: ResourceType,
    pub resource_id: String,
    pub namespace: String,
    pub cluster: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceType {
    Pod,
    Deployment,
    StatefulSet,
    Service,
    ConfigMap,
    Secret,
    Ingress,
    Job,
    CronJob,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionParameters {
    RestartParams { pod_name: String, namespace: String },
    ScaleParams { deployment: String, replicas: u32 },
    ConfigParams { config_map: String, data: BTreeMap<String, String> },
    ScriptParams { script: String, args: Vec<String>, timeout: Duration },
    NotificationParams { channel: String, message: String },
}

// Approval gate configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlastRadius {
    SingleService,
    LocalCluster,
    MultiCluster,
    GlobalOutage,
}

impl RemediationWorkflow {
    pub fn requires_approval(&self) -> bool {
        match (self.workflow_type, self.blast_radius) {
            // Low risk: No approval
            (WorkflowType::ClearCache, BlastRadius::SingleService) => false,
            (WorkflowType::RestartService, BlastRadius::SingleService) => false,

            // Medium risk: 1 approver
            (WorkflowType::ScaleHorizontal, BlastRadius::LocalCluster) => true,
            (WorkflowType::RollbackDeployment, BlastRadius::LocalCluster) => true,

            // High risk: 2+ approvers
            (_, BlastRadius::MultiCluster) => true,
            (_, BlastRadius::GlobalOutage) => true,

            // Production: Always require approval
            (_, _) if self.is_production() => true,

            // Default: No approval
            _ => false,
        }
    }
}
```

### Invariants

1. Workflow steps must execute sequentially
2. Approval gates must have all required approvers before proceeding
3. Rollback actions must be inverses of original actions
4. Critical actions (production changes) always require approval
5. Workflow timeout is 30 minutes per step

---

## Cross-Context Event Matrix

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                           CROSS-CONTEXT EVENT FLOW MATRIX                             │
├───────────────────┬─────────────────────────────────────────────────────────────────────┤
│                   │                     CONSUMERS                                        │
│      PUBLISHER    ├─────────┬─────────┬─────────┬─────────┬─────────┬─────────┬─────────┤
│                   │Tele │Anomaly│Incident│Topology│Integrat │Knowledge│Remediat │
├───────────────────┼──────┼────────┼────────┼────────┼────────┼────────┼────────┤
│ Telemetry         │      │        │        │        │        │        │        │
│  MetricsCollected │      │   YES  │        │        │        │        │        │
│  LogsReceived     │      │   YES  │        │        │        │        │        │
│  TraceReceived    │      │   YES  │        │   YES  │        │        │        │
│                   │      │        │        │        │        │        │        │
│ Anomaly Detection │      │        │        │        │        │        │        │
│  AnomalyDetected  │      │        │   YES  │        │        │        │        │
│  AnomalyConfirmed │      │        │   YES  │        │        │        │        │
│  AnomalyResolved  │      │        │   YES  │        │        │        │        │
│  BaselineUpdated  │  YES │        │        │        │        │        │        │
│  RootCauseIdentif │      │        │   YES  │        │        │   YES  │        │
│                   │      │        │        │        │        │        │        │
│ Incident Mgmt     │      │        │        │        │        │        │        │
│  IncidentCreated  │      │        │        │        │   YES  │        │   YES  │
│  IncidentAssigned │      │        │        │        │   YES  │        │        │
│  StatusChanged    │      │        │        │        │   YES  │   YES  │        │
│  RemediationReq   │      │        │        │        │        │        │   YES  │
│                   │      │        │        │        │        │        │        │
│ Service Topology  │      │        │        │        │        │        │        │
│  ServiceDiscovere │  YES │        │   YES  │        │        │        │        │
│  DependencyAdd    │      │        │        │        │        │        │        │
│  TopologyUpdated  │      │        │        │        │        │        │        │
│  ImpactAnalysis   │      │        │   YES  │        │        │        │        │
│                   │      │        │        │        │        │        │        │
│ Integration       │      │        │        │        │        │        │        │
│  SyncCompleted    │      │        │        │        │        │        │        │
│  MappingCreated   │      │        │        │        │        │        │        │
│                   │      │        │        │        │        │        │        │
│ Knowledge         │      │        │        │        │        │        │        │
│  PatternLearned   │      │   YES  │        │        │        │        │   YES  │
│  PatternValidated │      │        │        │        │        │        │   YES  │
│                   │      │        │        │        │        │        │        │
│ Remediation       │      │        │        │        │        │        │        │
│  WorkflowCreated  │      │        │        │        │   YES  │        │        │
│  WorkflowCompletd │      │        │   YES  │        │        │   YES  │        │
│  WorkflowFailed   │      │        │   YES  │        │        │   YES  │        │
└───────────────────┴──────┴────────┴────────┴────────┴────────┴────────┴────────┘
```

---

## Implementation Roadmap

### Phase 1: Foundation (Weeks 1-4)

**Goal**: Complete Common context and establish event infrastructure

| Task | Context | File | Effort |
|------|---------|------|--------|
| Add missing ID types | Common | `ids.rs` | 2 days |
| Expand EventType enum | Common | `events.rs` | 1 day |
| Add Repository trait | Common | `lib.rs` (new) | 2 days |
| Add Aggregate trait | Common | `lib.rs` (new) | 1 day |
| Implement event bus | Common | `event_bus.rs` (new) | 3 days |
| Add common value objects | Common | `domain.rs` (new) | 2 days |

### Phase 2: Telemetry & Anomaly (Weeks 5-8)

**Goal**: Complete data ingestion and detection pipeline

| Task | Context | File | Effort |
|------|---------|------|--------|
| Complete TelemetrySource aggregate | Telemetry | `sources.rs` | 3 days |
| Implement MetricBatch aggregate | Telemetry | `collectors.rs` | 2 days |
| Add repository implementations | Telemetry | `repository.rs` (new) | 3 days |
| Implement AnomalyDetector aggregate | Anomaly | `detector.rs` | 3 days |
| Add ONNX integration | Anomaly | `inference.rs` (new) | 5 days |
| Implement RootCauseAnalysis aggregate | Anomaly | `models.rs` (update) | 4 days |
| Add repository implementations | Anomaly | `repository.rs` (new) | 2 days |

### Phase 3: Incident & Topology (Weeks 9-12)

**Goal**: Complete incident management and service topology

| Task | Context | File | Effort |
|------|---------|------|--------|
| Enhance Incident aggregate | Incident | `incident.rs` | 2 days |
| Implement event sourcing | Incident | `events.rs` (update) | 4 days |
| Add snapshot support | Incident | `snapshot.rs` (new) | 3 days |
| Implement ServiceGraph aggregate | Topology | `graph.rs` | 4 days |
| Add ImpactAnalysis aggregate | Topology | `discovery.rs` | 3 days |
| Implement graph algorithms | Topology | `query.rs` | 3 days |
| Add repository implementations | Topology | `repository.rs` (new) | 2 days |

### Phase 4: Integration & Knowledge (Weeks 13-16)

**Goal**: Complete external integration and knowledge management

| Task | Context | File | Effort |
|------|---------|------|--------|
| Implement ExternalSystem aggregate | Integration | `adapter.rs` | 2 days |
| Add EntityMapping aggregate | Integration | `mappings.rs` (new) | 2 days |
| Implement SyncJob aggregate | Integration | `sync.rs` (new) | 3 days |
| Implement ServiceNow ACL | Integration | `servicenow.rs` | 3 days |
| Implement PagerDuty ACL | Integration | `pagerduty.rs` (new) | 2 days |
| Add vector embedding store | Knowledge | `embeddings.rs` | 4 days |
| Implement HNSW indexing | Knowledge | `hnsw.rs` (update) | 3 days |
| Add LearnedPattern aggregate | Knowledge | `patterns.rs` | 3 days |
| Implement SONA integration | Knowledge | `sona.rs` (update) | 5 days |

### Phase 5: Remediation & Integration (Weeks 17-20)

**Goal**: Complete remediation workflows and cross-context integration

| Task | Context | File | Effort |
|------|---------|------|--------|
| Implement RemediationWorkflow aggregate | Remediation | `workflow.rs` | 4 days |
| Add ApprovalGate aggregate | Remediation | `policy.rs` | 3 days |
| Implement safety checks | Remediation | `safety.rs` (update) | 3 days |
| Add activity logging | Remediation | `activity.rs` (update) | 2 days |
| Wire up event subscriptions | All | `subscribers.rs` (new) | 5 days |
| Add event handlers | All | `handlers.rs` (new) | 5 days |
| Implement saga orchestrator | Common | `saga.rs` (new) | 4 days |

### Phase 6: Testing & Documentation (Weeks 21-24)

**Goal**: Comprehensive testing and documentation

| Task | Context | File | Effort |
|------|---------|------|--------|
| Add unit tests for all aggregates | All | `*_test.rs` | 10 days |
| Add integration tests | All | `tests/integration_tests.rs` | 5 days |
| Add property-based tests | Common | `testing/property_tests.rs` (update) | 3 days |
| Write context documentation | All | `README.md` per context | 5 days |
| Create sequence diagrams | Docs | `docs/sequences/` | 3 days |
| Update main README | Docs | `README.md` | 1 day |

---

## File Organization

```
crates/
├── common/                    # Shared kernel (COMPLETE)
│   ├── src/
│   │   ├── ids.rs             # Type-safe IDs (expand with 6 new IDs)
│   │   ├── events.rs          # Domain events (expand EventType)
│   │   ├── error.rs           # Error types
│   │   ├── config.rs          # Configuration
│   │   ├── telemetry.rs       # Telemetry primitives
│   │   ├── domain.rs          # NEW: Common value objects
│   │   ├── event_bus.rs       # NEW: Event bus infrastructure
│   │   ├── repository.rs      # NEW: Repository trait
│   │   ├── aggregate.rs       # NEW: Aggregate trait
│   │   └── saga.rs            # NEW: Saga orchestrator
│   └── tests/
│       └── property_tests.rs
│
├── telemetry/                 # Telemetry Collection (EXPAND)
│   ├── src/
│   │   ├── collectors.rs      # Expand: add batch handling
│   │   ├── normalizer.rs
│   │   ├── sources.rs         # NEW: TelemetrySource aggregate
│   │   ├── metrics.rs         # Refactor: move to sources
│   │   └── repository.rs      # NEW: Repository implementations
│   └── tests/
│
├── anomaly/                   # Anomaly Detection (EXPAND)
│   ├── src/
│   │   ├── detector.rs        # Expand: add detector types
│   │   ├── models.rs          # Expand: add RootCauseAnalysis
│   │   ├── statistical.rs
│   │   ├── inference.rs       # NEW: ONNX inference
│   │   ├── router.rs
│   │   └── repository.rs      # NEW: Repository implementations
│   └── tests/
│
├── incident/                  # Incident Management (ENHANCE)
│   ├── src/
│   │   ├── incident.rs        # Enhance: add event sourcing
│   │   ├── events.rs          # Expand: add event types
│   │   ├── correlation.rs
│   │   ├── deduplication.rs
│   │   ├── repository.rs      # Expand: add snapshot support
│   │   └── snapshot.rs        # NEW: Snapshot management
│   └── tests/
│
├── topology/                  # Service Topology (IMPLEMENT)
│   ├── src/
│   │   ├── graph.rs           # Expand: add ServiceGraph aggregate
│   │   ├── discovery.rs       # Expand: add ImpactAnalysis
│   │   ├── query.rs           # Expand: add graph algorithms
│   │   ├── error.rs
│   │   └── repository.rs      # NEW: Repository implementations
│   └── tests/
│
├── integration/               # Integration (EXPAND)
│   ├── src/
│   │   ├── adapter.rs         # Expand: add ExternalSystem aggregate
│   │   ├── mappings.rs        # NEW: EntityMapping aggregate
│   │   ├── sync.rs            # NEW: SyncJob aggregate
│   │   ├── itsm/
│   │   │   ├── servicenow.rs  # Expand: add ACL
│   │   │   └── pagerduty.rs   # NEW: PagerDuty adapter
│   │   ├── telemetry/
│   │   │   └── prometheus.rs
│   │   ├── infrastructure/
│   │   │   └── kubernetes.rs
│   │   ├── circuit_breaker.rs
│   │   ├── retry.rs
│   │   ├── resilience.rs
│   │   ├── rate_limiter.rs
│   │   └── repository.rs      # NEW: Repository implementations
│   └── tests/
│
├── knowledge/                 # Knowledge Management (EXPAND)
│   ├── src/
│   │   ├── runbooks.rs        # Enhance: KnowledgeEntry aggregate
│   │   ├── patterns.rs        # Expand: LearnedPattern aggregate
│   │   ├── embeddings.rs      # Expand: vector store
│   │   ├── hnsw.rs            # Enhance: HNSW indexing
│   │   ├── sona.rs            # Enhance: SONA integration
│   │   ├── memory.rs
│   │   └── repository.rs      # NEW: Repository implementations
│   └── tests/
│
└── remediation/               # Remediation (EXPAND)
    ├── src/
    │   ├── workflow.rs        # Expand: RemediationWorkflow aggregate
    │   ├── policy.rs          # Expand: ApprovalGate aggregate
    │   ├── activity.rs        # Expand: ExecutionLog aggregate
    │   ├── safety.rs          # Enhance: safety checks
    │   └── repository.rs      # NEW: Repository implementations
    └── tests/
```

---

## Testing Strategy

### Unit Tests

- Each aggregate has comprehensive unit tests
- Test all invariants and business rules
- Test state transitions
- Test error conditions

### Integration Tests

- Test cross-context event flow
- Test repository implementations
- Test ACL transformations
- Test saga orchestrations

### Property-Based Tests

- Use proptest for invariant testing
- Test aggregate consistency
- Test event ordering
- Test serialization roundtrips

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Code Coverage | >80% | Unit + integration tests |
| Type Safety | 100% | No raw IDs in domain layer |
| Event Throughput | >10K events/sec | Event bus benchmarks |
| Repository Latency | <10ms p95 | Database query benchmarks |
| ACL Accuracy | 100% | Transformation validation |

---

**Document End**
