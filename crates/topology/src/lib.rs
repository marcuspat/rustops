//! # RustOps Service Topology
//!
//! Service topology discovery and management with graph database.

pub mod discovery;
pub mod error;
pub mod graph;
pub mod query;

pub use error::{Error, Result};
pub use graph::{
    EdgeType, GraphDatabase, Neo4jGraph, ServiceEdge, ServiceNode, TopologyDiff,
};
pub use query::{ImpactAnalysis, QueryEngine, TopologyQuery};
pub use discovery::{Discovery, DiscoveryManager, KubernetesDiscovery, PrometheusDiscovery};

/// Topology configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TopologyConfig {
    /// Neo4j connection URI
    pub neo4j_uri: String,
    /// Neo4j username
    pub neo4j_username: String,
    /// Neo4j password
    pub neo4j_password: String,
    /// Discovery interval in seconds
    pub discovery_interval_secs: u64,
    /// Enable real-time updates
    pub enable_realtime_updates: bool,
    /// Topology retention in days
    pub retention_days: u32,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        Self {
            neo4j_uri: "bolt://localhost:7687".to_string(),
            neo4j_username: "neo4j".to_string(),
            neo4j_password: "password".to_string(),
            discovery_interval_secs: 300,
            enable_realtime_updates: true,
            retention_days: 90,
        }
    }
}

/// Service type
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Deployment,
    StatefulSet,
    DaemonSet,
    External,
    Database,
    Cache,
    MessageQueue,
}

/// Service health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Communication protocol
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Http,
    Https,
    Grpc,
    Tcp,
    Udp,
    WebSocket,
}
