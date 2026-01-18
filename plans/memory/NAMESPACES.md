# Memory Namespaces Schema

**ADR-006: Unified Memory Namespace Definitions**

## Overview

The RustOps memory system uses namespaces to isolate different types of operational data while enabling unified semantic search across all namespaces. Each namespace has specific schemas, retention policies, and query patterns optimized for its data type.

---

## Namespace Definitions

### 1. incidents namespace

**Purpose**: Store complete incident records with resolutions for historical analysis and pattern extraction.

#### Schema

```rust
pub struct Incident {
    /// Unique incident ID (ULID)
    pub id: String,

    /// Incident title/summary
    pub title: String,

    /// Detailed description
    pub description: String,

    /// Affected service
    pub service: String,

    /// Environment (prod/staging/dev)
    pub environment: Environment,

    /// Severity level
    pub severity: SeverityLevel,

    /// Detection timestamp
    pub detected_at: DateTime<Utc>,

    /// Resolution timestamp
    pub resolved_at: Option<DateTime<Utc>>,

    /// Root cause analysis
    pub root_cause: Option<RootCause>,

    /// Resolution steps taken
    pub resolution_steps: Vec<ResolutionStep>,

    /// Related incidents (cascade, duplicate)
    pub related_incidents: Vec<String>,

    /// Impact metrics
    pub impact: ImpactMetrics,

    /// Detection method (alert/log/trace)
    pub detection_method: DetectionMethod,

    /// Assigned to (agent/human)
    pub assigned_to: Assignment,

    /// Status
    pub status: IncidentStatus,

    /// Tags and labels
    pub tags: Vec<String>,

    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

pub struct RootCause {
    /// Primary root cause
    pub primary: String,

    /// Contributing factors
    pub contributing_factors: Vec<String>,

    /// Category (code/infra/config/external)
    pub category: RootCauseCategory,

    /// Confidence score (0-1)
    pub confidence: f32,
}

pub struct ImpactMetrics {
    /// Duration of incident (seconds)
    pub duration_seconds: u64,

    /// Affected users
    pub users_affected: u64,

    /// Error rate during incident
    pub error_rate: f32,

    /// Latency degradation
    pub latency_increase_ms: u64,

    /// Business impact (revenue, SLA)
    pub business_impact: BusinessImpact,
}

pub enum IncidentStatus {
    /// Active and being investigated
    Active,

    /// Identified but not resolved
    Identified,

    /// Resolution in progress
    Resolving,

    /// Resolved and verified
    Resolved,

    /// False positive
    FalsePositive,

    /// Duplicate of another incident
    Duplicate,
}
```

#### Query Patterns

```rust
impl IncidentQueries {
    /// Find similar incidents by semantic similarity
    pub async fn find_similar(
        memory: &UnifiedMemoryService,
        incident: &Incident,
        threshold: f32,
    ) -> Result<Vec<Incident>> {
        memory.query(MemoryQuery {
            query_type: QueryType::Semantic,
            namespace: Some(MemoryNamespace::Incidents),
            content: Some(incident.description.clone()),
            filters: vec![
                QueryFilter::Service(incident.service.clone()),
                QueryFilter::MinSimilarity(threshold),
                QueryFilter::ExcludeStatus(IncidentStatus::FalsePositive),
            ],
            limit: 10,
            threshold: Some(threshold),
        })
        .await
    }

    /// Find incidents by time range
    pub async fn find_by_time_range(
        memory: &UnifiedMemoryService,
        service: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Incident>> {
        memory.query(MemoryQuery {
            query_type: QueryType::Structured,
            namespace: Some(MemoryNamespace::Incidents),
            content: None,
            filters: vec![
                QueryFilter::Service(service.to_string()),
                QueryFilter::TimeRange(TimeRange { start, end }),
            ],
            limit: 1000,
            threshold: None,
        })
        .await
    }

    /// Find recurring incidents (same service, similar symptoms)
    pub async fn find_recurring(
        memory: &UnifiedMemoryService,
        service: &str,
        days: u64,
    ) -> Result<Vec<IncidentGroup>> {
        // Find incidents in time window
        let incidents = Self::find_by_time_range(
            memory,
            service,
            Utc::now() - Duration::days(days as i64),
            Utc::now(),
        )
        .await?;

        // Cluster by similarity
        let groups = cluster_by_similarity(&incidents, 0.8)?;

        Ok(groups)
    }
}
```

#### Retention Policy

| Setting | Value |
|---------|-------|
| TTL | 90 days |
| Decay Rate | 0.05/day |
| Archive | Compress after 90 days |
| Delete | After 1 year |

---

### 2. patterns namespace

**Purpose**: Store successful remediation patterns extracted from incident resolutions for reuse.

#### Schema

```rust
pub struct Pattern {
    /// Unique pattern ID
    pub id: String,

    /// Pattern name
    pub name: String,

    /// Description of when pattern applies
    pub description: String,

    /// Trigger conditions (symptoms, metrics, logs)
    pub triggers: Vec<TriggerCondition>,

    /// Remediation actions (ordered steps)
    pub actions: Vec<Action>,

    /// Expected outcome
    pub expected_outcome: String,

    /// Success rate (0-1)
    pub success_rate: f32,

    /// Times used
    pub usage_count: u64,

    /// Last used timestamp
    pub last_used: DateTime<Utc>,

    /// Applicable services/environments
    pub applicability: Applicability,

    /// Risk level (low/medium/high)
    pub risk_level: RiskLevel,

    /// Estimated duration
    pub estimated_duration_seconds: u64,

    /// Related incidents where this pattern worked
    pub provenance: Vec<String>, // Incident IDs

    /// Validation status
    pub validation_status: ValidationStatus,

    /// SONA mode recommendation
    pub recommended_sona_mode: SonamMode,
}

pub struct TriggerCondition {
    /// Condition type (metric threshold, log pattern, etc.)
    pub condition_type: TriggerType,

    /// Condition expression
    pub expression: String,

    /// Threshold value
    pub threshold: Option<serde_json::Value>,

    /// Time window for evaluation
    pub time_window_seconds: u64,
}

pub enum TriggerType {
    /// Metric threshold violation
    MetricThreshold {
        metric_name: String,
        operator: ComparisonOp,
        value: f64,
    },

    /// Log pattern detected
    LogPattern {
        pattern: String,
        min_occurrences: u64,
        time_window: u64,
    },

    /// Error rate spike
    ErrorRateSpike {
        baseline: f32,
        threshold: f32,
        window: u64,
    },

    /// Latency degradation
    LatencyDegradation {
        percentile: u8, // 50, 95, 99
        increase_percent: f32,
    },

    /// Service dependency failure
    DependencyFailure {
        dependency: String,
        failure_type: FailureType,
    },
}

pub struct Action {
    /// Action order in sequence
    pub order: u32,

    /// Action type
    pub action_type: ActionType,

    /// Action parameters
    pub parameters: HashMap<String, serde_json::Value>,

    /// Timeout for this action
    pub timeout_seconds: u64,

    /// Rollback action if this fails
    pub rollback: Option<Action>,

    /// Validation checks after execution
    pub validation_checks: Vec<ValidationCheck>,
}

pub enum ActionType {
    /// Restart service/pod
    Restart {
        service: String,
        graceful: bool,
    },

    /// Scale up/down
    Scale {
        service: String,
        replicas: u32,
    },

    /// Rollback deployment
    Rollback {
        service: String,
        target_version: String,
    },

    /// Execute command
    ExecuteCommand {
        command: String,
        args: Vec<String>,
    },

    /// Call API endpoint
    CallAPI {
        method: String,
        url: String,
        body: Option<serde_json::Value>,
    },

    /// Update configuration
    UpdateConfig {
        service: String,
        config_key: String,
        value: serde_json::Value,
    },

    /// Clear cache
    ClearCache {
        cache_type: CacheType,
    },

    /// Traffic failover
    Failover {
        from: String,
        to: String,
    },
}

pub enum ValidationStatus {
    /// Pattern validated in production
    Validated,

    /// Pattern tested in staging
    Tested,

    /// Pattern extracted from incidents but not tested
    Extracted,

    /// Pattern failed validation
    Invalid,
}
```

#### Query Patterns

```rust
impl PatternQueries {
    /// Find applicable patterns for current incident
    pub async fn find_applicable(
        memory: &UnifiedMemoryService,
        incident: &Incident,
    ) -> Result<Vec<Pattern>> {
        // Semantic search for similar patterns
        let patterns = memory.query(MemoryQuery {
            query_type: QueryType::Semantic,
            namespace: Some(MemoryNamespace::Patterns),
            content: Some(incident.description.clone()),
            filters: vec![
                QueryFilter::Service(incident.service.clone()),
                QueryFilter::Environment(incident.environment.clone()),
                QueryFilter::MinSuccessRate(0.7),
                QueryFilter::ValidationStatus(ValidationStatus::Validated),
            ],
            limit: 10,
            threshold: Some(0.75),
        })
        .await?;

        // Filter by trigger conditions
        let applicable = patterns
            .into_iter()
            .filter(|p| p.matches_triggers(incident))
            .collect();

        Ok(applicable)
    }

    /// Find patterns by action type
    pub async fn find_by_action(
        memory: &UnifiedMemoryService,
        action_type: ActionType,
    ) -> Result<Vec<Pattern>> {
        memory.query(MemoryQuery {
            query_type: QueryType::Structured,
            namespace: Some(MemoryNamespace::Patterns),
            content: None,
            filters: vec![QueryFilter::ActionType(action_type)],
            limit: 50,
            threshold: None,
        })
        .await
    }
}
```

#### Retention Policy

| Setting | Value |
|---------|-------|
| TTL | Persistent |
| Decay Rate | 0.01/day |
| Archive | Never |
| Delete | Only if explicitly invalidated |

---

### 3. topology namespace

**Purpose**: Store service dependency topology snapshots over time for impact analysis and root cause inference.

#### Schema

```rust
pub struct TopologySnapshot {
    /// Unique snapshot ID
    pub id: String,

    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,

    /// Service graph nodes
    pub nodes: Vec<ServiceNode>,

    /// Service graph edges (dependencies)
    pub edges: Vec<DependencyEdge>,

    /// Environment
    pub environment: Environment,

    /// Snapshot source (k8s, mesh, discovery)
    pub source: TopologySource,

    /// Health status at snapshot time
    pub health_status: HashMap<String, ServiceHealth>,

    /// Metadata
    pub metadata: TopologyMetadata,
}

pub struct ServiceNode {
    /// Service identifier
    pub id: String,

    /// Service name
    pub name: String,

    /// Service type (service, database, queue, cache)
    pub service_type: ServiceType,

    /// Owner team
    pub owner: String,

    /// Criticality (tier-1, tier-2, tier-3)
    pub criticality: ServiceCriticality,

    /// Health status
    pub health: ServiceHealth,

    /// Capacity info
    pub capacity: CapacityInfo,

    /// Tags
    pub tags: Vec<String>,
}

pub struct DependencyEdge {
    /// Edge ID
    pub id: String,

    /// Source service
    pub from: String,

    /// Target service
    pub to: String,

    /// Dependency type (sync, async, shared)
    pub dependency_type: DependencyType,

    /// Protocol (http, grpc, tcp, etc.)
    pub protocol: String,

    /// Call rate (calls/second)
    pub call_rate: Option<f64>,

    /// Error rate
    pub error_rate: Option<f32>,

    /// Latency (p50, p95, p99)
    pub latency: LatencyMetrics,

    /// Criticality of this dependency
    pub criticality: EdgeCriticality,
}

pub struct TopologyMetadata {
    /// Total number of services
    pub service_count: usize,

    /// Total dependencies
    pub dependency_count: usize,

    /// Graph depth (longest path)
    pub depth: usize,

    /// Critical path (highest impact path)
    pub critical_path: Vec<String>,

    /// Change delta from previous snapshot
    pub change_delta: Option<TopologyChange>,
}

pub struct TopologyChange {
    /// Previous snapshot ID
    pub previous_snapshot_id: String,

    /// Nodes added
    pub nodes_added: Vec<String>,

    /// Nodes removed
    pub nodes_removed: Vec<String>,

    /// Edges added
    pub edges_added: Vec<String>,

    /// Edges removed
    pub edges_removed: Vec<String>,

    /// Health changes
    pub health_changes: HashMap<String, (ServiceHealth, ServiceHealth)>,
}
```

#### Query Patterns

```rust
impl TopologyQueries {
    /// Get latest topology for environment
    pub async fn get_latest(
        memory: &UnifiedMemoryService,
        environment: Environment,
    ) -> Result<TopologySnapshot> {
        let mut snapshots = memory.query(MemoryQuery {
            query_type: QueryType::Structured,
            namespace: Some(MemoryNamespace::Topology),
            content: None,
            filters: vec![QueryFilter::Environment(environment)],
            limit: 1,
            threshold: None,
        })
        .await?;

        snapshots
            .pop()
            .ok_or_else(|| anyhow!("No topology found"))
    }

    /// Get topology at specific time
    pub async fn get_at_time(
        memory: &UnifiedMemoryService,
        time: DateTime<Utc>,
        environment: Environment,
    ) -> Result<TopologySnapshot> {
        let mut snapshots = memory.query(MemoryQuery {
            query_type: QueryType::Structured,
            namespace: Some(MemoryNamespace::Topology),
            content: None,
            filters: vec![
                QueryFilter::Environment(environment),
                QueryFilter::ClosestToTime(time),
            ],
            limit: 1,
            threshold: None,
        })
        .await?;

        snapshots
            .pop()
            .ok_or_else(|| anyhow!("No topology found"))
    }

    /// Find impact analysis for service failure
    pub async fn find_downstream_impact(
        memory: &UnifiedMemoryService,
        service: &str,
        environment: Environment,
    ) -> Result<ImpactAnalysis> {
        let topology = Self::get_latest(memory, environment).await?;

        // Build downstream graph
        let downstream = topology.build_downstream_graph(service);

        // Calculate impact by criticality
        let impact = ImpactAnalysis {
            affected_services: downstream.services,
            total_dependencies: downstream.count,
            critical_path: downstream.critical_path,
            estimated_users_affected: downstream.estimate_users_affected(),
            sla_risk: downstream.calculate_sla_risk(),
        };

        Ok(impact)
    }

    /// Track topology changes over time
    pub async fn get_evolution(
        memory: &UnifiedMemoryService,
        service: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<TopologyChange>> {
        let snapshots = memory.query(MemoryQuery {
            query_type: QueryType::Structured,
            namespace: Some(MemoryNamespace::Topology),
            content: None,
            filters: vec![
                QueryFilter::Service(service.to_string()),
                QueryFilter::TimeRange(TimeRange { start, end }),
            ],
            limit: 100,
            threshold: None,
        })
        .await?;

        let changes = snapshots
            .windows(2)
            .map(|w| w[1].metadata.change_delta.clone().unwrap())
            .collect();

        Ok(changes)
    }
}
```

#### Retention Policy

| Setting | Value |
|---------|-------|
| TTL | 30 days |
| Decay Rate | 0.1/day |
| Archive | Keep latest 100 snapshots |
| Delete | After 90 days |

---

### 4. anomalies namespace

**Purpose**: Store detected anomalies with ground truth labels for supervised learning and threshold optimization.

#### Schema

```rust
pub struct AnomalyRecord {
    /// Unique anomaly ID
    pub id: String,

    /// Anomaly type
    pub anomaly_type: AnomalyType,

    /// Detection timestamp
    pub detected_at: DateTime<Utc>,

    /// Affected service/metric
    pub target: AnomalyTarget,

    /// Anomaly score (0-1, higher = more anomalous)
    pub anomaly_score: f32,

    /// Detection method
    pub detection_method: DetectionMethod,

    /// Raw data that triggered detection
    pub trigger_data: serde_json::Value,

    /// Context (time window, related metrics)
    pub context: AnomalyContext,

    /// Ground truth label (true positive, false positive, etc.)
    pub label: Option<AnomalyLabel>,

    /// Label source (human, feedback, automated)
    pub label_source: Option<LabelSource>,

    /// Related incident ID
    pub related_incident: Option<String>,

    /// Model version that detected this
    pub model_version: String,

    /// Model confidence
    pub model_confidence: f32,
}

pub enum AnomalyType {
    /// Metric spike or dip
    MetricSpike {
        metric_name: String,
        expected_value: f64,
        actual_value: f64,
        deviation_percent: f32,
    },

    /// Statistical outlier
    StatisticalOutlier {
        metric_name: String,
        z_score: f64,
        percentile: f32,
    },

    /// Log pattern anomaly
    LogPattern {
        pattern: String,
        occurrence_count: u64,
        baseline_count: u64,
    },

    /// Error rate anomaly
    ErrorRateSike {
        baseline_rate: f32,
        current_rate: f32,
        threshold: f32,
    },

    /// Latency anomaly
    LatencyAnomaly {
        percentile: u8,
        baseline_ms: u64,
        current_ms: u64,
    },

    /// Topology anomaly (unexpected dependency)
    TopologyAnomaly {
        unexpected_edge: String,
        reason: String,
    },
}

pub struct AnomalyContext {
    /// Time window considered
    pub time_window_seconds: u64,

    /// Related metrics that also changed
    pub correlated_metrics: Vec<String>,

    /// Recent deployments or config changes
    pub recent_changes: Vec<String>,

    /// Seasonal baseline for this time
    pub seasonal_baseline: Option<f64>,

    /// Topology state at detection time
    pub topology_snapshot_id: Option<String>,
}

pub enum AnomalyLabel {
    /// True positive - real issue
    TruePositive,

    /// False positive - no issue
    FalsePositive,

    /// True negative - correctly no alert
    TrueNegative,

    /// False negative - missed issue
    FalseNegative,

    /// Unknown - not labeled yet
    Unknown,
}

pub enum LabelSource {
    /// Human operator labeled
    Human { user_id: String },

    /// Feedback from incident resolution
    Feedback { incident_id: String },

    /// Automated labeling
    Automated { method: String },
}
```

#### Query Patterns

```rust
impl AnomalyQueries {
    /// Find false positives for model retraining
    pub async fn find_false_positives(
        memory: &UnifiedMemoryService,
        model_version: &str,
        limit: usize,
    ) -> Result<Vec<AnomalyRecord>> {
        memory.query(MemoryQuery {
            query_type: QueryType::Structured,
            namespace: Some(MemoryNamespace::Anomalies),
            content: None,
            filters: vec![
                QueryFilter::ModelVersion(model_version.to_string()),
                QueryFilter::Label(AnomalyLabel::FalsePositive),
            ],
            limit,
            threshold: None,
        })
        .await
    }

    /// Calculate precision/recall for model
    pub async fn calculate_model_metrics(
        memory: &UnifiedMemoryService,
        model_version: &str,
    ) -> Result<ModelMetrics> {
        let anomalies = memory.query(MemoryQuery {
            query_type: QueryType::Structured,
            namespace: Some(MemoryNamespace::Anomalies),
            content: None,
            filters: vec![QueryFilter::ModelVersion(model_version.to_string())],
            limit: 10000,
            threshold: None,
        })
        .await?;

        let true_positives = anomalies
            .iter()
            .filter(|a| matches!(a.label, Some(AnomalyLabel::TruePositive)))
            .count();

        let false_positives = anomalies
            .iter()
            .filter(|a| matches!(a.label, Some(AnomalyLabel::FalsePositive)))
            .count();

        let false_negatives = anomalies
            .iter()
            .filter(|a| matches!(a.label, Some(AnomalyLabel::FalseNegative)))
            .count();

        Ok(ModelMetrics {
            precision: true_positives as f32 / (true_positives + false_positives) as f32,
            recall: true_positives as f32 / (true_positives + false_negatives) as f32,
            f1_score: 2.0 * true_positives as f32 / (2.0 * true_positives as f32 + false_positives as f32 + false_negatives as f32),
        })
    }
}
```

#### Retention Policy

| Setting | Value |
|---------|-------|
| TTL | 60 days |
| Decay Rate | 0.07/day |
| Archive | Keep labeled anomalies forever |
| Delete | Unlabeled after 60 days |

---

### 5. runbooks namespace

**Purpose**: Store extracted and validated runbook procedures for automation.

#### Schema

```rust
pub struct Runbook {
    /// Unique runbook ID
    pub id: String,

    /// Runbook name
    pub name: String,

    /// Description
    pub description: String,

    /// Trigger conditions
    pub triggers: Vec<TriggerCondition>,

    /// Procedure steps (ordered)
    pub steps: Vec<RunbookStep>,

    /// Prerequisites (services, permissions, tools)
    pub prerequisites: Vec<Prerequisite>,

    /// Expected outcomes
    pub expected_outcomes: Vec<String>,

    /// Rollback procedure
    pub rollback_steps: Vec<RunbookStep>,

    /// Estimated duration
    pub estimated_duration_seconds: u64,

    /// Risk level
    pub risk_level: RiskLevel,

    /// Validation status
    pub validation_status: ValidationStatus,

    /// Last validated timestamp
    pub last_validated: Option<DateTime<Utc>>,

    /// Success rate in production
    pub success_rate: f32,

    /// Usage count
    pub usage_count: u64,

    /// Source (manual, extracted, generated)
    pub source: RunbookSource,

    /// Version
    pub version: String,

    /// SONA mode recommendation
    pub recommended_sona_mode: SonamMode,
}

pub struct RunbookStep {
    /// Step number
    pub step_number: u32,

    /// Step description
    pub description: String,

    /// Action to execute
    pub action: Action,

    /// Validation checks
    pub validation_checks: Vec<ValidationCheck>,

    /// Timeout
    pub timeout_seconds: u64,

    /// Can this step be parallelized?
    pub parallelizable: bool,

    /// Dependencies on other steps
    pub depends_on_steps: Vec<u32>,
}

pub struct Prerequisite {
    /// Prerequisite type
    pub prerequisite_type: PrerequisiteType,

    /// Description
    pub description: String,

    /// Check command (to verify prerequisite)
    pub check_command: Option<String>,
}

pub enum PrerequisiteType {
    /// Service access
    ServiceAccess { service: String },

    /// Permission/role
    Permission { permission: String },

    /// Tool availability
    Tool { tool: String, min_version: Option<String> },

    /// Configuration state
    ConfigState { key: String, value: serde_json::Value },
}

pub enum RunbookSource {
    /// Manually created by human
    Manual { author: String },

    /// Extracted from incident resolution
    Extracted { incident_id: String },

    /// Generated by AI from patterns
    Generated { pattern_ids: Vec<String> },

    /// Imported from external system
    Imported { system: String, external_id: String },
}
```

#### Query Patterns

```rust
impl RunbookQueries {
    /// Find applicable runbooks for incident
    pub async fn find_applicable(
        memory: &UnifiedMemoryService,
        incident: &Incident,
    ) -> Result<Vec<Runbook>> {
        let runbooks = memory.query(MemoryQuery {
            query_type: QueryType::Semantic,
            namespace: Some(MemoryNamespace::Runbooks),
            content: Some(incident.description.clone()),
            filters: vec![
                QueryFilter::Service(incident.service.clone()),
                QueryFilter::Environment(incident.environment.clone()),
                QueryFilter::ValidationStatus(ValidationStatus::Validated),
                QueryFilter::MaxRiskLevel(RiskLevel::Medium),
            ],
            limit: 5,
            threshold: Some(0.8),
        })
        .await?;

        // Filter by prerequisites
        let applicable = runbooks
            .into_iter()
            .filter(|r| r.check_prerequisites(incident).await)
            .collect();

        Ok(applicable)
    }
}
```

#### Retention Policy

| Setting | Value |
|---------|-------|
| TTL | Persistent |
| Decay Rate | 0.0/day |
| Archive | Never |
| Delete | Only if explicitly deprecated |

---

### 6. feedback namespace

**Purpose**: Store human feedback and overrides for learning and improving automated decisions.

#### Schema

```rust
pub struct Feedback {
    /// Unique feedback ID
    pub id: String,

    /// Feedback type
    pub feedback_type: FeedbackType,

    /// Target (what is this feedback about)
    pub target: FeedbackTarget,

    /// Feedback value
    pub feedback: FeedbackValue,

    /// Who provided feedback
    pub source: FeedbackSource,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Context (incident, anomaly, etc.)
    pub context: FeedbackContext,

    /// Applied? (was feedback acted on)
    pub applied: bool,
}

pub enum FeedbackType {
    /// Override automated decision
    Override,

    /// Confirm automated decision
    Confirm,

    /// Reject automated decision
    Reject,

    /// Suggest improvement
    Suggest,

    /// Report issue
    ReportIssue,
}

pub enum FeedbackTarget {
    /// Anomaly detection
    Anomaly { anomaly_id: String },

    /// Pattern application
    Pattern { pattern_id: String },

    /// Runbook execution
    Runbook { runbook_id: String },

    /// Root cause analysis
    RootCause { incident_id: String },

    /// Threshold setting
    Threshold { metric_name: String },
}

pub enum FeedbackValue {
    /// True/false feedback
    Boolean(bool),

    /// Numerical rating (1-5)
    Rating(u8),

    /// Free-form text
    Text(String),

    /// Alternative action
    AlternativeAction(Action),

    /// Corrected value
    CorrectedValue {
        key: String,
        original: serde_json::Value,
        corrected: serde_json::Value,
    },
}

pub struct FeedbackContext {
    /// Related incident ID
    pub incident_id: Option<String>,

    /// Related service
    pub service: Option<String>,

    /// Environment
    pub environment: Option<Environment>,

    /// Additional context
    pub metadata: HashMap<String, serde_json::Value>,
}
```

---

### 7. config-changes namespace

**Purpose**: Track configuration changes with impact correlation for learning.

#### Schema

```rust
pub struct ConfigChange {
    /// Unique change ID
    pub id: String,

    /// Change timestamp
    pub timestamp: DateTime<Utc>,

    /// Service/component
    pub service: String,

    /// Environment
    pub environment: Environment,

    /// Config key affected
    pub config_key: String,

    /// Previous value
    pub previous_value: serde_json::Value,

    /// New value
    pub new_value: serde_json::Value,

    /// Change method (manual, CI/CD, API)
    pub change_method: ChangeMethod,

    /// Who made the change
    pub changed_by: String,

    /// Reason for change
    pub reason: Option<String>,

    /// Related deployment/release
    pub related_deployment: Option<String>,

    /// Impact (incidents, anomalies)
    pub impact: Option<ChangeImpact>,

    /// Rollback info
    pub rollback_info: Option<RollbackInfo>,
}

pub struct ChangeImpact {
    /// Related incidents
    pub incidents: Vec<String>,

    /// Related anomalies
    pub anomalies: Vec<String>,

    /// Performance impact
    pub performance_delta: Option<PerformanceDelta>,

    /// User-reported issues
    pub user_reports: u64,
}

pub struct RollbackInfo {
    /// Was this change rolled back?
    pub rolled_back: bool,

    /// Rollback timestamp
    pub rollback_timestamp: Option<DateTime<Utc>>,

    /// Reason for rollback
    pub rollback_reason: Option<String>,
}
```

---

## Cross-Namespace Queries

### Semantic Search Across All Namespaces

```rust
/// Search all namespaces semantically
pub async fn search_all(
    memory: &UnifiedMemoryService,
    query: &str,
    limit: usize,
) -> Result<CrossNamespaceResults> {
    let results = memory.query(MemoryQuery {
        query_type: QueryType::Semantic,
        namespace: None, // All namespaces
        content: Some(query.to_string()),
        filters: vec![],
        limit,
        threshold: Some(0.7),
    })
    .await?;

    Ok(CrossNamespaceResults {
        incidents: filter_namespace(results, MemoryNamespace::Incidents),
        patterns: filter_namespace(results, MemoryNamespace::Patterns),
        topology: filter_namespace(results, MemoryNamespace::Topology),
        anomalies: filter_namespace(results, MemoryNamespace::Anomalies),
        runbooks: filter_namespace(results, MemoryNamespace::Runbooks),
        feedback: filter_namespace(results, MemoryNamespace::Feedback),
        config_changes: filter_namespace(results, MemoryNamespace::ConfigChanges),
    })
}
```

---

## Summary

| Namespace | Purpose | TTL | Decay | Search Type |
|-----------|---------|-----|-------|-------------|
| incidents | Incident history | 90 days | 0.05/day | Semantic + Structured |
| patterns | Remediation patterns | Persistent | 0.01/day | Semantic |
| topology | Service dependencies | 30 days | 0.1/day | Structured |
| anomalies | Detection results | 60 days | 0.07/day | Structured |
| runbooks | Automation procedures | Persistent | 0.0/day | Semantic |
| feedback | Human feedback | 180 days | 0.03/day | Structured |
| config-changes | Configuration tracking | 365 days | 0.02/day | Structured |

---

**Document Version**: 1.0
**Last Updated**: 2026-01-18
**Author**: Memory Architecture Team
