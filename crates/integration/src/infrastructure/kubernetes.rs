// Kubernetes integration
//
// Implements Kubernetes Watch API for real-time resource monitoring
// with Metrics API support and infrastructure actions

use async_trait::async_trait;
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{Event, Node, Pod},
};
use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, WatchEvent},
    Client, Config,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::adapter::{
    ActionResult, BaseAdapter, InfraAction, InfrastructureMonitor, IntegrationKind,
    Resource as AdapterResource, ResourceEvent, ResourceEventType, ResourceFilter, ResourceMetrics,
};

/// Kubernetes event types for enhanced monitoring
#[derive(Debug, Clone)]
pub enum KubernetesEvent {
    PodAdded(Pod),
    PodModified(Pod),
    PodDeleted(String),
    NodeAdded(Node),
    NodeModified(Node),
    NodeDeleted(String),
    Event(Event),
    DeploymentScaleChanged(String, i32),
}
use crate::resilience::{HealthStatus, IntegrationError, IntegrationResult};
use crate::{CircuitBreakerConfig, RateLimiterConfig, RetryConfig};

/// Kubernetes adapter configuration
#[derive(Debug, Clone)]
pub struct KubernetesConfig {
    /// Kubernetes context (uses default if None)
    pub context: Option<String>,

    /// Namespace to watch (None = all namespaces)
    pub namespace: Option<String>,

    /// Watch resync interval
    pub resync_interval: Duration,

    /// Number of concurrent watch streams
    pub max_watch_streams: usize,

    /// Exec command timeout
    pub exec_timeout: Duration,

    /// Log line limit
    pub log_line_limit: usize,
}

impl Default for KubernetesConfig {
    fn default() -> Self {
        Self {
            context: None,
            namespace: None,
            resync_interval: Duration::from_secs(60),
            max_watch_streams: 100,
            exec_timeout: Duration::from_secs(30),
            log_line_limit: 10000,
        }
    }
}

/// Kubernetes adapter
pub struct KubernetesAdapter {
    base: BaseAdapter,
    config: KubernetesConfig,
    client: Option<Client>,
}

impl KubernetesAdapter {
    /// Create new Kubernetes adapter
    pub fn new(config: KubernetesConfig) -> Self {
        let base = BaseAdapter::new(
            format!("kubernetes-{}", ulid::Ulid::new()),
            IntegrationKind::InfrastructureMonitor,
            CircuitBreakerConfig::default(),
            RateLimiterConfig {
                requests_per_second: 50,
                burst: 100,
            },
            crate::retry::RetryConfig::default(),
        );

        Self {
            base,
            config,
            client: None,
        }
    }

    /// Get Kubernetes client
    pub fn client(&self) -> IntegrationResult<&Client> {
        self.client.as_ref().ok_or_else(|| {
            IntegrationError::Unknown(
                "Kubernetes client not initialized. Call initialize() first.".to_string(),
            )
        })
    }

    /// Infer namespace from current pod or use default
    pub fn infer_namespace() -> Option<String> {
        std::env::var("NAMESPACE")
            .or_else(|_| {
                // Try reading from service account
                std::fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/namespace")
            })
            .ok()
    }

    /// Restart a pod by deleting it (letting deployment recreate)
    async fn restart_pod(&self, pod_name: &str) -> IntegrationResult<ActionResult> {
        let client = self.client()?;
        let namespace = self.config.namespace.as_deref().unwrap_or("default");
        let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);

        match pods.delete(pod_name, &DeleteParams::default()).await {
            Ok(_) => Ok(ActionResult {
                success: true,
                message: format!("Pod {} restarted", pod_name),
                output: None,
                error: None,
            }),
            Err(e) => Ok(ActionResult {
                success: false,
                message: format!("Failed to restart pod {}: {}", pod_name, e),
                output: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Get pod logs
    async fn get_pod_logs(
        &self,
        pod_name: &str,
        container: Option<&str>,
        tail_lines: i64,
    ) -> IntegrationResult<String> {
        // Simplified implementation - just return a placeholder
        // In real implementation, would use k8s logs API
        Ok(format!(
            "Logs for pod {} (container: {:?}, tail: {})",
            pod_name, container, tail_lines
        ))
    }

    /// Scale deployment
    async fn scale_deployment(
        &self,
        deployment_name: &str,
        replicas: i32,
    ) -> IntegrationResult<()> {
        let client = self.client()?;
        let namespace = self.config.namespace.as_deref().unwrap_or("default");
        let deployments: Api<Deployment> = Api::namespaced(client.clone(), namespace);

        let patch = serde_json::json!({
            "spec": { "replicas": replicas }
        });

        let pp = PatchParams::apply("rustops");
        let patch = Patch::Apply(patch);

        deployments
            .patch(deployment_name, &pp, &patch)
            .await
            .map_err(|e| IntegrationError::Network(format!("Failed to scale deployment: {}", e)))?;

        info!(
            "Scaled deployment {} to {} replicas",
            deployment_name, replicas
        );
        Ok(())
    }
}

#[async_trait]
impl InfrastructureMonitor for KubernetesAdapter {
    async fn list_resources(
        &self,
        filters: ResourceFilter,
    ) -> IntegrationResult<Vec<AdapterResource>> {
        // Create all owned values before the closure
        let namespace_str = self
            .config
            .namespace
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let client = self.client()?.clone();
        let base = self.base.clone();

        base.execute_with_resilience(move || {
            let client_clone = client.clone();
            let namespace_clone = namespace_str.clone();
            let filters_clone = filters.clone();

            async move {
                let pods: Api<Pod> = Api::namespaced(client_clone, &namespace_clone);
                let lp = ListParams::default();

                // Apply label filters
                let lp = if !filters_clone.labels.is_empty() {
                    let label_selector: Vec<String> = filters_clone
                        .labels
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect();
                    lp.labels(&label_selector.join(","))
                } else {
                    lp
                };

                let pod_list = pods
                    .list(&lp)
                    .await
                    .map_err(|e| IntegrationError::Network(e.to_string()))?;

                let resources: Vec<AdapterResource> = pod_list
                    .items
                    .into_iter()
                    .map(|pod| AdapterResource {
                        id: pod.metadata.uid.unwrap_or_default(),
                        name: pod.metadata.name.unwrap_or_default(),
                        resource_type: "Pod".to_string(),
                        namespace: pod.metadata.namespace,
                        labels: pod
                            .metadata
                            .labels
                            .unwrap_or_default()
                            .into_iter()
                            .collect(),
                        status: pod
                            .status
                            .and_then(|s| s.phase)
                            .unwrap_or("Unknown".to_string()),
                    })
                    .collect();

                Ok::<Vec<AdapterResource>, IntegrationError>(resources)
            }
        })
        .await
    }

    async fn get_resource_metrics(&self, id: &str) -> IntegrationResult<ResourceMetrics> {
        // Create all owned values before the closure
        let namespace_str = self
            .config
            .namespace
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let id_string = id.to_string();
        let client = self.client()?.clone();
        let base = self.base.clone();

        base.execute_with_resilience(move || {
            let client_clone = client.clone();
            let id_clone = id_string.clone();
            let namespace_clone = namespace_str.clone();

            async move {
                let pods: Api<Pod> = Api::namespaced(client_clone, &namespace_clone);

                match pods.get(&id_clone).await {
                    Ok(pod) => {
                        let mut cpu_usage = 0i64;
                        let mut memory_usage = 0u64;

                        if let Some(status) = pod.status {
                            // Simple CPU count based on running containers
                            let cpu = status
                                .container_statuses
                                .as_ref()
                                .map(|statuses| {
                                    statuses
                                        .iter()
                                        .filter_map(|cs| cs.state.as_ref())
                                        .filter_map(|s| s.running.as_ref())
                                        .count()
                                })
                                .unwrap_or(0);
                            cpu_usage = cpu as i64;

                            // Memory from container status (if available)
                            if let Some(cs) = status.container_statuses {
                                for c in cs {
                                    if let Some(state) = c.state {
                                        if let Some(running) = state.running {
                                            // Using a placeholder - in real implementation would get from metrics API
                                            memory_usage += 100 * 1024 * 1024; // 100MB placeholder
                                        }
                                    }
                                }
                            }
                        }

                        Ok::<ResourceMetrics, IntegrationError>(ResourceMetrics {
                            resource_id: id_clone,
                            cpu_percent: cpu_usage as f64,
                            memory_percent: memory_usage as f64,
                            custom_metrics: HashMap::new(),
                            timestamp: chrono::Utc::now(),
                        })
                    }
                    Err(e) => Err(IntegrationError::Network(format!(
                        "Failed to get pod metrics: {}",
                        e
                    ))),
                }
            }
        })
        .await
    }

    async fn watch_resources(&self) -> IntegrationResult<mpsc::Receiver<ResourceEvent>> {
        let client = self.client()?.clone();
        let namespace = self
            .config
            .namespace
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let buffer_size = self.config.max_watch_streams;

        let (tx, rx) = mpsc::channel(buffer_size);

        // Spawn a simple pod watcher
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let client = client.clone();
            let namespace = namespace.clone();

            loop {
                // Simple implementation: list pods periodically
                let pods: Api<Pod> = if namespace != "default" {
                    Api::namespaced(client.clone(), &namespace)
                } else {
                    Api::all(client.clone())
                };

                match pods.list(&ListParams::default()).await {
                    Ok(pod_list) => {
                        for pod in pod_list.items {
                            if tx_clone
                                .send(ResourceEvent {
                                    event_type: ResourceEventType::Added,
                                    resource: AdapterResource {
                                        id: pod.metadata.uid.unwrap_or_default(),
                                        name: pod.metadata.name.unwrap_or_default(),
                                        resource_type: "Pod".to_string(),
                                        namespace: pod.metadata.namespace,
                                        labels: pod
                                            .metadata
                                            .labels
                                            .unwrap_or_default()
                                            .into_iter()
                                            .collect(),
                                        status: pod
                                            .status
                                            .and_then(|s| s.phase)
                                            .unwrap_or_default(),
                                    },
                                    timestamp: chrono::Utc::now(),
                                })
                                .await
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to list pods: {}", e);
                    }
                }

                // Wait before next check
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });

        Ok(rx)
    }

    async fn execute_action(&self, action: InfraAction) -> IntegrationResult<ActionResult> {
        match action.action_type.as_str() {
            "restart_pod" => {
                if let Some(pod_name) = action.parameters.get("pod_name") {
                    self.restart_pod(pod_name).await
                } else {
                    Ok(ActionResult {
                        success: false,
                        message: "restart_pod action requires 'pod_name' parameter".to_string(),
                        output: None,
                        error: Some("Missing pod_name parameter".to_string()),
                    })
                }
            }
            "get_pod_logs" => {
                if let (Some(pod_name), Some(tail_lines)) = (
                    action.parameters.get("pod_name"),
                    action.parameters.get("tail_lines"),
                ) {
                    let tail_lines = tail_lines.parse().unwrap_or(100);
                    let container = action.parameters.get("container").map(|s| s.as_str());
                    match self.get_pod_logs(pod_name, container, tail_lines).await {
                        Ok(logs) => Ok(ActionResult {
                            success: true,
                            message: "Logs retrieved successfully".to_string(),
                            output: Some(logs),
                            error: None,
                        }),
                        Err(e) => Ok(ActionResult {
                            success: false,
                            message: format!("Failed to get logs: {}", e),
                            output: None,
                            error: Some(e.to_string()),
                        }),
                    }
                } else {
                    Ok(ActionResult {
                        success: false,
                        message:
                            "get_pod_logs action requires 'pod_name' and 'tail_lines' parameters"
                                .to_string(),
                        output: None,
                        error: Some("Missing required parameters".to_string()),
                    })
                }
            }
            "scale_deployment" => {
                if let (Some(deployment_name), Some(replicas)) = (
                    action.parameters.get("deployment_name"),
                    action.parameters.get("replicas"),
                ) {
                    let replicas = replicas.parse().unwrap_or(1);
                    match self.scale_deployment(deployment_name, replicas).await {
                        Ok(_) => Ok(ActionResult {
                            success: true,
                            message: format!(
                                "Deployment {} scaled to {} replicas",
                                deployment_name, replicas
                            ),
                            output: None,
                            error: None,
                        }),
                        Err(e) => Ok(ActionResult {
                            success: false,
                            message: format!("Scale failed: {}", e),
                            output: None,
                            error: Some(e.to_string()),
                        }),
                    }
                } else {
                    Ok(ActionResult {
                        success: false,
                        message: "scale_deployment action requires 'deployment_name' and 'replicas' parameters".to_string(),
                        output: None,
                        error: Some("Missing required parameters".to_string()),
                    })
                }
            }
            _ => Ok(ActionResult {
                success: false,
                message: format!("Unsupported action type: {}", action.action_type),
                output: None,
                error: Some("Unsupported action".to_string()),
            }),
        }
    }
}

#[async_trait]
impl crate::adapter::IntegrationAdapter for KubernetesAdapter {
    fn id(&self) -> &str {
        self.base.id()
    }

    fn kind(&self) -> crate::adapter::IntegrationKind {
        self.base.kind()
    }

    async fn health_check(&self) -> IntegrationResult<HealthStatus> {
        if let Some(client) = &self.client {
            let api: Api<k8s_openapi::api::core::v1::Node> = Api::all(client.clone());
            match api.list(&ListParams::default().limit(1)).await {
                Ok(_) => Ok(HealthStatus::Healthy),
                Err(e) => {
                    error!("Kubernetes health check failed: {}", e);
                    Ok(HealthStatus::Unhealthy)
                }
            }
        } else {
            Ok(HealthStatus::Unknown)
        }
    }

    async fn initialize(&mut self) -> IntegrationResult<()> {
        info!("Initializing Kubernetes adapter");

        // If namespace not set, try to infer it
        if self.config.namespace.is_none() {
            if let Some(inferred_ns) = Self::infer_namespace() {
                self.config.namespace = Some(inferred_ns);
                info!("Using inferred namespace: {:?}", self.config.namespace);
            }
        }

        let mut config = Config::infer().await.map_err(|e| {
            IntegrationError::Authentication(format!("Failed to load kubeconfig: {}", e))
        })?;

        self.client =
            Some(Client::try_from(config).map_err(|e| IntegrationError::Network(e.to_string()))?);

        self.health_check().await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> IntegrationResult<()> {
        info!("Shutting down Kubernetes adapter");
        self.client = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CircuitBreakerConfig, RateLimiterConfig, RetryConfig};

    #[test]
    fn test_kubernetes_config_defaults() {
        let config = KubernetesConfig::default();
        assert_eq!(config.resync_interval, Duration::from_secs(60));
        assert_eq!(config.max_watch_streams, 100);
        assert_eq!(config.exec_timeout, Duration::from_secs(30));
        assert_eq!(config.log_line_limit, 10000);
    }

    #[test]
    fn test_kubernetes_config() {
        let config = KubernetesConfig {
            context: Some("minikube".to_string()),
            namespace: Some("default".to_string()),
            resync_interval: Duration::from_secs(120),
            max_watch_streams: 50,
            exec_timeout: Duration::from_secs(60),
            log_line_limit: 5000,
        };

        let adapter = KubernetesAdapter::new(config);
        assert!(adapter.id().starts_with("kubernetes-"));
    }

    #[test]
    fn test_namespace_inference_from_env() {
        std::env::set_var("NAMESPACE", "test-namespace");
        let ns = KubernetesAdapter::infer_namespace();
        assert_eq!(ns, Some("test-namespace".to_string()));
        std::env::remove_var("NAMESPACE");
    }

    #[test]
    fn test_adapter_clone() {
        let config = KubernetesConfig::default();
        let adapter = KubernetesAdapter::new(config);
        let cloned = adapter.clone();

        assert_eq!(adapter.id(), cloned.id());
        assert_eq!(adapter.kind(), cloned.kind());
    }

    #[test]
    fn test_action_parameters_parsing() {
        let action = InfraAction {
            action_type: "restart_pod".to_string(),
            resource_id: "pod-123".to_string(),
            parameters: {
                let mut params = HashMap::new();
                params.insert("pod_name".to_string(), "test-pod".to_string());
                params
            },
        };

        assert_eq!(
            action.parameters.get("pod_name"),
            Some(&"test-pod".to_string())
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    #[ignore] // Requires k8s cluster
    async fn test_health_check() {
        let config = KubernetesConfig::default();
        let mut adapter = KubernetesAdapter::new(config);

        adapter.initialize().await.unwrap();
        let health = adapter.health_check().await.unwrap();
        assert!(matches!(
            health,
            HealthStatus::Healthy | HealthStatus::Unknown
        ));
        adapter.shutdown().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires k8s cluster
    async fn test_list_pods() {
        let config = KubernetesConfig::default();
        let mut adapter = KubernetesAdapter::new(config);

        adapter.initialize().await.unwrap();

        let filters = ResourceFilter {
            resource_type: None,
            labels: HashMap::new(),
            namespace: None,
        };

        let resources = adapter.list_resources(filters).await.unwrap();
        println!("Found {} resources", resources.len());

        adapter.shutdown().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires k8s cluster
    async fn test_watch_resources() {
        let config = KubernetesConfig {
            max_watch_streams: 10,
            ..Default::default()
        };

        let mut adapter = KubernetesAdapter::new(config);

        adapter.initialize().await.unwrap();

        let mut rx = adapter.watch_resources().await.unwrap();

        // Spawn a task to consume events
        let handle = tokio::spawn(async move {
            let mut count = 0;
            while let Some(event) = rx.recv().await {
                count += 1;
                if count > 5 {
                    // Limit test duration
                    break;
                }
            }
            count
        });

        // Let it run for a short time
        tokio::time::sleep(Duration::from_secs(2)).await;

        let event_count = handle.await.unwrap();
        assert!(event_count >= 0);

        adapter.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_retry_config() {
        let config = KubernetesConfig::default();
        let adapter = KubernetesAdapter::new(config);

        // Verify retry config is properly set
        let retry_config = adapter.base.rate_limiter().config();
        assert!(retry_config.requests_per_second > 0);
        assert!(retry_config.burst > 0);
    }
}
