//! # RustOps Remediation Engine
//!
//! Automated remediation engine with Temporal workflow orchestration,
//! approval gates, and safety interlocks.
//!
//! ## Features
//!
//! - **Temporal Workflows**: Durable, replayable workflow execution
//! - **Approval Gates**: Multi-factor approval based on risk level
//! - **Blast Radius Limits**: Namespace and cluster-level constraints
//! - **Circuit Breakers**: Stop after N failures
//! - **Instant Rollback**: Automatic rollback on failure
//! - **Safety Interlocks**: Multi-layer protection for critical actions
//!
//! ## Architecture
//!
//! ```text
//! Incident Detection
//!       ↓
//! Policy Engine (Risk Assessment)
//!       ↓
//! Decision: Auto-approve | Manual Approval | Block
//!       ↓
//! Temporal Workflow Execution
//!       ↓
//! Activity Executors (K8s, AWS, Azure, GCP)
//!       ↓
//! Verification & Rollback (if needed)
//! ```

pub mod activity;
pub mod error;
pub mod policy;
pub mod safety;
pub mod workflow;

pub use error::{Error, Result};
pub use policy::{ActionType, ApprovalStatus, PolicyDecision, PolicyEngine, RemediationPolicy, RiskLevel};
pub use safety::{BlastRadius, CircuitBreaker, RollbackManager, SafetyInterlock};
pub use workflow::{RemediationWorkflow, WorkflowContext, WorkflowStatus};

/// Remediation engine configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RemediationConfig {
    /// Maximum number of concurrent remediation actions
    pub max_concurrent_actions: usize,

    /// Default timeout for workflows
    pub default_workflow_timeout_secs: u64,

    /// Enable circuit breakers
    pub enable_circuit_breakers: bool,

    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: usize,

    /// Circuit breaker reset timeout in seconds
    pub circuit_breaker_reset_timeout_secs: u64,

    /// Enable blast radius limits
    pub enable_blast_radius_limits: bool,

    /// Default blast radius (namespace, cluster, region)
    pub default_blast_radius: String,

    /// Enable audit logging
    pub enable_audit_logging: bool,

    /// Audit log retention in days
    pub audit_log_retention_days: u32,
}

impl Default for RemediationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_actions: 10,
            default_workflow_timeout_secs: 300,
            enable_circuit_breakers: true,
            circuit_breaker_threshold: 3,
            circuit_breaker_reset_timeout_secs: 300,
            enable_blast_radius_limits: true,
            default_blast_radius: "namespace".to_string(),
            enable_audit_logging: true,
            audit_log_retention_days: 90,
        }
    }
}

/// Incident context for remediation
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IncidentContext {
    /// Unique incident identifier
    pub incident_id: String,

    /// Incident severity
    pub severity: IncidentSeverity,

    /// Affected service name
    pub service_name: String,

    /// Namespace
    pub namespace: String,

    /// Cluster name
    pub cluster: String,

    /// Incident description
    pub description: String,

    /// Incident start time
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Incident severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IncidentSeverity {
    /// Critical - immediate action required
    Critical,
    /// High - urgent action required
    High,
    /// Medium - normal priority
    Medium,
    /// Low - can be deferred
    Low,
}

impl IncidentSeverity {
    /// Returns the numeric score for severity
    pub fn score(&self) -> u8 {
        match self {
            Self::Critical => 4,
            Self::High => 3,
            Self::Medium => 2,
            Self::Low => 1,
        }
    }
}

/// Remediation action result
#[derive(Debug, Clone, serde::Serialize)]
pub struct RemediationResult {
    /// Workflow ID
    pub workflow_id: String,

    /// Incident ID
    pub incident_id: String,

    /// Action performed
    pub action: ActionType,

    /// Success status
    pub success: bool,

    /// Result message
    pub message: String,

    /// Timestamp of completion
    pub completed_at: chrono::DateTime<chrono::Utc>,

    /// Rollback was performed
    pub rolled_back: bool,

    /// Additional details
    pub details: Option<serde_json::Value>,
}
