//! Error types for the remediation engine

use thiserror::Error;

/// Result type for remediation operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the remediation engine
#[derive(Error, Debug)]
pub enum Error {
    /// Policy engine error
    #[error("Policy error: {0}")]
    Policy(String),

    /// Workflow execution error
    #[error("Workflow error: {0}")]
    Workflow(String),

    /// Activity execution error
    #[error("Activity error: {0}")]
    Activity(String),

    /// Approval timeout
    #[error("Approval timeout after {timeout_secs} seconds")]
    ApprovalTimeout { timeout_secs: u64 },

    /// Approval rejected
    #[error("Approval rejected: {reason}")]
    ApprovalRejected { reason: String },

    /// Circuit breaker is open
    #[error("Circuit breaker is open for action {action}")]
    CircuitBreakerOpen { action: String },

    /// Blast radius exceeded
    #[error("Blast radius exceeded: {0}")]
    BlastRadiusExceeded(String),

    /// Constraint violation
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    /// Rollback failed
    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    /// Safety interlock triggered
    #[error("Safety interlock triggered: {0}")]
    SafetyInterlock(String),

    /// Kubernetes API error
    #[error("Kubernetes error: {0}")]
    Kubernetes(String),

    /// Cloud provider error
    #[error("Cloud provider error: {provider} - {message}")]
    CloudProvider { provider: String, message: String },

    /// Temporal error
    #[cfg(feature = "temporal")]
    #[error("Temporal error: {0}")]
    Temporal(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Create a policy error
    pub fn policy(msg: impl Into<String>) -> Self {
        Self::Policy(msg.into())
    }

    /// Create a workflow error
    pub fn workflow(msg: impl Into<String>) -> Self {
        Self::Workflow(msg.into())
    }

    /// Create an activity error
    pub fn activity(msg: impl Into<String>) -> Self {
        Self::Activity(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::CloudProvider { .. }
                | Self::Kubernetes(_)
                | Self::Temporal(_)
                | Self::Io(_)
        )
    }

    /// Check if error should trigger circuit breaker
    pub fn should_trigger_circuit_breaker(&self) -> bool {
        matches!(
            self,
            Self::Activity(_)
                | Self::CloudProvider { .. }
                | Self::Kubernetes(_)
                | Self::RollbackFailed(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryable() {
        let err = Error::CloudProvider {
            provider: "aws".to_string(),
            message: "timeout".to_string(),
        };
        assert!(err.is_retryable());

        let err = Error::Policy("test".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_circuit_breaker_trigger() {
        let err = Error::Activity("failed".to_string());
        assert!(err.should_trigger_circuit_breaker());

        let err = Error::ApprovalRejected {
            reason: "denied".to_string(),
        };
        assert!(!err.should_trigger_circuit_breaker());
    }
}
