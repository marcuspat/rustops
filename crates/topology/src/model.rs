//! # Domain Models
//!
//! Core domain models for service topology following DDD patterns.

use rustops_common::{ServiceId, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tracing::warn;

/// Service node in the topology graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceNode {
    /// Unique service identifier
    pub id: ServiceId,

    /// Human-readable service name
    pub name: Option<String>,

    /// Kubernetes namespace
    pub namespace: String,

    /// Cluster name
    pub cluster: String,

    /// Service type (deployment, statefulset, etc.)
    pub service_type: ServiceType,

    /// Number of replicas
    pub replicas: u32,

    /// Service labels (Kubernetes labels)
    pub labels: HashMap<String, String>,

    /// Service annotations (Kubernetes annotations)
    pub annotations: HashMap<String, String>,

    /// Health status
    pub health: HealthStatus,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl ServiceNode {
    /// Create new service node
    pub fn new(
        id: ServiceId,
        name: Option<String>,
        namespace: String,
        cluster: String,
        service_type: ServiceType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            namespace,
            cluster,
            service_type,
            replicas: 1,
            labels: HashMap::new(),
            annotations: HashMap::new(),
            health: HealthStatus::Unknown,
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate Kubernetes-style ID
    pub fn generate_k8s_id(namespace: &str, name: &str) -> String {
        format!("{}/{}", namespace, name)
    }

    /// Add label to service
    pub fn add_label(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.labels.insert(key.into(), value.into());
        self.updated_at = Utc::now();
    }

    /// Add annotation to service
    pub fn add_annotation(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.annotations.insert(key.into(), value.into());
        self.updated_at = Utc::now();
    }

    /// Set health status
    pub fn set_health(&mut self, health: HealthStatus) {
        self.health = health;
        self.updated_at = Utc::now();
    }

    /// Check if service is critical
    pub fn is_critical(&self) -> bool {
        self.labels.get("criticality").map(|s| s.as_str()) == Some("high")
            || self.labels.get("app").map(|s| s.as_str()) == Some("critical")
    }

    /// Check if service is external
    pub fn is_external(&self) -> bool {
        matches!(self.service_type, ServiceType::External)
    }

    /// Check if service is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.health, HealthStatus::Healthy)
    }

    /// Get service identifier string
    pub fn service_identifier(&self) -> String {
        if let Some(name) = &self.name {
            format!("{}-{}", self.namespace, name)
        } else {
            self.id.to_string()
        }
    }

    /// Validate service node
    pub fn validate(&self) -> Result<()> {
        if self.id.to_string().is_empty() {
            return Err(rustops_common::Error::InvalidInput {
                message: "Service ID cannot be empty".to_string(),
            });
        }

        if self.namespace.is_empty() {
            return Err(rustops_common::Error::InvalidInput {
                message: "Namespace cannot be empty".to_string(),
            });
        }

        Ok(())
    }
}

/// Dependency edge between services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Source service ID
    pub from: ServiceId,

    /// Target service ID
    pub to: ServiceId,

    /// Type of dependency
    pub edge_type: DependencyType,

    /// Additional metadata about the dependency
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DependencyEdge {
    /// Create new dependency edge
    pub fn new(from: ServiceId, to: ServiceId, edge_type: DependencyType) -> Self {
        Self {
            from,
            to,
            edge_type,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to dependency
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl serde::Serialize) -> Result<Self> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| rustops_common::Error::Serialization {
                message: format!("Failed to serialize metadata: {}", e),
            })?;
        self.metadata.insert(key.into(), json_value);
        Ok(self)
    }

    /// Check if this is a call dependency
    pub fn is_call(&self) -> bool {
        matches!(self.edge_type, DependencyType::Calls)
    }

    /// Check if this is a data dependency
    pub fn is_data(&self) -> bool {
        matches!(self.edge_type, DependencyType::Reads | DependencyType::Writes)
    }

    /// Get dependency weight for graph algorithms
    pub fn weight(&self) -> f64 {
        // Default weight based on dependency type
        match self.edge_type {
            DependencyType::Calls => 1.0,
            DependencyType::Reads => 0.8,
            DependencyType::Writes => 1.2,
            DependencyType::DeploysTo => 0.5,
            DependencyType::HostsOn => 0.3,
            DependencyType::FailsOver => 1.5,
        }
    }

    /// Validate dependency edge
    pub fn validate(&self) -> Result<()> {
        if self.from.to_string().is_empty() {
            return Err(rustops_common::Error::InvalidInput {
                message: "From service ID cannot be empty".to_string(),
            });
        }

        if self.to.to_string().is_empty() {
            return Err(rustops_common::Error::InvalidInput {
                message: "To service ID cannot be empty".to_string(),
            });
        }

        if self.from == self.to {
            return Err(rustops_common::Error::InvalidInput {
                message: "Service cannot depend on itself".to_string(),
            });
        }

        Ok(())
    }
}

/// Service type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    /// Kubernetes Deployment
    Deployment,
    /// Kubernetes StatefulSet
    StatefulSet,
    /// Kubernetes DaemonSet
    DaemonSet,
    /// External service (not in Kubernetes)
    External,
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceType::Deployment => write!(f, "deployment"),
            ServiceType::StatefulSet => write!(f, "statefulset"),
            ServiceType::DaemonSet => write!(f, "daemonset"),
            ServiceType::External => write!(f, "external"),
        }
    }
}

impl Default for ServiceType {
    fn default() -> Self {
        ServiceType::Deployment
    }
}

/// Health status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Service is healthy
    Healthy,
    /// Service is degraded but functional
    Degraded,
    /// Service is unhealthy
    Unhealthy,
    /// Health status is unknown
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        HealthStatus::Unknown
    }
}

/// Dependency type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    /// Service calls another service
    Calls,
    /// Service reads from database/cache
    Reads,
    /// Service writes to database
    Writes,
    /// Service deploys to workload
    DeploysTo,
    /// Service hosts on node
    HostsOn,
    /// Service fails over to
    FailsOver,
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyType::Calls => write!(f, "calls"),
            DependencyType::Reads => write!(f, "reads"),
            DependencyType::Writes => write!(f, "writes"),
            DependencyType::DeploysTo => write!(f, "deploys_to"),
            DependencyType::HostsOn => write!(f, "hosts_on"),
            DependencyType::FailsOver => write!(f, "fails_over"),
        }
    }
}

impl Default for DependencyType {
    fn default() -> Self {
        DependencyType::Calls
    }
}

/// Protocol enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    /// HTTP protocol
    Http,
    /// gRPC protocol
    Grpc,
    /// TCP protocol
    Tcp,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Http => write!(f, "http"),
            Protocol::Grpc => write!(f, "grpc"),
            Protocol::Tcp => write!(f, "tcp"),
        }
    }
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::Http
    }
}

/// Topology service factory
pub struct ServiceFactory;

impl ServiceFactory {
    /// Create service from Kubernetes deployment
    pub fn from_deployment(
        name: String,
        namespace: String,
        cluster: String,
        replicas: u32,
        labels: HashMap<String, String>,
    ) -> ServiceNode {
        ServiceNode::new(
            ServiceId::new(),
            Some(name),
            namespace,
            cluster,
            ServiceType::Deployment,
        )
        .with_replicas(replicas)
        .with_labels(labels)
    }

    /// Create service from Kubernetes statefulset
    pub fn from_statefulset(
        name: String,
        namespace: String,
        cluster: String,
        replicas: u32,
        labels: HashMap<String, String>,
    ) -> ServiceNode {
        ServiceNode::new(
            ServiceId::new(),
            Some(name),
            namespace,
            cluster,
            ServiceType::StatefulSet,
        )
        .with_replicas(replicas)
        .with_labels(labels)
    }

    /// Create external service
    pub fn from_external(name: String, cluster: String) -> ServiceNode {
        ServiceNode::new(
            ServiceId::new(),
            Some(name),
            "external".to_string(),
            cluster,
            ServiceType::External,
        )
        .with_replicas(1)
    }
}

/// Service node builder
pub struct ServiceNodeBuilder {
    service: ServiceNode,
}

impl ServiceNodeBuilder {
    /// Start building a new service
    pub fn new() -> Self {
        Self {
            service: ServiceNode {
                id: ServiceId::new(),
                name: None,
                namespace: "default".to_string(),
                cluster: "default".to_string(),
                service_type: ServiceType::Deployment,
                replicas: 1,
                labels: HashMap::new(),
                annotations: HashMap::new(),
                health: HealthStatus::Unknown,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        }
    }

    /// Set service ID
    pub fn with_id(mut self, id: ServiceId) -> Self {
        self.service.id = id;
        self
    }

    /// Set service name
    pub fn with_name(mut self, name: Option<String>) -> Self {
        self.service.name = name;
        self
    }

    /// Set namespace
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.service.namespace = namespace;
        self
    }

    /// Set cluster
    pub fn with_cluster(mut self, cluster: String) -> Self {
        self.service.cluster = cluster;
        self
    }

    /// Set service type
    pub fn with_type(mut self, service_type: ServiceType) -> Self {
        self.service.service_type = service_type;
        self
    }

    /// Set replica count
    pub fn with_replicas(mut self, replicas: u32) -> Self {
        self.service.replicas = replicas;
        self
    }

    /// Add label
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.service.labels.insert(key.into(), value.into());
        self
    }

    /// Add multiple labels
    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.service.labels.extend(labels);
        self
    }

    /// Add annotation
    pub fn with_annotation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.service.annotations.insert(key.into(), value.into());
        self
    }

    /// Set health status
    pub fn with_health(mut self, health: HealthStatus) -> Self {
        self.service.health = health;
        self
    }

    /// Build the service node
    pub fn build(self) -> ServiceNode {
        self.service
    }
}

impl ServiceNode {
    /// Start building a new service
    pub fn builder() -> ServiceNodeBuilder {
        ServiceNodeBuilder::new()
    }

    /// Set replica count (builder pattern)
    pub fn with_replicas(mut self, replicas: u32) -> Self {
        self.replicas = replicas;
        self
    }

    /// Add label (builder pattern)
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Add labels (builder pattern)
    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.labels.extend(labels);
        self
    }

    /// Add annotation (builder pattern)
    pub fn with_annotation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.annotations.insert(key.into(), value.into());
        self
    }
}

/// Dependency edge builder
pub struct DependencyEdgeBuilder {
    dependency: DependencyEdge,
}

impl DependencyEdgeBuilder {
    /// Start building a new dependency
    pub fn new(from: ServiceId, to: ServiceId) -> Self {
        Self {
            dependency: DependencyEdge::new(from, to, DependencyType::Calls),
        }
    }

    /// Set dependency type
    pub fn with_type(mut self, edge_type: DependencyType) -> Self {
        self.dependency.edge_type = edge_type;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl serde::Serialize) -> Result<Self> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| rustops_common::Error::Serialization {
                message: format!("Failed to serialize metadata: {}", e),
            })?;
        self.dependency.metadata.insert(key.into(), json_value);
        Ok(self)
    }

    /// Build the dependency edge
    pub fn build(self) -> DependencyEdge {
        self.dependency
    }
}

impl DependencyEdge {
    /// Start building a new dependency
    pub fn builder(from: ServiceId, to: ServiceId) -> DependencyEdgeBuilder {
        DependencyEdgeBuilder::new(from, to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_node_creation() {
        let service = ServiceNode::new(
            ServiceId::new(),
            Some("test-service".to_string()),
            "default".to_string(),
            "cluster-1".to_string(),
            ServiceType::Deployment,
        );

        assert!(service.name.is_some());
        assert_eq!(service.namespace, "default");
        assert_eq!(service.cluster, "cluster-1");
        assert_eq!(service.replicas, 1);
        assert!(service.labels.is_empty());
    }

    #[test]
    fn test_service_builder() {
        let service = ServiceNode::builder()
            .with_name(Some("built-service".to_string()))
            .with_namespace("production".to_string())
            .with_replicas(3)
            .with_label("app".to_string(), "my-app".to_string())
            .build();

        assert_eq!(service.name, Some("built-service".to_string()));
        assert_eq!(service.namespace, "production");
        assert_eq!(service.replicas, 3);
        assert_eq!(service.labels.get("app"), Some(&"my-app".to_string()));
    }

    #[test]
    fn test_dependency_creation() {
        let from = ServiceId::new();
        let to = ServiceId::new();
        let dependency = DependencyEdge::new(from, to, DependencyType::Calls);

        assert_eq!(dependency.from, from);
        assert_eq!(dependency.to, to);
        assert!(dependency.is_call());
        assert!(!dependency.is_data());
        assert_eq!(dependency.weight(), 1.0);
    }

    #[test]
    fn test_dependency_builder() {
        let from = ServiceId::new();
        let to = ServiceId::new();
        let dependency = DependencyEdge::builder(from, to)
            .with_type(DependencyType::Reads)
            .unwrap()
            .with_metadata("rate".to_string(), 100.0)
            .unwrap()
            .build();

        assert_eq!(dependency.edge_type, DependencyType::Reads);
        assert_eq!(dependency.metadata.get("rate"), Some(&serde_json::Value::Number(serde_json::Number::from_f64(100.0).unwrap())));
        assert!(dependency.is_data());
    }

    #[test]
    fn test_service_validation() {
        let valid_service = ServiceNode::new(
            ServiceId::new(),
            Some("valid".to_string()),
            "ns".to_string(),
            "cluster".to_string(),
            ServiceType::Deployment,
        );

        assert!(valid_service.validate().is_ok());

        let invalid_service = ServiceNode {
            id: ServiceId::new(), // Not empty
            name: None,
            namespace: "".to_string(), // Empty
            cluster: "cluster".to_string(),
            service_type: ServiceType::Deployment,
            replicas: 1,
            labels: HashMap::new(),
            annotations: HashMap::new(),
            health: HealthStatus::Unknown,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(invalid_service.validate().is_err());
    }

    #[test]
    fn test_dependency_validation() {
        let from = ServiceId::new();
        let to = ServiceId::new();
        let valid_dependency = DependencyEdge::new(from, to, DependencyType::Calls);

        assert!(valid_dependency.validate().is_ok());

        let invalid_dependency = DependencyEdge::new(from, ServiceId::new(), DependencyType::Calls);

        assert!(invalid_dependency.validate().is_ok()); // IDs are not empty

        let self_dependency = DependencyEdge::new(from, from, DependencyType::Calls);

        assert!(self_dependency.validate().is_err());
    }

    #[test]
    fn test_service_identifier() {
        let service = ServiceNode {
            id: ServiceId::new(),
            name: Some("my-service".to_string()),
            namespace: "default".to_string(),
            cluster: "cluster".to_string(),
            service_type: ServiceType::Deployment,
            replicas: 1,
            labels: HashMap::new(),
            annotations: HashMap::new(),
            health: HealthStatus::Unknown,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(service.service_identifier(), "default-my-service");
    }

    #[test]
    fn test_service_without_name() {
        let service = ServiceNode {
            id: ServiceId::new(),
            name: None,
            namespace: "default".to_string(),
            cluster: "cluster".to_string(),
            service_type: ServiceType::Deployment,
            replicas: 1,
            labels: HashMap::new(),
            annotations: HashMap::new(),
            health: HealthStatus::Unknown,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(service.service_identifier(), service.id.to_string());
    }
}