# Kubernetes Integration Guide

**Integration Type**: Infrastructure Monitoring
**Priority**: Critical (Phase 1)
**Status**: Design

---

## Overview

Kubernetes is the de facto standard for container orchestration. RustOps integrates deeply with Kubernetes for real-time monitoring of pods, deployments, services, and events, enabling automatic remediation actions within the cluster.

### Integration Capabilities

| Capability | Description | Use Case |
|------------|-------------|----------|
| **Watch API** | Real-time event stream for cluster state | Detect pod failures, scaling events |
| **Pod Metrics** | Resource usage per pod (CPU, memory, network) | Performance analysis, capacity planning |
| **Events** | Kubernetes events (warnings, errors) | Root cause correlation |
| **Logs** | Pod logs streaming | Log aggregation and analysis |
| **Exec Actions** | Execute commands in pods | Debugging, remediation |
| **Scale Actions** | Horizontal pod autoscaling | Auto-remediation, capacity adjustment |
| **Deployment Rollback** | Rollback failed deployments | Self-healing |

### Why Kubernetes is #2 Priority

- **90%+** of containerized deployments use Kubernetes
- **Native watch API** enables real-time event streaming
- **Rich metadata** for service topology mapping
- **Declarative state** for drift detection
- **Control plane access** for autonomous remediation

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    RustOps - Kubernetes Integration                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    Kubernetes Cluster                           │   │
│  │  ┌───────────────────────────────────────────────────────────┐  │   │
│  │  │              Kubernetes API Server                        │  │   │
│  │  │  - Watch API (events streaming)                          │  │   │
│  │  │  - REST API (resource queries)                           │  │   │
│  │  │  - Exec API (pod commands)                               │  │   │
│  │  │  - Logs API (pod logs)                                   │  │   │
│  │  └───────────────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                           │                                             │
│                    Watch│ REST│ Exec│ Logs                             │
│                           ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                  RustOps Kubernetes Adapter                     │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │   │
│  │  │  Watch Listener │  │  Resource Query │  │ Action Runner │  │   │
│  │  │  - Pod events   │  │  - List pods    │  │  - Scale pods │  │   │
│  │  │  - Node events  │  │  - Get metrics  │  │  - Rollback   │  │   │
│  │  │  - Deployment   │  │  - Describe     │  │  - Exec       │  │   │
│  │  └─────────────────┘  └─────────────────┘  └────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                           │                                             │
│                           ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │              RustOps Core Platform                              │   │
│  │  - Event correlation                                           │   │
│  │  - Service topology mapping                                     │   │
│  │  - Remediation orchestration                                   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation

### Rust Dependencies

```toml
[dependencies]
# Kubernetes client
kube = { version = "0.87", features = ["runtime", "client", "ws", "oauth"] }
k8s-openapi = { version = "0.20", features = ["v1_26"] }

# Authentication
hyper = { version = "0.14", features = ["client", "http1", "tcp"] }
tokio = { version = "1.0", features = ["full"] }

# Watch API utilities
futures = "0.3"
tokio-stream = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Kubernetes Client Setup

```rust
use kube::{
    Api, Client, Config,
    api::{ListParams, WatchParams, LogParams, Exec},
    runtime::watcher,
    ResourceExt,
};
use k8s_openapi::api::core::v1::{Pod, Node, Event};
use std::time::Duration;

/// Kubernetes adapter for RustOps
pub struct KubernetesAdapter {
    client: Client,
    namespace: Option<String>,
    config: KubernetesConfig,
}

#[derive(Debug, Clone)]
pub struct KubernetesConfig {
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
            resync_interval: Duration::from_secs(60),
            max_watch_streams: 100,
            exec_timeout: Duration::from_secs(30),
            log_line_limit: 10000,
        }
    }
}

impl KubernetesAdapter {
    /// Create new Kubernetes adapter
    pub async fn new(config: KubernetesConfig) -> Result<Self, KubeError> {
        // Load kube config from environment or file
        let kube_config = Config::infer()
            .await
            .map_err(|e| KubeError::Config(e.to_string()))?;

        let client = Client::try_from(kube_config)
            .map_err(|e| KubeError::Client(e.to_string()))?;

        Ok(Self {
            client,
            namespace: Self::infer_namespace(),
            config,
        })
    }

    /// Infer namespace from current pod or use default
    fn infer_namespace() -> Option<String> {
        std::env::var("NAMESPACE")
            .or_else(|_| {
                // Try reading from service account
                std::fs::read_to_string(
                    "/var/run/secrets/kubernetes.io/serviceaccount/namespace"
                )
            })
            .ok()
    }

    /// Get Kubernetes client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Health check - verify API server connectivity
    pub async fn health_check(&self) -> Result<HealthStatus, KubeError> {
        let pods: Api<Pod> = match &self.namespace {
            Some(ns) => Api::namespaced(self.client.clone(), ns),
            None => Api::all(self.client.clone()),
        };

        // Try to list pods as connectivity check
        let _ = pods
            .list(&ListParams::default().limit(1))
            .await
            .map_err(|e| KubeError::HealthCheck(e.to_string()))?;

        Ok(HealthStatus::Healthy)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KubeError {
    #[error("Config error: {0}")]
    Config(String),

    #[error("Client error: {0}")]
    Client(String),

    #[error("Health check failed: {0}")]
    HealthCheck(String),

    #[error("Watch error: {0}")]
    Watch(String),

    #[error("Exec error: {0}")]
    Exec(String),

    #[error("API error: {0}")]
    Api(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}
```

### Watch API Implementation

```rust
use futures::{StreamExt, TryStreamExt};
use tokio::sync::mpsc;
use std::sync::Arc;

/// Kubernetes event types
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

/// Watch Kubernetes resources for real-time events
impl KubernetesAdapter {
    /// Watch all pod events in configured namespace(s)
    pub async fn watch_pods(
        &self,
        mut tx: mpsc::Sender<KubernetesEvent>,
    ) -> Result<(), KubeError> {
        let pods: Api<Pod> = match &self.namespace {
            Some(ns) => Api::namespaced(self.client.clone(), ns),
            None => Api::all(self.client.clone()),
        };

        let lp = ListParams::default();
        let wp = WatchParams::default();

        tracing::info!("Starting pod watch for namespace: {:?}", self.namespace);

        let stream = pods
            .watch(wp, lp)
            .await
            .map_err(|e| KubeError::Watch(e.to_string()))?;

        tokio::pin!(stream);

        while let Some(event) = stream.try_next().await
            .map_err(|e| KubeError::Watch(e.to_string()))?
        {
            match event {
                kube::api::WatchEvent::Added(pod) => {
                    tracing::debug!("Pod added: {}", pod.name_any());
                    let _ = tx.send(KubernetesEvent::PodAdded(pod)).await;
                }
                kube::api::WatchEvent::Modified(pod) => {
                    tracing::debug!("Pod modified: {}", pod.name_any());
                    let _ = tx.send(KubernetesEvent::PodModified(pod)).await;
                }
                kube::api::WatchEvent::Deleted(pod) => {
                    let name = pod.name_any();
                    tracing::debug!("Pod deleted: {}", name);
                    let _ = tx.send(KubernetesEvent::PodDeleted(name)).await;
                }
                kube::api::WatchEvent::Error(e) => {
                    tracing::error!("Pod watch error: {:?}", e);
                }
            }
        }

        Ok(())
    }

    /// Watch Kubernetes events (warnings, errors)
    pub async fn watch_events(
        &self,
        mut tx: mpsc::Sender<KubernetesEvent>,
    ) -> Result<(), KubeError> {
        let events: Api<Event> = match &self.namespace {
            Some(ns) => Api::namespaced(self.client.clone(), ns),
            None => Api::all(self.client.clone()),
        };

        let lp = ListParams::default();
        let wp = WatchParams::default();

        tracing::info!("Starting event watch for namespace: {:?}", self.namespace);

        let stream = events
            .watch(wp, lp)
            .await
            .map_err(|e| KubeError::Watch(e.to_string()))?;

        tokio::pin!(stream);

        while let Some(event) = stream.try_next().await
            .map_err(|e| KubeError::Watch(e.to_string()))?
        {
            match event {
                kube::api::WatchEvent::Added(event) => {
                    // Filter to warnings and errors
                    if event.type_ == "Warning" || event.type_ == "Error" {
                        tracing::warn!(
                            "K8s event: {} - {}",
                            event.reason.unwrap_or_default(),
                            event.message.unwrap_or_default()
                        );
                        let _ = tx.send(KubernetesEvent::Event(event)).await;
                    }
                }
                kube::api::WatchEvent::Modified(event) => {
                    if event.type_ == "Warning" || event.type_ == "Error" {
                        let _ = tx.send(KubernetesEvent::Event(event)).await;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Watch multiple resources concurrently
    pub async fn watch_all(
        &self,
        buffer_size: usize,
    ) -> Result<mpsc::Receiver<KubernetesEvent>, KubeError> {
        let (tx, rx) = mpsc::channel(buffer_size);

        let adapter = Arc::new(self.clone());

        // Spawn pod watcher
        let tx_clone = tx.clone();
        let adapter_clone = Arc::clone(&adapter);
        tokio::spawn(async move {
            if let Err(e) = adapter_clone.watch_pods(tx_clone).await {
                tracing::error!("Pod watch failed: {}", e);
            }
        });

        // Spawn event watcher
        let tx_clone = tx.clone();
        let adapter_clone = Arc::clone(&adapter);
        tokio::spawn(async move {
            if let Err(e) = adapter_clone.watch_events(tx_clone).await {
                tracing::error!("Event watch failed: {}", e);
            }
        });

        Ok(rx)
    }
}

impl Clone for KubernetesAdapter {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            namespace: self.namespace.clone(),
            config: self.config.clone(),
        }
    }
}
```

### Pod Actions Implementation

```rust
use k8s_openapi::api::core::v1::Pod;
use kube::api::{PostParams, DeleteParams, Patch, PatchParams};

/// Remediation actions on pods
impl KubernetesAdapter {
    /// Restart a pod by deleting it (letting deployment recreate)
    pub async fn restart_pod(&self, pod_name: &str) -> Result<(), KubeError> {
        let pods: Api<Pod> = match &self.namespace {
            Some(ns) => Api::namespaced(self.client.clone(), ns),
            None => Api::all(self.client.clone()),
        };

        pods
            .delete(pod_name, &DeleteParams::default())
            .await
            .map_err(|e| KubeError::Api(format!("Failed to restart pod {}: {}", pod_name, e)))?;

        tracing::info!("Restarted pod: {}", pod_name);
        Ok(())
    }

    /// Execute command in pod
    pub async fn exec_in_pod(
        &self,
        pod_name: &str,
        command: Vec<String>,
    ) -> Result<String, KubeError> {
        let pods: Api<Pod> = match &self.namespace {
            Some(ns) => Api::namespaced(self.client.clone(), ns),
            None => Api::all(self.client.clone()),
        };

        // Get pod to find container
        let pod = pods
            .get(pod_name)
            .await
            .map_err(|e| KubeError::Api(format!("Failed to get pod {}: {}", pod_name, e)))?;

        let container = pod
            .spec
            .and_then(|s| s.containers.first().map(|c| c.name.clone()))
            .ok_or_else(|| KubeError::Api("No container found in pod".into()))?;

        // Execute command
        let exec = Exec::new(pod_name, &container)
            .command(command.clone())
            .stdout(true)
            .stderr(true);

        let mut process = pods
            .exec(exec, &Pod::exec_url)
            .await
            .map_err(|e| KubeError::Exec(format!("Exec failed: {}", e)))?;

        let stdout = process
            .take_stdout()
            .ok_or_else(|| KubeError::Exec("No stdout".into()))?;

        use futures::AsyncReadExt;
        let mut output = String::new();
        stdout
            .read_to_string(&mut output)
            .await
            .map_err(|e| KubeError::Exec(format!("Read failed: {}", e)))?;

        tracing::info!("Executed {:?} in pod {}: {}", command, pod_name, output);
        Ok(output)
    }

    /// Get pod logs
    pub async fn get_pod_logs(
        &self,
        pod_name: &str,
        container: Option<&str>,
        tail_lines: i64,
    ) -> Result<String, KubeError> {
        let pods: Api<Pod> = match &self.namespace {
            Some(ns) => Api::namespaced(self.client.clone(), ns),
            None => Api::all(self.client.clone()),
        };

        let lp = LogParams {
            container: container.map(|s| s.to_string()),
            tail_lines: Some(tail_lines),
            ..Default::default()
        };

        let logs = pods
            .log_stream(pod_name, &lp)
            .await
            .map_err(|e| KubeError::Api(format!("Failed to get logs: {}", e)))?;

        use futures::TryStreamExt;
        let log_lines = logs
            .try_fold(String::new(), |mut acc, line| async move {
                acc.push_str(&String::from_utf8_lossy(&line));
                Ok(acc)
            })
            .await
            .map_err(|e| KubeError::Api(format!("Log stream error: {}", e)))?;

        Ok(log_lines)
    }

    /// Scale deployment
    pub async fn scale_deployment(
        &self,
        deployment_name: &str,
        replicas: i32,
    ) -> Result<(), KubeError> {
        use k8s_openapi::api::apps::v1::{Deployment, Scale};

        let deployments: Api<Deployment> = match &self.namespace {
            Some(ns) => Api::namespaced(self.client.clone(), ns),
            None => Api::all(self.client.clone()),
        };

        let patch = serde_json::json!({
            "spec": { "replicas": replicas }
        });

        let pp = PatchParams::apply("rustops");
        let patch = Patch::Apply(&patch);

        deployments
            .patch(deployment_name, &pp, patch)
            .await
            .map_err(|e| KubeError::Api(format!("Failed to scale deployment: {}", e)))?;

        tracing::info!("Scaled deployment {} to {} replicas", deployment_name, replicas);
        Ok(())
    }

    /// Rollback deployment
    pub async fn rollback_deployment(
        &self,
        deployment_name: &str,
    ) -> Result<(), KubeError> {
        use k8s_openapi::api::apps::v1::{Deployment, RollbackConfig};

        // Get current deployment
        let deployments: Api<Deployment> = match &self.namespace {
            Some(ns) => Api::namespaced(self.client.clone(), ns),
            None => Api::all(self.client.clone()),
        };

        let deployment = deployments
            .get(deployment_name)
            .await
            .map_err(|e| KubeError::Api(format!("Failed to get deployment: {}", e)))?;

        // Get previous revision from annotations
        let prev_revision = deployment
            .annotations
            .and_then(|a| a.get("deployment.kubernetes.io/revision"))
            .and_then(|r| r.parse::<i32>().ok())
            .and_then(|r| if r > 1 { Some(r - 1) } else { None })
            .ok_or_else(|| KubeError::Api("No previous revision found".into()))?;

        // Patch to rollback
        let patch = serde_json::json!({
            "spec": {
                "rollbackTo": {
                    "revision": prev_revision
                }
            }
        });

        let pp = PatchParams::apply("rustops");
        deployments
            .patch(deployment_name, &pp, Patch::Apply(&patch))
            .await
            .map_err(|e| KubeError::Api(format!("Failed to rollback deployment: {}", e)))?;

        tracing::info!("Rolled back deployment {} to revision {}", deployment_name, prev_revision);
        Ok(())
    }
}
```

### Metrics Collection

```rust
use k8s_openapi::api::metrics::v1beta1::{PodMetrics, NodeMetrics};

/// Collect metrics from Kubernetes Metrics API
impl KubernetesAdapter {
    /// Get pod metrics (CPU, memory)
    pub async fn get_pod_metrics(
        &self,
        pod_name: &str,
    ) -> Result<PodResourceMetrics, KubeError> {
        let pod_metrics: Api<PodMetrics> = match &self.namespace {
            Some(ns) => Api::namespaced(self.client.clone(), ns),
            None => Api::all(self.client.clone()),
        };

        let metrics = pod_metrics
            .get(pod_name)
            .await
            .map_err(|e| KubeError::Api(format!("Failed to get pod metrics: {}", e)))?;

        let mut cpu_usage = 0i64;
        let mut memory_usage = 0u64;

        for container in metrics.containers {
            for usage in container.usage {
                match usage.0.as_str() {
                    "cpu" => {
                        cpu_usage += usage.1
                            .as_amount()
                            .and_then(|a| a.0.parse().ok())
                            .unwrap_or(0);
                    }
                    "memory" => {
                        memory_usage += usage.1
                            .as_amount()
                            .and_then(|a| a.0.parse().ok())
                            .unwrap_or(0);
                    }
                    _ => {}
                }
            }
        }

        Ok(PodResourceMetrics {
            pod_name: pod_name.to_string(),
            cpu_cores: cpu_usage as f64 / 1_000_000_000.0, // Nano to cores
            memory_bytes: memory_usage,
        })
    }

    /// Get node metrics
    pub async fn get_node_metrics(
        &self,
        node_name: &str,
    ) -> Result<NodeResourceMetrics, KubeError> {
        let node_metrics: Api<NodeMetrics> = Api::all(self.client.clone());

        let metrics = node_metrics
            .get(node_name)
            .await
            .map_err(|e| KubeError::Api(format!("Failed to get node metrics: {}", e)))?;

        let mut cpu_usage = 0i64;
        let mut memory_usage = 0u64;

        for usage in metrics.usage {
            match usage.0.as_str() {
                "cpu" => {
                    cpu_usage = usage.1
                        .as_amount()
                        .and_then(|a| a.0.parse().ok())
                        .unwrap_or(0);
                }
                "memory" => {
                    memory_usage = usage.1
                        .as_amount()
                        .and_then(|a| a.0.parse().ok())
                        .unwrap_or(0);
                }
                _ => {}
            }
        }

        Ok(NodeResourceMetrics {
            node_name: node_name.to_string(),
            cpu_cores: cpu_usage as f64 / 1_000_000_000.0,
            memory_bytes: memory_usage,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PodResourceMetrics {
    pub pod_name: String,
    pub cpu_cores: f64,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct NodeResourceMetrics {
    pub node_name: String,
    pub cpu_cores: f64,
    pub memory_bytes: u64,
}
```

---

## Configuration

### Kubernetes Configuration

```yaml
integrations:
  kubernetes:
    enabled: true

    # Cluster connection
    cluster:
      # Auto-detect from environment (in-cluster config)
      in_cluster: true
      # Or use kubeconfig file
      kubeconfig_path: "~/.kube/config"
      # Context to use (empty = default)
      context: ""

    # Namespace to watch (empty = all namespaces)
    namespace: "${KUBERNETES_NAMESPACE:-rustops}"

    # Watch configuration
    watch:
      enabled: true
      resync_interval: 60s
      max_streams: 100

    # Resource filters
    resources:
      # Watch pods
      pods:
        enabled: true
        label_selector: "app.kubernetes.io/managed-by=rustops"
      # Watch events
      events:
        enabled: true
        types:
          - Warning
          - Error
      # Watch deployments
      deployments:
        enabled: true
      # Watch nodes
      nodes:
        enabled: true

    # Metrics API
    metrics:
      enabled: true
      poll_interval: 15s

    # Actions (auto-remediation)
    actions:
      enabled: true
      # Maximum number of concurrent actions
      max_concurrent: 10
      # Action timeout
      timeout: 30s
      # Allowed actions
      allowed:
        - restart_pod
        - scale_deployment
        - rollback_deployment
        - exec_command

    # RBAC (required permissions)
    rbac:
      create: true
      rules:
        - apiGroups: [""]
          resources: ["pods", "pods/log", "pods/exec"]
          verbs: ["get", "list", "watch", "delete"]
        - apiGroups: [""]
          resources: ["events"]
          verbs: ["get", "list", "watch"]
        - apiGroups: ["apps"]
          resources: ["deployments", "deployments/scale"]
          verbs: ["get", "list", "watch", "patch", "update"]
        - apiGroups: [""]
          resources: ["nodes"]
          verbs: ["get", "list", "watch"]
        - apiGroups: ["metrics.k8s.io"]
          resources: ["pods", "nodes"]
          verbs: ["get", "list"]
```

### Kubernetes RBAC

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: rustops
  namespace: rustops

---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: rustops
rules:
  # Pods (watch, read, delete)
  - apiGroups: [""]
    resources: ["pods"]
    verbs: ["get", "list", "watch", "delete"]
  - apiGroups: [""]
    resources: ["pods/log"]
    verbs: ["get"]
  - apiGroups: [""]
    resources: ["pods/exec"]
    verbs: ["create"]

  # Events (watch)
  - apiGroups: [""]
    resources: ["events"]
    verbs: ["get", "list", "watch"]

  # Deployments (watch, scale, rollback)
  - apiGroups: ["apps"]
    resources: ["deployments"]
    verbs: ["get", "list", "watch", "patch", "update"]
  - apiGroups: ["apps"]
    resources: ["deployments/scale"]
    verbs: ["get", "patch", "update"]

  # Nodes (watch, read)
  - apiGroups: [""]
    resources: ["nodes"]
    verbs: ["get", "list", "watch"]

  # Metrics API
  - apiGroups: ["metrics.k8s.io"]
    resources: ["pods", "nodes"]
    verbs: ["get", "list"]

  # StatefulSets (watch)
  - apiGroups: ["apps"]
    resources: ["statefulsets"]
    verbs: ["get", "list", "watch"]

  # Namespaces (watch for multi-tenant)
  - apiGroups: [""]
    resources: ["namespaces"]
    verbs: ["get", "list", "watch"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: rustops
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: rustops
subjects:
  - kind: ServiceAccount
    name: rustops
    namespace: rustops
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_inference() {
        std::env::set_var("NAMESPACE", "test-namespace");
        let ns = KubernetesAdapter::infer_namespace();
        assert_eq!(ns, Some("test-namespace".to_string()));
        std::env::remove_var("NAMESPACE");
    }

    #[test]
    fn test_config_defaults() {
        let config = KubernetesConfig::default();
        assert_eq!(config.resync_interval, Duration::from_secs(60));
        assert_eq!(config.max_watch_streams, 100);
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore]  // Requires k8s cluster
    async fn test_health_check() {
        let adapter = KubernetesAdapter::new(KubernetesConfig::default())
            .await
            .unwrap();

        let health = adapter.health_check().await.unwrap();
        assert_eq!(health, HealthStatus::Healthy);
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_pods() {
        let adapter = KubernetesAdapter::new(KubernetesConfig::default())
            .await
            .unwrap();

        let pods: Api<Pod> = Api::all(adapter.client().clone());
        let pod_list = pods.list(&ListParams::default()).await.unwrap();

        println!("Found {} pods", pod_list.items.len());
    }

    #[tokio::test]
    #[ignore]
    async fn test_watch_pods() {
        let adapter = KubernetesAdapter::new(KubernetesConfig::default())
            .await
            .unwrap();

        let (tx, mut rx) = mpsc::channel(100);

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                println!("Received event: {:?}", event);
            }
        });

        let _ = adapter.watch_pods(tx).await;
    }
}
```

### Mock Kubernetes Server

```rust
#[cfg(test)]
pub mod mock_kube {
    use kube::client::Builder;
    use tower_test::mock::Builder as MockBuilder;

    /// Create mock Kubernetes client for testing
    pub async fn mock_client() -> Client {
        // Use tower-test for mock Kubernetes API
        let (mock_service, handle) = MockBuilder::new()
            .axis(|_req| {
                // Mock response
                http::Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(r#"{"kind": "PodList", "items": []}"#)
                    .unwrap()
            })
            .build();

        // Create client with mock service
        // ... implementation depends on kube-rs version
        unimplemented!()
    }
}
```

---

## Deployment

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustops
  namespace: rustops
spec:
  replicas: 1
  selector:
    matchLabels:
      app: rustops
  template:
    metadata:
      labels:
        app: rustops
    spec:
      serviceAccountName: rustops
      containers:
        - name: rustops
          image: rustops:latest
          ports:
            - name: api
              containerPort: 8080
            - name: metrics
              containerPort: 9090
          env:
            - name: RUSTOPS_KUBERNETES_ENABLED
              value: "true"
            - name: RUSTOPS_KUBERNETES_IN_CLUSTER
              value: "true"
            - name: RUSTOPS_KUBERNETES_NAMESPACE
              valueFrom:
                fieldRef:
                  fieldPath: metadata.namespace
          readinessProbe:
            httpGet:
              path: /health
              port: 8080
          livenessProbe:
            httpGet:
              path: /health
              port: 8080
          resources:
            requests:
              cpu: 100m
              memory: 128Mi
            limits:
              cpu: 500m
              memory: 512Mi
```

---

## Monitoring & Troubleshooting

### Key Metrics

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `rustops_k8s_watch_errors_total` | Watch API errors | > 0 in 5m |
| `rustops_k8s_api_latency_seconds` | API call latency | p99 > 5s |
| `rustops_k8s_action_success_rate` | Action success rate | < 95% |
| `rustops_k8s_pods_not_ready` | Not-ready pod count | > 0 for 10m |

### Common Issues

| Issue | Symptom | Solution |
|-------|---------|----------|
| **RBAC denied** | 403 errors | Create ClusterRoleBinding |
| **Watch timeout** | Watch reconnects | Check network, increase timeout |
| **Metrics API not found** | 404 on metrics | Enable metrics-server |
| **High memory** | OOMKilled | Reduce watch streams, add limits |

---

## References

- [kube-rs Documentation](https://kube.rs/)
- [Kubernetes API Reference](https://kubernetes.io/docs/reference/kubernetes-api/)
- [Kubernetes Watch API](https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes)
- [Kubernetes Metrics API](https://kubernetes.io/docs/tasks/debug/debug-cluster/resource-usage-metrics-pipeline/)

---

**Version**: 1.0
**Last Updated**: 2026-01-18
**Integration Phase**: Phase 1 (Foundation)
