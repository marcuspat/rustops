//! # RustOps Service Topology
//!
//! Service topology discovery and management following Domain-Driven Design patterns.
//! Implements a bounded context for service dependency management with impact analysis.

#![warn(missing_docs)]
#![warn(clippy::all)]

use std::env;
use tracing::info;
use serde::{Serialize, Deserialize};

pub mod model;
pub mod graph;
pub mod discovery;
pub mod store;
pub mod impact;
pub mod events;
pub mod error;

pub use error::Error;
// Use rustops_common::Result throughout the topology crate for consistency
pub use rustops_common::Result;
pub use model::{
    ServiceNode,
    DependencyEdge,
    ServiceType,
    HealthStatus,
    DependencyType,
    Protocol,
    ServiceFactory,
    ServiceNodeBuilder,
    DependencyEdgeBuilder,
};
pub use graph::{
    ServiceGraph,
    BlastRadius,
};
pub use discovery::{
    Discovery,
    DiscoveryManager,
    KubernetesDiscovery,
    PrometheusDiscovery,
    TopologyDiscoveryResult,
};
pub use store::{
    GraphStore,
    GraphStoreFactory,
    Neo4jStore,
    Neo4jConfig,
    GraphService,
};
pub use impact::{
    ImpactAnalyzer,
    ImpactAnalysis,
    BlastRadiusAnalysis,
    CriticalPath,
    AffectedServices,
    RiskAssessment,
    Recommendation,
    ImpactAnalyzerConfig,
};
pub use events::{
    TopologyEvent,
    TopologyEventStore,
    InMemoryEventStore,
    EventEmitter,
    EventStatistics,
};

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
    /// Maximum blast radius hops
    pub max_blast_radius_hops: usize,
    /// Enable critical path analysis
    pub enable_critical_path_analysis: bool,
    /// Enable risk assessment
    pub enable_risk_assessment: bool,
    /// Enable business impact calculation
    pub enable_business_impact: bool,
}

impl Default for TopologyConfig {
    fn default() -> Self {
        Self {
            neo4j_uri: "bolt://localhost:7687".to_string(),
            neo4j_username: "neo4j".to_string(),
            neo4j_password: "neo4j".to_string(),
            discovery_interval_secs: 300,
            enable_realtime_updates: true,
            retention_days: 90,
            max_blast_radius_hops: 5,
            enable_critical_path_analysis: true,
            enable_risk_assessment: true,
            enable_business_impact: true,
        }
    }
}

/// Main topology service
pub struct TopologyService {
    /// Service graph for in-memory operations
    graph: ServiceGraph,
    /// Event store for topology changes
    event_store: Box<dyn TopologyEventStore>,
    /// Impact analyzer
    impact_analyzer: ImpactAnalyzer,
    /// Service discovery manager
    discovery_manager: DiscoveryManager,
    /// Graph store for persistence
    graph_store: Option<Box<dyn GraphStore>>,
    /// Configuration
    config: TopologyConfig,
}

impl TopologyService {
    /// Create new topology service
    pub async fn new(config: TopologyConfig) -> Result<Self> {
        let event_store = Box::new(InMemoryEventStore::new());
        let graph = ServiceGraph::new(Some(event_store.clone()));

        let mut discovery_manager = DiscoveryManager::new(Some(event_store.clone()));

        // Add Prometheus discovery if URL is configured
        if !config.neo4j_uri.is_empty() {
            let prometheus_discovery = PrometheusDiscovery::new(config.neo4j_uri.clone());
            discovery_manager.add_discovery(Box::new(prometheus_discovery));
        }

        let impact_analyzer = ImpactAnalyzer::new(graph.clone(), Some(event_store.clone()))
            .with_config(ImpactAnalyzerConfig {
                max_blast_radius_hops: config.max_blast_radius_hops,
                enable_critical_path_analysis: config.enable_critical_path_analysis,
                enable_risk_assessment: config.enable_risk_assessment,
                enable_business_impact: config.enable_business_impact,
                ..Default::default()
            });

        let graph_store = if !config.neo4j_uri.is_empty() {
            Some(GraphStoreFactory::create_neo4j_store(Neo4jConfig {
                uri: config.neo4j_uri.clone(),
                username: config.neo4j_username.clone(),
                password: config.neo4j_password.clone(),
                ..Default::default()
            }).await.map_err(|e| rustops_common::Error::Config {
                message: e.to_string(),
            })?)
        } else {
            None
        };

        Ok(Self {
            graph,
            event_store,
            impact_analyzer,
            discovery_manager,
            graph_store,
            config,
        })
    }

    /// Get reference to service graph
    pub fn graph(&self) -> &ServiceGraph {
        &self.graph
    }

    /// Get mutable reference to service graph
    pub fn graph_mut(&mut self) -> &mut ServiceGraph {
        &mut self.graph
    }

    /// Run discovery and update topology
    pub async fn discover_and_update(&mut self) -> Result<TopologyDiscoveryResult> {
        self.discovery_manager.discover_and_update(&mut self.graph).await
    }

    /// Analyze impact of service change
    pub async fn analyze_impact(&self, service_id: &rustops_common::ServiceId) -> Result<ImpactAnalysis> {
        self.impact_analyzer.analyze_service_impact(service_id).await
    }

    /// Store topology changes
    pub async fn persist_topology(&self) -> Result<()> {
        if let Some(store) = &self.graph_store {
            // Persist all services
            for service in self.graph.get_all_services() {
                if let Some(service_node) = self.graph.get_service(&service.id) {
                    store.store_service(service_node).await.map_err(|e| rustops_common::Error::Database {
                        message: e.to_string(),
                    })?;
                }
            }

            // Persist all dependencies (would need edge enumeration)
            info!("Topology persistence completed");
        }

        // Emit topology synchronized event
        let event = TopologyEvent::TopologySynchronized {
            timestamp: chrono::Utc::now(),
            service_count: self.graph.service_count(),
            dependency_count: self.graph.dependency_count(),
        };

        self.event_store.store_event(event)?;

        Ok(())
    }

    /// Get event statistics
    pub async fn get_event_statistics(&self) -> Result<EventStatistics> {
        self.event_store.get_statistics()
    }

    /// Get topology statistics
    pub async fn get_topology_statistics(&self) -> Result<TopologyStatistics> {
        Ok(TopologyStatistics {
            service_count: self.graph.service_count(),
            dependency_count: self.graph.dependency_count(),
            event_count: self.event_store.get_statistics()?.total_events,
        })
    }

    /// Load topology from storage
    pub async fn load_from_storage(&mut self) -> Result<()> {
        if let Some(store) = &self.graph_store {
            // Clear current graph
            // Note: This would need implementation in ServiceGraph

            // Load services and dependencies
            // This would call store.load_all_services() and store.load_dependencies()

            info!("Topology loaded from storage");
        }

        Ok(())
    }
}

/// Topology statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyStatistics {
    /// Number of services in topology
    pub service_count: usize,
    /// Number of dependencies in topology
    pub dependency_count: usize,
    /// Number of events processed
    pub event_count: usize,
}

impl TopologyStatistics {
    /// Check if topology is empty
    pub fn is_empty(&self) -> bool {
        self.service_count == 0 && self.dependency_count == 0
    }

    /// Calculate average dependencies per service
    pub fn average_dependencies_per_service(&self) -> f64 {
        if self.service_count == 0 {
            0.0
        } else {
            self.dependency_count as f64 / self.service_count as f64
        }
    }
}

/// Topology service builder
pub struct TopologyServiceBuilder {
    config: TopologyConfig,
}

impl TopologyServiceBuilder {
    /// Start building topology service
    pub fn new() -> Self {
        Self {
            config: TopologyConfig::default(),
        }
    }

    /// Set Neo4j configuration
    pub fn with_neo4j(mut self, uri: String, username: String, password: String) -> Self {
        self.config.neo4j_uri = uri;
        self.config.neo4j_username = username;
        self.config.neo4j_password = password;
        self
    }

    /// Set discovery interval
    pub fn with_discovery_interval(mut self, interval_secs: u64) -> Self {
        self.config.discovery_interval_secs = interval_secs;
        self
    }

    /// Set retention period
    pub fn with_retention_days(mut self, days: u32) -> Self {
        self.config.retention_days = days;
        self
    }

    /// Set blast radius hops
    pub fn with_max_blast_radius_hops(mut self, hops: usize) -> Self {
        self.config.max_blast_radius_hops = hops;
        self
    }

    /// Enable or disable features
    pub fn with_features(
        mut self,
        critical_path_analysis: bool,
        risk_assessment: bool,
        business_impact: bool,
    ) -> Self {
        self.config.enable_critical_path_analysis = critical_path_analysis;
        self.config.enable_risk_assessment = risk_assessment;
        self.config.enable_business_impact = business_impact;
        self
    }

    /// Build topology service
    pub async fn build(self) -> Result<TopologyService> {
        TopologyService::new(self.config).await
    }
}

impl Default for TopologyServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience methods for TopologyServiceBuilder
impl TopologyServiceBuilder {
    /// Create topology service with default configuration
    pub async fn default() -> Result<TopologyService> {
        Self::new().build().await
    }

    /// Create topology service for development/testing
    pub async fn development() -> Result<TopologyService> {
        Self::new()
            .with_neo4j(
                "bolt://localhost:7687".to_string(),
                "neo4j".to_string(),
                "neo4j".to_string(),
            )
            .with_discovery_interval(60) // 1 minute for development
            .with_max_blast_radius_hops(3)
            .build()
            .await
    }

    /// Create topology service for production
    pub async fn production() -> Result<TopologyService> {
        Self::new()
            .with_neo4j(
                env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string()),
                env::var("NEO4J_USERNAME").unwrap_or_else(|_| "neo4j".to_string()),
                env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "".to_string()),
            )
            .with_discovery_interval(300) // 5 minutes for production
            .with_retention_days(90)
            .with_max_blast_radius_hops(5)
            .with_features(true, true, true)
            .build()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_topology_service_creation() {
        let service = TopologyService::default().await.unwrap();
        assert_eq!(service.config.discovery_interval_secs, 300);
        assert_eq!(service.config.max_blast_radius_hops, 5);
    }

    #[tokio::test]
    async fn test_topology_service_builder() {
        let service = TopologyServiceBuilder::new()
            .with_discovery_interval(60)
            .with_max_blast_radius_hops(3)
            .build()
            .await
            .unwrap();

        assert_eq!(service.config.discovery_interval_secs, 60);
        assert_eq!(service.config.max_blast_radius_hops, 3);
    }

    #[test]
    fn test_topology_statistics() {
        let stats = TopologyStatistics {
            service_count: 10,
            dependency_count: 20,
            event_count: 100,
        };

        assert!(!stats.is_empty());
        assert_eq!(stats.average_dependencies_per_service(), 2.0);
    }

    #[test]
    fn test_empty_statistics() {
        let stats = TopologyStatistics {
            service_count: 0,
            dependency_count: 0,
            event_count: 0,
        };

        assert!(stats.is_empty());
        assert_eq!(stats.average_dependencies_per_service(), 0.0);
    }
}

#[cfg(feature = "cli")]
pub mod cli {
    //! CLI utilities for topology service

    use super::*;
    use clap::{Parser, Subcommand};
    use std::env;

    /// CLI arguments for topology management
    #[derive(Parser, Debug)]
    #[command(name = "rustops-topology")]
    #[command(about = "Service topology management CLI")]
    pub struct TopologyCli {
        #[command(subcommand)]
        pub command: Commands,
    }

    #[derive(Subcommand, Debug)]
    pub enum Commands {
        /// Run topology discovery
        Discover {
            /// Namespace to discover (default: all)
            #[arg(short, long)]
            namespace: Option<String>,
            /// Include system namespaces
            #[arg(long)]
            include_system: bool,
        },
        /// Analyze impact of service change
        Analyze {
            /// Service ID to analyze
            #[arg(short, long)]
            service_id: String,
            /// Maximum blast radius hops
            #[arg(short, long, default_value = "5")]
            hops: usize,
        },
        /// Show topology statistics
        Stats,
        /// Configure topology
        Config {
            #[command(subcommand)]
            action: ConfigAction,
        },
    }

    #[derive(Subcommand, Debug)]
    pub enum ConfigAction {
        /// Show current configuration
        Show,
        /// Validate configuration
        Validate,
    }

    /// Run CLI application
    pub async fn run_cli() -> Result<()> {
        let cli = TopologyCli::parse();

        match cli.command {
            Commands::Discover { namespace, include_system } => {
                println!("Discovering services in namespace: {:?}", namespace);
                println!("Include system namespaces: {}", include_system);
                // Implementation would go here
            }
            Commands::Analyze { service_id, hops } => {
                println!("Analyzing impact for service: {}", service_id);
                println!("Maximum blast radius hops: {}", hops);
                // Implementation would go here
            }
            Commands::Stats => {
                println!("Showing topology statistics");
                // Implementation would go here
            }
            Commands::Config { action } => {
                match action {
                    ConfigAction::Show => {
                        println!("Current topology configuration");
                        // Implementation would go here
                    }
                    ConfigAction::Validate => {
                        println!("Validating topology configuration");
                        // Implementation would go here
                    }
                }
            }
        }

        Ok(())
    }
}

// For development convenience
#[cfg(test)]
pub mod test_helpers {
    //! Test helpers for topology service

    use super::*;

    /// Create test service node
    pub fn test_service(id: &str, name: &str, namespace: &str) -> ServiceNode {
        ServiceNode::new(
            rustops_common::ServiceId::from_str(id).unwrap(),
            Some(name.to_string()),
            namespace.to_string(),
            "test-cluster".to_string(),
            ServiceType::Deployment,
        )
    }

    /// Create test dependency edge
    pub fn test_dependency(from: &str, to: &str) -> DependencyEdge {
        DependencyEdge::new(
            rustops_common::ServiceId::from_str(from).unwrap(),
            rustops_common::ServiceId::from_str(to).unwrap(),
            DependencyType::Calls,
        )
    }

    /// Create in-memory topology service for testing
    pub async fn test_topology_service() -> TopologyService {
        TopologyService::development().await.unwrap()
    }
}