//! Activity executors for remediation actions
//!
//! Activities are the actual execution units that perform actions
//! on Kubernetes, AWS, Azure, GCP, or custom infrastructure.

use crate::{error::Result, Error};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Activity execution options
#[derive(Debug, Clone)]
pub struct ActivityOptions {
    /// Activity type name
    pub activity_type: String,

    /// Timeout
    pub timeout: Duration,

    /// Maximum retry attempts
    pub max_attempts: u32,

    /// Retry backoff
    pub retry_backoff: Duration,
}

impl Default for ActivityOptions {
    fn default() -> Self {
        Self {
            activity_type: String::new(),
            timeout: Duration::from_secs(30),
            max_attempts: 3,
            retry_backoff: Duration::from_secs(1),
        }
    }
}

/// Activity input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityInput {
    /// Activity type
    pub activity_type: String,

    /// Input data
    pub data: serde_json::Value,
}

/// Activity output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityOutput {
    /// Success status
    pub success: bool,

    /// Output data
    pub data: Option<serde_json::Value>,

    /// Error message if failed
    pub error: Option<String>,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Trait for activity executors
#[async_trait::async_trait]
pub trait ActivityExecutor: Send + Sync {
    /// Execute activity
    async fn execute(&self, input: ActivityInput) -> Result<ActivityOutput>;

    /// Get activity type name
    fn activity_type(&self) -> &str;

    /// Check if executor supports this activity
    fn supports(&self, activity_type: &str) -> bool {
        self.activity_type() == activity_type
    }
}

/// Simulated activity executor.
///
/// Logs each activity and returns a successful result **without touching
/// any real system** — every payload carries `"simulated": true` and the
/// execution time is the real elapsed time of the (trivial) call. This is
/// the only executor that ships today; real Kubernetes/cloud executors do
/// not exist yet. Use it to exercise the workflow engine, policy gates,
/// and safety interlocks end to end.
pub struct SimulatedActivityExecutor;

impl SimulatedActivityExecutor {
    /// Create a new simulated executor.
    pub fn new() -> Self {
        Self
    }

    fn simulate(
        activity: &str,
        input: &ActivityInput,
        start: std::time::Instant,
        mut data: serde_json::Value,
    ) -> ActivityOutput {
        tracing::info!(activity, data = %input.data, "simulating activity (no real system is touched)");
        if let Some(obj) = data.as_object_mut() {
            obj.insert("simulated".to_string(), serde_json::Value::Bool(true));
        }
        ActivityOutput {
            success: true,
            data: Some(data),
            error: None,
            execution_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    fn require<'a>(input: &'a ActivityInput, key: &str) -> Result<&'a str> {
        input.data[key]
            .as_str()
            .ok_or_else(|| Error::activity(format!("Missing {key}")))
    }
}

impl Default for SimulatedActivityExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ActivityExecutor for SimulatedActivityExecutor {
    async fn execute(&self, input: ActivityInput) -> Result<ActivityOutput> {
        let start = std::time::Instant::now();

        match input.activity_type.as_str() {
            "check_service_health" => Ok(Self::simulate(
                "check_service_health",
                &input,
                start,
                serde_json::json!({ "healthy": true, "message": "Service healthy" }),
            )),
            "restart_service" => Ok(Self::simulate(
                "restart_service",
                &input,
                start,
                serde_json::json!({ "message": "Service restarted", "service": input.data }),
            )),
            "scale_service" => {
                let service_name = Self::require(&input, "service_name")?;
                let replicas = input.data["replicas"]
                    .as_u64()
                    .ok_or_else(|| Error::activity("Missing replicas"))?;
                Ok(Self::simulate(
                    "scale_service",
                    &input,
                    start,
                    serde_json::json!({ "service": service_name, "replicas": replicas, "message": "Service scaled" }),
                ))
            }
            "delete_pod" => {
                let pod_name = Self::require(&input, "pod_name")?;
                let namespace = input.data["namespace"].as_str().unwrap_or("default");
                Ok(Self::simulate(
                    "delete_pod",
                    &input,
                    start,
                    serde_json::json!({ "pod": pod_name, "namespace": namespace, "message": "Pod deleted" }),
                ))
            }
            "get_deployment" => {
                let deployment_name = Self::require(&input, "deployment_name")?;
                Ok(Self::simulate(
                    "get_deployment",
                    &input,
                    start,
                    serde_json::json!({ "deployment": deployment_name, "replicas": 3, "ready_replicas": 3, "updated_replicas": 3 }),
                ))
            }
            other => Err(Error::activity(format!("Unknown activity: {other}"))),
        }
    }

    fn activity_type(&self) -> &str {
        "simulated"
    }

    fn supports(&self, activity_type: &str) -> bool {
        matches!(
            activity_type,
            "check_service_health"
                | "restart_service"
                | "scale_service"
                | "delete_pod"
                | "get_deployment"
        )
    }
}

/// Composite activity executor that routes to appropriate provider
pub struct CompositeActivityExecutor {
    executors: Vec<Box<dyn ActivityExecutor>>,
}

impl CompositeActivityExecutor {
    /// Create new composite executor
    pub fn new() -> Self {
        Self {
            executors: Vec::new(),
        }
    }

    /// Add executor
    pub fn add_executor(mut self, executor: Box<dyn ActivityExecutor>) -> Self {
        self.executors.push(executor);
        self
    }

    /// Find executor for activity type
    fn find_executor(&self, activity_type: &str) -> Option<&dyn ActivityExecutor> {
        self.executors
            .iter()
            .find(|e| e.supports(activity_type))
            .map(|e| e.as_ref())
    }
}

impl Default for CompositeActivityExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ActivityExecutor for CompositeActivityExecutor {
    async fn execute(&self, input: ActivityInput) -> Result<ActivityOutput> {
        let executor = self
            .find_executor(&input.activity_type)
            .ok_or_else(|| Error::activity(format!("No executor for {}", input.activity_type)))?;

        executor.execute(input).await
    }

    fn activity_type(&self) -> &str {
        "composite"
    }

    fn supports(&self, _activity_type: &str) -> bool {
        true // Composite executor delegates to children
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulated_restart() {
        let executor = SimulatedActivityExecutor::new();

        let input = ActivityInput {
            activity_type: "restart_service".to_string(),
            data: serde_json::json!({
                "service_name": "test-service",
                "namespace": "default"
            }),
        };

        let output = executor.execute(input).await.unwrap();
        assert!(output.success);
    }

    #[tokio::test]
    async fn test_simulated_scale() {
        let executor = SimulatedActivityExecutor::new();

        let input = ActivityInput {
            activity_type: "scale_service".to_string(),
            data: serde_json::json!({
                "service_name": "test-service",
                "replicas": 5
            }),
        };

        let output = executor.execute(input).await.unwrap();
        assert!(output.success);
    }

    #[tokio::test]
    async fn test_composite_executor() {
        let executor = CompositeActivityExecutor::new();

        let input = ActivityInput {
            activity_type: "unknown".to_string(),
            data: serde_json::json!({}),
        };

        let result = executor.execute(input).await;
        assert!(result.is_err());
    }
}
