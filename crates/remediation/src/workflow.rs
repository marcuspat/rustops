//! Workflow orchestration for remediation
//!
//! Temporal workflow definitions for automated remediation actions.

use crate::{
    activity::{ActivityExecutor, ActivityInput},
    error::{Error, Result},
    IncidentContext, RemediationConfig, RemediationResult,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Workflow context
#[derive(Debug, Clone)]
pub struct WorkflowContext {
    /// Workflow ID
    pub workflow_id: String,

    /// Incident context
    pub incident: IncidentContext,

    /// Current state
    pub state: WorkflowState,

    /// Execution history
    pub history: Vec<WorkflowEvent>,

    /// Metadata
    pub metadata: serde_json::Value,
}

/// Workflow state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowState {
    /// Workflow is pending
    Pending,
    /// Workflow is running
    Running,
    /// Workflow is waiting for approval
    WaitingForApproval,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed
    Failed,
    /// Workflow was cancelled
    Cancelled,
    /// Workflow is rolling back
    RollingBack,
}

/// Workflow event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Event type
    pub event_type: String,

    /// Event message
    pub message: String,

    /// Event data
    pub data: Option<serde_json::Value>,
}

/// Workflow status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStatus {
    /// Workflow ID
    pub workflow_id: String,

    /// Current state
    pub state: WorkflowState,

    /// Progress percentage (0-100)
    pub progress: u8,

    /// Current step
    pub current_step: Option<String>,

    /// Total steps
    pub total_steps: usize,

    /// Completed steps
    pub completed_steps: usize,

    /// Error message if failed
    pub error: Option<String>,

    /// Start time
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// End time (if completed)
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Remediation workflow trait
#[async_trait::async_trait]
pub trait RemediationWorkflow: Send + Sync {
    /// Execute the workflow
    async fn execute(&self, context: &mut WorkflowContext) -> Result<RemediationResult>;

    /// Get workflow name
    fn name(&self) -> &str;

    /// Get workflow steps
    fn steps(&self) -> Vec<String>;

    /// Check if workflow can be cancelled
    fn can_cancel(&self) -> bool {
        true
    }
}

/// Restart service workflow
pub struct RestartServiceWorkflow {
    executor: Arc<dyn ActivityExecutor>,
    config: RemediationConfig,
}

impl RestartServiceWorkflow {
    /// Create new restart workflow
    pub fn new(executor: Arc<dyn ActivityExecutor>, config: RemediationConfig) -> Self {
        Self { executor, config }
    }
}

impl RestartServiceWorkflow {
    /// The workflow body; `execute` bounds it with the configured timeout.
    async fn run(&self, context: &mut WorkflowContext) -> Result<RemediationResult> {
        let workflow_id = context.workflow_id.clone();
        let incident_id = context.incident.incident_id.clone();

        // Step 1: Pre-flight health check
        context.state = WorkflowState::Running;
        context.history.push(WorkflowEvent {
            timestamp: chrono::Utc::now(),
            event_type: "step_start".to_string(),
            message: "Starting pre-flight health check".to_string(),
            data: None,
        });

        let health_input = ActivityInput {
            activity_type: "check_service_health".to_string(),
            data: serde_json::json!({
                "service_name": context.incident.service_name,
                "namespace": context.incident.namespace,
            }),
        };

        let health_output = self.executor.execute(health_input).await?;

        if !health_output.success {
            return Ok(RemediationResult {
                workflow_id: workflow_id.clone(),
                incident_id: incident_id.clone(),
                action: crate::ActionType::RestartService,
                success: false,
                message: "Pre-flight health check failed".to_string(),
                completed_at: chrono::Utc::now(),
                rolled_back: false,
                details: health_output.data,
            });
        }

        // Step 2: Execute restart
        context.history.push(WorkflowEvent {
            timestamp: chrono::Utc::now(),
            event_type: "step_start".to_string(),
            message: "Restarting service".to_string(),
            data: None,
        });

        let restart_input = ActivityInput {
            activity_type: "restart_service".to_string(),
            data: serde_json::json!({
                "service_name": context.incident.service_name,
                "namespace": context.incident.namespace,
            }),
        };

        let restart_output = self.executor.execute(restart_input).await?;

        if !restart_output.success {
            context.state = WorkflowState::Failed;
            return Ok(RemediationResult {
                workflow_id: workflow_id.clone(),
                incident_id: incident_id.clone(),
                action: crate::ActionType::RestartService,
                success: false,
                message: restart_output
                    .error
                    .unwrap_or_else(|| "Restart failed".to_string()),
                completed_at: chrono::Utc::now(),
                rolled_back: false,
                details: restart_output.data,
            });
        }

        // Step 3: Post-action verification
        context.history.push(WorkflowEvent {
            timestamp: chrono::Utc::now(),
            event_type: "step_start".to_string(),
            message: "Verifying service health".to_string(),
            data: None,
        });

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let verify_input = ActivityInput {
            activity_type: "check_service_health".to_string(),
            data: serde_json::json!({
                "service_name": context.incident.service_name,
                "namespace": context.incident.namespace,
            }),
        };

        let verify_output = self.executor.execute(verify_input).await?;

        context.state = WorkflowState::Completed;
        let success = verify_output.success;

        Ok(RemediationResult {
            workflow_id,
            incident_id,
            action: crate::ActionType::RestartService,
            success,
            message: if success {
                "Service restarted and verified healthy".to_string()
            } else {
                "Service restarted but health check failed".to_string()
            },
            completed_at: chrono::Utc::now(),
            rolled_back: false,
            details: Some(serde_json::json!({
                "restart_output": restart_output.data,
                "verify_output": verify_output.data
            })),
        })
    }
}

#[async_trait::async_trait]
impl RemediationWorkflow for RestartServiceWorkflow {
    async fn execute(&self, context: &mut WorkflowContext) -> Result<RemediationResult> {
        let timeout = std::time::Duration::from_secs(self.config.default_workflow_timeout_secs);
        let workflow_id = context.workflow_id.clone();
        let incident_id = context.incident.incident_id.clone();
        // timeout_secs == 0 means unbounded: tokio::time::timeout with a
        // zero duration fires on the first poll, which would silently fail
        // every workflow for an operator who set 0 expecting "no timeout".
        let outcome = if self.config.default_workflow_timeout_secs == 0 {
            Ok(self.run(context).await)
        } else {
            tokio::time::timeout(timeout, self.run(context)).await
        };
        match outcome {
            Ok(result) => result,
            Err(_) => {
                context.state = WorkflowState::Failed;
                Ok(RemediationResult {
                    workflow_id,
                    incident_id,
                    action: crate::ActionType::RestartService,
                    success: false,
                    message: format!("Workflow timed out after {}s", timeout.as_secs()),
                    completed_at: chrono::Utc::now(),
                    rolled_back: false,
                    details: None,
                })
            }
        }
    }

    fn name(&self) -> &str {
        "restart_service"
    }

    fn steps(&self) -> Vec<String> {
        vec![
            "Pre-flight health check".to_string(),
            "Restart service".to_string(),
            "Post-action verification".to_string(),
        ]
    }
}

/// Scale service workflow
pub struct ScaleServiceWorkflow {
    executor: Arc<dyn ActivityExecutor>,
    config: RemediationConfig,
}

impl ScaleServiceWorkflow {
    /// Create new scale workflow
    pub fn new(executor: Arc<dyn ActivityExecutor>, config: RemediationConfig) -> Self {
        Self { executor, config }
    }
}

impl ScaleServiceWorkflow {
    /// The workflow body; `execute` bounds it with the configured timeout.
    async fn run(&self, context: &mut WorkflowContext) -> Result<RemediationResult> {
        let workflow_id = context.workflow_id.clone();
        let incident_id = context.incident.incident_id.clone();

        // Step 1: Get current replica count
        let get_input = ActivityInput {
            activity_type: "get_deployment".to_string(),
            data: serde_json::json!({
                "deployment_name": context.incident.service_name,
                "namespace": context.incident.namespace,
            }),
        };

        let get_output = self.executor.execute(get_input).await?;
        let current_replicas = get_output
            .data
            .and_then(|d| d["replicas"].as_u64())
            .unwrap_or(1);

        // Calculate target replicas (scale up by 50%)
        let target_replicas = (current_replicas * 3 / 2).max(1);

        // Step 2: Scale service
        let scale_input = ActivityInput {
            activity_type: "scale_service".to_string(),
            data: serde_json::json!({
                "service_name": context.incident.service_name,
                "namespace": context.incident.namespace,
                "replicas": target_replicas,
            }),
        };

        let scale_output = self.executor.execute(scale_input).await?;

        if !scale_output.success {
            context.state = WorkflowState::Failed;
            return Ok(RemediationResult {
                workflow_id,
                incident_id,
                action: crate::ActionType::ScaleService,
                success: false,
                message: scale_output
                    .error
                    .unwrap_or_else(|| "Scale failed".to_string()),
                completed_at: chrono::Utc::now(),
                rolled_back: false,
                details: scale_output.data,
            });
        }

        // Step 3: Monitor for stability
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        context.state = WorkflowState::Completed;

        Ok(RemediationResult {
            workflow_id,
            incident_id,
            action: crate::ActionType::ScaleService,
            success: true,
            message: format!(
                "Scaled service from {} to {} replicas",
                current_replicas, target_replicas
            ),
            completed_at: chrono::Utc::now(),
            rolled_back: false,
            details: Some(serde_json::json!({
                "previous_replicas": current_replicas,
                "new_replicas": target_replicas
            })),
        })
    }
}

#[async_trait::async_trait]
impl RemediationWorkflow for ScaleServiceWorkflow {
    async fn execute(&self, context: &mut WorkflowContext) -> Result<RemediationResult> {
        let timeout = std::time::Duration::from_secs(self.config.default_workflow_timeout_secs);
        let workflow_id = context.workflow_id.clone();
        let incident_id = context.incident.incident_id.clone();
        // timeout_secs == 0 means unbounded: tokio::time::timeout with a
        // zero duration fires on the first poll, which would silently fail
        // every workflow for an operator who set 0 expecting "no timeout".
        let outcome = if self.config.default_workflow_timeout_secs == 0 {
            Ok(self.run(context).await)
        } else {
            tokio::time::timeout(timeout, self.run(context)).await
        };
        match outcome {
            Ok(result) => result,
            Err(_) => {
                context.state = WorkflowState::Failed;
                Ok(RemediationResult {
                    workflow_id,
                    incident_id,
                    action: crate::ActionType::ScaleService,
                    success: false,
                    message: format!("Workflow timed out after {}s", timeout.as_secs()),
                    completed_at: chrono::Utc::now(),
                    rolled_back: false,
                    details: None,
                })
            }
        }
    }

    fn name(&self) -> &str {
        "scale_service"
    }

    fn steps(&self) -> Vec<String> {
        vec![
            "Get current state".to_string(),
            "Scale service".to_string(),
            "Monitor stability".to_string(),
        ]
    }
}

/// Workflow engine
pub struct WorkflowEngine {
    executor: Arc<dyn ActivityExecutor>,
    config: RemediationConfig,
    active_workflows: Arc<RwLock<std::collections::HashMap<String, WorkflowContext>>>,
    /// Insertion order of workflow ids, for FIFO eviction of terminal
    /// contexts past `config.workflow_history_retention`.
    workflow_order: Arc<std::sync::Mutex<std::collections::VecDeque<String>>>,
}

impl WorkflowEngine {
    /// Create new workflow engine
    pub fn new(executor: Arc<dyn ActivityExecutor>, config: RemediationConfig) -> Self {
        Self {
            executor,
            config,
            active_workflows: Arc::new(RwLock::new(std::collections::HashMap::new())),
            workflow_order: Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new())),
        }
    }

    /// Start workflow
    pub async fn start_workflow(
        &self,
        incident: IncidentContext,
        workflow: Box<dyn RemediationWorkflow>,
    ) -> Result<String> {
        let workflow_id = Uuid::new_v4().to_string();

        let mut context = WorkflowContext {
            workflow_id: workflow_id.clone(),
            incident,
            state: WorkflowState::Pending,
            history: Vec::new(),
            metadata: serde_json::json!({}),
        };

        context.history.push(WorkflowEvent {
            timestamp: chrono::Utc::now(),
            event_type: "workflow_created".to_string(),
            message: "Workflow created".to_string(),
            data: Some(serde_json::json!({
                "workflow_name": workflow.name()
            })),
        });

        // Store workflow context, enforcing the configured concurrency limit.
        let mut workflows = self.active_workflows.write().await;
        let in_flight = workflows
            .values()
            .filter(|c| matches!(c.state, WorkflowState::Pending | WorkflowState::Running))
            .count();
        if in_flight >= self.config.max_concurrent_actions {
            return Err(Error::Workflow(format!(
                "concurrency limit reached: {} workflows already in flight (max {})",
                in_flight, self.config.max_concurrent_actions
            )));
        }
        workflows.insert(workflow_id.clone(), context.clone());
        self.workflow_order
            .lock()
            .expect("workflow_order lock poisoned")
            .push_back(workflow_id.clone());

        // Execute workflow (in production, this would be async)
        let _executor = self.executor.clone();
        let workflows_ref = self.active_workflows.clone();
        let workflow_id_clone = workflow_id.clone();
        let order_ref = self.workflow_order.clone();
        let retention = self.config.workflow_history_retention;

        tokio::spawn(async move {
            let result = workflow.execute(&mut context).await;

            // Write the whole mutated context back into the map — state,
            // history and metadata — not just the terminal state. Note the
            // write-back lands at termination: during the run the stored
            // entry still reads Pending, so
            // get_workflow_status reports the run only once execute()
            // timed out (its RemediationResult carries success=false but the
            // local context mutations were the only record of the steps).
            // A context cancelled while running keeps state=Cancelled but
            // still receives the run's history: for a remediation engine,
            // what a cancelled workflow actually did to the target system
            // is the record that matters most.
            let mut workflows = workflows_ref.write().await;
            if let Some(ctx) = workflows.get_mut(&workflow_id_clone) {
                if ctx.state == WorkflowState::Cancelled {
                    ctx.history = context.history;
                } else {
                    context.state = if result.as_ref().map(|r| r.success).unwrap_or(false) {
                        WorkflowState::Completed
                    } else {
                        WorkflowState::Failed
                    };
                    *ctx = context;
                }
            }

            // Evict oldest terminal contexts past the retention cap so the
            // map cannot grow without bound over the process lifetime.
            // Only a prefix of terminal entries is evicted; if the oldest
            // is still in flight, eviction waits for the next completion.
            if workflows.len() > retention {
                let mut order = order_ref.lock().expect("workflow_order lock poisoned");
                while workflows.len() > retention {
                    let Some(oldest) = order.front() else { break };
                    let oldest = oldest.clone();
                    let terminal = workflows
                        .get(&oldest)
                        .map(|c| {
                            !matches!(c.state, WorkflowState::Pending | WorkflowState::Running)
                        })
                        .unwrap_or(true);
                    if terminal {
                        order.pop_front();
                        workflows.remove(&oldest);
                    } else {
                        break;
                    }
                }
            }

            tracing::info!(
                workflow_id = %workflow_id_clone,
                result = ?result,
                "Workflow completed"
            );

            result
        });

        Ok(workflow_id)
    }

    /// Get workflow status
    pub async fn get_workflow_status(&self, workflow_id: &str) -> Option<WorkflowStatus> {
        let workflows = self.active_workflows.read().await;
        let context = workflows.get(workflow_id)?;

        let completed_steps = context
            .history
            .iter()
            .filter(|e| e.event_type == "step_complete")
            .count();

        Some(WorkflowStatus {
            workflow_id: workflow_id.to_string(),
            state: context.state.clone(),
            progress: ((completed_steps as f32 / context.history.len() as f32) * 100.0) as u8,
            current_step: context.history.last().map(|h| h.message.clone()),
            total_steps: context.history.len(),
            completed_steps,
            error: None,
            started_at: context
                .history
                .first()
                .map(|h| h.timestamp)
                .unwrap_or_else(chrono::Utc::now),
            ended_at: if context.state == WorkflowState::Completed
                || context.state == WorkflowState::Failed
            {
                Some(chrono::Utc::now())
            } else {
                None
            },
        })
    }

    /// Cancel workflow
    pub async fn cancel_workflow(&self, workflow_id: &str) -> Result<()> {
        let mut workflows = self.active_workflows.write().await;
        let context = workflows
            .get_mut(workflow_id)
            .ok_or_else(|| Error::workflow("Workflow not found"))?;

        context.state = WorkflowState::Cancelled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::{CompositeActivityExecutor, SimulatedActivityExecutor};

    #[tokio::test]
    async fn test_restart_workflow() {
        let composite = CompositeActivityExecutor::new()
            .add_executor(Box::new(SimulatedActivityExecutor::new()));

        let executor: Arc<dyn ActivityExecutor> = Arc::new(composite);
        let config = RemediationConfig::default();
        let workflow = RestartServiceWorkflow::new(executor.clone(), config);

        let incident = IncidentContext {
            incident_id: "test-1".to_string(),
            severity: crate::IncidentSeverity::High,
            service_name: "test-service".to_string(),
            namespace: "default".to_string(),
            cluster: "test-cluster".to_string(),
            description: "Test incident".to_string(),
            started_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        };

        let mut context = WorkflowContext {
            workflow_id: Uuid::new_v4().to_string(),
            incident,
            state: WorkflowState::Pending,
            history: Vec::new(),
            metadata: serde_json::json!({}),
        };

        let result = workflow.execute(&mut context).await.unwrap();
        assert!(result.success);
        assert_eq!(result.action, crate::ActionType::RestartService);
    }

    #[tokio::test]
    async fn test_workflow_engine() {
        let composite = CompositeActivityExecutor::new()
            .add_executor(Box::new(SimulatedActivityExecutor::new()));

        let executor: Arc<dyn ActivityExecutor> = Arc::new(composite);
        let config = RemediationConfig::default();
        let engine = WorkflowEngine::new(executor, config);

        let incident = IncidentContext {
            incident_id: "test-1".to_string(),
            severity: crate::IncidentSeverity::Medium,
            service_name: "test-service".to_string(),
            namespace: "default".to_string(),
            cluster: "test-cluster".to_string(),
            description: "Test".to_string(),
            started_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        };

        let workflow = Box::new(RestartServiceWorkflow::new(
            engine.executor.clone(),
            engine.config.clone(),
        ));

        let workflow_id = engine.start_workflow(incident, workflow).await.unwrap();
        let status = engine.get_workflow_status(&workflow_id).await;

        assert!(status.is_some());
    }
}

#[cfg(test)]
mod gate_round3_tests {
    use super::*;
    use crate::activity::{ActivityOutput, SimulatedActivityExecutor};
    use std::time::Duration;

    fn test_incident() -> IncidentContext {
        IncidentContext {
            incident_id: "gate-1".to_string(),
            severity: crate::IncidentSeverity::Medium,
            service_name: "svc".to_string(),
            namespace: "default".to_string(),
            cluster: "c".to_string(),
            description: "test".to_string(),
            started_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        }
    }

    fn test_context() -> WorkflowContext {
        WorkflowContext {
            workflow_id: "wf-gate".to_string(),
            incident: test_incident(),
            state: WorkflowState::Pending,
            history: Vec::new(),
            metadata: serde_json::json!({}),
        }
    }

    /// Executor whose activities sleep, so the workflow can be driven past
    /// a short timeout deterministically (1s timeout vs 30s sleeps).
    struct SlowExecutor;

    #[async_trait::async_trait]
    impl ActivityExecutor for SlowExecutor {
        async fn execute(&self, _input: ActivityInput) -> Result<ActivityOutput> {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            Ok(ActivityOutput {
                success: true,
                data: None,
                error: None,
                execution_time_ms: 30_000,
            })
        }

        fn activity_type(&self) -> &str {
            "slow"
        }

        fn supports(&self, activity_type: &str) -> bool {
            activity_type == "slow"
        }
    }

    #[tokio::test]
    async fn timeout_fails_result_and_marks_context_failed() {
        let executor: Arc<dyn ActivityExecutor> = Arc::new(SlowExecutor);
        let config = RemediationConfig {
            default_workflow_timeout_secs: 1,
            ..Default::default()
        };
        let workflow = RestartServiceWorkflow::new(executor, config);

        let mut context = test_context();
        let result = workflow.execute(&mut context).await.unwrap();

        assert!(!result.success);
        assert!(
            result.message.contains("timed out"),
            "message should say it timed out: {}",
            result.message
        );
        assert!(matches!(context.state, WorkflowState::Failed));
    }

    #[tokio::test]
    async fn timeout_zero_means_unbounded_not_instant_failure() {
        // Regression: timeout(0s) fires on the first poll, so a config of 0
        // used to fail every workflow instantly instead of meaning "no
        // timeout". With the fast simulated executor an unbounded run
        // completes successfully.
        let executor: Arc<dyn ActivityExecutor> = Arc::new(SimulatedActivityExecutor::new());
        let config = RemediationConfig {
            default_workflow_timeout_secs: 0,
            ..Default::default()
        };
        let workflow = RestartServiceWorkflow::new(executor, config);

        let mut context = test_context();
        let result = workflow.execute(&mut context).await.unwrap();
        assert!(
            result.success,
            "timeout_secs=0 must mean unbounded, not instant timeout: {}",
            result.message
        );
    }

    /// Workflow whose execution blocks long enough to hold a concurrency
    /// slot while the test starts a second workflow.
    struct BlockingWorkflow {
        incident_id: String,
    }

    #[async_trait::async_trait]
    impl RemediationWorkflow for BlockingWorkflow {
        async fn execute(&self, context: &mut WorkflowContext) -> Result<RemediationResult> {
            tokio::time::sleep(Duration::from_millis(150)).await;
            context.history.push(WorkflowEvent {
                timestamp: chrono::Utc::now(),
                event_type: "step_complete".to_string(),
                message: "blocked step done".to_string(),
                data: None,
            });
            Ok(RemediationResult {
                workflow_id: context.workflow_id.clone(),
                incident_id: self.incident_id.clone(),
                action: crate::ActionType::RestartService,
                success: true,
                message: "done".to_string(),
                completed_at: chrono::Utc::now(),
                rolled_back: false,
                details: None,
            })
        }

        fn name(&self) -> &str {
            "blocking_test_workflow"
        }

        fn steps(&self) -> Vec<String> {
            vec!["block".to_string()]
        }
    }

    #[tokio::test]
    async fn engine_enforces_and_releases_concurrency_limit() {
        let executor: Arc<dyn ActivityExecutor> = Arc::new(SimulatedActivityExecutor::new());
        let config = RemediationConfig {
            max_concurrent_actions: 1,
            ..Default::default()
        };
        let engine = WorkflowEngine::new(executor, config);

        let _first = engine
            .start_workflow(
                test_incident(),
                Box::new(BlockingWorkflow {
                    incident_id: "gate-1".to_string(),
                }),
            )
            .await
            .unwrap();

        // While the first is in flight, a second start must be rejected.
        let second = engine
            .start_workflow(
                test_incident(),
                Box::new(BlockingWorkflow {
                    incident_id: "gate-1".to_string(),
                }),
            )
            .await;
        assert!(
            second.is_err(),
            "concurrency limit of 1 must reject a second in-flight workflow"
        );

        // After it terminates the slot frees.
        tokio::time::sleep(Duration::from_millis(400)).await;
        let third = engine
            .start_workflow(
                test_incident(),
                Box::new(BlockingWorkflow {
                    incident_id: "gate-1".to_string(),
                }),
            )
            .await;
        assert!(third.is_ok(), "slot must free after termination");
    }

    #[tokio::test]
    async fn cancelled_workflow_keeps_its_run_history() {
        // Regression: the write-back used to skip cancelled contexts
        // entirely, discarding the history of what actually ran.
        let executor: Arc<dyn ActivityExecutor> = Arc::new(SimulatedActivityExecutor::new());
        let config = RemediationConfig::default();
        let engine = WorkflowEngine::new(executor, config);

        let workflow_id = engine
            .start_workflow(
                test_incident(),
                Box::new(BlockingWorkflow {
                    incident_id: "gate-1".to_string(),
                }),
            )
            .await
            .unwrap();

        // Cancel while the blocking workflow is mid-run.
        engine.cancel_workflow(&workflow_id).await.unwrap();

        // Wait for the run to finish and the write-back to happen.
        tokio::time::sleep(Duration::from_millis(400)).await;

        let status = engine
            .get_workflow_status(&workflow_id)
            .await
            .expect("context must remain queryable");
        assert_eq!(status.state, WorkflowState::Cancelled);
        assert!(
            status.total_steps > 1,
            "cancelled workflow must keep its run history (steps: {})",
            status.total_steps
        );
    }
}
