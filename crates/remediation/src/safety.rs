//! Safety interlocks for remediation actions
//!
//! Multi-layer safety protection including:
//! - Blast radius limits
//! - Circuit breakers
//! - Rollback mechanisms
//! - Cooldown periods

use crate::{error::Result, ActionType, Error, IncidentContext};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Blast radius configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadius {
    /// Maximum scope of impact
    pub scope: BlastRadiusScope,

    /// Affected namespaces
    pub namespaces: Vec<String>,

    /// Affected clusters
    pub clusters: Vec<String>,

    /// Affected regions
    pub regions: Vec<String>,
}

/// Scope of blast radius
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlastRadiusScope {
    /// Single pod/container
    Pod,
    /// Single namespace
    Namespace,
    /// Single cluster
    Cluster,
    /// Multiple clusters in region
    Region,
    /// Global impact
    Global,
}

impl BlastRadius {
    /// Create new blast radius
    pub fn new(scope: BlastRadiusScope) -> Self {
        Self {
            scope,
            namespaces: Vec::new(),
            clusters: Vec::new(),
            regions: Vec::new(),
        }
    }

    /// Add namespace
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespaces.push(namespace);
        self
    }

    /// Add cluster
    pub fn with_cluster(mut self, cluster: String) -> Self {
        self.clusters.push(cluster);
        self
    }

    /// Check if action is within blast radius
    pub fn check(&self, context: &IncidentContext) -> Result<()> {
        match self.scope {
            BlastRadiusScope::Pod => {
                // Only affects single pod
                Ok(())
            }
            BlastRadiusScope::Namespace => {
                if !self.namespaces.is_empty() && !self.namespaces.contains(&context.namespace) {
                    return Err(Error::BlastRadiusExceeded(format!(
                        "Namespace {} not in allowed list: {:?}",
                        context.namespace, self.namespaces
                    )));
                }
                Ok(())
            }
            BlastRadiusScope::Cluster => {
                if !self.clusters.is_empty() && !self.clusters.contains(&context.cluster) {
                    return Err(Error::BlastRadiusExceeded(format!(
                        "Cluster {} not in allowed list: {:?}",
                        context.cluster, self.clusters
                    )));
                }
                Ok(())
            }
            BlastRadiusScope::Region => Ok(()),
            BlastRadiusScope::Global => {
                Err(Error::BlastRadiusExceeded("Global impact not allowed".to_string()))
            }
        }
    }
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CircuitBreakerState {
    /// Circuit is closed (normal operation)
    Closed,
    /// Circuit is open (blocking actions)
    Open,
    /// Circuit is half-open (testing)
    HalfOpen,
}

/// Circuit breaker for preventing cascade failures
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    action: ActionType,
    state: Arc<RwLock<CircuitBreakerState>>,
    failure_count: Arc<RwLock<usize>>,
    failure_threshold: usize,
    last_failure_time: Arc<RwLock<Option<DateTime<Utc>>>>,
    reset_timeout_secs: u64,
    success_count: Arc<RwLock<usize>>,
    success_threshold: usize,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(action: ActionType, failure_threshold: usize, reset_timeout_secs: u64) -> Self {
        Self {
            action,
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            failure_threshold,
            last_failure_time: Arc::new(RwLock::new(None)),
            reset_timeout_secs,
            success_count: Arc::new(RwLock::new(0)),
            success_threshold: 2, // Require 2 successes to close
        }
    }

    /// Check if action is allowed
    pub async fn allow_action(&self) -> Result<()> {
        let mut state = self.state.write().await;

        match *state {
            CircuitBreakerState::Open => {
                // Check if reset timeout has passed
                let last_failure = *self.last_failure_time.read().await;
                if let Some(failure_time) = last_failure {
                    let elapsed = Utc::now().signed_duration_since(failure_time);
                    if elapsed.num_seconds() > self.reset_timeout_secs as i64 {
                        // Transition to half-open
                        *state = CircuitBreakerState::HalfOpen;
                        *self.success_count.write().await = 0;
                        return Ok(());
                    }
                }

                Err(Error::CircuitBreakerOpen {
                    action: format!("{:?}", self.action),
                })
            }
            CircuitBreakerState::HalfOpen | CircuitBreakerState::Closed => Ok(()),
        }
    }

    /// Record successful action
    pub async fn record_success(&self) {
        *self.failure_count.write().await = 0;

        let mut state = self.state.write().await;
        if *state == CircuitBreakerState::HalfOpen {
            let mut success_count = self.success_count.write().await;
            *success_count += 1;

            if *success_count >= self.success_threshold {
                *state = CircuitBreakerState::Closed;
                *success_count = 0;
            }
        }
    }

    /// Record failed action
    pub async fn record_failure(&self) {
        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;
        *self.last_failure_time.write().await = Some(Utc::now());

        let mut state = self.state.write().await;
        if *failure_count >= self.failure_threshold {
            *state = CircuitBreakerState::Open;
        } else if *state == CircuitBreakerState::HalfOpen {
            *state = CircuitBreakerState::Open;
        }
    }

    /// Get current state
    pub async fn state(&self) -> CircuitBreakerState {
        *self.state.read().await
    }

    /// Get failure count
    pub async fn failure_count(&self) -> usize {
        *self.failure_count.read().await
    }

    /// Reset circuit breaker
    pub async fn reset(&self) {
        *self.state.write().await = CircuitBreakerState::Closed;
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;
        *self.last_failure_time.write().await = None;
    }
}

/// Safety interlock manager
pub struct SafetyInterlock {
    /// Circuit breakers for each action type
    circuit_breakers: Arc<RwLock<HashMap<ActionType, CircuitBreaker>>>,

    /// Blast radius limits
    blast_radius_limits: Arc<RwLock<HashMap<ActionType, BlastRadius>>>,

    /// Cooldown trackers
    cooldown_trackers: Arc<RwLock<HashMap<ActionType, DateTime<Utc>>>>,

    /// Default cooldown period in seconds
    default_cooldown_secs: u64,
}

impl SafetyInterlock {
    /// Create new safety interlock
    pub fn new(default_cooldown_secs: u64) -> Self {
        Self {
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            blast_radius_limits: Arc::new(RwLock::new(HashMap::new())),
            cooldown_trackers: Arc::new(RwLock::new(HashMap::new())),
            default_cooldown_secs,
        }
    }

    /// Add circuit breaker for action
    pub async fn add_circuit_breaker(&self, breaker: CircuitBreaker) {
        let mut breakers = self.circuit_breakers.write().await;
        breakers.insert(breaker.action.clone(), breaker);
    }

    /// Set blast radius limit for action
    pub async fn set_blast_radius(&self, action: ActionType, radius: BlastRadius) {
        let mut limits = self.blast_radius_limits.write().await;
        limits.insert(action, radius);
    }

    /// Check if action is safe to execute
    pub async fn check_safe(&self, action: &ActionType, context: &IncidentContext) -> Result<()> {
        // Check circuit breaker
        let breakers = self.circuit_breakers.read().await;
        if let Some(breaker) = breakers.get(action) {
            breaker.allow_action().await?;
        }

        // Check blast radius
        let limits = self.blast_radius_limits.read().await;
        if let Some(radius) = limits.get(action) {
            radius.check(context)?;
        }

        // Check cooldown
        let cooldowns = self.cooldown_trackers.read().await;
        if let Some(last_action) = cooldowns.get(action) {
            let elapsed = Utc::now().signed_duration_since(*last_action);
            let cooldown_secs = self.default_cooldown_secs as i64;
            if elapsed.num_seconds() < cooldown_secs {
                return Err(Error::SafetyInterlock(format!(
                    "Cooldown period active. {} seconds remaining",
                    cooldown_secs - elapsed.num_seconds()
                )));
            }
        }

        Ok(())
    }

    /// Record action execution
    pub async fn record_action(&self, action: &ActionType, success: bool) {
        // Update cooldown tracker
        let mut cooldowns = self.cooldown_trackers.write().await;
        cooldowns.insert(action.clone(), Utc::now());

        // Update circuit breaker
        let breakers = self.circuit_breakers.read().await;
        if let Some(breaker) = breakers.get(action) {
            if success {
                breaker.record_success().await;
            } else {
                breaker.record_failure().await;
            }
        }
    }

    /// Get circuit breaker state for action
    pub async fn circuit_breaker_state(&self, action: &ActionType) -> Option<CircuitBreakerState> {
        let breakers = self.circuit_breakers.read().await;
        breakers.get(action).map(|b| b.state().await)
    }
}

/// Rollback manager for failed actions
pub struct RollbackManager {
    /// Rollback strategies for each action type
    strategies: HashMap<ActionType, RollbackStrategy>,
}

impl RollbackManager {
    /// Create new rollback manager
    pub fn new() -> Self {
        let mut strategies = HashMap::new();

        // Default rollback strategies
        strategies.insert(
            ActionType::RestartService,
            RollbackStrategy::NoRollback("Restart is transient".to_string()),
        );
        strategies.insert(
            ActionType::ScaleService,
            RollbackStrategy::RestorePrevious {
                description: "Restore previous replica count".to_string(),
            },
        );
        strategies.insert(
            ActionType::RollbackDeployment,
            RollbackStrategy::NoRollback("Already a rollback".to_string()),
        );
        strategies.insert(
            ActionType::DeleteResource,
            RollbackStrategy::Recreate {
                description: "Recreate deleted resource".to_string(),
            },
        );

        Self { strategies }
    }

    /// Get rollback strategy for action
    pub fn get_strategy(&self, action: &ActionType) -> Option<&RollbackStrategy> {
        self.strategies.get(action)
    }

    /// Execute rollback
    pub async fn execute(&self, action: &ActionType, context: &RollbackContext) -> Result<()> {
        let strategy = self
            .get_strategy(action)
            .ok_or_else(|| Error::RollbackFailed("No rollback strategy found".to_string()))?;

        match strategy {
            RollbackStrategy::NoRollback(reason) => {
                tracing::info!("No rollback possible: {}", reason);
                Ok(())
            }
            RollbackStrategy::RestorePrevious { .. } => {
                // In production, this would execute actual rollback
                tracing::info!("Executing restore previous rollback");
                Ok(())
            }
            RollbackStrategy::Recreate { .. } => {
                // In production, this would recreate the resource
                tracing::info!("Executing recreate rollback");
                Ok(())
            }
            RollbackStrategy::Custom { rollback_fn } => {
                rollback_fn(context).await
            }
        }
    }

    /// Add custom rollback strategy
    pub fn add_strategy(&mut self, action: ActionType, strategy: RollbackStrategy) {
        self.strategies.insert(action, strategy);
    }
}

impl Default for RollbackManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Rollback strategy
#[derive(Clone)]
pub enum RollbackStrategy {
    /// No rollback possible or needed
    NoRollback(String),
    /// Restore previous state
    RestorePrevious { description: String },
    /// Recreate deleted resource
    Recreate { description: String },
    /// Custom rollback function
    Custom {
        rollback_fn: Arc<dyn Fn(&RollbackContext) -> Result<()> + Send + Sync>,
    },
}

/// Rollback context
#[derive(Debug, Clone)]
pub struct RollbackContext {
    pub action: ActionType,
    pub incident_id: String,
    pub original_state: Option<serde_json::Value>,
    pub current_state: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let breaker = CircuitBreaker::new(ActionType::RestartService, 3, 60);

        assert_eq!(breaker.state().await, CircuitBreakerState::Closed);

        // Record failures
        for _ in 0..3 {
            breaker.record_failure().await;
        }

        assert_eq!(breaker.state().await, CircuitBreakerState::Open);
        assert!(breaker.allow_action().await.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_resets_after_timeout() {
        let breaker = CircuitBreaker::new(ActionType::RestartService, 2, 1); // 1 second timeout

        // Open the circuit
        breaker.record_failure().await;
        breaker.record_failure().await;
        assert_eq!(breaker.state().await, CircuitBreakerState::Open);

        // Wait for timeout
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Should transition to half-open
        assert!(breaker.allow_action().await.is_ok());
        assert_eq!(breaker.state().await, CircuitBreakerState::HalfOpen);

        // Success should close it
        breaker.record_success().await;
        breaker.record_success().await;
        assert_eq!(breaker.state().await, CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_blast_radius_check() {
        let radius = BlastRadius::new(BlastRadiusScope::Namespace)
            .with_namespace("production".to_string());

        let context = IncidentContext {
            incident_id: "test".to_string(),
            severity: IncidentSeverity::High,
            service_name: "test".to_string(),
            namespace: "staging".to_string(),
            cluster: "test".to_string(),
            description: "Test".to_string(),
            started_at: Utc::now(),
            metadata: serde_json::json!({}),
        };

        assert!(radius.check(&context).is_err());
    }

    #[tokio::test]
    async fn test_safety_interlock_cooldown() {
        let interlock = SafetyInterlock::new(1); // 1 second cooldown

        let context = IncidentContext {
            incident_id: "test".to_string(),
            severity: IncidentSeverity::Medium,
            service_name: "test".to_string(),
            namespace: "default".to_string(),
            cluster: "test".to_string(),
            description: "Test".to_string(),
            started_at: Utc::now(),
            metadata: serde_json::json!({}),
        };

        // First action should succeed
        assert!(interlock.check_safe(&ActionType::RestartService, &context).await.is_ok());
        interlock.record_action(&ActionType::RestartService, true).await;

        // Second action should fail due to cooldown
        assert!(interlock.check_safe(&ActionType::RestartService, &context).await.is_err());

        // Wait for cooldown
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Should succeed now
        assert!(interlock.check_safe(&ActionType::RestartService, &context).await.is_ok());
    }

    #[tokio::test]
    async fn test_rollback_manager() {
        let manager = RollbackManager::new();

        let strategy = manager.get_strategy(&ActionType::RestartService);
        assert!(strategy.is_some());

        let context = RollbackContext {
            action: ActionType::RestartService,
            incident_id: "test".to_string(),
            original_state: None,
            current_state: None,
            metadata: serde_json::json!({}),
        };

        assert!(manager.execute(&ActionType::RestartService, &context).await.is_ok());
    }
}
