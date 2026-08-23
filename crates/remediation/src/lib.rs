//! # RustOps Remediation Engine (experimental, design-stage)
//!
//! Remediation workflow scaffolding: a policy engine with risk-based
//! approval decisions, circuit breakers, blast-radius constraints,
//! rollback strategies, and an in-process workflow engine.
//!
//! ## What is real
//!
//! - **Policy engine**: risk assessment and auto-approve / manual / block
//!   decisions ([`policy`])
//! - **Safety interlocks**: circuit breakers, blast-radius limits,
//!   cooldowns, rollback bookkeeping ([`safety`])
//! - **Workflow engine**: in-process orchestration of activity steps with
//!   history ([`workflow`])
//!
//! ## What is not
//!
//! - **Activities are simulated.** The only shipped executor is
//!   [`activity::SimulatedActivityExecutor`], which logs and returns
//!   `"simulated": true` payloads. Nothing here touches a real cluster or
//!   cloud API yet.
//! - There is **no Temporal integration** — workflows are plain in-process
//!   async, not durable/replayable.
//!
//! This crate is not wired into the RustOps pipeline.

pub mod activity;
pub mod error;
pub mod policy;
pub mod safety;
pub mod workflow;

pub use error::{Error, Result};
pub use policy::{
    ActionType, ApprovalStatus, PolicyDecision, PolicyEngine, RemediationPolicy, RiskLevel,
};
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
