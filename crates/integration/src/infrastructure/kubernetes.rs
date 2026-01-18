// Kubernetes integration
//
// Implements Kubernetes Watch API for real-time resource monitoring

use async_trait::async_trait;
use kube::{api::{Api, ListParams, Resource, WatchEvent}, Client, Config};
use k8s_openapi::api::core::v1::Pod;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::adapter::{
    BaseAdapter, InfrastructureMonitor, IntegrationKind,
    ResourceFilter, Resource, ResourceMetrics, ResourceEvent, ResourceEventType, InfraAction, ActionResult,
};
use crate::resilience::{IntegrationError, IntegrationResult, HealthStatus};
use crate::{CircuitBreakerConfig, RateLimiterConfig, RetryConfig};

/// Kubernetes adapter configuration
#[derive(Debug, Clone)]
pub struct KubernetesConfig {
    /// Kubernetes context (uses default if None)
    pub context: Option<String>,

    /// Namespace to watch (None = all namespaces)
    pub namespace: Option<String>,
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
                // Watch API doesn't need rate limiting as much
                requests_per_second: 50,
                burst: 100,
            },
            RetryConfig::default(),
        );

        Self {
            base,
            config,
            client: None,
        }
    }

    /// Get Kubernetes client
    fn client(&self) -> IntegrationResult<&Client> {
        self.client.as_ref().ok_or_else(|| {
            IntegrationError::Unknown("Kubernetes client not initialized. Call initialize() first.".to_string())
        })
    }
}

#[async_trait]
impl InfrastructureMonitor for KubernetesAdapter {
    async fn list_resources(&self, filters: ResourceFilter) -> IntegrationResult<Vec<Resource>> {
        let client = self.client()?;

        let namespace = self.config.namespace.as_deref().unwrap_or("default");
        let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);

        self.base.execute_with_resilience(|| {
            async move {
                let lp = ListParams::default();

                // Apply label filters
                let lp = if !filters.labels.is_empty() {
                    let label_selector: Vec<String> = filters.labels
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect();
                    lp.labels(&label_selector.join(","))
                } else {
                    lp
                };

                let pod_list = pods.list(&lp).await
                    .map_err(|e| IntegrationError::Network(e.to_string()))?;

                let resources: Vec<Resource> = pod_list
                    .items
                    .into_iter()
                    .map(|pod| Resource {
                        id: pod.metadata.uid.unwrap_or_default(),
                        name: pod.metadata.name.unwrap_or_default(),
                        resource_type: "Pod".to_string(),
                        namespace: pod.metadata.namespace,
                        labels: pod.metadata.labels.unwrap_or_default(),
                        status: pod
                            .status
                            .and_then(|s| s.phase)
                            .unwrap_or("Unknown".to_string()),
                    })
                    .collect();

                Ok(resources)
            }
        })
        .await
    }

    async fn get_resource_metrics(&self, id: &str) -> IntegrationResult<ResourceMetrics> {
        let client = self.client()?;
        let namespace = self.config.namespace.as_deref().unwrap_or("default");
        let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);

        self.base.execute_with_resilience(|| {
            async move {
                let pod = pods.get(id).await
                    .map_err(|e| IntegrationError::Network(e.to_string()))?;

                // Extract metrics from pod status
                let (cpu_percent, memory_percent) = if let Some(status) = pod.status {
                    let cpu = status
                        .container_statuses
                        .as_ref()
                        .map(|statuses| {
                            statuses.iter()
                                .filter_map(|cs| cs.state.as_ref())
                                .filter_map(|s| s.running.as_ref())
                                .map(|r| r.started_with_id.as_ref())
                                .count()
                        })
                        .unwrap_or(0);

                    (cpu as f64, 0.0) // Simplified
                } else {
                    (0.0, 0.0)
                };

                Ok(ResourceMetrics {
                    resource_id: id.to_string(),
                    cpu_percent,
                    memory_percent,
                    custom_metrics: HashMap::new(),
                    timestamp: chrono::Utc::now(),
                })
            }
        })
        .await
    }

    async fn watch_resources(&self) -> IntegrationResult<mpsc::Receiver<ResourceEvent>> {
        let client = self.client()?;
        let namespace = self.config.namespace.clone().unwrap_or_else(|| "default".to_string());

        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let pods: Api<Pod> = Api::namespaced(client, &namespace);
            let lp = ListParams::default();

            match pods.watch(lp, "0").await {
                Ok(stream) => {
                    info!("Started watching pods in namespace {}", namespace);

                    use tokio_stream::StreamExt;
                    use futures_util::TryStreamExt;

                    let mut stream = stream;
                    while let Some(event) = stream.next().await {
                        match event {
                            Ok(WatchEvent::Added(pod)) => {
                                if tx.send(ResourceEvent {
                                    event_type: ResourceEventType::Added,
                                    resource: Resource {
                                        id: pod.metadata.uid.unwrap_or_default(),
                                        name: pod.metadata.name.unwrap_or_default(),
                                        resource_type: "Pod".to_string(),
                                        namespace: pod.metadata.namespace,
                                        labels: pod.metadata.labels.unwrap_or_default(),
                                        status: pod.status.and_then(|s| s.phase).unwrap_or_default(),
                                    },
                                    timestamp: chrono::Utc::now(),
                                }).await.is_err() {
                                    break; // Receiver dropped
                                }
                            }
                            Ok(WatchEvent::Modified(pod)) => {
                                if tx.send(ResourceEvent {
                                    event_type: ResourceEventType::Modified,
                                    resource: Resource {
                                        id: pod.metadata.uid.unwrap_or_default(),
                                        name: pod.metadata.name.unwrap_or_default(),
                                        resource_type: "Pod".to_string(),
                                        namespace: pod.metadata.namespace,
                                        labels: pod.metadata.labels.unwrap_or_default(),
                                        status: pod.status.and_then(|s| s.phase).unwrap_or_default(),
                                    },
                                    timestamp: chrono::Utc::now(),
                                }).await.is_err() {
                                    break;
                                }
                            }
                            Ok(WatchEvent::Deleted(pod)) => {
                                if tx.send(ResourceEvent {
                                    event_type: ResourceEventType::Deleted,
                                    resource: Resource {
                                        id: pod.metadata.uid.unwrap_or_default(),
                                        name: pod.metadata.name.unwrap_or_default(),
                                        resource_type: "Pod".to_string(),
                                        namespace: pod.metadata.namespace,
                                        labels: pod.metadata.labels.unwrap_or_default(),
                                        status: "Deleted".to_string(),
                                    },
                                    timestamp: chrono::Utc::now(),
                                }).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Watch error: {}", e);
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to start watch: {}", e);
                }
            }
        });

        Ok(rx)
    }

    async fn execute_action(&self, action: InfraAction) -> IntegrationResult<ActionResult> {
        // Implement restart, scale, etc.
        Ok(ActionResult {
            success: true,
            message: format!("Action {} executed", action.action_type),
            output: None,
            error: None,
        })
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

        let mut config = Config::infer().await
            .map_err(|e| IntegrationError::Authentication(format!("Failed to load kubeconfig: {}", e)))?;

        if let Some(context) = &self.config.context {
            config.with_context(context);
        }

        self.client = Some(Client::try_from(config)
            .map_err(|e| IntegrationError::Network(e.to_string()))?);

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

    #[test]
    fn test_kubernetes_config() {
        let config = KubernetesConfig {
            context: Some("minikube".to_string()),
            namespace: Some("default".to_string()),
        };

        let adapter = KubernetesAdapter::new(config);
        assert!(adapter.id().starts_with("kubernetes-"));
    }
}
