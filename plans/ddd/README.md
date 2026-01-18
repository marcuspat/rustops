# Domain-Driven Design for RustOps AIOps Platform

**Document Version**: 1.0
**Date**: 2026-01-18
**Status**: Design Document
**Authors**: Architecture Team

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Bounded Contexts Overview](#bounded-contexts-overview)
3. [Context Mapping](#context-mapping)
4. [Ubiquitous Language](#ubiquitous-language)
5. [Detailed Bounded Contexts](#detailed-bounded-contexts)
6. [Domain Events Catalog](#domain-events-catalog)
7. [Rust Implementation](#rust-implementation)
8. [Architecture Decision Records](#architecture-decision-records)

---

## Executive Summary

This document presents a comprehensive Domain-Driven Design (DDD) model for the RustOps AIOps platform. The design divides the complex AIOps domain into seven bounded contexts, each with clear responsibilities and well-defined boundaries.

### Design Principles

1. **Bounded Contexts**: 7 independent contexts with clear ownership
2. **Ubiquitous Language**: Shared terminology across all contexts
3. **Domain Events**: Event-driven communication between contexts
4. **Aggregates**: Consistency boundaries for transactional operations
5. **Anti-Corruption Layers**: Protect domain logic from external integrations

### Success Metrics Alignment

| DDD Target | PRD Requirement | Rationale |
|------------|----------------|-----------|
| 50% Auto-remediation | Remediation Context | Workflow automation with approval gates |
| 90% Prediction Accuracy | Anomaly Detection Context | ML models with feedback loops |
| 80% Alert Noise Reduction | Incident Management Context | Correlation and deduplication |
| 30% Faster MTTR | Cross-context coordination | Event-driven workflows |

---

## Bounded Contexts Overview

### Context Map Diagram

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

### Context Responsibility Matrix

| Context | Core Responsibility | Key Stakeholders | External Systems |
|---------|-------------------|------------------|------------------|
| **Telemetry Collection** | Ingest and normalize metrics, logs, traces, events | Platform Team | Prometheus, CloudWatch, OpenTelemetry |
| **Anomaly Detection** | ML-based pattern recognition and alerting | Data Science Team | ONNX Runtime, MLflow |
| **Incident Management** | Alert correlation, deduplication, lifecycle | SRE Team | ServiceNow, PagerDuty, Jira |
| **Remediation** | Workflow orchestration and self-healing | SRE Team | Temporal, Kubernetes API |
| **Service Topology** | Dependency discovery and impact analysis | Platform Team | Kubernetes API, Service Mesh |
| **Integration** | External system synchronization | DevOps Team | ITSM platforms, APM tools |
| **Knowledge Management** | Runbook learning and pattern storage | Data Science Team | Vector DB, Document Store |

---

## Context Mapping

### Relationship Types

| Relationship | Context Pair | Pattern | Rationale |
|--------------|--------------|---------|-----------|
| **Partnership** | Anomaly Detection <-> Incident Management | Shared Kernel | Both need aligned anomaly/incident definitions |
| **Customer-Supplier** | Incident Management -> Remediation | Domain Events | Incidents drive remediation workflows |
| **Anti-Corruption Layer** | Integration <-> All Contexts | ACL Pattern | Protect domain from external ITSM volatility |
| **Conformist** | Integration -> ServiceNow | Conformist | Adopt ServiceNow data model for tickets |
| **Open Host Service** | Service Topology -> All Contexts | Public API | Dependency data consumed by multiple contexts |
| **Published Language** | Knowledge Management <-> All Contexts | Domain Events | Patterns published as domain events |

### Integration Patterns

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Cross-Context Communication                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────┐      Domain Events      ┌─────────────────┐       │
│  │   Telemetry     │───────────────────────▶│   Anomaly        │       │
│  │   Collection    │  (AnomalyDetected)     │   Detection      │       │
│  └─────────────────┘                        └─────────────────┘       │
│         │                                          │                    │
│         │                                          │ Domain Events      │
│         │                                          │ (IncidentCreated)  │
│         │                                          ▼                    │
│         │                           ┌─────────────────┐                 │
│         │                           │   Incident      │                 │
│         └───────────────────────────│   Management    │                 │
│                 (MetricsAvailable)   └─────────────────┘                 │
│                                           │                              │
│                                           │ Domain Events                │
│                                           │ (RemediationRequested)       │
│                                           ▼                              │
│                                ┌─────────────────┐                       │
│                                │   Remediation   │                       │
│                                └─────────────────┘                       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Shared Kernel

The **Telemetry Collection** and **Service Topology** contexts share a kernel for:

- **ServiceIdentity**: Unique service identification
- **MetricDefinition**: Standardized metric schema
- **HealthStatus**: Enum for health states

```rust
// Shared kernel types
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

pub struct ServiceIdentity {
    pub id: Uuid,
    pub name: String,
    pub namespace: String,
    pub cluster: String,
}
```

---

## Ubiquitous Language

### Core Domain Terms

| Term | Definition | Example |
|------|------------|---------|
| **Anomaly** | Statistically significant deviation from expected behavior | CPU usage 3x above baseline |
| **Incident** | An anomaly requiring investigation or action | Database connection pool exhausted |
| **Remediation** | Automated or manual action to resolve an incident | Restart exhausted service |
| **Topology** | Graph of service dependencies and communication patterns | Microservice call graph |
| **Correlation** | Grouping related alerts to reduce noise | 3 alerts from same root cause |
| **Deduplication** | Removing duplicate or redundant alerts | Suppressing repeated notifications |
| **Impact Analysis** | Determining affected services and users | Service A outage affects 5 downstream services |
| **Runbook** | Documented procedure for incident resolution | Steps to restart database cluster |
| **Baseline** | Expected normal behavior for metrics | 95th percentile response time = 200ms |
| **Forecast** | Predicted future values based on historical data | Disk space exhaustion in 48 hours |

### Relationship Definitions

| Relationship | Description | Inverse |
|--------------|-------------|---------|
| **Dependency** | Service A depends on Service B if A calls B | Dependents |
| **Correlation** | Two anomalies share a temporal or causal relationship | - |
| **Impact** | Service A is impacted if Service B fails | Impacts |
| **Root Cause** | The originating anomaly causing other symptoms | Symptoms |
| **Remediates** | Action A resolves Incident B | Remediated By |

### State Transitions

#### Incident Lifecycle

```
┌───────────┐    Detection    ┌───────────┐    Analysis    ┌───────────┐
│  New      │────────────────▶│ Detected  │────────────────▶ Analyzing │
│  Incident │                 └───────────┘                 └───────────┘
└───────────┘                                                    │
       │                                                          │
       │                                                          ▼
       │                                                   ┌───────────┐
       │              Remediation                         │ Remediating│
       └───────────────────────────────────────────────────┴───────────┘
                                                              │
                                                              ▼
                                                       ┌───────────┐
                                                       │  Resolved  │
                                                       └───────────┘
```

#### Anomaly States

```
┌───────────┐    Threshold    ┌───────────┐    ML Model    ┌───────────┐
│ Normal    │────────────────▶│ Suspicious│────────────────▶│ Anomalous │
└───────────┘   Violation     └───────────┘   Confirmation └───────────┘
     ▲                                                           │
     │                                                           │
     │              Return to Baseline                           ▼
     └─────────────────────────────────────────────────┌───────────┐
                                                       │ Recovered │
                                                       └───────────┘
```

### Business Rules

| Rule | Context | Validation |
|------|---------|------------|
| BR-001 | Incident Management | Auto-remediation only for confidence > 80% |
| BR-002 | Anomaly Detection | Minimum 30 minutes of data for baseline |
| BR-003 | Remediation | Approval required for production changes |
| BR-004 | Topology | Service dependencies must have latency < 100ms |
| BR-005 | Knowledge | Runbook must be tested before auto-use |

---

## Detailed Bounded Contexts

### 1. Telemetry Collection Context

**Responsibility**: Ingest, normalize, and distribute telemetry data from all sources.

#### Core Entities

```rust
/// Aggregate root for telemetry ingestion
pub struct TelemetrySource {
    pub id: TelemetrySourceId,
    pub source_type: SourceType,
    pub config: SourceConfig,
    pub status: ConnectionStatus,
    pub metrics_collected: u64,
    pub last_collected: DateTime<Utc>,
}

/// Represents a single metric data point
pub struct MetricData {
    pub metric_name: MetricName,
    pub value: MetricValue,
    pub timestamp: DateTime<Utc>,
    pub labels: Labels,
    pub source_id: TelemetrySourceId,
}

/// Log entry with structured fields
pub struct LogEntry {
    pub id: LogEntryId,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub structured_data: BTreeMap<String, Value>,
    pub source_service: ServiceIdentity,
}

/// Distributed trace span
pub struct TraceSpan {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_span_id: Option<SpanId>,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub duration: Duration,
    pub attributes: BTreeMap<String, Value>,
}

/// Infrastructure event (deployment, scaling, etc.)
pub struct InfrastructureEvent {
    pub id: EventId,
    pub event_type: EventType,
    pub timestamp: DateTime<Utc>,
    pub source: EventSource,
    pub metadata: EventMetadata,
}
```

#### Value Objects

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MetricName(String);

#[derive(Debug, Clone, PartialEq)]
pub enum MetricValue {
    Counter(f64),
    Gauge(f64),
    Histogram(HistogramData),
    Summary(SummaryData),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels(BTreeMap<String, String>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Prometheus,
    CloudWatch,
    Datadog,
    OpenTelemetry,
    Fluentd,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
```

#### Domain Events

```rust
pub enum TelemetryEvent {
    MetricsCollected(MetricsCollectedEvent),
    LogsReceived(LogsReceivedEvent),
    TraceReceived(TraceReceivedEvent),
    SourceConnected(SourceConnectedEvent),
    SourceDisconnected(SourceDisconnectedEvent),
}

pub struct MetricsCollectedEvent {
    pub source_id: TelemetrySourceId,
    pub metric_count: u64,
    pub timestamp: DateTime<Utc>,
}
```

#### Aggregates

- **TelemetrySource Aggregate**: Manages connection lifecycle and collection state
- **MetricBatch Aggregate**: Groups metrics for efficient storage

#### Invariants

1. Metric timestamps must be monotonically increasing per source
2. Log levels must be valid enum values
3. Trace spans must have valid parent-child relationships
4. Source connections must be mutually exclusive (connected/disconnected)

---

### 2. Anomaly Detection Context

**Responsibility**: Detect anomalous behavior using statistical and ML techniques.

#### Core Entities

```rust
/// Aggregate root for anomaly detection
pub struct AnomalyDetector {
    pub id: DetectorId,
    pub detector_type: DetectorType,
    pub target_metric: MetricName,
    pub config: DetectorConfig,
    pub baseline: Baseline,
    pub state: DetectorState,
    pub detection_count: u64,
}

/// Represents a detected anomaly
pub struct Anomaly {
    pub id: AnomalyId,
    pub detector_id: DetectorId,
    pub anomaly_type: AnomalyType,
    pub severity: AnomalySeverity,
    pub detected_at: DateTime<Utc>,
    pub description: String,
    pub affected_metrics: Vec<MetricName>,
    pub confidence_score: f64,
    pub features: AnomalyFeatures,
}

/// Statistical baseline for comparison
pub struct Baseline {
    pub metric_name: MetricName,
    pub period: BaselinePeriod,
    pub statistics: BaselineStatistics,
    pub seasonality: Option<Seasonality>,
    pub last_updated: DateTime<Utc>,
}

/// Root cause hypothesis
pub struct RootCauseHypothesis {
    pub id: HypothesisId,
    pub anomaly_id: AnomalyId,
    pub causal_factors: Vec<CausalFactor>,
    pub confidence_score: f64,
    pub evidence: Evidence,
}
```

#### Value Objects

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectorType {
    StatisticalThreshold,
    MovingAverage,
    ExponentialSmoothing,
    LSTM,
    IsolationForest,
    DBSCAN,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyType {
    Spike,
    Drop,
    TrendChange,
    PatternBreak,
    SeasonalityViolation,
    NewErrorPattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct BaselineStatistics {
    pub mean: f64,
    pub std_dev: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone)]
pub struct AnomalyFeatures {
    pub magnitude: f64,
    pub duration: Duration,
    pub affected_services: Vec<ServiceIdentity>,
    pub correlation_score: f64,
}
```

#### Domain Events

```rust
pub enum AnomalyEvent {
    AnomalyDetected(AnomalyDetectedEvent),
    AnomalyConfirmed(AnomalyConfirmedEvent),
    AnomalyResolved(AnomalyResolvedEvent),
    BaselineUpdated(BaselineUpdatedEvent),
    RootCauseIdentified(RootCauseIdentifiedEvent),
}

pub struct AnomalyDetectedEvent {
    pub anomaly_id: AnomalyId,
    pub detector_id: DetectorId,
    pub severity: AnomalySeverity,
    pub confidence: f64,
    pub detected_at: DateTime<Utc>,
}
```

#### Aggregates

- **AnomalyDetector Aggregate**: Manages detection lifecycle and baseline
- **Anomaly Aggregate**: Represents a single detected anomaly
- **RootCauseAnalysis Aggregate**: Manages hypothesis generation and validation

#### Invariants

1. Confidence scores must be between 0.0 and 1.0
2. Anomaly severity must be proportional to confidence
3. Baseline periods must be at least 30 minutes
4. Root cause hypotheses must have at least one causal factor

---

### 3. Incident Management Context

**Responsibility**: Correlate anomalies, deduplicate alerts, and manage incident lifecycle.

#### Core Entities

```rust
/// Aggregate root for incident management
pub struct Incident {
    pub id: IncidentId,
    pub incident_number: IncidentNumber,
    pub title: String,
    pub description: String,
    pub status: IncidentStatus,
    pub severity: IncidentSeverity,
    pub detected_at: DateTime<Utc>,
    pub assigned_to: Option<UserId>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub related_anomalies: Vec<AnomalyId>,
    pub correlation_group: CorrelationGroupId,
    pub remediation_plan: Option<RemediationPlanId>,
}

/// Group of correlated incidents
pub struct CorrelationGroup {
    pub id: CorrelationGroupId,
    pub incidents: Vec<IncidentId>,
    pub root_cause_anomaly: Option<AnomalyId>,
    pub correlation_strategy: CorrelationStrategy,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Alert deduplication tracker
pub struct AlertDeduplicator {
    pub fingerprint: AlertFingerprint,
    pub last_seen: DateTime<Utc>,
    pub count: u64,
    pub status: DeduplicationStatus,
}
```

#### Value Objects

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentStatus {
    New,
    Detected,
    Analyzing,
    Remediating,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentSeverity {
    P1, // Critical
    P2, // High
    P3, // Medium
    P4, // Low,
    CorrelationByTopology,
    CorrelationByTimeWindow,
    CorrelationByCausalGraph,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AlertFingerprint(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeduplicationStatus {
    Active,
    Suppressed,
    Resolved,
}
```

#### Domain Events

```rust
pub enum IncidentEvent {
    IncidentCreated(IncidentCreatedEvent),
    IncidentAssigned(IncidentAssignedEvent),
    IncidentStatusChanged(IncidentStatusChangedEvent),
    IncidentsCorrelated(IncidentsCorrelatedEvent),
    AlertDeduplicated(AlertDeduplicatedEvent),
    RemediationRequested(RemediationRequestedEvent),
}

pub struct IncidentCreatedEvent {
    pub incident_id: IncidentId,
    pub incident_number: IncidentNumber,
    pub severity: IncidentSeverity,
    pub anomaly_ids: Vec<AnomalyId>,
    pub created_at: DateTime<Utc>,
}

pub struct IncidentsCorrelatedEvent {
    pub correlation_group_id: CorrelationGroupId,
    pub incident_ids: Vec<IncidentId>,
    pub root_cause_hypothesis: Option<AnomalyId>,
}
```

#### Aggregates

- **Incident Aggregate**: Manages incident lifecycle and state transitions
- **CorrelationGroup Aggregate**: Groups related incidents for unified handling
- **AlertDeduplicator Aggregate**: Tracks and suppresses duplicate alerts

#### Invariants

1. Incident numbers must be unique and monotonically increasing
2. Correlation groups must have at least 2 incidents
3. Alert deduplication window is 5 minutes
4. Incidents must be in valid status for each transition

---

### 4. Remediation Context

**Responsibility**: Orchestrate self-healing workflows and manual remediation actions.

#### Core Entities

```rust
/// Aggregate root for remediation workflows
pub struct RemediationWorkflow {
    pub id: WorkflowId,
    pub incident_id: IncidentId,
    pub workflow_type: WorkflowType,
    pub status: WorkflowStatus,
    pub steps: Vec<WorkflowStep>,
    pub current_step: Option<usize>,
    pub approval_state: ApprovalState,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<WorkflowResult>,
}

/// Single remediation action
pub struct RemediationAction {
    pub id: ActionId,
    pub action_type: ActionType,
    pub target: Target,
    pub parameters: ActionParameters,
    pub timeout: Duration,
    pub rollback_action: Option<Box<RemediationAction>>,
}

/// Approval gate for critical actions
pub struct ApprovalGate {
    pub id: ApprovalId,
    pub required_approvers: u8,
    pub current_approvals: u8,
    pub approvers: HashSet<UserId>,
    pub status: ApprovalStatus,
    pub deadline: DateTime<Utc>,
}
```

#### Value Objects

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStatus {
    PendingApproval,
    Approved,
    InProgress,
    Completed,
    Failed,
    RolledBack,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    RestartPod,
    ScaleDeployment,
    UpdateConfig,
    ExecuteScript,
    SendNotification,
    CreateTicket,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub service: ServiceIdentity,
    pub resource_type: ResourceType,
    pub resource_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Pod,
    Deployment,
    StatefulSet,
    Service,
    ConfigMap,
    Secret,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionParameters {
    RestartParams { pod_name: String, namespace: String },
    ScaleParams { deployment: String, replicas: u32 },
    ConfigParams { config_map: String, data: BTreeMap<String, String> },
}
```

#### Domain Events

```rust
pub enum RemediationEvent {
    WorkflowCreated(WorkflowCreatedEvent),
    WorkflowApproved(WorkflowApprovedEvent),
    WorkflowRejected(WorkflowRejectedEvent),
    WorkflowStepStarted(WorkflowStepStartedEvent),
    WorkflowStepCompleted(WorkflowStepCompletedEvent),
    WorkflowCompleted(WorkflowCompletedEvent),
    WorkflowFailed(WorkflowFailedEvent),
    WorkflowRolledBack(WorkflowRolledBackEvent),
}

pub struct WorkflowCreatedEvent {
    pub workflow_id: WorkflowId,
    pub incident_id: IncidentId,
    pub workflow_type: WorkflowType,
    pub requires_approval: bool,
    pub created_at: DateTime<Utc>,
}

pub struct WorkflowCompletedEvent {
    pub workflow_id: WorkflowId,
    pub incident_id: IncidentId,
    pub result: WorkflowResult,
    pub completed_at: DateTime<Utc>,
}
```

#### Aggregates

- **RemediationWorkflow Aggregate**: Orchestrates multi-step remediation
- **ApprovalGate Aggregate**: Manages approval process
- **ExecutionLog Aggregate**: Tracks action execution history

#### Invariants

1. Workflow steps must execute sequentially
2. Approval gates must have all required approvers before proceeding
3. Rollback actions must be inverses of original actions
4. Critical actions (production changes) always require approval

---

### 5. Service Topology Context

**Responsibility**: Discover, map, and analyze service dependencies.

#### Core Entities

```rust
/// Aggregate root for topology management
pub struct ServiceGraph {
    pub id: GraphId,
    pub cluster: ClusterIdentity,
    pub services: Vec<ServiceNode>,
    pub dependencies: Vec<DependencyEdge>,
    pub last_updated: DateTime<Utc>,
    pub version: GraphVersion,
}

/// Represents a service in the topology
pub struct ServiceNode {
    pub id: ServiceId,
    pub identity: ServiceIdentity,
    pub service_type: ServiceType,
    pub health_status: HealthStatus,
    pub capabilities: Vec<Capability>,
    pub metadata: ServiceMetadata,
}

/// Dependency relationship between services
pub struct DependencyEdge {
    pub id: EdgeId,
    pub from_service: ServiceId,
    pub to_service: ServiceId,
    pub dependency_type: DependencyType,
    pub latency: LatencyProfile,
    pub call_frequency: CallFrequency,
    pub strength: DependencyStrength,
}

/// Impact analysis result
pub struct ImpactAnalysis {
    pub id: AnalysisId,
    pub failed_service: ServiceId,
    pub impacted_services: Vec<ImpactedService>,
    pub blast_radius: BlastRadius,
    pub user_impact: UserImpact,
    pub generated_at: DateTime<Utc>,
}
```

#### Value Objects

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    Synchronous,
    Asynchronous,
    DataDependency,
    WeakDependency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyStrength {
    Strong,
    Moderate,
    Weak,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LatencyProfile {
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallFrequency {
    PerSecond(f64),
    PerMinute(f64),
    PerHour(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlastRadius {
    SingleService,
    LocalCluster,
    MultiCluster,
    GlobalOutage,
}
```

#### Domain Events

```rust
pub enum TopologyEvent {
    ServiceDiscovered(ServiceDiscoveredEvent),
    ServiceRemoved(ServiceRemovedEvent),
    DependencyAdded(DependencyAddedEvent),
    DependencyRemoved(DependencyRemovedEvent),
    TopologyUpdated(TopologyUpdatedEvent),
    ImpactAnalysisCompleted(ImpactAnalysisCompletedEvent),
}

pub struct ServiceDiscoveredEvent {
    pub service_id: ServiceId,
    pub service_identity: ServiceIdentity,
    pub discovered_at: DateTime<Utc>,
}

pub struct ImpactAnalysisCompletedEvent {
    pub analysis_id: AnalysisId,
    pub failed_service: ServiceId,
    pub impacted_count: usize,
    pub blast_radius: BlastRadius,
}
```

#### Aggregates

- **ServiceGraph Aggregate**: Manages the complete dependency graph
- **ServiceNode Aggregate**: Represents individual service state
- **ImpactAnalysis Aggregate**: Computes downstream impact

#### Invariants

1. Dependency edges must reference valid service nodes
2. Graph must remain acyclic (no circular dependencies)
3. Latency profiles must be updated every 5 minutes
4. Services without dependencies are "leaf nodes"

---

### 6. Integration Context

**Responsibility**: Synchronize with external ITSM and monitoring systems.

#### Core Entities

```rust
/// Aggregate root for external system integration
pub struct ExternalSystem {
    pub id: SystemId,
    pub system_type: SystemType,
    pub config: SystemConfig,
    pub connection_status: ConnectionStatus,
    pub sync_state: SyncState,
    pub last_sync: Option<DateTime<Utc>>,
}

/// Mapping between internal and external entities
pub struct EntityMapping {
    pub id: MappingId,
    pub internal_id: InternalEntityId,
    pub external_id: ExternalEntityId,
    pub external_system: SystemId,
    pub mapping_type: MappingType,
    pub sync_direction: SyncDirection,
}

/// Synchronization job
pub struct SyncJob {
    pub id: JobId,
    pub external_system: SystemId,
    pub job_type: SyncJobType,
    pub status: JobStatus,
    pub entities_processed: u64,
    pub errors: Vec<SyncError>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Anti-corruption layer translation
pub struct TranslationRule {
    pub id: RuleId,
    pub from_schema: Schema,
    pub to_schema: Schema,
    pub transformations: Vec<Transform>,
}
```

#### Value Objects

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemType {
    ServiceNow,
    Jira,
    PagerDuty,
    OpsGenie,
    Slack,
    MSTeams,
    Datadog,
    Dynatrace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    InSync,
    PendingSync,
    Conflict,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingType {
    IncidentToTicket,
    ServiceToCI,
    UserToContact,
    AlertToEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Bidirectional,
    InboundOnly,
    OutboundOnly,
}
```

#### Domain Events

```rust
pub enum IntegrationEvent {
    SystemConnected(SystemConnectedEvent),
    SystemDisconnected(SystemDisconnectedEvent),
    MappingCreated(MappingCreatedEvent),
    SyncCompleted(SyncCompletedEvent),
    SyncFailed(SyncFailedEvent),
    ConflictDetected(ConflictDetectedEvent),
}

pub struct SyncCompletedEvent {
    pub job_id: JobId,
    pub external_system: SystemId,
    pub entities_processed: u64,
    pub completed_at: DateTime<Utc>,
}
```

#### Aggregates

- **ExternalSystem Aggregate**: Manages connection and sync state
- **EntityMapping Aggregate**: Tracks bidirectional entity mappings
- **SyncJob Aggregate**: Orchestrates synchronization tasks

#### Invariants

1. Entity mappings must be unique per external system
2. Sync jobs must have a maximum timeout of 10 minutes
3. ACL transformations must preserve data semantics
4. Conflicts must be resolved within 1 hour

---

### 7. Knowledge Management Context

**Responsibility**: Store, retrieve, and learn from remediation patterns.

#### Core Entities

```rust
/// Aggregate root for knowledge storage
pub struct KnowledgeBase {
    pub id: KnowledgeBaseId,
    pub name: String,
    pub entries: Vec<KnowledgeEntry>,
    pub indexing_state: IndexingState,
    pub last_updated: DateTime<Utc>,
}

/// Single knowledge entry (runbook, pattern, etc.)
pub struct KnowledgeEntry {
    pub id: EntryId,
    pub entry_type: EntryType,
    pub title: String,
    pub content: EntryContent,
    pub tags: Vec<String>,
    pub embeddings: Option<VectorEmbedding>,
    pub usage_stats: UsageStats,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Learned pattern from successful remediations
pub struct LearnedPattern {
    pub id: PatternId,
    pub pattern_type: PatternType,
    pub trigger_conditions: Vec<TriggerCondition>,
    pub remediation_steps: Vec<RemediationStep>,
    pub success_rate: f64,
    pub confidence_score: f64,
    pub times_used: u64,
    pub last_success: Option<DateTime<Utc>>,
}
```

#### Value Objects

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    Runbook,
    KnownIssue,
    Pattern,
    Playbook,
    TroubleshootingGuide,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryContent {
    Markdown(String),
    Structured(StructuredContent),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructuredContent {
    pub summary: String,
    pub symptoms: Vec<String>,
    pub diagnosis: String,
    pub steps: Vec<Step>,
    pub verification: VerificationCriteria,
}

#[derive(Debug, Clone)]
pub struct VectorEmbedding {
    pub vector: Vec<f32>,
    pub model: String,
    pub dimension: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    AnomalyResponse,
    IncidentResponse,
    ProactiveRemediation,
    ScalingPattern,
}
```

#### Domain Events

```rust
pub enum KnowledgeEvent {
    EntryCreated(EntryCreatedEvent),
    EntryUpdated(EntryUpdatedEvent),
    EntryAccessed(EntryAccessedEvent),
    PatternLearned(PatternLearnedEvent),
    PatternValidated(PatternValidatedEvent),
    EmbeddingGenerated(EmbeddingGeneratedEvent),
}

pub struct PatternLearnedEvent {
    pub pattern_id: PatternId,
    pub pattern_type: PatternType,
    pub source_incident: IncidentId,
    pub confidence: f64,
    pub learned_at: DateTime<Utc>,
}
```

#### Aggregates

- **KnowledgeBase Aggregate**: Manages knowledge storage and retrieval
- **LearnedPattern Aggregate**: Represents extracted remediation patterns
- **UsageStats Aggregate**: Tracks entry effectiveness

#### Invariants

1. Embeddings must be normalized (unit vectors)
2. Pattern confidence must be > 0.7 for auto-use
3. Entries must be indexed within 1 second of creation
4. Usage stats must update on each access

---

## Domain Events Catalog

### Event Taxonomy

| Event | Producer | Consumers | Frequency | Criticality |
|-------|----------|-----------|-----------|-------------|
| `MetricsCollected` | Telemetry Collection | Anomaly Detection | Very High (1M/min) | Low |
| `AnomalyDetected` | Anomaly Detection | Incident Management | High (1K/min) | High |
| `IncidentCreated` | Incident Management | Remediation, Integration | Medium (100/min) | Critical |
| `WorkflowCompleted` | Remediation | Knowledge Management | Low (10/min) | Medium |
| `TopologyUpdated` | Service Topology | All Contexts | Low (1/min) | High |
| `PatternLearned` | Knowledge Management | Anomaly Detection, Remediation | Very Low (1/hour) | Low |

### Event Schema Standards

All events implement the `DomainEvent` trait:

```rust
pub trait DomainEvent: Send + Sync + Clone {
    fn event_id(&self) -> Uuid;
    fn event_type(&self) -> &'static str;
    fn occurred_at(&self) -> DateTime<Utc>;
    fn causation_id(&self) -> Option<Uuid>;
    fn correlation_id(&self) -> Option<Utc>;
}
```

### Event Payload Examples

```rust
// AnomalyDetectedEvent
{
    "event_id": "550e8400-e29b-41d4-a716-446655440000",
    "event_type": "AnomalyDetected",
    "occurred_at": "2026-01-18T10:30:00Z",
    "anomaly_id": "660e8400-e29b-41d4-a716-446655440001",
    "detector_id": "770e8400-e29b-41d4-a716-446655440002",
    "severity": "High",
    "confidence": 0.87,
    "affected_metrics": ["cpu_usage", "memory_usage"],
    "detected_at": "2026-01-18T10:30:00Z"
}

// IncidentCreatedEvent
{
    "event_id": "880e8400-e29b-41d4-a716-446655440003",
    "event_type": "IncidentCreated",
    "occurred_at": "2026-01-18T10:31:00Z",
    "incident_id": "990e8400-e29b-41d4-a716-446655440004",
    "incident_number": "INC-2026-001234",
    "severity": "P2",
    "title": "High CPU usage on payment-service",
    "related_anomalies": ["660e8400-e29b-41d4-a716-446655440001"]
}

// RemediationCompletedEvent
{
    "event_id": "aa0e8400-e29b-41d4-a716-446655440005",
    "event_type": "WorkflowCompleted",
    "occurred_at": "2026-01-18T10:45:00Z",
    "workflow_id": "bb0e8400-e29b-41d4-a716-446655440006",
    "incident_id": "990e8400-e29b-41d4-a716-446655440004",
    "workflow_type": "RestartService",
    "result": "Success",
    "steps_executed": 3,
    "duration_seconds": 120
}
```

---

## Rust Implementation

### Module Structure

```
rustops/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── telemetry/              # Telemetry Collection Context
│   │   ├── mod.rs
│   │   ├── collectors.rs
│   │   ├── normalizers.rs
│   │   ├── sources.rs
│   │   └── models.rs
│   ├── anomaly/                # Anomaly Detection Context
│   │   ├── mod.rs
│   │   ├── detectors.rs
│   │   ├── baselines.rs
│   │   ├── models.rs
│   │   └── inference.rs
│   ├── incident/               # Incident Management Context
│   │   ├── mod.rs
│   │   ├── incidents.rs
│   │   ├── correlation.rs
│   │   ├── deduplication.rs
│   │   └── lifecycle.rs
│   ├── remediation/            # Remediation Context
│   │   ├── mod.rs
│   │   ├── workflows.rs
│   │   ├── actions.rs
│   │   ├── approvals.rs
│   │   └── execution.rs
│   ├── topology/               # Service Topology Context
│   │   ├── mod.rs
│   │   ├── graph.rs
│   │   ├── discovery.rs
│   │   └── impact.rs
│   ├── integration/            # Integration Context
│   │   ├── mod.rs
│   │   ├── adapters.rs
│   │   ├── mappings.rs
│   │   └── sync.rs
│   ├── knowledge/              # Knowledge Management Context
│   │   ├── mod.rs
│   │   ├── entries.rs
│   │   ├── patterns.rs
│   │   ├── embeddings.rs
│   │   └── retrieval.rs
│   ├── shared/                 # Shared Kernel
│   │   ├── mod.rs
│   │   ├── domain.rs
│   │   ├── events.rs
│   │   └── time.rs
│   └── infrastructure/         # Infrastructure Layer
│       ├── mod.rs
│       ├── persistence.rs
│       ├── messaging.rs
│       └── config.rs
```

### Type System Design

```rust
// Domain ID types (Newtype pattern for type safety)
pub type TelemetrySourceId = Uuid;
pub type MetricName = String;
pub type AnomalyId = Uuid;
pub type DetectorId = Uuid;
pub type IncidentId = Uuid;
pub type IncidentNumber = String;
pub type WorkflowId = Uuid;
pub type ServiceId = Uuid;
pub type KnowledgeEntryId = Uuid;

// Result types for domain operations
pub type DomainResult<T> = Result<T, DomainError>;

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
}

// Aggregate trait
pub trait Aggregate {
    type Id;
    type Event;

    fn id(&self) -> &Self::Id;
    fn version(&self) -> u64;
    fn apply_event(&mut self, event: Self::Event) -> DomainResult<()>;
}

// Repository trait
#[async_trait::async_trait]
pub trait Repository<A: Aggregate> {
    async fn get(&self, id: &A::Id) -> DomainResult<Option<A>>;
    async fn save(&self, aggregate: &A) -> DomainResult<()>;
    async fn delete(&self, id: &A::Id) -> DomainResult<()>;
}
```

### Event Sourcing Support

```rust
// Event store for event sourcing
pub trait EventStore {
    async fn append_events(
        &self,
        stream_id: Uuid,
        expected_version: Option<u64>,
        events: Vec<DomainEventData>,
    ) -> DomainResult<()>;

    async fn read_events(
        &self,
        stream_id: Uuid,
        from_version: Option<u64>,
    ) -> DomainResult<Vec<DomainEventData>>;
}

// Event projection
pub trait Projection {
    async fn project(&self, event: &DomainEventData) -> DomainResult<()>;
    async fn rebuild(&self) -> DomainResult<()>;
}
```

---

## Architecture Decision Records

### ADR-001: Bounded Context Isolation

**Status**: Accepted
**Date**: 2026-01-18

**Context**: The AIOps domain is complex with multiple concerns (telemetry, detection, remediation, etc.).

**Decision**: Use bounded contexts with strict boundaries and event-driven communication.

**Consequences**:
- (+) Clear ownership and reduced coupling
- (+) Independent deployment and scaling
- (+) Easier testing and maintenance
- (-) More complex initial setup
- (-) Event-driven communication adds latency

### ADR-002: Event Sourcing for Incident History

**Status**: Accepted
**Date**: 2026-01-18

**Context**: Incidents have rich history and audit requirements. Full state reconstruction is needed.

**Decision**: Use event sourcing for Incident aggregate with snapshot optimization.

**Consequences**:
- (+) Complete audit trail
- (+) Temporal queries possible
- (+) Event replay for debugging
- (-) Increased storage requirements
- (-) Snapshot complexity

### ADR-003: Vector Embeddings for Knowledge

**Status**: Accepted
**Date**: 2026-01-18

**Context**: Knowledge retrieval needs semantic search capabilities.

**Decision**: Use vector embeddings (HNSW-indexed) for all knowledge entries.

**Consequences**:
- (+) Semantic search capabilities
- (+) Fast similarity queries
- (+) ML-powered recommendations
- (-) Embedding computation overhead
- (-) Additional infrastructure

### ADR-004: Approval Gates for Remediation

**Status**: Accepted
**Date**: 2026-01-18

**Context**: Auto-remediation can cause unintended consequences in production.

**Decision**: Implement graduated approval gates based on blast radius and risk.

**Consequences**:
- (+) Controlled automation rollout
- (+) Human oversight for critical actions
- (+) Audit trail for compliance
- (-) Slower remediation for critical actions
- (-) Approval workflow complexity

---

## Appendix

### ER Diagrams

#### Telemetry Collection ERD

```
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│ MetricData      │       │ TelemetrySource │       │  LogEntry       │
├─────────────────┤       ├─────────────────┤       ├─────────────────┤
│ id: PK          │───◄───│ id: PK          │───◄───│ id: PK          │
│ metric_name: FK │       │ source_type     │       │ timestamp       │
│ value           │       │ config          │       │ level           │
│ timestamp       │       │ status          │       │ message         │
│ labels          │       │ last_collected  │       │ source_service  │
│ source_id: FK   │       └─────────────────┘       └─────────────────┘
└─────────────────┘
         │
         │
┌────────▼────────┐       ┌─────────────────┐
│ TraceSpan       │       │InfraEvent       │
├─────────────────┤       ├─────────────────┤
│ trace_id: PK    │       │ id: PK          │
│ span_id         │       │ event_type      │
│ parent_span_id  │       │ timestamp       │
│ operation_name  │       │ source          │
│ start_time      │       │ metadata        │
│ duration        │       └─────────────────┘
│ attributes      │
└─────────────────┘
```

#### Incident Management ERD

```
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│ Incident        │       │CorrelationGroup │       │AlertDeduplicator│
├─────────────────┤       ├─────────────────┤       ├─────────────────┤
│ id: PK          │───◄───│ id: PK          │       │ fingerprint: PK │
│ incident_number │       │ incidents       │       │ last_seen       │
│ title           │       │ root_cause_     │       │ count           │
│ status          │       │   anomaly: FK   │       │ status          │
│ severity        │       │ strategy        │       └─────────────────┘
│ detected_at     │       │ created_at      │               ▲
│ related_anomalies│      └─────────────────┘               │
└─────────────────┘                                        │
         │                                                  │
         │ belongs to                                       │ matches
         │                                                  │
┌────────▼────────┐       ┌─────────────────┐               │
│ Anomaly         │       │RemediationPlan  │               │
├─────────────────┤       ├─────────────────┤               │
│ id: PK          │       │ id: PK          │               │
│ detector_id: FK │       │ incident_id: FK │               │
│ anomaly_type    │       │ workflow_id: FK │               │
│ severity        │       │ status          │               │
│ confidence      │       │ steps           │               │
│ features        │       │ approval_state  │               │
└─────────────────┘       └─────────────────┘               │
                                                                  │
                         ┌──────────────────────────────────────┘
                         │
                ┌────────▼────────┐
                │  DomainEvent    │
                ├─────────────────┤
                │ id: PK          │
                │ event_type      │
                │ occurred_at     │
                │ payload         │
                └─────────────────┘
```

### Glossary

| Term | Definition |
|------|------------|
| **Aggregate** | Cluster of domain objects treated as a unit for data changes |
| **Aggregate Root** | The only member of an aggregate that external objects hold references to |
| **Bounded Context** | A linguistic boundary where a particular domain model applies |
| **Context Mapping** | The relationship between bounded contexts |
| **Domain Event** | Something that happened in the domain that domain experts care about |
| **Ubiquitous Language** | A shared language used by both developers and domain experts |
| **Value Object** | An immutable object without identity that describes a characteristic |

---

**Document End**
