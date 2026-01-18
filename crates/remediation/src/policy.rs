//! Policy engine for remediation decision-making
//!
//! The policy engine evaluates proposed remediation actions against
//! security policies, risk levels, and approval requirements.

use crate::{
    error::{Error, Result},
    IncidentContext, IncidentSeverity,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Types of remediation actions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Restart a service or pod
    RestartService,
    /// Scale service replicas
    ScaleService,
    /// Rollback a deployment
    RollbackDeployment,
    /// Delete a resource
    DeleteResource,
    /// Modify configuration
    ModifyConfig,
    /// Reboot an instance
    RebootInstance,
    /// Failover database
    FailoverDatabase,
    /// Execute custom script
    ExecuteScript,
    /// Create/restore backup
    BackupRestore,
    /// Network configuration change
    NetworkChange,
}

impl ActionType {
    /// Get default risk level for action type
    pub fn default_risk_level(&self) -> RiskLevel {
        match self {
            Self::RestartService => RiskLevel::Low,
            Self::ScaleService => RiskLevel::Low,
            Self::RollbackDeployment => RiskLevel::Medium,
            Self::ModifyConfig => RiskLevel::Medium,
            Self::RebootInstance => RiskLevel::Medium,
            Self::FailoverDatabase => RiskLevel::High,
            Self::DeleteResource => RiskLevel::High,
            Self::ExecuteScript => RiskLevel::Critical,
            Self::BackupRestore => RiskLevel::Low,
            Self::NetworkChange => RiskLevel::Critical,
        }
    }

    /// Check if action requires approval
    pub fn requires_approval(&self) -> bool {
        !matches!(self, Self::RestartService | Self::ScaleService | Self::BackupRestore)
    }
}

/// Risk levels for remediation actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    /// Low risk - safe to auto-approve
    Low = 1,
    /// Medium risk - requires consideration
    Medium = 2,
    /// High risk - requires approval
    High = 3,
    /// Critical risk - requires multiple approvers
    Critical = 4,
}

impl RiskLevel {
    /// Maximum number of failures allowed at this risk level
    pub fn max_failure_threshold(&self) -> usize {
        match self {
            Self::Low => 5,
            Self::Medium => 3,
            Self::High => 2,
            Self::Critical => 1,
        }
    }

    /// Number of approvers required
    pub fn required_approvers(&self) -> usize {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Critical => 3,
        }
    }

    /// Approval timeout in seconds
    pub fn approval_timeout_secs(&self) -> u64 {
        match self {
            Self::Low => 0,
            Self::Medium => 300,  // 5 minutes
            Self::High => 600,    // 10 minutes
            Self::Critical => 900, // 15 minutes
        }
    }

    /// Cooldown period between actions in seconds
    pub fn cooldown_secs(&self) -> u64 {
        match self {
            Self::Low => 30,
            Self::Medium => 60,
            Self::High => 300,
            Self::Critical => 600,
        }
    }
}

/// Remediation policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationPolicy {
    /// Action type
    pub action_type: ActionType,

    /// Risk level
    pub risk_level: RiskLevel,

    /// Approval required
    pub approval_required: bool,

    /// Number of approvers required
    pub required_approvers: usize,

    /// Constraints
    pub constraints: Vec<Constraint>,

    /// Rate limit (actions per hour)
    pub rate_limit_per_hour: Option<usize>,

    /// Blast radius limit
    pub blast_radius_limit: Option<BlastRadiusLimit>,

    /// Enabled
    pub enabled: bool,
}

impl Default for RemediationPolicy {
    fn default() -> Self {
        Self {
            action_type: ActionType::RestartService,
            risk_level: RiskLevel::Low,
            approval_required: false,
            required_approvers: 0,
            constraints: Vec::new(),
            rate_limit_per_hour: None,
            blast_radius_limit: None,
            enabled: true,
        }
    }
}

impl RemediationPolicy {
    /// Create policy for action type
    pub fn for_action(action_type: ActionType) -> Self {
        let risk_level = action_type.default_risk_level();
        let approval_required = action_type.requires_approval();
        let required_approvers = risk_level.required_approvers();

        Self {
            action_type,
            risk_level,
            approval_required,
            required_approvers,
            ..Default::default()
        }
    }

    /// Add constraint
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Set rate limit
    pub fn with_rate_limit(mut self, limit: usize) -> Self {
        self.rate_limit_per_hour = Some(limit);
        self
    }

    /// Set blast radius limit
    pub fn with_blast_radius(mut self, limit: BlastRadiusLimit) -> Self {
        self.blast_radius_limit = Some(limit);
        self
    }
}

/// Constraint for remediation actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Constraint {
    /// Time window constraint
    TimeWindow {
        start_hour: u8,
        end_hour: u8,
        timezone: String,
    },
    /// Namespace constraint
    Namespace { allowed: Vec<String> },
    /// Cluster constraint
    Cluster { allowed: Vec<String> },
    /// Service health constraint
    ServiceHealth { min_health_percentage: u8 },
    /// Resource limit constraint
    ResourceLimit {
        resource_type: String,
        max_value: u64,
    },
    /// Custom constraint
    Custom { name: String, condition: String },
}

impl Constraint {
    /// Check if constraint is satisfied
    pub async fn check(&self, _context: &IncidentContext) -> Result<bool> {
        match self {
            Self::Namespace { allowed } => {
                // In production, this would check against actual namespace
                Ok(true) // Placeholder
            }
            Self::TimeWindow { .. } => {
                // Check if current time is within allowed window
                Ok(true) // Placeholder
            }
            Self::ServiceHealth { .. } => {
                // Check service health
                Ok(true) // Placeholder
            }
            Self::Cluster { .. } => Ok(true),
            Self::ResourceLimit { .. } => Ok(true),
            Self::Custom { .. } => Ok(true),
        }
    }

    /// Get constraint reason
    pub fn reason(&self) -> String {
        match self {
            Self::TimeWindow { .. } => "Outside allowed time window".to_string(),
            Self::Namespace { .. } => "Namespace not allowed".to_string(),
            Self::Cluster { .. } => "Cluster not allowed".to_string(),
            Self::ServiceHealth { .. } => "Service health below threshold".to_string(),
            Self::ResourceLimit { .. } => "Resource limit exceeded".to_string(),
            Self::Custom { name, .. } => format!("Custom constraint {} failed", name),
        }
    }
}

/// Blast radius limit
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlastRadiusLimit {
    /// Single namespace only
    Namespace,
    /// Single cluster only
    Cluster,
    /// Single region only
    Region,
    /// Custom scope
    Custom { scope: String },
}

impl BlastRadiusLimit {
    /// Check if action is within blast radius
    pub fn check(&self, context: &IncidentContext) -> bool {
        match self {
            Self::Namespace => true, // Always within namespace
            Self::Cluster => true,
            Self::Region => true,
            Self::Custom { .. } => true,
        }
    }
}

/// Policy decision
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    /// Auto-approve the action
    AutoApprove,
    /// Requires manual approval
    ManualApproval {
        reason: String,
        timeout_secs: u64,
        required_approvers: usize,
    },
    /// Action is blocked
    Blocked { reason: String },
    /// Throttled
    Throttled {
        reason: String,
        retry_after_secs: u64,
    },
}

/// Policy engine
pub struct PolicyEngine {
    policies: Arc<RwLock<HashMap<ActionType, RemediationPolicy>>>,
    rate_limit_tracker: Arc<RwLock<HashMap<ActionType, Vec<chrono::DateTime<chrono::Utc>>>>>,
}

impl PolicyEngine {
    /// Create new policy engine
    pub fn new() -> Self {
        let mut policies = HashMap::new();

        // Initialize default policies
        for action_type in [
            ActionType::RestartService,
            ActionType::ScaleService,
            ActionType::RollbackDeployment,
            ActionType::DeleteResource,
            ActionType::ModifyConfig,
            ActionType::RebootInstance,
            ActionType::FailoverDatabase,
            ActionType::ExecuteScript,
            ActionType::BackupRestore,
            ActionType::NetworkChange,
        ] {
            policies.insert(action_type.clone(), RemediationPolicy::for_action(action_type));
        }

        Self {
            policies: Arc::new(RwLock::new(policies)),
            rate_limit_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add or update policy
    pub async fn set_policy(&self, policy: RemediationPolicy) {
        let mut policies = self.policies.write().await;
        policies.insert(policy.action_type.clone(), policy);
    }

    /// Get policy for action type
    pub async fn get_policy(&self, action_type: &ActionType) -> Option<RemediationPolicy> {
        let policies = self.policies.read().await;
        policies.get(action_type).cloned()
    }

    /// Evaluate action against policies
    pub async fn evaluate_action(
        &self,
        action: &ActionType,
        context: &IncidentContext,
    ) -> Result<PolicyDecision> {
        let policy = self
            .get_policy(action)
            .await
            .unwrap_or_else(|| RemediationPolicy::for_action(action.clone()));

        // Check if policy is enabled
        if !policy.enabled {
            return Ok(PolicyDecision::Blocked {
                reason: format!("Policy for {:?} is disabled", action),
            });
        }

        // Check constraints
        for constraint in &policy.constraints {
            if !constraint.check(context).await? {
                return Ok(PolicyDecision::Blocked {
                    reason: constraint.reason(),
                });
            }
        }

        // Check rate limits
        if let Some(rate_limit) = policy.rate_limit_per_hour {
            if !self.check_rate_limit(action, rate_limit).await {
                return Ok(PolicyDecision::Throttled {
                    reason: "Rate limit exceeded".to_string(),
                    retry_after_secs: 3600,
                });
            }
        }

        // Check blast radius
        if let Some(limit) = &policy.blast_radius_limit {
            if !limit.check(context) {
                return Ok(PolicyDecision::Blocked {
                    reason: format!("Blast radius limit exceeded: {:?}", limit),
                });
            }
        }

        // Make decision based on risk level and approval requirements
        let risk_level = self.assess_risk(action, context).await?;

        if risk_level >= RiskLevel::High || policy.approval_required {
            Ok(PolicyDecision::ManualApproval {
                reason: format!("{:?} action requires approval", action),
                timeout_secs: risk_level.approval_timeout_secs(),
                required_approvers: risk_level.required_approvers(),
            })
        } else {
            Ok(PolicyDecision::AutoApprove)
        }
    }

    /// Assess risk level for action
    async fn assess_risk(&self, action: &ActionType, context: &IncidentContext) -> Result<RiskLevel> {
        let base_risk = action.default_risk_level();

        // Adjust risk based on incident severity
        let adjusted_risk = match (base_risk, context.severity) {
            (RiskLevel::Low, IncidentSeverity::Critical) => RiskLevel::Medium,
            (RiskLevel::Medium, IncidentSeverity::Critical) => RiskLevel::High,
            (RiskLevel::High, IncidentSeverity::Critical) => RiskLevel::Critical,
            _ => base_risk,
        };

        Ok(adjusted_risk)
    }

    /// Check rate limit for action
    async fn check_rate_limit(&self, action: &ActionType, limit: usize) -> bool {
        let mut tracker = self.rate_limit_tracker.write().await;
        let now = chrono::Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);

        let actions = tracker.entry(action.clone()).or_insert_with(Vec::new);

        // Remove old entries
        actions.retain(|&timestamp| timestamp > one_hour_ago);

        // Check limit
        if actions.len() >= limit {
            return false;
        }

        // Record this action
        actions.push(now);
        true
    }

    /// Record policy decision for audit
    pub async fn record_decision(
        &self,
        action: &ActionType,
        decision: &PolicyDecision,
        incident_id: &str,
    ) -> Result<()> {
        // In production, this would write to audit log
        tracing::info!(
            action = ?action,
            decision = ?decision,
            incident_id,
            "Policy decision recorded"
        );
        Ok(())
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Approval status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Pending approval
    Pending,
    /// Approved
    Approved,
    /// Rejected
    Rejected,
    /// Cancelled
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_policy_engine_auto_approve() {
        let engine = PolicyEngine::new();
        let context = IncidentContext {
            incident_id: "test-1".to_string(),
            severity: IncidentSeverity::Low,
            service_name: "test-service".to_string(),
            namespace: "default".to_string(),
            cluster: "test-cluster".to_string(),
            description: "Test incident".to_string(),
            started_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        };

        let decision = engine
            .evaluate_action(&ActionType::RestartService, &context)
            .await
            .unwrap();

        assert!(matches!(decision, PolicyDecision::AutoApprove));
    }

    #[tokio::test]
    async fn test_policy_engine_requires_approval() {
        let engine = PolicyEngine::new();
        let context = IncidentContext {
            incident_id: "test-2".to_string(),
            severity: IncidentSeverity::High,
            service_name: "test-service".to_string(),
            namespace: "default".to_string(),
            cluster: "test-cluster".to_string(),
            description: "Test incident".to_string(),
            started_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        };

        let decision = engine
            .evaluate_action(&ActionType::DeleteResource, &context)
            .await
            .unwrap();

        match decision {
            PolicyDecision::ManualApproval {
                timeout_secs,
                required_approvers,
                ..
            } => {
                assert!(timeout_secs > 0);
                assert!(required_approvers >= 1);
            }
            _ => panic!("Expected manual approval"),
        }
    }

    #[test]
    fn test_risk_level_thresholds() {
        assert_eq!(RiskLevel::Low.max_failure_threshold(), 5);
        assert_eq!(RiskLevel::Medium.max_failure_threshold(), 3);
        assert_eq!(RiskLevel::High.max_failure_threshold(), 2);
        assert_eq!(RiskLevel::Critical.max_failure_threshold(), 1);
    }

    #[test]
    fn test_action_type_defaults() {
        assert_eq!(
            ActionType::RestartService.default_risk_level(),
            RiskLevel::Low
        );
        assert_eq!(
            ActionType::DeleteResource.default_risk_level(),
            RiskLevel::High
        );
        assert_eq!(
            ActionType::ExecuteScript.default_risk_level(),
            RiskLevel::Critical
        );
    }
}
