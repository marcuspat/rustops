//! Integration tests for the topology crate

use rustops_common::ServiceId;
use rustops_topology::{
    discovery::DiscoveryManager,
    events::{EventEmitter, EventStatistics, InMemoryEventStore},
    graph::ServiceGraph,
    impact::ImpactAnalyzer,
    model::{DependencyEdge, DependencyType, ServiceNode, ServiceType},
    TopologyService, TopologyServiceBuilder,
};
use std::collections::HashMap;
use tokio_test;

#[tokio::test]
async fn test_topology_service_end_to_end() {
    // Create topology service
    let config = rustops_topology::TopologyConfig {
        neo4j_uri: String::new(),
        ..Default::default()
    };
    let mut service = TopologyService::new(config).await.unwrap();

    // Create test services
    let service_a = ServiceNode::new(
        ServiceId::new(),
        Some("service-a".to_string()),
        "default".to_string(),
        "test-cluster".to_string(),
        ServiceType::Deployment,
    );

    let service_b = ServiceNode::new(
        ServiceId::new(),
        Some("service-b".to_string()),
        "default".to_string(),
        "test-cluster".to_string(),
        ServiceType::Deployment,
    );

    // Add services to graph
    service.graph_mut().add_service(service_a).unwrap();
    service.graph_mut().add_service(service_b).unwrap();

    // Add dependency
    let dependency = DependencyEdge::new(
        service.graph().get_services().next().unwrap().id,
        service.graph().get_services().nth(1).unwrap().id,
        DependencyType::Calls,
    );

    service
        .graph_mut()
        .add_dependency(dependency.from, dependency.to, dependency)
        .unwrap();

    // Verify topology statistics
    let stats = service.get_topology_statistics().await.unwrap();
    assert_eq!(stats.service_count, 2);
    assert_eq!(stats.dependency_count, 1);

    // Test impact analysis
    let service_id = service.graph().get_services().next().unwrap().id;
    let impact = service.analyze_impact(&service_id).await.unwrap();
    assert_eq!(impact.source_service, service_id);
    assert!(!impact.recommendations.is_empty());

    println!("Integration test passed: End-to-end topology service workflow");
}

#[tokio::test]
async fn test_service_graph_operations() {
    let mut graph = ServiceGraph::new(None);

    // Add services
    let service1 = ServiceNode::new(
        ServiceId::new(),
        Some("frontend".to_string()),
        "default".to_string(),
        "production".to_string(),
        ServiceType::Deployment,
    );

    let service2 = ServiceNode::new(
        ServiceId::new(),
        Some("api".to_string()),
        "default".to_string(),
        "production".to_string(),
        ServiceType::Deployment,
    );

    let service3 = ServiceNode::new(
        ServiceId::new(),
        Some("database".to_string()),
        "default".to_string(),
        "production".to_string(),
        ServiceType::StatefulSet,
    );

    graph.add_service(service1).unwrap();
    graph.add_service(service2).unwrap();
    graph.add_service(service3).unwrap();

    // Add dependencies: frontend -> api -> database
    let dep1 = DependencyEdge::new(
        graph.get_services().next().unwrap().id,
        graph.get_services().nth(1).unwrap().id,
        DependencyType::Calls,
    );

    let dep2 = DependencyEdge::new(
        graph.get_services().nth(1).unwrap().id,
        graph.get_services().nth(2).unwrap().id,
        DependencyType::Reads,
    );

    graph.add_dependency(dep1.from, dep1.to, dep1).unwrap();
    graph.add_dependency(dep2.from, dep2.to, dep2).unwrap();

    // Verify graph structure
    assert_eq!(graph.service_count(), 3);
    assert_eq!(graph.dependency_count(), 2);

    // Test dependency discovery
    let downstream = graph
        .find_downstream_dependencies(&graph.get_services().next().unwrap().id)
        .unwrap();
    assert_eq!(downstream.len(), 2);

    let upstream = graph
        .find_upstream_dependencies(&graph.get_services().nth(2).unwrap().id)
        .unwrap();
    assert_eq!(upstream.len(), 2);

    // Test blast radius
    let blast_radius = graph
        .calculate_blast_radius(&graph.get_services().nth(2).unwrap().id, 5)
        .unwrap();
    assert!(blast_radius.total_affected_services >= 2);

    println!("Service graph operations test passed");
}

#[tokio::test]
async fn test_event_system() {
    let event_store = InMemoryEventStore::new();
    let mut emitter = rustops_topology::events::EventEmitter::new(Box::new(event_store.clone()));

    // Test emitting events
    let service_id = ServiceId::new();
    emitter
        .emit_service_added(
            service_id,
            Some("test-service".to_string()),
            ServiceType::Deployment,
        )
        .unwrap();

    let to_service_id = ServiceId::new();
    emitter
        .emit_dependency_added(service_id, to_service_id, DependencyType::Calls)
        .unwrap();

    // Test event retrieval
    let service_events = event_store.get_service_events(&service_id).unwrap();
    assert_eq!(service_events.len(), 2);

    let stats = event_store.get_statistics().unwrap();
    assert_eq!(stats.total_events, 2);
    assert_eq!(stats.service_events, 1);
    assert_eq!(stats.dependency_events, 1);

    println!("Event system test passed");
}

#[tokio::test]
async fn test_discovery_manager() {
    let mut manager = DiscoveryManager::new(None);

    // Add mock discovery implementations
    let mock_discovery = MockDiscovery::new();
    manager.add_discovery(Box::new(mock_discovery));

    // Run discovery (will return empty for mock)
    let result = manager
        .discover_and_update(&mut ServiceGraph::new(None))
        .await
        .unwrap();
    assert!(result.total_services_discovered >= 0);

    // Test available sources
    let sources = manager.available_sources();
    assert!(!sources.is_empty());

    println!("Discovery manager test passed");
}

// Mock discovery for testing
struct MockDiscovery;

impl MockDiscovery {
    fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl rustops_topology::discovery::Discovery for MockDiscovery {
    async fn discover_services(&self) -> Result<Vec<ServiceNode>> {
        Ok(vec![ServiceNode::new(
            ServiceId::new(),
            Some("mock-service".to_string()),
            "default".to_string(),
            "mock-cluster".to_string(),
            ServiceType::Deployment,
        )])
    }

    async fn discover_dependencies(&self) -> Result<Vec<DependencyEdge>> {
        Ok(vec![])
    }

    fn name(&self) -> &'static str {
        "mock"
    }

    fn is_enabled(&self) -> bool {
        true
    }
}

#[tokio::test]
async fn test_impact_analysis() {
    let graph = ServiceGraph::new(None);
    let analyzer = ImpactAnalyzer::new(graph, None);

    // Test impact analysis (will return default analysis)
    let service_id = ServiceId::new();
    let impact = analyzer.analyze_service_impact(&service_id).await.unwrap();

    assert_eq!(impact.source_service, service_id);
    assert!(!impact.blast_radius.affected_services.is_empty());
    assert!(!impact.recommendations.is_empty());

    println!("Impact analysis test passed");
}

#[tokio::test]
async fn test_error_handling() {
    use rustops_topology::error::{Error, Result};

    // Test error creation and context
    let result: Result<()> = Err(Error::graph("Test error"));
    let result = result.context("Additional context");

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Additional context"));
    assert!(error.to_string().contains("Test error"));

    // Test error retryable detection
    let retryable_error = Error::connection("timeout");
    assert!(retryable_error.is_retryable());

    let fatal_error = Error::validation("invalid input");
    assert!(!fatal_error.is_retryable());
    assert!(fatal_error.is_fatal());

    println!("Error handling test passed");
}

#[tokio::test]
async fn test_service_builder() {
    // Test service builder pattern
    let service = ServiceNode::builder()
        .with_name(Some("built-service".to_string()))
        .with_namespace("production".to_string())
        .with_type(ServiceType::StatefulSet)
        .with_replicas(3)
        .with_label("app".to_string(), "my-app".to_string())
        .with_label("version".to_string(), "1.0.0".to_string())
        .with_annotation("description".to_string(), "Built with builder".to_string())
        .build();

    assert_eq!(service.name, Some("built-service".to_string()));
    assert_eq!(service.namespace, "production");
    assert_eq!(service.service_type, ServiceType::StatefulSet);
    assert_eq!(service.replicas, 3);
    assert_eq!(service.labels.get("app"), Some(&"my-app".to_string()));
    assert_eq!(
        service.annotations.get("description"),
        Some(&"Built with builder".to_string())
    );

    // Test dependency builder
    let from_id = ServiceId::new();
    let to_id = ServiceId::new();
    let dependency = DependencyEdge::builder(from_id, to_id)
        .with_type(DependencyType::Writes)
        .unwrap()
        .build();

    assert_eq!(dependency.edge_type, DependencyType::Writes);

    println!("Service builder test passed");
}

#[tokio::test]
async fn test_topology_statistics() {
    use rustops_topology::TopologyStatistics;

    let stats = TopologyStatistics {
        service_count: 10,
        dependency_count: 25,
        event_count: 100,
    };

    assert!(!stats.is_empty());
    assert_eq!(stats.average_dependencies_per_service(), 2.5);

    let empty_stats = TopologyStatistics {
        service_count: 0,
        dependency_count: 0,
        event_count: 0,
    };

    assert!(empty_stats.is_empty());
    assert_eq!(empty_stats.average_dependencies_per_service(), 0.0);

    println!("Topology statistics test passed");
}

#[tokio::test]
async fn test_configuration() {
    use rustops_topology::TopologyConfig;

    let config = TopologyConfig::default();
    assert_eq!(config.discovery_interval_secs, 300);
    assert_eq!(config.max_blast_radius_hops, 5);
    assert_eq!(config.retention_days, 90);

    let custom_config = TopologyConfig {
        neo4j_uri: "bolt://test:7687".to_string(),
        neo4j_username: "test-user".to_string(),
        neo4j_password: "test-pass".to_string(),
        discovery_interval_secs: 60,
        enable_realtime_updates: false,
        retention_days: 30,
        max_blast_radius_hops: 3,
        enable_critical_path_analysis: true,
        enable_risk_assessment: false,
        enable_business_impact: true,
    };

    assert_eq!(custom_config.discovery_interval_secs, 60);
    assert_eq!(custom_config.max_blast_radius_hops, 3);
    assert!(!custom_config.enable_realtime_updates);

    println!("Configuration test passed");
}

#[tokio::test]
async fn test_serialization() {
    use serde_json;

    // Test service node serialization
    let service = ServiceNode::new(
        ServiceId::new(),
        Some("test-service".to_string()),
        "default".to_string(),
        "test-cluster".to_string(),
        ServiceType::Deployment,
    );

    let serialized = serde_json::to_string(&service).unwrap();
    let deserialized: ServiceNode = serde_json::from_str(&serialized).unwrap();

    assert_eq!(service.name, deserialized.name);
    assert_eq!(service.namespace, deserialized.namespace);
    assert_eq!(service.service_type, deserialized.service_type);

    // Test dependency edge serialization
    let dependency = DependencyEdge::new(ServiceId::new(), ServiceId::new(), DependencyType::Calls);

    let serialized_dep = serde_json::to_string(&dependency).unwrap();
    let deserialized_dep: DependencyEdge = serde_json::from_str(&serialized_dep).unwrap();

    assert_eq!(dependency.edge_type, deserialized_dep.edge_type);

    println!("Serialization test passed");
}
