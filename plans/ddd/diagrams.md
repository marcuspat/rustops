# DDD Diagrams - RustOps AIOps Platform

This document contains ASCII diagrams for the RustOps domain model.

---

## 1. Context Map Diagram

```
                    ┌─────────────────────────────────────────┐
                    │         RustOps AIOps Platform           │
                    │         (7 Bounded Contexts)             │
                    └─────────────────────────────────────────┘
                                      │
        ┌─────────────────────────────┼─────────────────────────────┐
        │                             │                             │
        ▼                             ▼                             ▼
┌───────────────────┐      ┌───────────────────┐      ┌───────────────────┐
│   Telemetry       │      │   Anomaly         │      │   Incident        │
│   Collection      │──────│   Detection       │──────│   Management      │
│   Context         │      │   Context         │      │   Context         │
│                   │      │                   │      │                   │
│ - Metrics         │      │ - Detectors       │      │ - Incidents       │
│ - Logs            │      │ - Baselines       │      │ - Correlation     │
│ - Traces          │      │ - Anomalies       │      │ - Deduplication   │
│ - Events          │      │ - Root Cause      │      │ - Lifecycle       │
└───────────────────┘      └───────────────────┘      └───────────────────┘
        │                             │                             │
        │                             ▼                             │
        │                    ┌───────────────────┐                    │
        │                    │   Service         │                    │
        └────────────────────│   Topology        │────────────────────┘
                             │   Context         │
                             │                   │
                             │ - Service Graph   │
                             │ - Dependencies    │
                             │ - Impact Analysis │
                             └───────────────────┘
                                      │
        ┌─────────────────────────────┼─────────────────────────────┐
        │                             │                             │
        ▼                             ▼                             ▼
┌───────────────────┐      ┌───────────────────┐      ┌───────────────────┐
│   Remediation     │      │   Integration     │      │   Knowledge       │
│   Context         │      │   Context         │      │   Management      │
│                   │      │                   │      │                   │
│ - Workflows       │      │ - External Systs. │      │ - Runbooks        │
│ - Actions         │      │ - Mappings        │      │ - Patterns        │
│ - Approvals       │      │ - Sync Jobs       │      │ - Embeddings      │
│ - Execution       │      │ - ACLs            │      │ - Learning        │
└───────────────────┘      └───────────────────┘      └───────────────────┘
```

---

## 2. Cross-Context Communication (Domain Events)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Domain Event Flow                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────┐                                                   │
│  │   Telemetry     │                                                   │
│  │   Collection    │                                                   │
│  └────────┬────────┘                                                   │
│           │                                                            │
│           │ MetricsCollected Event                                      │
│           │ (High frequency: 1M/min)                                    │
│           ▼                                                            │
│  ┌─────────────────┐                                                   │
│  │   Anomaly       │                                                   │
│  │   Detection     │                                                   │
│  └────────┬────────┘                                                   │
│           │                                                            │
│           │ AnomalyDetected Event                                       │
│           │ (Medium frequency: 1K/min)                                  │
│           ▼                                                            │
│  ┌─────────────────┐                                                   │
│  │   Incident      │                                                   │
│  │   Management    │                                                   │
│  └────────┬────────┘                                                   │
│           │                                                            │
│           ├──────────────────┐                                         │
│           │                  │                                         │
│           │ IncidentCreated  │ RemediationRequested                   │
│           │ (Med: 100/min)   │                                         │
│           ▼                  ▼                                         │
│  ┌─────────────────┐  ┌─────────────┐                                 │
│  │   Integration   │  │ Remediation │                                 │
│  │   Context       │  │  Context    │                                 │
│  └─────────────────┘  └──────┬──────┘                                 │
│                              │                                         │
│                              │ WorkflowCompleted                       │
│                              ▼                                         │
│                     ┌─────────────────┐                                │
│                     │  Knowledge      │                                │
│                     │  Management     │                                │
│                     │  (Learn)        │                                │
│                     └─────────────────┘                                │
│                                                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Telemetry Collection Context ERD

```
┌───────────────────────────────┐
│   TelemetrySource (Aggregate) │
├───────────────────────────────┤
│ PK  id: Uuid                  │
│    source_type: SourceType    │
│    config: SourceConfig       │
│    status: ConnectionStatus   │
│    metrics_collected: u64     │
│    last_collected: DateTime   │
└───────────────────────────────┘
            │ 1
            │
            │ has
            │ *
┌───────────▼─────────────────┐     ┌─────────────────────────────┐
│      MetricData              │     │       LogEntry              │
├──────────────────────────────┤     ├─────────────────────────────┤
│ PK  id: Uuid                 │     │ PK  id: Uuid                │
│ FK  source_id: Uuid          │     │    timestamp: DateTime      │
│    metric_name: MetricName   │     │    level: LogLevel          │
│    value: MetricValue        │     │    message: String          │
│    timestamp: DateTime       │     │    structured_data: Map      │
│    labels: Labels            │     │ FK  source_service: ServiceId│
└──────────────────────────────┘     └─────────────────────────────┘

┌─────────────────────────────┐     ┌─────────────────────────────┐
│       TraceSpan             │     │    InfrastructureEvent      │
├─────────────────────────────┤     ├─────────────────────────────┤
│ PK  trace_id: Uuid          │     │ PK  id: Uuid                │
│ PK  span_id: Uuid           │     │    event_type: EventType    │
│ FK  parent_span_id: Uuid?   │     │    timestamp: DateTime      │
│    operation_name: String   │     │    source: EventSource      │
│    start_time: DateTime     │     │    metadata: EventMetadata  │
│    duration: Duration       │     └─────────────────────────────┘
│    attributes: Map          │
└─────────────────────────────┘
```

---

## 4. Anomaly Detection Context ERD

```
┌──────────────────────────────┐
│   AnomalyDetector (Aggregate)│
├──────────────────────────────┤
│ PK  id: DetectorId           │
│    detector_type: DetectorType│
│    target_metric: MetricName │
│    config: DetectorConfig    │
│    state: DetectorState      │
│    detection_count: u64      │
└──────────────────────────────┘
            │ 1
            │ has
            │ *
┌───────────▼───────────────────┐     ┌──────────────────────────┐
│         Baseline              │     │      Anomaly (Aggregate)  │
├───────────────────────────────┤     ├──────────────────────────┤
│ PK  metric_name: MetricName   │     │ PK  id: AnomalyId        │
│    period: BaselinePeriod     │     │ FK  detector_id: DetectorId│
│    statistics: Stats          │     │    anomaly_type: AnomalyType│
│    seasonality: Seasonality?  │     │    severity: AnomalySeverity│
│    last_updated: DateTime     │     │    confidence_score: f64  │
└───────────────────────────────┘     │    features: Features     │
                                      │    status: AnomalyStatus  │
                                      └──────────────────────────┘
                                               │ 1
                                               │ causes
                                               │ *
┌─────────────────────────────────────────────────────────────────┐
│                    RootCauseHypothesis                           │
├─────────────────────────────────────────────────────────────────┤
│ PK  id: HypothesisId                                            │
│ FK  anomaly_id: AnomalyId                                       │
│    causal_factors: Vec<CausalFactor>                            │
│    confidence_score: f64                                        │
│    evidence: Evidence                                           │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. Incident Management Context ERD

```
┌──────────────────────────────────┐
│    Incident (Aggregate)          │
├──────────────────────────────────┤
│ PK  id: IncidentId               │
│    incident_number: String       │
│    title: String                 │
│    status: IncidentStatus        │
│    severity: IncidentSeverity    │
│    detected_at: DateTime         │
│    assigned_to: UserId?          │
│    resolved_at: DateTime?        │
└──────────────────────────────────┘
            │ *
            │ grouped under
            │ 1
┌───────────▼─────────────────────┐
│   CorrelationGroup              │
├─────────────────────────────────┤
│ PK  id: CorrelationGroupId      │
│    incidents: Vec<IncidentId>   │
│    root_cause_anomaly: AnomalyId?│
│    strategy: CorrelationStrategy│
│    created_at: DateTime         │
└─────────────────────────────────┘
            │ 1
            │ maps to
            │ *
┌───────────▼─────────────────────┐     ┌───────────────────────────┐
│   AlertDeduplicator             │     │  RemediationPlan          │
├─────────────────────────────────┤     ├───────────────────────────┤
│ PK  fingerprint: String         │     │ FK  incident_id: IncidentId│
│    last_seen: DateTime          │     │ FK  workflow_id: WorkflowId│
│    count: u64                   │     │    status: PlanStatus     │
│    status: DeduplicationStatus  │     │    steps: Vec<Step>       │
└─────────────────────────────────┘     │    approval_state: State  │
                                          └───────────────────────────┘
```

---

## 6. Remediation Context ERD

```
┌──────────────────────────────────────┐
│  RemediationWorkflow (Aggregate)     │
├──────────────────────────────────────┤
│ PK  id: WorkflowId                   │
│ FK  incident_id: IncidentId          │
│    workflow_type: WorkflowType       │
│    status: WorkflowStatus            │
│    steps: Vec<WorkflowStep>          │
│    current_step: Option<usize>       │
│    started_at: DateTime              │
│    completed_at: DateTime?           │
│    result: Option<WorkflowResult>    │
└──────────────────────────────────────┘
            │ 1
            │ has
            │ *
┌───────────▼──────────────────────────┐     ┌─────────────────────────┐
│         WorkflowStep                 │     │    ApprovalGate         │
├──────────────────────────────────────┤     ├─────────────────────────┤
│ PK  id: Uuid                         │     │ PK  id: ApprovalId      │
│ FK  workflow_id: WorkflowId          │     │ FK  workflow_id: WorkflowId│
│    step_number: usize                │     │    required_approvers: u8│
│ FK  action_id: Uuid                  │     │    current_approvals: u8│
│    status: StepStatus                │     │    approvers: Set<UserId>│
│    started_at: DateTime?             │     │    status: ApprovalStatus│
│    completed_at: DateTime?           │     │    deadline: DateTime?   │
│    output: Option<String>            │     └─────────────────────────┘
│    error: Option<String>             │
└──────────────────────────────────────┘
            │ 1
            │ has
            │ 1
┌───────────▼──────────────────────────┐
│      RemediationAction               │
├──────────────────────────────────────┤
│ PK  id: Uuid                         │
│    action_type: ActionType           │
│    target: Target                    │
│    parameters: ActionParameters      │
│    timeout: Duration                 │
│ FK  rollback_action: Uuid?           │
└──────────────────────────────────────┘
```

---

## 7. Service Topology Context ERD

```
┌──────────────────────────────────────┐
│     ServiceGraph (Aggregate)         │
├──────────────────────────────────────┤
│ PK  id: GraphId                      │
│    cluster: String                   │
│    version: u64                      │
│    last_updated: DateTime            │
└──────────────────────────────────────┘
            │ 1
            │ contains
            │ *
┌───────────▼──────────────────────┐     ┌──────────────────────────────┐
│        ServiceNode               │     │      DependencyEdge          │
├──────────────────────────────────┤     ├──────────────────────────────┤
│ PK  id: ServiceId                │     │ PK  id: EdgeId               │
│ FK  identity: ServiceIdentity     │     │ FK  from_service: ServiceId  │
│    service_type: ServiceType      │     │ FK  to_service: ServiceId    │
│    health_status: HealthStatus    │     │    dependency_type: DepType  │
│    capabilities: Vec<String>      │     │    latency: LatencyProfile?  │
│    metadata: Map<String, String>  │     │    call_frequency: CallFreq? │
└──────────────────────────────────┘     │    strength: Strength        │
                                          └──────────────────────────────┘
            │ 1
            │ analyzes
            │ 1
┌───────────▼──────────────────────────┐
│       ImpactAnalysis                 │
├──────────────────────────────────────┤
│ PK  id: AnalysisId                   │
│ FK  failed_service: ServiceId        │
│    impacted_services: Vec<Impact>    │
│    blast_radius: BlastRadius         │
│    user_impact: UserImpact           │
│    generated_at: DateTime            │
└──────────────────────────────────────┘
```

---

## 8. Knowledge Management Context ERD

```
┌──────────────────────────────────────┐
│   KnowledgeBase (Aggregate)          │
├──────────────────────────────────────┤
│ PK  id: KnowledgeBaseId              │
│    name: String                      │
│    indexing_state: IndexingState     │
│    last_updated: DateTime            │
└──────────────────────────────────────┘
            │ 1
            │ contains
            │ *
┌───────────▼──────────────────────────┐     ┌─────────────────────────┐
│      KnowledgeEntry                  │     │   LearnedPattern        │
├──────────────────────────────────────┤     ├─────────────────────────┤
│ PK  id: EntryId                      │     │ PK  id: PatternId       │
│    entry_type: EntryType             │     │    pattern_type: PatternType│
│    title: String                     │     │    trigger_conditions: Vec│
│    content: EntryContent             │     │    remediation_steps: Vec│
│    tags: Vec<String>                 │     │    success_rate: f64     │
│ FK  embeddings: VectorEmbedding?     │     │    confidence_score: f64 │
│    usage_stats: UsageStats           │     │    times_used: u64       │
└──────────────────────────────────────┘     │    last_success: DateTime?│
                                              └─────────────────────────┘
            │ 1
            │ generates
            │ 1
┌───────────▼──────────────────────────┐
│     VectorEmbedding                  │
├──────────────────────────────────────┤
│ PK  entry_id: EntryId                │
│    vector: Vec<f32>                  │
│    model: String                     │
│    dimension: usize                  │
└──────────────────────────────────────┘
```

---

## 9. Incident Lifecycle State Machine

```
     ┌─────────┐
     │   New   │
     └────┬────┘
          │
          │ Detection
          ▼
    ┌───────────┐
    │ Detected  │
    └─────┬─────┘
          │
          │ Analysis Started
          ▼
    ┌───────────┐
    │ Analyzing │
    └─────┬─────┘
          │
          │ Remediation Needed
          ▼
    ┌───────────┐
    │Remediating│◄─────────────────┐
    └─────┬─────┘                   │
          │                         │
          │ Resolved                │ Failed
          ▼                         │
    ┌───────────┐                   │
    │ Resolved  │                   │
    └─────┬─────┘                   │
          │                         │
          │ Close Incident          │
          ▼                         ▼
    ┌───────────┐           ┌───────────┐
    │  Closed   │           │  Failed   │
    └───────────┘           └───────────┘
```

---

## 10. Anomaly State Machine

```
     ┌──────────┐
     │  Normal  │
     └────┬─────┘
          │
          │ Threshold Violation
          ▼
    ┌─────────────┐
    │ Suspicious  │
    └──────┬──────┘
           │
           │ ML Confirmation
           ▼
    ┌─────────────┐
    │  Anomalous  │
    └──────┬──────┘
           │
           │ Return to Baseline
           ▼
    ┌─────────────┐
    │  Recovered  │
    └──────┬──────┘
           │
           │ Back to Normal
           ▼
     ┌──────────┐
     │  Normal  │
     └──────────┘
```

---

## 11. Remediation Workflow State Machine

```
     ┌──────────────────┐
     │ PendingApproval  │◄────────┐
     └─────────┬────────┘         │
               │                  │
               │ Approved         │ Rejected
               ▼                  │
        ┌─────────────┐           │
        │   Approved  │           │
        └──────┬──────┘           │
               │                  │
               │ Start            │
               ▼                  │
        ┌─────────────┐           │
        │  InProgress │           │
        └──────┬──────┘           │
               │                  │
        ┌──────┴──────┐           │
        │             │           │
        ▼             ▼           │
    ┌───────┐    ┌─────────┐     │
    │Success│    │ Failed  │─────┤
    └───┬───┘    └────┬────┘     │
        │             │          │
        ▼             │          │
    ┌───────┐         │          │
    │Completed◄───────┘          │
    └───────┘                    │
        │                        │
        │ Rollback Needed        │
        ▼                        │
    ┌───────────┐                │
    │RolledBack │────────────────┘
    └───────────┘
```

---

## 12. Integration Pattern: ACL (Anti-Corruption Layer)

```
┌──────────────────────────────────────────────────────────────────┐
│                    Anti-Corruption Layer                        │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐        ┌─────────────────┐                │
│  │   RustOps       │        │  External       │                │
│  │   Domain        │        │  System         │                │
│  │   (Incident)    │        │  (ServiceNow)   │                │
│  └────────┬────────┘        └────────┬────────┘                │
│           │                          │                          │
│           │  Domain Model            │ External Model           │
│           │  - IncidentId            │ - sys_id                 │
│           │  - IncidentSeverity      │ - priority               │
│           │  - IncidentStatus        │ - state                  │
│           │                          │                          │
│           ▼                          ▼                          │
│  ┌─────────────────────────────────────────────────┐           │
│  │            Translation Layer                     │           │
│  │  ┌───────────────┐      ┌───────────────┐       │           │
│  │  │   Adapter     │      │   Mapper      │       │           │
│  │  │   (HTTP/gRPC) │      │   (Transform) │       │           │
│  │  └───────────────┘      └───────────────┘       │           │
│  │                                                    │           │
│  │  Domain Incident ──▶ External Ticket              │           │
│  │  External Ticket ──▶ Domain Incident              │           │
│  └─────────────────────────────────────────────────┘           │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 13. Event Sourcing Pattern

```
┌──────────────────────────────────────────────────────────────────┐
│                   Event Sourcing Architecture                    │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐                                           │
│  │   Command       │                                           │
│  │  (CreateIncident│                                           │
│  │   Resolve, etc.)│                                           │
│  └────────┬────────┘                                           │
│           │                                                    │
│           ▼                                                    │
│  ┌─────────────────┐                                           │
│  │   Aggregate     │                                           │
│  │  (Incident)     │                                           │
│  └────────┬────────┘                                           │
│           │                                                    │
│           │ Generate Events                                    │
│           ▼                                                    │
│  ┌─────────────────────────────────────────────────┐           │
│  │              Domain Events                       │           │
│  │  1. IncidentCreated                             │           │
│  │  2. IncidentAssigned                            │           │
│  │  3. IncidentStatusChanged                       │           │
│  │  4. IncidentResolved                            │           │
│  └────────────────┬────────────────────────────────┘           │
│                   │                                             │
│                   │ Append to Event Store                       │
│                   ▼                                             │
│  ┌─────────────────────────────────────────────────┐           │
│  │           Event Store (Append-Only)              │           │
│  │  ┌─────────────────────────────────────────┐    │           │
│  │  │ IncidentStream: INC-123                   │    │           │
│  │  │  - Event 1: IncidentCreated              │    │           │
│  │  │  - Event 2: IncidentAssigned             │    │           │
│  │  │  - Event 3: IncidentStatusChanged        │    │           │
│  │  │  - Event 4: IncidentResolved             │    │           │
│  │  └─────────────────────────────────────────┘    │           │
│  └────────────────┬────────────────────────────────┘           │
│                   │                                             │
│                   │ Read & Replay                              │
│                   ▼                                             │
│  ┌─────────────────────────────────────────────────┐           │
│  │            Projections (Read Models)            │           │
│  │  ┌───────────────┐  ┌───────────────┐          │           │
│  │  │ Incident View│  │ Dashboard    │          │           │
│  │  │ (Current State│  │ (Analytics)  │          │           │
│  │  └───────────────┘  └───────────────┘          │           │
│  └─────────────────────────────────────────────────┘           │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 14. Repository Pattern

```
┌──────────────────────────────────────────────────────────────────┐
│                    Repository Pattern                            │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐                                           │
│  │   Application   │                                           │
│  │   Service       │                                           │
│  └────────┬────────┘                                           │
│           │                                                    │
│           │ Uses Interface                                     │
│           ▼                                                    │
│  ┌─────────────────────────────────────────────────┐           │
│  │         Repository<T> (trait)                   │           │
│  │  ┌─────────────────────────────────────────┐    │           │
│  │  │ async fn get(id: &T::Id) -> Option<T>   │    │           │
│  │  │ async fn save(aggregate: &T) -> Result   │    │           │
│  │  │ async fn delete(id: &T::Id) -> Result    │    │           │
│  │  └─────────────────────────────────────────┘    │           │
│  └────────────────┬────────────────────────────────┘           │
│                   │                                             │
│                   │ Implemented by                              │
│                   ▼                                             │
│  ┌─────────────────────────────────────────────────┐           │
│  │    PostgresIncidentRepository                   │           │
│  │    - SQL implementation                          │           │
│  │    - Connection pooling                          │           │
│  │    - Transaction management                     │           │
│  └─────────────────────────────────────────────────┘           │
│                   │                                             │
│                   │ OR                                          │
│                   ▼                                             │
│  ┌─────────────────────────────────────────────────┐           │
│  │    EventSourcedIncidentRepository               │           │
│  │    - Event store append                          │           │
│  │    - Snapshot management                         │           │
│  │    - Event replay for reconstruction             │           │
│  └─────────────────────────────────────────────────┘           │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 15. Aggregate Boundary Example

```
┌──────────────────────────────────────────────────────────────────┐
│              Incident Aggregate Boundary                         │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────┐           │
│  │            Incident (Aggregate Root)            │           │
│  │  ┌─────────────────────────────────────────┐    │           │
│  │  │  - id: IncidentId                        │    │           │
│  │  │  - status: IncidentStatus                │    │           │
│  │  │  - severity: IncidentSeverity            │    │           │
│  │  │  - version: u64 (for optimistic locking)  │    │           │
│  │  └─────────────────────────────────────────┘    │           │
│  │                                                   │           │
│  │  ┌─────────────────────────────────────────┐    │           │
│  │  │          Invariants                      │    │           │
│  │  │  1. Status transitions must be valid    │    │           │
│  │  │  2. Cannot resolve without assignment   │    │           │
│  │  │  3. Incident numbers are unique         │    │           │
│  │  └─────────────────────────────────────────┘    │           │
│  │                                                   │           │
│  │  ┌─────────────────────────────────────────┐    │           │
│  │  │          Methods                         │    │           │
│  │  │  - assign(user_id)                       │    │           │
│  │  │  - transition_to(new_status)             │    │           │
│  │  │  - add_anomaly(anomaly_id)               │    │           │
│  │  └─────────────────────────────────────────┘    │           │
│  │                                                   │           │
│  │  ┌─────────────────────────────────────────┐    │           │
│  │  │       Events (emit on state change)      │    │           │
│  │  │  - IncidentCreated                       │    │           │
│  │  │  - IncidentAssigned                      │    │           │
│  │  │  - IncidentStatusChanged                 │    │           │
│  │  └─────────────────────────────────────────┘    │           │
│  └─────────────────────────────────────────────────┘           │
│                                                                  │
│  Outside aggregate: Reference by ID only                       │
│  - related_anomalies: Vec<AnomalyId> (not Anomaly entities)     │
│  - remediation_plan: Option<WorkflowId> (not Workflow entity)   │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 16. Context Relationships (Partnership, ACL, etc.)

```
┌──────────────────────────────────────────────────────────────────┐
│                    Context Mapping                               │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────┐                                          │
│  │ Anomaly Detection│                                          │
│  └────────┬─────────┘                                          │
│           │                                                    │
│           │ PARTNERSHIP (Shared Kernel)                         │
│           │ - Anomaly definition                                │
│           │ - Severity levels                                   │
│           │ - Confidence scoring                                │
│           ▼                                                    │
│  ┌──────────────────┐                                          │
│  │Incident Mgmt     │                                          │
│  └────────┬─────────┘                                          │
│           │                                                    │
│           │ CUSTOMER-SUPPLIER (Domain Events)                  │
│           │ - IncidentCreated                                  │
│           │ - RemediationRequested                             │
│           ▼                                                    │
│  ┌──────────────────┐                                          │
│  │  Remediation     │                                          │
│  └──────────────────┘                                          │
│                                                                  │
│  ┌────────────────────────────────────────┐                     │
│  │         Integration Context             │                     │
│  │                                         │                     │
│  │  ACL (Anti-Corruption Layer)            │                     │
│  │  ┌────────────────────────────────┐    │                     │
│  │  │ ServiceNow Adapter              │    │                     │
│  │  │ - Domain Incident ──▶ Ticket    │    │                     │
│  │  │ - Ticket ──▶ Domain Incident    │    │                     │
│  │  └────────────────────────────────┘    │                     │
│  │                                         │                     │
│  │  CONFORMIST                             │                     │
│  │  - Adopt ServiceNow data model          │                     │
│  │  - No transformation layer              │                     │
│  └────────────────────────────────────────┘                     │
│                                                                  │
│  ┌──────────────────┐                                          │
│  │Service Topology  │                                          │
│  └────────┬─────────┘                                          │
│           │                                                    │
│           │ OPEN HOST SERVICE (Public API)                     │
│           │ - GetDependencies(service_id)                      │
│           │ - GetImpactAnalysis(service_id)                    │
│           │ - GetDownstreamServices(service_id)                 │
│           │                                                      │
│           └────────────┬──────────────────────────────────┐    │
│                        │                                  │    │
│                   All Contexts consume                   │    │
│                   topology data                          │    │
│                                                        │    │
│  ┌──────────────────┐                            ┌─────▼────▼────┐
│  │ Knowledge        │                            │   All         │
│  │ Management       │                            │   Contexts    │
│  └──────────────────┘                            └───────────────┘
│       │                                                     │
│       │ PUBLISHED LANGUAGE (Domain Events)                 │
│       │ - PatternLearned                                   │
│       │ - KnowledgeEntryUpdated                            │
│       └───────────────────────────────────────────────────┘
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```
