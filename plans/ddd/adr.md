# Architecture Decision Records - RustOps DDD

This document contains architecture decisions for the Domain-Driven Design implementation of RustOps.

---

## ADR-001: Bounded Context Isolation

**Status**: Accepted
**Date**: 2026-01-18
**Context**: Telemetry Collection, Anomaly Detection

### Problem

The AIOps domain is complex with multiple concerns: data collection, anomaly detection, incident management, remediation, topology, integration, and knowledge management. A monolithic design would create tight coupling and make the system difficult to understand, maintain, and scale.

### Decision

Divide the domain into **7 bounded contexts** with strict boundaries:

1. **Telemetry Collection** - Ingest and normalize data
2. **Anomaly Detection** - Detect patterns and anomalies
3. **Incident Management** - Correlate and manage incidents
4. **Remediation** - Orchestrate self-healing workflows
5. **Service Topology** - Map dependencies and analyze impact
6. **Integration** - Synchronize with external systems
7. **Knowledge Management** - Store and retrieve patterns

Contexts communicate via **domain events** only. No direct method calls between contexts.

### Consequences

**Positive**:
- Clear ownership and reduced coupling
- Independent deployment and scaling
- Easier testing and maintenance
- Teams can work on contexts in parallel

**Negative**:
- More complex initial setup
- Event-driven communication adds latency (~10-50ms)
- Requires event schema versioning
- Debugging distributed workflows is harder

### Alternatives Considered

1. **Monolithic Architecture** - Rejected due to complexity
2. **Microservices by Layer** - Rejected (doesn't align with domain)
3. **Feature Modules** - Considered but rejected (insufficient isolation)

---

## ADR-002: Event Sourcing for Incident History

**Status**: Accepted
**Date**: 2026-01-18
**Context**: Incident Management Context

### Problem

Incidents have rich history requirements:
- Complete audit trail for compliance
- Temporal queries (what happened at time T?)
- Debugging support (replay events to understand state)
- State reconstruction for analytics

Traditional CRUD doesn't provide these capabilities efficiently.

### Decision

Use **event sourcing** for the Incident aggregate with:

- **Event Store**: Append-only log of all domain events
- **Snapshots**: Periodic state snapshots (every 50 events)
- **Projections**: Read models built from event stream
- **Event Replay**: Ability to rebuild state from events

```rust
// Event stream for incident INC-123
IncidentCreated    { incident_id: "...", severity: "P2", at: "2026-01-18T10:00:00Z" }
IncidentAssigned   { incident_id: "...", assigned_to: "alice", at: "2026-01-18T10:01:00Z" }
StatusChanged      { incident_id: "...", from: "New", to: "Analyzing", at: "2026-01-18T10:05:00Z" }
AnomalyAdded       { incident_id: "...", anomaly_id: "...", at: "2026-01-18T10:06:00Z" }
RemediationStarted { incident_id: "...", workflow_id: "...", at: "2026-01-18T10:10:00Z" }
StatusChanged      { incident_id: "...", from: "Analyzing", to: "Remediating", at: "2026-01-18T10:10:00Z" }
StatusChanged      { incident_id: "...", from: "Remediating", to: "Resolved", at: "2026-01-18T10:30:00Z" }
```

### Consequences

**Positive**:
- Complete audit trail (compliance)
- Temporal queries (what was state at time T?)
- Event replay for debugging
- Easy to add new projections

**Negative**:
- Increased storage (store all events, not just current state)
- Snapshot complexity
- Event schema versioning required
- Event replay is slow for long histories

### Implementation Notes

- Use Kafka for event streaming (ordered, retained)
- Snapshot after every 50 events or 1 hour
- Keep events for 90 days (compliance requirement)
- Use event versioning for schema evolution

### Alternatives Considered

1. **CRUD with audit table** - Rejected (doesn't support replay)
2. **CQRS without event sourcing** - Rejected (no audit trail)
3. **Hybrid (event sourcing for hot data)** - Rejected (complexity)

---

## ADR-003: Vector Embeddings for Knowledge

**Status**: Accepted
**Date**: 2026-01-18
**Context**: Knowledge Management Context

### Problem

Knowledge retrieval needs:
- Semantic search (find similar runbooks)
- ML-powered recommendations
- Fast similarity queries (sub-100ms)
- Multi-modal content (text, code, metrics)

Traditional keyword search doesn't provide semantic understanding.

### Decision

Use **vector embeddings** for all knowledge entries:

- **Embedding Model**: `all-MiniLM-L6-v2` (384 dimensions, fast)
- **Indexing**: HNSW (Hierarchical Navigable Small World) for 150x-12,500x faster search
- **Similarity**: Cosine similarity for normalized vectors
- **Storage**: Vector DB (AgentDB with HNSW backend)

```rust
// Knowledge entry with embedding
pub struct KnowledgeEntry {
    pub id: EntryId,
    pub title: String,
    pub content: EntryContent,
    pub embeddings: Option<VectorEmbedding>,  // <- Vector representation
    pub tags: Vec<String>,
}

// Vector embedding
pub struct VectorEmbedding {
    pub vector: Vec<f32>,  // Normalized unit vector
    pub model: String,
    pub dimension: usize,
}

// Semantic search
impl KnowledgeBase {
    pub fn search_semantic(&self, query: &str, top_k: usize) -> Vec<EntryId> {
        let query_embedding = self.embed(query);
        self.index.search(&query_embedding, top_k)
    }
}
```

### Consequences

**Positive**:
- Semantic search capabilities
- Fast similarity queries (HNSW)
- ML-powered recommendations
- Multi-modal support (text, code)

**Negative**:
- Embedding computation overhead (~50ms per entry)
- Additional infrastructure (vector DB)
- Need to re-index on model updates
- Embedding quality depends on training data

### Implementation Notes

- Embed on entry creation (async, non-blocking)
- Use batch processing for bulk imports
- Cache frequently accessed embeddings
- Re-index quarterly with new models

### Alternatives Considered

1. **Keyword search only** - Rejected (no semantic understanding)
2. **BERT embeddings** - Rejected (too slow: ~500ms vs 50ms)
3. **Hybrid (keyword + vector)** - Accepted (use both)

---

## ADR-004: Approval Gates for Remediation

**Status**: Accepted
**Date**: 2026-01-18
**Context**: Remediation Context

### Problem

Auto-remediation can cause unintended consequences:
- Production outages from bad actions
- Cascading failures
- Compliance violations
- Loss of human oversight

Need to balance automation speed with safety.

### Decision

Implement **graduated approval gates** based on blast radius and risk:

```rust
pub struct RemediationWorkflow {
    pub approval_state: ApprovalState,
    pub workflow_type: WorkflowType,
    pub blast_radius: BlastRadius,
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

### Approval Workflow

```
┌─────────────────┐
│ Workflow Created │
└────────┬────────┘
         │
         ▼
┌─────────────────┐    Yes    ┌─────────────────┐
│ Approval Needed? │─────────▶│ PendingApproval  │
└────────┬────────┘           └────────┬────────┘
         │ No                          │
         ▼                             │ Approve
┌─────────────────┐                   ▼
│     Approved    │◄─────────────────┤
└────────┬────────┘                   │
         │                            │
         ▼                            ▼
┌─────────────────┐           ┌─────────────────┐
│    InProgress   │           │    Rejected     │
└────────┬────────┘           └─────────────────┘
         │
         ▼
┌─────────────────┐
│   Completed     │
└─────────────────┘
```

### Consequences

**Positive**:
- Controlled automation rollout
- Human oversight for critical actions
- Audit trail for compliance
- Gradual trust building

**Negative**:
- Slower remediation for critical actions (~5-10 min delay)
- Approval workflow complexity
- Potential bottlenecks (approver availability)

### Implementation Notes

- Default: 1 approver required
- High risk: 2+ approvers
- Timeout: 30 minutes (auto-reject)
- Audit all approval decisions
- Allow emergency bypass (with extra logging)

### Alternatives Considered

1. **No approvals** - Rejected (too risky for production)
2. **All actions require approval** - Rejected (defeats automation purpose)
3. **Human-in-loop only** - Rejected (slow, doesn't scale)

---

## ADR-005: Anti-Corruption Layer for Integration

**Status**: Accepted
**Date**: 2026-01-18
**Context**: Integration Context

### Problem

External ITSM systems (ServiceNow, Jira, PagerDuty) have:
- Different data models
- Volatile APIs (version changes, deprecations)
- Inconsistent terminology
- Rate limits and downtime

Direct coupling would pollute the domain model.

### Decision

Implement **Anti-Corruption Layers (ACLs)** for all external integrations:

```rust
// Domain model (clean)
pub struct Incident {
    pub id: IncidentId,
    pub severity: IncidentSeverity,  // P1, P2, P3, P4
    pub status: IncidentStatus,      // New, Detected, ...
}

// External model (ServiceNow)
pub struct ServiceNowTicket {
    pub sys_id: String,
    pub priority: String,  // 1, 2, 3, 4, 5 (different!)
    pub state: String,     // New, Active, Pending, ...
}

// ACL: Translation layer
pub struct ServiceNowAdapter {
    client: ServiceNowClient,
    mapper: ServiceNowMapper,
}

impl ServiceNowAdapter {
    pub fn domain_to_ticket(&self, incident: &Incident) -> ServiceNowTicket {
        ServiceNowTicket {
            sys_id: incident.id.to_string(),
            priority: self.map_severity(incident.severity),  // Translation!
            state: self.map_status(incident.status),          // Translation!
        }
    }

    pub fn ticket_to_domain(&self, ticket: ServiceNowTicket) -> Incident {
        Incident {
            id: IncidentId::parse(&ticket.sys_id),
            severity: self.map_priority(ticket.priority),  // Reverse translation!
            status: self.map_state(ticket.state),          // Reverse translation!
            // ...
        }
    }

    fn map_severity(&self, severity: IncidentSeverity) -> String {
        match severity {
            IncidentSeverity::P1 => "1",  // Critical
            IncidentSeverity::P2 => "2",  // High
            IncidentSeverity::P3 => "3",  // Moderate
            IncidentSeverity::P4 => "4",  // Low
        }
    }
}
```

### Consequences

**Positive**:
- Domain model stays clean
- External changes isolated to adapters
- Easy to add new integrations
- Testable (mock adapters)

**Negative**:
- More code to maintain
- Translation overhead
- Potential sync issues
- Need to keep mappings up-to-date

### Implementation Notes

- One adapter per external system
- Use domain events for sync (eventual consistency)
- Retry with exponential backoff
- Circuit breaker for failing systems
- Sync every 30 seconds (configurable)

### Alternatives Considered

1. **Direct integration** - Rejected (pollutes domain)
2. **Shared kernel** - Rejected (can't control external APIs)
3. **Conformist** - Accepted for simple cases (adopt external model)

---

## ADR-006: Snapshot Strategy for Event Sourcing

**Status**: Accepted
**Date**: 2026-01-18
**Context**: Incident Management, Event Sourcing Infrastructure

### Problem

Event sourcing stores all events. For long-lived entities:
- Replay time grows linearly with event count
- 1000 events = ~5 seconds replay time
- Need to optimize for both hot and cold data

### Decision

Implement **hybrid snapshot strategy**:

```rust
pub struct Snapshot {
    pub aggregate_id: Uuid,
    pub aggregate_version: u64,
    pub state: AggregateState,
    pub created_at: DateTime<Utc>,
}

pub trait SnapshotStrategy {
    fn should_snapshot(&self, aggregate: &Aggregate) -> bool {
        // Snapshot every 50 events OR 1 hour
        aggregate.version() % 50 == 0
            || aggregate.last_event_time().elapsed() > Duration::from_hours(1)
    }

    fn load(&self, id: Uuid) -> DomainResult<Aggregate> {
        // 1. Load latest snapshot
        if let Some(snapshot) = self.load_latest_snapshot(id)? {
            // 2. Load events after snapshot
            let events = self.load_events_since(id, snapshot.version)?;
            // 3. Replay events onto snapshot
            let mut aggregate = snapshot.state;
            for event in events {
                aggregate.apply(event)?;
            }
            Ok(aggregate)
        } else {
            // No snapshot: replay all events
            self.replay_all(id)
        }
    }
}
```

### Snapshot Strategy

| Condition | Action |
|-----------|--------|
| Every 50 events | Create snapshot |
| Every 1 hour | Create snapshot |
| On aggregate completion | Create snapshot |
| Snapshot older than 24 hours | Refresh snapshot |

### Consequences

**Positive**:
- Fast load times (snapshot + few events)
- Reduced replay overhead
- Supports long event histories
- Configurable strategy per aggregate

**Negative**:
- Snapshot storage overhead
- Snapshot staleness issues
- Complex load logic
- Need snapshot versioning

### Implementation Notes

- Store snapshots in separate table/collection
- Include snapshot version in event metadata
- Compress snapshots (gzip)
- Retain snapshots for 90 days

### Alternatives Considered

1. **No snapshots** - Rejected (slow replay for long histories)
2. **Snapshot every event** - Rejected (too much overhead)
3. **Fixed interval snapshots** - Rejected (inflexible)

---

## ADR-007: Repository Interface for Persistence

**Status**: Accepted
**Date**: 2026-01-18
**Context**: All Contexts (Infrastructure Layer)

### Problem

Need persistence abstraction for aggregates:
- Hide implementation details (SQL, NoSQL, Event Sourcing)
- Enable testing (mock repositories)
- Support multiple persistence strategies
- Clean separation from domain logic

### Decision

Define **Repository trait** for all aggregates:

```rust
#[async_trait::async_trait]
pub trait Repository<A: Aggregate>: Send + Sync {
    async fn get(&self, id: &A::Id) -> DomainResult<Option<A>>;
    async fn save(&self, aggregate: &A) -> DomainResult<()>;
    async fn delete(&self, id: &A::Id) -> DomainResult<()>;
}

// Implementation for Postgres
pub struct PostgresIncidentRepository {
    pool: PgPool,
}

#[async_trait::async_trait]
impl Repository<Incident> for PostgresIncidentRepository {
    async fn get(&self, id: &IncidentId) -> DomainResult<Option<Incident>> {
        // SQL query to load incident
    }

    async fn save(&self, incident: &Incident) -> DomainResult<()> {
        // UPSERT with optimistic locking
    }

    async fn delete(&self, id: &IncidentId) -> DomainResult<()> {
        // DELETE query
    }
}

// Implementation for Event Sourcing
pub struct EventSourcedIncidentRepository {
    event_store: EventStore,
    snapshot_store: SnapshotStore,
}

#[async_trait::async_trait]
impl Repository<Incident> for EventSourcedIncidentRepository {
    async fn get(&self, id: &IncidentId) -> DomainResult<Option<Incident>> {
        // Load from snapshot + replay events
    }

    async fn save(&self, incident: &Incident) -> DomainResult<()> {
        // Append events to event store
    }

    async fn delete(&self, id: &IncidentId) -> DomainResult<()> {
        // Mark aggregate as deleted (soft delete)
    }
}
```

### Consequences

**Positive**:
- Clean separation (domain doesn't know about DB)
- Easy to test (mock repositories)
- Multiple implementations possible
- Optimistic locking built-in

**Negative**:
- More abstraction layers
- Potential performance overhead
- Need to manage transactions
- Limited query capabilities (use projections)

### Implementation Notes

- Use optimistic locking (version field)
- Transactions span multiple repository calls
- Projections for complex queries
- Repositories don't expose query logic

### Alternatives Considered

1. **Active Record** - Rejected (couples domain to DB)
2. **Data Mapper** - Considered (similar to Repository)
3. **Direct SQL** - Rejected (no abstraction)

---

## ADR-008: Domain Event Standards

**Status**: Accepted
**Date**: 2026-01-18
**Context**: All Contexts (Cross-Context Communication)

### Problem

Domain events need consistency:
- Event structure
- Metadata requirements
- Serialization format
- Versioning strategy

Without standards, event handling becomes brittle.

### Decision

Define **Domain Event trait** and standards:

```rust
pub trait DomainEvent: Send + Sync + Clone {
    fn event_id(&self) -> Uuid;
    fn event_type(&self) -> &'static str;
    fn occurred_at(&self) -> DateTime<Utc>;
    fn causation_id(&self) -> Option<Uuid>;      // What caused this event?
    fn correlation_id(&self) -> Option<Uuid>;     // Request/operation grouping
}

// Standard event metadata
pub struct EventMetadata {
    pub event_id: Uuid,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub causation_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
}

// Event wrapper for serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEventData {
    pub metadata: EventMetadata,
    pub payload: serde_json::Value,
}

// Example event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentCreatedEvent {
    pub incident_id: IncidentId,
    pub incident_number: IncidentNumber,
    pub severity: IncidentSeverity,
    pub title: String,
}

impl DomainEvent for IncidentCreatedEvent {
    fn event_id(&self) -> Uuid {
        // Generate unique ID
    }

    fn event_type(&self) -> &'static str {
        "IncidentCreated"
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn causation_id(&self) -> Option<Uuid> {
        // What caused this incident to be created?
        // e.g., AnomalyDetected event ID
    }

    fn correlation_id(&self) -> Option<Uuid> {
        // Group related events (e.g., same alert)
    }
}
```

### Event Standards

| Field | Required | Description |
|-------|----------|-------------|
| `event_id` | Yes | Unique event identifier (UUID) |
| `event_type` | Yes | Event type name (e.g., "IncidentCreated") |
| `occurred_at` | Yes | When the event occurred |
| `causation_id` | No | What caused this event? |
| `correlation_id` | No | Group related events |
| `aggregate_id` | Yes | ID of the aggregate |
| `aggregate_version` | Yes | Version after applying event |

### Event Versioning

```rust
// Use event versioning for breaking changes
pub struct AnomalyDetectedEventV2 {
    pub anomaly_id: AnomalyId,
    pub severity: AnomalySeverity,
    pub confidence: f64,
    pub features: AnomalyFeatures,  // New field in V2
}

// Support both versions during migration
impl From<AnomalyDetectedEventV1> for AnomalyDetectedEventV2 {
    fn from(v1: AnomalyDetectedEventV1) -> Self {
        Self {
            anomaly_id: v1.anomaly_id,
            severity: v1.severity,
            confidence: v1.confidence,
            features: AnomalyFeatures::default(),  // Default for V1
        }
    }
}
```

### Consequences

**Positive**:
- Consistent event structure
- Easy to debug (trace causation)
- Event correlation possible
- Clean versioning strategy

**Negative**:
- More boilerplate per event
- Metadata overhead (~100 bytes per event)
- Need to handle versioning

### Alternatives Considered

1. **Free-form events** - Rejected (no consistency)
2. **Binary protocol** - Rejected (not human-readable)
3. **CloudEvents standard** - Considered (adopt subset)

---

## ADR-009: Aggregate Consistency Boundaries

**Status**: Accepted
**Date**: 2026-01-18
**Context**: All Contexts (Domain Model Design)

### Problem

Need to define transactional boundaries:
- What operations must be atomic?
- What can be eventually consistent?
- How to handle distributed transactions?

### Decision

**Aggregates are consistency boundaries**:

```rust
// Rule: All invariants enforced within aggregate
impl Incident {
    pub fn transition_to(&mut self, new_status: IncidentStatus) -> DomainResult<()> {
        // Invariant: Valid status transitions only
        self.validate_transition(self.status, new_status)?;
        self.status = new_status;
        self.version += 1;
        Ok(())
    }
}

// Rule: Reference other aggregates by ID only
pub struct Incident {
    pub related_anomalies: Vec<AnomalyId>,  // Not Vec<Anomaly>
    pub remediation_plan: Option<WorkflowId>,  // Not Option<Workflow>
}

// Rule: Cross-aggregate operations use domain events
impl Incident {
    pub fn request_remediation(&self) -> DomainResult<Vec<DomainEvent>> {
        Ok(vec![
            RemediationRequestedEvent {
                incident_id: self.id,
                severity: self.severity,
            }
        ])
    }
}
```

### Consistency Rules

| Scope | Consistency | Example |
|-------|-------------|---------|
| Within aggregate | Strong (atomic) | Incident status transition |
| Between aggregates (same context) | Eventual | Anomaly → Incident |
| Between contexts | Eventual | Incident → Remediation |
| Projections | Eventual | Dashboard views |

### Transaction Management

```rust
// Single aggregate: Single transaction
async fn create_incident(incident: Incident) -> DomainResult<()> {
    let mut tx = db.begin().await?;
    incident_repository.save(&mut tx, &incident).await?;
    tx.commit().await?;
    Ok(())
}

// Multiple aggregates: Separate transactions + events
async fn detect_anomaly_and_create_incident(
    anomaly: Anomaly,
) -> DomainResult<()> {
    // Save anomaly
    let mut tx1 = db.begin().await?;
    anomaly_repository.save(&mut tx1, &anomaly).await?;
    tx1.commit().await?;

    // Publish event (async, non-blocking)
    event_bus.publish(AnomalyDetectedEvent::from(&anomaly)).await?;

    // Incident handler creates incident (separate transaction)
    Ok(())
}
```

### Consequences

**Positive**:
- Clear consistency boundaries
- No distributed transactions
- Optimistic locking built-in
- Scales horizontally

**Negative**:
- Eventual consistency between aggregates
- More complex workflows
- Need to handle compensation
- Temporary inconsistencies possible

### Alternatives Considered

1. **Global transactions** - Rejected (doesn't scale)
2. **Sagas** - Considered (use for complex workflows)
3. **Loose boundaries** - Rejected (race conditions)

---

## ADR-010: Type-Safe IDs with Newtype Pattern

**Status**: Accepted
**Date**: 2026-01-18
**Context**: All Contexts (Domain Model)

### Problem

Using raw UUIDs or strings for IDs leads to errors:
```rust
// Error-prone: Can mix up IDs
fn get_incident(id: &Uuid) -> Incident { ... }
fn get_service(id: &Uuid) -> Service { ... }

// Oops! Passed service ID to incident function
let incident = get_incident(&service_id);  // Compiles but wrong!
```

### Decision

Use **newtype pattern** for type-safe IDs:

```rust
// Type-safe IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IncidentId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnomalyId(pub Uuid);

// Now compiler catches type errors
fn get_incident(id: &IncidentId) -> Incident { ... }
fn get_service(id: &ServiceId) -> Service { ... }

// Compile error! Type mismatch
let incident = get_incident(&service_id);  // ERROR: expected IncidentId, found ServiceId

// Correct usage
let incident = get_incident(&incident_id);  // OK

// Conversion (explicit)
impl From<Uuid> for IncidentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl IncidentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}
```

### Benefits

1. **Compile-time type checking**
2. **Self-documenting code** (ID type tells you what it is)
3. **Prevents accidental mixing** of different ID types
4. **No runtime overhead** (compiled away)

### Consequences

**Positive**:
- Type-safe ID usage
- Compiler catches errors
- Self-documenting
- Zero runtime cost

**Negative**:
- More type definitions
- Need conversion functions
- Slightly more verbose

### Alternatives Considered

1. **Raw UUIDs** - Rejected (error-prone)
2. **String IDs** - Rejected (no type safety)
3. **Generics** - Considered (too complex)

---

## Summary of Decisions

| ADR | Decision | Impact |
|-----|----------|--------|
| ADR-001 | 7 Bounded Contexts | Clear ownership, event-driven |
| ADR-002 | Event Sourcing for Incidents | Complete audit, temporal queries |
| ADR-003 | Vector Embeddings | Semantic search, ML-powered |
| ADR-004 | Approval Gates | Safe automation rollout |
| ADR-005 | Anti-Corruption Layers | Clean domain, external isolation |
| ADR-006 | Snapshot Strategy | Fast load, long histories |
| ADR-007 | Repository Interface | Persistence abstraction |
| ADR-008 | Domain Event Standards | Consistent events, versioning |
| ADR-009 | Aggregate Boundaries | Clear consistency, no distributed tx |
| ADR-010 | Type-Safe IDs | Compiler catches errors |
