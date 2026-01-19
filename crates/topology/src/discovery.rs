//! # Service Discovery
//!
//! Implements automatic discovery of services and dependencies from Kubernetes.
//! Supports multiple discovery strategies including K8s API, service mesh, and metrics.

use crate::{
    model::{ServiceNode, DependencyEdge, ServiceType, HealthStatus, DependencyType, Protocol},
    graph::ServiceGraph,
    events::{TopologyEvent, TopologyEventStore},
};
use rustops_common::{ServiceId, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn, error};
use chrono::Utc;

/// Discovery trait for pluggable discovery strategies
#[async_trait::async_trait]
pub trait Discovery: Send + Sync {
    /// Discover all services
    async fn discover_services(&self) -> Result<Vec<ServiceNode>>;

    /// Discover all dependencies/edges
    async fn discover_dependencies(&self) -> Result<Vec<DependencyEdge>>;

    /// Get the name of this discovery source
    fn name(&self) -> &'static str;

    /// Check if discovery is enabled
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Kubernetes discovery implementation
pub struct KubernetesDiscovery {
    /// Kubernetes client
    client: kube::Client,
    /// Namespace to discover (None for all namespaces)
    namespace: Option<String>,
    /// Include system namespaces
    include_system: bool,
    /// Label selector for services
    label_selector: Option<String>,
}

impl KubernetesDiscovery {
    /// Create new Kubernetes discovery
    pub fn new(client: kube::Client, namespace: Option<String>) -> Self {
        Self {
            client,
            namespace,
            include_system: false,
            label_selector: None,
        }
    }

    /// Set whether to include system namespaces
    pub fn with_system(mut self, include: bool) -> Self {
        self.include_system = include;
        self
    }

    /// Set label selector for filtering
    pub fn with_label_selector(mut self, selector: Option<String>) -> Self {
        self.label_selector = selector;
        self
    }
}

#[async_trait::async_trait]
impl Discovery for KubernetesDiscovery {
    async fn discover_services(&self) -> Result<Vec<ServiceNode>> {
        let mut services = Vec::new();

        // Get deployments
        if let Ok(deployments) = self.discover_deployments().await {
            services.extend(deployments);
        }

        // Get stateful sets
        if let Ok(statefulsets) = self.discover_statefulsets().await {
            services.extend(statefulsets);
        }

        // Get daemon sets
        if let Ok(daemonsets) = self.discover_daemonsets().await {
            services.extend(daemonsets);
        }

        // Get pods for more detailed service information
        if let Ok(pods) = self.discover_pods().await {
            services.extend(pods);
        }

        // Remove duplicates based on service ID
        self.deduplicate_services(services)
    }

    async fn discover_dependencies(&self) -> Result<Vec<DependencyEdge>> {
        let mut edges = Vec::new();

        // Discover from service mesh (if available)
        if let Ok(mesh_edges) = self.discover_from_service_mesh().await {
            edges.extend(mesh_edges);
        }

        // Discover from network policies
        if let Ok(np_edges) = self.discover_from_network_policies().await {
            edges.extend(np_edges);
        }

        // Discover from environment variables and configmaps
        if let Ok(env_edges) = self.discover_from_environment().await {
            edges.extend(env_edges);
        }

        // Discover from Prometheus metrics
        if let Ok(metrics_edges) = self.discover_from_prometheus().await {
            edges.extend(metrics_edges);
        }

        Ok(edges)
    }

    fn name(&self) -> &'static str {
        "kubernetes"
    }

    fn is_enabled(&self) -> bool {
        true
    }
}

impl KubernetesDiscovery {
    /// Discover all deployments
    async fn discover_deployments(&self) -> Result<Vec<ServiceNode>> {
        let api: kube::Api<k8s_openapi::api::apps::v1::Deployment> = match &self.namespace {
            Some(ns) => kube::Api::namespaced(self.client.clone(), ns),
            None => kube::Api::all(self.client.clone()),
        };

        let list_params = if let Some(selector) = &self.label_selector {
            kube::api::ListParams::default().labels(selector)
        } else {
            kube::api::ListParams::default()
        };

        let deployments = api.list(&list_params).await
            .map_err(|e| rustops_common::Error::network(format!("Failed to list deployments: {}", e)))?;

        let mut services = Vec::new();

        for deployment in deployments.items {
            if !self.should_include_namespace(&deployment.metadata.namespace) {
                continue;
            }

            let now = Utc::now();
            let service = ServiceNode {
                id: ServiceId::new(),
                name: Some(deployment.metadata.name.clone().unwrap_or_else(|| "unknown".to_string())),
                namespace: deployment.metadata.namespace.clone().unwrap_or_else(|| "default".to_string()),
                cluster: self.get_cluster_name(),
                service_type: ServiceType::Deployment,
                replicas: deployment.spec.as_ref().and_then(|s| s.replicas).map(|r| r as u32).unwrap_or(0),
                labels: deployment.metadata.labels.clone().unwrap_or_default().into_iter().collect(),
                annotations: deployment.metadata.annotations.clone().unwrap_or_default().into_iter().collect(),
                health: self.get_deployment_health(&deployment),
                created_at: now,
                updated_at: now,
            };

            services.push(service);
        }

        debug!("Discovered {} deployments", services.len());
        Ok(services)
    }

    /// Discover all stateful sets
    async fn discover_statefulsets(&self) -> Result<Vec<ServiceNode>> {
        let api: kube::Api<k8s_openapi::api::apps::v1::StatefulSet> = match &self.namespace {
            Some(ns) => kube::Api::namespaced(self.client.clone(), ns),
            None => kube::Api::all(self.client.clone()),
        };

        let statefulsets = api.list(&kube::api::ListParams::default()).await
            .map_err(|e| rustops_common::Error::network(format!("Failed to list statefulsets: {}", e)))?;

        let mut services = Vec::new();

        for statefulset in statefulsets.items {
            if !self.should_include_namespace(&statefulset.metadata.namespace) {
                continue;
            }

            let now = Utc::now();
            let service = ServiceNode {
                id: ServiceId::new(),
                name: Some(statefulset.metadata.name.clone().unwrap_or_else(|| "unknown".to_string())),
                namespace: statefulset.metadata.namespace.clone().unwrap_or_else(|| "default".to_string()),
                cluster: self.get_cluster_name(),
                service_type: ServiceType::StatefulSet,
                replicas: statefulset.spec.as_ref().and_then(|s| s.replicas).map(|r| r as u32).unwrap_or(0),
                labels: statefulset.metadata.labels.clone().unwrap_or_default().into_iter().collect(),
                annotations: statefulset.metadata.annotations.clone().unwrap_or_default().into_iter().collect(),
                health: self.get_statefulset_health(&statefulset),
                created_at: now,
                updated_at: now,
            };

            services.push(service);
        }

        debug!("Discovered {} statefulsets", services.len());
        Ok(services)
    }

    /// Discover all daemon sets
    async fn discover_daemonsets(&self) -> Result<Vec<ServiceNode>> {
        let api: kube::Api<k8s_openapi::api::apps::v1::DaemonSet> = match &self.namespace {
            Some(ns) => kube::Api::namespaced(self.client.clone(), ns),
            None => kube::Api::all(self.client.clone()),
        };

        let daemonsets = api.list(&kube::api::ListParams::default()).await
            .map_err(|e| rustops_common::Error::network(format!("Failed to list daemonsets: {}", e)))?;

        let mut services = Vec::new();

        for daemonset in daemonsets.items {
            if !self.should_include_namespace(&daemonset.metadata.namespace) {
                continue;
            }

            let now = Utc::now();
            let service = ServiceNode {
                id: ServiceId::new(),
                name: Some(daemonset.metadata.name.clone().unwrap_or_else(|| "unknown".to_string())),
                namespace: daemonset.metadata.namespace.clone().unwrap_or_else(|| "default".to_string()),
                cluster: self.get_cluster_name(),
                service_type: ServiceType::DaemonSet,
                replicas: daemonset.status.as_ref()
                    .and_then(|s| Some(s.number_ready))
                    .map(|r| r as u32)
                    .unwrap_or(0),
                labels: daemonset.metadata.labels.clone().unwrap_or_default().into_iter().collect(),
                annotations: daemonset.metadata.annotations.clone().unwrap_or_default().into_iter().collect(),
                health: self.get_daemonset_health(&daemonset),
                created_at: now,
                updated_at: now,
            };

            services.push(service);
        }

        debug!("Discovered {} daemonsets", services.len());
        Ok(services)
    }

    /// Discover all pods as workload nodes
    async fn discover_pods(&self) -> Result<Vec<ServiceNode>> {
        let api: kube::Api<k8s_openapi::api::core::v1::Pod> = match &self.namespace {
            Some(ns) => kube::Api::namespaced(self.client.clone(), ns),
            None => kube::Api::all(self.client.clone()),
        };

        let pods = api.list(&kube::api::ListParams::default()).await
            .map_err(|e| rustops_common::Error::network(format!("Failed to list pods: {}", e)))?;

        let mut services = Vec::new();

        for pod in pods.items {
            if !self.should_include_namespace(&pod.metadata.namespace) {
                continue;
            }

            // Skip system pods if not including system namespaces
            if !self.include_system && self.is_system_pod(&pod) {
                continue;
            }

            let now = Utc::now();
            let service = ServiceNode {
                id: ServiceId::new(),
                name: Some(pod.metadata.name.clone().unwrap_or_else(|| "unknown".to_string())),
                namespace: pod.metadata.namespace.clone().unwrap_or_else(|| "default".to_string()),
                cluster: self.get_cluster_name(),
                service_type: ServiceType::Deployment, // Treat pods as deployments for now
                replicas: if pod.status.as_ref()
                    .and_then(|s| s.phase.as_ref())
                    .map(|phase| phase == "Running") == Some(true) { 1 } else { 0 },
                labels: pod.metadata.labels.clone().unwrap_or_default().into_iter().collect(),
                annotations: pod.metadata.annotations.clone().unwrap_or_default().into_iter().collect(),
                health: self.get_pod_health(&pod),
                created_at: now,
                updated_at: now,
            };

            services.push(service);
        }

        debug!("Discovered {} pods", services.len());
        Ok(services)
    }

    /// Discover dependencies from service mesh (Istio/Linkerd)
    async fn discover_from_service_mesh(&self) -> Result<Vec<DependencyEdge>> {
        // Try Istio first
        if let Some(istio_edges) = self.discover_istio_dependencies().await? {
            return Ok(istio_edges);
        }

        // Try Linkerd
        if let Some(linkerd_edges) = self.discover_linkerd_dependencies().await? {
            return Ok(linkerd_edges);
        }

        Ok(Vec::new())
    }

    /// Discover Istio dependencies
    async fn discover_istio_dependencies(&self) -> Result<Option<Vec<DependencyEdge>>> {
        // Check if Istio is installed
        let api: kube::Api<k8s_openapi::api::networking::v1::NetworkPolicy> =
            kube::Api::all(self.client.clone());

        match api.list(&kube::api::ListParams::default().labels("app.kubernetes.io/name=istio")).await {
            Ok(_) => {
                // Istio is installed, discover virtual services
                info!("Istio detected, discovering service mesh dependencies");
                self.discover_istio_virtual_services().await
            }
            Err(_) => Ok(None), // Istio not installed
        }
    }

    /// Discover Istio virtual service dependencies
    async fn discover_istio_virtual_services(&self) -> Result<Option<Vec<DependencyEdge>>> {
        // This would query Istio VirtualService and DestinationRule resources
        // For now, return empty implementation
        Ok(Some(Vec::new()))
    }

    /// Discover Linkerd dependencies
    async fn discover_linkerd_dependencies(&self) -> Result<Option<Vec<DependencyEdge>>> {
        // Check if Linkerd is installed
        let api: kube::Api<k8s_openapi::api::apps::v1::Deployment> =
            kube::Api::all(self.client.clone());

        match api.list(&kube::api::ListParams::default().labels("app=linkerd")).await {
            Ok(_) => {
                // Linkerd is installed
                info!("Linkerd detected, discovering service mesh dependencies");
                Ok(Some(Vec::new())) // Placeholder implementation
            }
            Err(_) => Ok(None), // Linkerd not installed
        }
    }

    /// Discover dependencies from network policies
    async fn discover_from_network_policies(&self) -> Result<Vec<DependencyEdge>> {
        let api: kube::Api<k8s_openapi::api::networking::v1::NetworkPolicy> = match &self.namespace {
            Some(ns) => kube::Api::namespaced(self.client.clone(), ns),
            None => kube::Api::all(self.client.clone()),
        };

        let policies = api.list(&kube::api::ListParams::default()).await
            .map_err(|e| rustops_common::Error::network(format!("Failed to list network policies: {}", e)))?;

        let mut edges = Vec::new();

        for policy in policies.items {
            if !self.should_include_namespace(&policy.metadata.namespace) {
                continue;
            }

            // Simplified: just skip detailed network policy parsing for now
            // Full implementation would parse spec.pod_selector, spec.ingress, spec.ports
            debug!("Skipping network policy parsing for: {:?}", policy.metadata.name);
        }

        debug!("Discovered {} network policy dependencies", edges.len());
        Ok(edges)
    }

    /// Discover dependencies from environment variables and configmaps
    async fn discover_from_environment(&self) -> Result<Vec<DependencyEdge>> {
        let api: kube::Api<k8s_openapi::api::core::v1::ConfigMap> = match &self.namespace {
            Some(ns) => kube::Api::namespaced(self.client.clone(), ns),
            None => kube::Api::all(self.client.clone()),
        };

        let configmaps = api.list(&kube::api::ListParams::default()).await
            .map_err(|e| rustops_common::Error::network(format!("Failed to list configmaps: {}", e)))?;

        let mut edges = Vec::new();

        for configmap in configmaps.items {
            if !self.should_include_namespace(&configmap.metadata.namespace) {
                continue;
            }

            // Simplified: just skip detailed configmap parsing for now
            debug!("Skipping configmap parsing for: {:?}", configmap.metadata.name);
        }

        debug!("Discovered {} environment-based dependencies", edges.len());
        Ok(edges)
    }

    /// Discover dependencies from Prometheus metrics
    async fn discover_from_prometheus(&self) -> Result<Vec<DependencyEdge>> {
        // This would query Prometheus for service graph metrics
        // For now, return empty implementation
        Ok(Vec::new())
    }

    /// Helper methods

    /// Check if namespace should be included
    fn should_include_namespace(&self, namespace: &Option<String>) -> bool {
        if let Some(ns) = namespace {
            // Skip system namespaces if not including system
            if !self.include_system && self.is_system_namespace(ns) {
                return false;
            }
        }

        // Check namespace filter
        if let Some(filter) = &self.namespace {
            return namespace.as_ref().map(|ns| ns == filter).unwrap_or(false);
        }

        true
    }

    /// Check if namespace is a system namespace
    fn is_system_namespace(&self, namespace: &str) -> bool {
        matches!(namespace, "kube-system" | "kube-public" | "kube-node-lease" | "istio-system")
    }

    /// Check if pod is a system pod
    fn is_system_pod(&self, pod: &k8s_openapi::api::core::v1::Pod) -> bool {
        if let Some(namespace) = &pod.metadata.namespace {
            self.is_system_namespace(namespace)
        } else {
            false
        }
    }

    /// Get deployment health status
    fn get_deployment_health(&self, deployment: &k8s_openapi::api::apps::v1::Deployment) -> HealthStatus {
        if let Some(status) = &deployment.status {
            if let Some(updated_replicas) = status.updated_replicas {
                if let Some(replicas) = deployment.spec.as_ref().and_then(|s| s.replicas) {
                    if updated_replicas == replicas {
                        return HealthStatus::Healthy;
                    } else {
                        return HealthStatus::Degraded;
                    }
                }
            }
        }
        HealthStatus::Unknown
    }

    /// Get statefulset health status
    fn get_statefulset_health(&self, statefulset: &k8s_openapi::api::apps::v1::StatefulSet) -> HealthStatus {
        if let Some(status) = &statefulset.status {
            if let Some(updated_replicas) = status.updated_replicas {
                if let Some(replicas) = statefulset.spec.as_ref().and_then(|s| s.replicas) {
                    if updated_replicas == replicas {
                        return HealthStatus::Healthy;
                    } else {
                        return HealthStatus::Degraded;
                    }
                }
            }
        }
        HealthStatus::Unknown
    }

    /// Get daemonset health status
    fn get_daemonset_health(&self, daemonset: &k8s_openapi::api::apps::v1::DaemonSet) -> HealthStatus {
        if let Some(status) = &daemonset.status {
            let number_ready = status.number_ready;
            let desired_number_scheduled = status.desired_number_scheduled;
            if number_ready == desired_number_scheduled {
                return HealthStatus::Healthy;
            } else {
                return HealthStatus::Degraded;
            }
        }
        HealthStatus::Unknown
    }

    /// Get pod health status
    fn get_pod_health(&self, pod: &k8s_openapi::api::core::v1::Pod) -> HealthStatus {
        if let Some(status) = &pod.status {
            match status.phase.as_ref().map(|s| s.as_str()) {
                Some("Running") => HealthStatus::Healthy,
                Some("Pending") | Some("Unknown") => HealthStatus::Degraded,
                Some("Failed") => HealthStatus::Unhealthy,
                _ => HealthStatus::Unknown,
            }
        } else {
            HealthStatus::Unknown
        }
    }

    /// Get cluster name
    fn get_cluster_name(&self) -> String {
        // In a real implementation, this would get the cluster name from the kubeconfig
        "default".to_string()
    }

    /// Deduplicate services based on ID
    fn deduplicate_services(&self, services: Vec<ServiceNode>) -> Result<Vec<ServiceNode>> {
        let mut seen = HashSet::new();
        let mut unique_services = Vec::new();

        for service in services {
            if !seen.contains(&service.id) {
                seen.insert(service.id);
                unique_services.push(service);
            }
        }

        Ok(unique_services)
    }

    /// Parse service URL from configmap value
    fn parse_service_url(&self, value: &str) -> Option<String> {
        value.strip_prefix("http://")
            .or(value.strip_prefix("https://"))
            .map(|s| s.split('/').next().unwrap_or("").to_string())
            .filter(|s| !s.is_empty())
    }

    /// Convert service URL to dependency edge
    fn service_url_to_edge(&self, namespace: &str, service_url: &str) -> Option<DependencyEdge> {
        // Parse service URL and convert to service ID
        // This is a simplified implementation
        if let Some((_, service_name)) = service_url.split_once('.') {
            let target_id = ServiceId::new(); // Would be actual service ID
            let source_id = ServiceId::new(); // Would be actual service ID

            Some(DependencyEdge {
                from: source_id,
                to: target_id,
                edge_type: DependencyType::Calls,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("url".to_string(), serde_json::Value::String(service_url.to_string()));
                    meta.insert("protocol".to_string(), serde_json::Value::String("http".to_string()));
                    meta
                },
            })
        } else {
            None
        }
    }

    /// Convert network policy to dependency edge
    fn network_policy_to_edge(
        &self,
        namespace: &str,
        pod_selector: &std::collections::BTreeMap<String, String>,
        ports: &[k8s_openapi::api::networking::v1::NetworkPolicyPort],
    ) -> Option<DependencyEdge> {
        // Simplified implementation - convert network policy to service dependency
        let target_id = ServiceId::new(); // Would be actual service ID
        let source_id = ServiceId::new(); // Would be actual service ID

        Some(DependencyEdge {
            from: source_id,
            to: target_id,
            edge_type: DependencyType::Calls,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("namespace".to_string(), serde_json::Value::String(namespace.to_string()));
                meta.insert("pod_selector".to_string(), serde_json::Value::String(format!("{:?}", pod_selector)));
                meta
            },
        })
    }
}

/// Prometheus discovery implementation
pub struct PrometheusDiscovery {
    /// Prometheus URL
    url: String,
    /// HTTP client
    client: reqwest::Client,
}

impl PrometheusDiscovery {
    /// Create new Prometheus discovery
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl Discovery for PrometheusDiscovery {
    async fn discover_services(&self) -> Result<Vec<ServiceNode>> {
        // Query Prometheus for service discovery
        let query = r#"
            group by (service) (up)
        "#;

        let response = self.query_prometheus(query).await?;

        let mut services = Vec::new();

        for result in response.data.result {
            if let Some(service_name) = result.metric.get("service") {
                let now = Utc::now();
                let service = ServiceNode {
                    id: ServiceId::new(),
                    name: Some(service_name.clone()),
                    namespace: "default".to_string(), // Would extract from metric
                    cluster: "default".to_string(),
                    service_type: ServiceType::External,
                    replicas: 1, // Would get from actual service
                    labels: result.metric.clone(),
                    annotations: HashMap::new(),
                    health: HealthStatus::Healthy,
                    created_at: now,
                    updated_at: now,
                };
                services.push(service);
            }
        }

        debug!("Discovered {} services from Prometheus", services.len());
        Ok(services)
    }

    async fn discover_dependencies(&self) -> Result<Vec<DependencyEdge>> {
        // Query Prometheus for service graph
        let query = r#"
            sum(rate(http_requests_total[5m])) by (source_service, destination_service)
        "#;

        let response = self.query_prometheus(query).await?;

        let mut edges = Vec::new();

        for result in response.data.result {
            if let (Some(source), Some(dest)) = (
                result.metric.get("source_service"),
                result.metric.get("destination_service"),
            ) {
                let edge = DependencyEdge {
                    from: ServiceId::new(), // Would be actual service ID
                    to: ServiceId::new(),   // Would be actual service ID
                    edge_type: DependencyType::Calls,
                    metadata: {
                        let mut meta = HashMap::new();
                        if let Some(values) = &result.value {
                            if values.len() > 1 {
                                if let Some(value) = values.get(1) {
                                    if let Some(num) = value.as_f64() {
                                        meta.insert("rate".to_string(), serde_json::Value::Number(
                                            serde_json::Number::from_f64(num).unwrap_or(serde_json::Number::from(0))
                                        ));
                                    }
                                }
                            }
                        }
                        meta
                    },
                };
                edges.push(edge);
            }
        }

        debug!("Discovered {} dependencies from Prometheus", edges.len());
        Ok(edges)
    }

    fn name(&self) -> &'static str {
        "prometheus"
    }

    fn is_enabled(&self) -> bool {
        !self.url.is_empty()
    }
}

impl PrometheusDiscovery {
    /// Query Prometheus API
    async fn query_prometheus(&self, query: &str) -> Result<PrometheusResponse> {
        let response = self.client
            .post(&format!("{}/api/v1/query", self.url))
            .json(&serde_json::json!({
                "query": query
            }))
            .send()
            .await
            .map_err(|e| rustops_common::Error::network(format!("Failed to query Prometheus: {}", e)))?;

        let response = response.json::<PrometheusResponse>().await
            .map_err(|e| rustops_common::Error::network(format!("Failed to parse Prometheus response: {}", e)))?;

        Ok(response)
    }
}

/// Prometheus API response
#[derive(Debug, Clone, Deserialize)]
struct PrometheusResponse {
    data: PrometheusData,
}

#[derive(Debug, Clone, Deserialize)]
struct PrometheusData {
    result: Vec<PrometheusResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct PrometheusResult {
    metric: HashMap<String, String>,
    value: Option<Vec<serde_json::Value>>,
}

/// Topology discovery manager
pub struct DiscoveryManager {
    /// List of discovery implementations
    discoveries: Vec<Box<dyn Discovery>>,
    /// Event store for topology changes
    event_store: Option<Box<dyn TopologyEventStore>>,
}

impl DiscoveryManager {
    /// Create new discovery manager
    pub fn new(event_store: Option<Box<dyn TopologyEventStore>>) -> Self {
        Self {
            discoveries: Vec::new(),
            event_store,
        }
    }

    /// Add a discovery implementation
    pub fn add_discovery(&mut self, discovery: Box<dyn Discovery>) {
        self.discoveries.push(discovery);
    }

    /// Run all discovery implementations and update the graph
    pub async fn discover_and_update(&self, graph: &mut ServiceGraph) -> Result<TopologyDiscoveryResult> {
        let mut all_services = Vec::new();
        let mut all_edges = Vec::new();
        let mut failed_discoveries = Vec::new();

        for discovery in &self.discoveries {
            if !discovery.is_enabled() {
                debug!("Skipping disabled discovery: {}", discovery.name());
                continue;
            }

            // Discover services
            match discovery.discover_services().await {
                Ok(services) => {
                    info!("Discovery '{}' found {} services", discovery.name(), services.len());
                    all_services.extend(services);
                }
                Err(e) => {
                    error!("Discovery '{}' failed: {}", discovery.name(), e);
                    failed_discoveries.push((discovery.name().to_string(), e.to_string()));
                }
            }

            // Discover dependencies
            match discovery.discover_dependencies().await {
                Ok(edges) => {
                    info!("Discovery '{}' found {} dependencies", discovery.name(), edges.len());
                    all_edges.extend(edges);
                }
                Err(e) => {
                    error!("Discovery '{}' failed to discover dependencies: {}", discovery.name(), e);
                    // Don't fail the entire discovery for dependency errors
                }
            }
        }

        // Apply services to graph
        let mut added_services = 0;
        let mut updated_services = 0;
        let mut removed_services = 0;

        let existing_services: HashSet<_> = graph.get_all_services().into_iter().map(|s| s.id).collect();
        let discovered_service_ids: HashSet<_> = all_services.iter().map(|s| s.id).collect();
        let total_services_discovered = all_services.len();
        let total_edges_discovered = all_edges.len();

        for service in all_services {
            if existing_services.contains(&service.id) {
                graph.add_service(service)?;
                updated_services += 1;
            } else {
                graph.add_service(service)?;
                added_services += 1;
            }
        }

        // Apply edges to graph
        let mut added_edges = 0;
        for edge in all_edges {
            if let Err(e) = graph.add_dependency(edge.from, edge.to, edge) {
                warn!("Failed to add dependency: {}", e);
            } else {
                added_edges += 1;
            }
        }

        // Calculate removed services (services that are no longer discovered)
        for service_id in existing_services {
            if !discovered_service_ids.contains(&service_id) {
                graph.remove_service(&service_id)?;
                removed_services += 1;
            }
        }

        Ok(TopologyDiscoveryResult {
            added_services,
            updated_services,
            removed_services,
            added_edges,
            failed_discoveries,
            total_services_discovered,
            total_edges_discovered,
        })
    }

    /// Run discovery for a specific source
    pub async fn discover_from(&self, source: &str) -> Result<(Vec<ServiceNode>, Vec<DependencyEdge>)> {
        for discovery in &self.discoveries {
            if discovery.name() == source {
                let services = discovery.discover_services().await?;
                let edges = discovery.discover_dependencies().await?;
                return Ok((services, edges));
            }
        }

        Err(rustops_common::Error::not_found("discovery", source))
    }

    /// Get list of available discovery sources
    pub fn available_sources(&self) -> Vec<String> {
        self.discoveries
            .iter()
            .filter(|d| d.is_enabled())
            .map(|d| d.name().to_string())
            .collect()
    }
}

/// Discovery result summary
#[derive(Debug, Clone, Serialize)]
pub struct TopologyDiscoveryResult {
    /// Number of services added
    pub added_services: usize,
    /// Number of services updated
    pub updated_services: usize,
    /// Number of services removed
    pub removed_services: usize,
    /// Number of dependencies added
    pub added_edges: usize,
    /// Failed discoveries with error details
    pub failed_discoveries: Vec<(String, String)>,
    /// Total services discovered
    pub total_services_discovered: usize,
    /// Total edges discovered
    pub total_edges_discovered: usize,
}

impl TopologyDiscoveryResult {
    /// Check if discovery was successful
    pub fn is_successful(&self) -> bool {
        self.failed_discoveries.is_empty()
    }

    /// Get summary message
    pub fn summary(&self) -> String {
        format!(
            "Discovery completed: {} services ({} added, {} updated, {} removed), {} dependencies added, {} failed",
            self.added_services + self.updated_services - self.removed_services,
            self.added_services,
            self.updated_services,
            self.removed_services,
            self.added_edges,
            self.failed_discoveries.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kubernetes_discovery_creation() {
        // This test would require a real Kubernetes cluster
        // For now, we just test the creation
        let config = kube::Config::new().unwrap();
        let client = kube::Client::new(config).unwrap();

        let discovery = KubernetesDiscovery::new(client, Some("default".to_string()));
        assert_eq!(discovery.name(), "kubernetes");
        assert!(discovery.is_enabled());
    }

    #[tokio::test]
    async fn test_prometheus_discovery() {
        let discovery = PrometheusDiscovery::new("http://localhost:9090".to_string());
        assert_eq!(discovery.name(), "prometheus");
        assert!(discovery.is_enabled());
    }

    #[tokio::test]
    async fn test_discovery_manager() {
        let manager = DiscoveryManager::new(None);
        assert!(manager.available_sources().is_empty());
    }

    #[test]
    fn test_topology_discovery_result() {
        let result = TopologyDiscoveryResult {
            added_services: 5,
            updated_services: 3,
            removed_services: 1,
            added_edges: 10,
            failed_discoveries: vec![],
            total_services_discovered: 8,
            total_edges_discovered: 12,
        };

        assert!(result.is_successful());
        assert!(result.summary().contains("7 services"));
        assert!(result.summary().contains("10 dependencies added"));
        assert!(result.summary().contains("0 failed"));
    }
}