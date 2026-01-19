//! # Service Dependency Graph
//!
//! Implements a service topology graph following the Neo4j schema defined in ADR-0009.
//! Provides graph operations for dependency queries, impact analysis, and blast radius calculation.

use crate::{
    events::{TopologyEvent, TopologyEventStore},
    model::{DependencyEdge, DependencyType, HealthStatus, ServiceNode, ServiceType},
    // Re-export these types from model for convenience
    // Note: ServiceType, HealthStatus, DependencyType, Protocol are defined in model.rs
};
use petgraph::{
    algo::{astar, dijkstra},
    stable_graph::NodeIndex,
    visit::{Dfs, EdgeRef, Walker},
    Directed, Graph,
};
use rustops_common::{Result, ServiceId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::{debug, info, warn};

/// Service topology graph with service nodes and dependency edges
pub struct ServiceGraph {
    /// Internal graph representation
    graph: Graph<ServiceNode, DependencyEdge, Directed, u32>,
    /// Service ID to node index mapping for quick lookups
    service_index: HashMap<ServiceId, NodeIndex>,
    /// Service name to node index mapping (for services without proper IDs yet)
    name_index: HashMap<String, NodeIndex>,
    /// Event store for topology changes
    event_store: Option<Box<dyn TopologyEventStore>>,
}

impl Clone for ServiceGraph {
    fn clone(&self) -> Self {
        Self {
            graph: self.graph.clone(),
            service_index: self.service_index.clone(),
            name_index: self.name_index.clone(),
            event_store: None, // Cannot clone trait object
        }
    }
}

impl ServiceGraph {
    /// Create a new empty service graph
    pub fn new(event_store: Option<Box<dyn TopologyEventStore>>) -> Self {
        Self {
            graph: Graph::new(),
            service_index: HashMap::new(),
            name_index: HashMap::new(),
            event_store,
        }
    }

    /// Add or update a service node
    pub fn add_service(&mut self, service: ServiceNode) -> Result<()> {
        let node_id = match self.service_index.get(&service.id) {
            Some(index) => {
                // Update existing node
                let mut node = self.graph.node_weight_mut(*index).unwrap();
                *node = service.clone();
                *index
            }
            None => {
                // Add new node
                let index = self.graph.add_node(service.clone());
                self.service_index.insert(service.id, index);
                if let Some(ref name) = service.name {
                    self.name_index.insert(name.clone(), index);
                }
                index
            }
        };

        // Emit topology event
        if let Some(ref store) = self.event_store {
            let event = TopologyEvent::ServiceAdded {
                service_id: service.id,
                service_name: service.name.clone(),
                service_type: service.service_type,
            };
            store.store_event(event)?;
        }

        info!(
            "Added/updated service: {}",
            service.name.as_deref().unwrap_or("<unnamed>")
        );
        Ok(())
    }

    /// Remove a service node
    pub fn remove_service(&mut self, service_id: &ServiceId) -> Result<()> {
        if let Some(&node_index) = self.service_index.get(service_id) {
            // Remove from service index
            self.service_index.remove(service_id);

            // Remove from name index
            if let Some(service) = self.graph.node_weight(node_index) {
                if let Some(ref name) = service.name {
                    self.name_index.remove(name);
                }
            }

            // Remove node and all connected edges
            self.graph.remove_node(node_index);

            // Emit topology event
            if let Some(ref store) = self.event_store {
                let event = TopologyEvent::ServiceRemoved {
                    service_id: *service_id,
                };
                store.store_event(event)?;
            }

            info!("Removed service: {}", service_id);
        }

        Ok(())
    }

    /// Add a dependency edge between services
    pub fn add_dependency(
        &mut self,
        from: ServiceId,
        to: ServiceId,
        edge: DependencyEdge,
    ) -> Result<()> {
        // Ensure both services exist
        let from_index =
            *self
                .service_index
                .get(&from)
                .ok_or_else(|| rustops_common::Error::NotFound {
                    resource: "service".to_string(),
                    identifier: from.to_string(),
                })?;
        let to_index =
            *self
                .service_index
                .get(&to)
                .ok_or_else(|| rustops_common::Error::NotFound {
                    resource: "service".to_string(),
                    identifier: to.to_string(),
                })?;

        // Check if edge already exists
        let edge_exists = {
            let mut found = false;
            // Walk the outgoing edges and check each one
            for edge_ref in self
                .graph
                .edges_directed(from_index, petgraph::Direction::Outgoing)
            {
                if let Some((_, target)) = self.graph.edge_endpoints(edge_ref.id()) {
                    if target == to_index {
                        let edge_weight = edge_ref.weight();
                        if edge_weight.edge_type == edge.edge_type {
                            found = true;
                            break;
                        }
                    }
                }
            }
            found
        };

        if edge_exists {
            warn!("Dependency already exists from {} to {}", from, to);
            return Ok(());
        }

        // Add the edge
        let edge_type = edge.edge_type;
        self.graph.add_edge(from_index, to_index, edge);

        // Emit topology event
        if let Some(ref store) = self.event_store {
            let event = TopologyEvent::DependencyAdded {
                from_service_id: from,
                to_service_id: to,
                edge_type,
            };
            store.store_event(event)?;
        }

        info!("Added dependency from {} to {}", from, to);
        Ok(())
    }

    /// Remove a dependency edge
    pub fn remove_dependency(
        &mut self,
        from: ServiceId,
        to: ServiceId,
        edge_type: DependencyType,
    ) -> Result<()> {
        let from_index =
            *self
                .service_index
                .get(&from)
                .ok_or_else(|| rustops_common::Error::NotFound {
                    resource: "service".to_string(),
                    identifier: from.to_string(),
                })?;
        let to_index =
            *self
                .service_index
                .get(&to)
                .ok_or_else(|| rustops_common::Error::NotFound {
                    resource: "service".to_string(),
                    identifier: to.to_string(),
                })?;

        // Find and remove the edge
        let mut edges_to_remove = Vec::new();
        for edge_ref in self
            .graph
            .edges_directed(from_index, petgraph::Direction::Outgoing)
        {
            if let Some((_, target)) = self.graph.edge_endpoints(edge_ref.id()) {
                if target == to_index {
                    let edge_weight = edge_ref.weight();
                    if edge_weight.edge_type == edge_type {
                        edges_to_remove.push(edge_ref.id());
                    }
                }
            }
        }

        for edge_idx in edges_to_remove {
            self.graph.remove_edge(edge_idx);
        }

        // Emit topology event
        if let Some(ref store) = self.event_store {
            let event = TopologyEvent::DependencyRemoved {
                from_service_id: from,
                to_service_id: to,
                edge_type,
            };
            store.store_event(event)?;
        }

        info!("Removed dependency from {} to {}", from, to);
        Ok(())
    }

    /// Find all services that depend on the given service (upstream dependencies)
    pub fn find_upstream_dependencies(&self, service_id: &ServiceId) -> Result<Vec<ServiceNode>> {
        let start_index =
            *self
                .service_index
                .get(service_id)
                .ok_or_else(|| rustops_common::Error::NotFound {
                    resource: "service".to_string(),
                    identifier: service_id.to_string(),
                })?;

        let mut upstream = Vec::new();
        let mut dfs = Dfs::new(&self.graph, start_index);

        while let Some(node_index) = dfs.next(&self.graph) {
            if node_index != start_index {
                if let Some(node) = self.graph.node_weight(node_index) {
                    upstream.push(node.clone());
                }
            }
        }

        Ok(upstream)
    }

    /// Find all services that the given service depends on (downstream dependencies)
    pub fn find_downstream_dependencies(&self, service_id: &ServiceId) -> Result<Vec<ServiceNode>> {
        let start_index =
            *self
                .service_index
                .get(service_id)
                .ok_or_else(|| rustops_common::Error::NotFound {
                    resource: "service".to_string(),
                    identifier: service_id.to_string(),
                })?;

        let mut downstream = Vec::new();
        let mut dfs = Dfs::new(&self.graph, start_index);

        while let Some(node_index) = dfs.next(&self.graph) {
            if node_index != start_index {
                if let Some(node) = self.graph.node_weight(node_index) {
                    downstream.push(node.clone());
                }
            }
        }

        Ok(downstream)
    }

    /// Calculate blast radius for a service
    pub fn calculate_blast_radius(
        &self,
        service_id: &ServiceId,
        max_hops: usize,
    ) -> Result<BlastRadius> {
        let start_index =
            *self
                .service_index
                .get(service_id)
                .ok_or_else(|| rustops_common::Error::NotFound {
                    resource: "service".to_string(),
                    identifier: service_id.to_string(),
                })?;

        let mut affected_services = HashSet::new();
        let mut total_paths = 0;
        let mut hops_distribution = HashMap::new();

        // BFS to find all services within max_hops
        let mut queue = VecDeque::new();
        queue.push_back((start_index, 0));

        while let Some((node_index, hops)) = queue.pop_front() {
            if hops > max_hops {
                continue;
            }

            if node_index != start_index {
                if let Some(node) = self.graph.node_weight(node_index) {
                    affected_services.insert(node.id);
                    *hops_distribution.entry(hops).or_insert(0) += 1;
                }
            }

            // Add neighbors to queue
            for edge_ref in self
                .graph
                .edges_directed(node_index, petgraph::Direction::Outgoing)
            {
                let next_hops = hops + 1;
                if next_hops <= max_hops {
                    if let Some((_, target)) = self.graph.edge_endpoints(edge_ref.id()) {
                        queue.push_back((target, next_hops));
                    }
                }
            }
        }

        // Calculate blast radius score based on number of affected services and criticality
        let blast_radius_score = affected_services.len() as f64;
        let critical_affected = affected_services
            .iter()
            .filter(|id| self.get_service_criticality(id))
            .count();
        let total_affected = affected_services.len();

        Ok(BlastRadius {
            affected_services: affected_services.into_iter().collect(),
            total_affected_services: total_affected,
            total_affected,
            critical_affected_services: critical_affected,
            hops_distribution,
            score: blast_radius_score,
        })
    }

    /// Find the shortest path between two services
    pub fn find_shortest_path(
        &self,
        from: &ServiceId,
        to: &ServiceId,
    ) -> Result<Option<Vec<ServiceNode>>> {
        let from_index =
            *self
                .service_index
                .get(from)
                .ok_or_else(|| rustops_common::Error::NotFound {
                    resource: "service".to_string(),
                    identifier: from.to_string(),
                })?;
        let to_index =
            *self
                .service_index
                .get(to)
                .ok_or_else(|| rustops_common::Error::NotFound {
                    resource: "service".to_string(),
                    identifier: to.to_string(),
                })?;

        // Use Dijkstra's algorithm to find if path exists
        let result = dijkstra(&self.graph, from_index, Some(to_index), |_| 1);

        // Check if target is reachable
        if result.contains_key(&to_index) {
            // For now, return a simple path with just the two nodes
            // A proper implementation would reconstruct the full path
            let mut service_path = Vec::new();
            if let Some(from_node) = self.graph.node_weight(from_index) {
                service_path.push(from_node.clone());
            }
            if let Some(to_node) = self.graph.node_weight(to_index) {
                service_path.push(to_node.clone());
            }
            Ok(Some(service_path))
        } else {
            Ok(None)
        }
    }

    /// Find all circular dependencies
    pub fn find_circular_dependencies(&self) -> Result<Vec<Vec<ServiceNode>>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut recursion_stack: HashSet<NodeIndex> = HashSet::new();

        // Find all strongly connected components (SCCs)
        for node_index in self.graph.node_indices() {
            if !visited.contains(&node_index) {
                let mut component = Vec::new();
                let mut stack = vec![node_index];

                while let Some(current) = stack.pop() {
                    if visited.insert(current) {
                        component.push(current);

                        for neighbor in self.graph.neighbors(current) {
                            if !visited.contains(&neighbor) {
                                stack.push(neighbor);
                            }
                        }
                    }
                }

                // If component has more than one node, it might contain a cycle
                if component.len() > 1 {
                    // Check if this component forms a cycle
                    let mut has_cycle = false;
                    let mut cycle_nodes = Vec::new();

                    for &node in &component {
                        for neighbor in self.graph.neighbors(node) {
                            if component.contains(&neighbor) {
                                has_cycle = true;
                                cycle_nodes.push(node);
                                break;
                            }
                        }
                    }

                    if has_cycle {
                        let cycle_services = cycle_nodes
                            .iter()
                            .filter_map(|&idx| self.graph.node_weight(idx))
                            .cloned()
                            .collect();
                        cycles.push(cycle_services);
                    }
                }
            }
        }

        Ok(cycles)
    }

    /// Get a service by ID
    pub fn get_service(&self, service_id: &ServiceId) -> Option<&ServiceNode> {
        let node_index = self.service_index.get(service_id)?;
        self.graph.node_weight(*node_index)
    }

    /// Get all services in the graph
    pub fn get_all_services(&self) -> Vec<&ServiceNode> {
        self.graph.node_weights().collect()
    }

    /// Get the number of services in the graph
    pub fn service_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of dependencies in the graph
    pub fn dependency_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get service criticality (for blast radius calculation)
    fn get_service_criticality(&self, service_id: &ServiceId) -> bool {
        if let Some(service) = self.get_service(service_id) {
            // Check service labels for criticality
            let labels = &service.labels;
            if labels.get("criticality").map(|s| s.as_str()) == Some("high") {
                return true;
            }
        }
        false
    }
}

/// Blast radius calculation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadius {
    /// List of affected service IDs
    pub affected_services: Vec<ServiceId>,
    /// Total number of affected services
    pub total_affected_services: usize,
    /// Total number of affected services (alias for compatibility)
    pub total_affected: usize,
    /// Number of critical services affected
    pub critical_affected_services: usize,
    /// Distribution of hops to affected services
    pub hops_distribution: HashMap<usize, usize>,
    /// Blast radius score (higher is worse)
    pub score: f64,
}

// Note: ServiceType, HealthStatus, DependencyType, Protocol are now defined in model.rs
// to avoid duplication. They are re-exported through lib.rs.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ServiceNode;
    use chrono::Utc;
    use rustops_common::ServiceId;

    fn create_test_service(id: ServiceId, name: impl Into<String>) -> ServiceNode {
        let now = Utc::now();
        ServiceNode {
            id,
            name: Some(name.into()),
            namespace: "default".to_string(),
            cluster: "default".to_string(),
            service_type: ServiceType::Deployment,
            replicas: 1,
            labels: HashMap::new(),
            annotations: HashMap::new(),
            health: HealthStatus::Healthy,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_add_service() {
        let mut graph = ServiceGraph::new(None);
        let service_id = ServiceId::new();
        let service = create_test_service(service_id, "test-service");

        graph.add_service(service).unwrap();
        assert_eq!(graph.service_count(), 1);
        assert!(graph.get_service(&service_id).is_some());
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = ServiceGraph::new(None);
        let service_a = ServiceId::new();
        let service_b = ServiceId::new();

        graph
            .add_service(create_test_service(service_a, "service-a"))
            .unwrap();
        graph
            .add_service(create_test_service(service_b, "service-b"))
            .unwrap();

        let edge = DependencyEdge {
            from: service_a,
            to: service_b,
            edge_type: DependencyType::Calls,
            metadata: HashMap::new(),
        };

        graph.add_dependency(service_a, service_b, edge).unwrap();
        assert_eq!(graph.dependency_count(), 1);
    }

    #[test]
    fn test_find_dependencies() {
        let mut graph = ServiceGraph::new(None);
        let service_a = ServiceId::new();
        let service_b = ServiceId::new();
        let service_c = ServiceId::new();

        graph
            .add_service(create_test_service(service_a, "service-a"))
            .unwrap();
        graph
            .add_service(create_test_service(service_b, "service-b"))
            .unwrap();
        graph
            .add_service(create_test_service(service_c, "service-c"))
            .unwrap();

        // A -> B -> C
        graph
            .add_dependency(
                service_a,
                service_b,
                create_dependency_edge(DependencyType::Calls),
            )
            .unwrap();
        graph
            .add_dependency(
                service_b,
                service_c,
                create_dependency_edge(DependencyType::Calls),
            )
            .unwrap();

        // Downstream from A should be B and C
        let downstream = graph.find_downstream_dependencies(&service_a).unwrap();
        assert_eq!(downstream.len(), 2);
        assert!(downstream.iter().any(|s| s.id == service_b));
        assert!(downstream.iter().any(|s| s.id == service_c));

        // Upstream from C should be A and B
        let upstream = graph.find_upstream_dependencies(&service_c).unwrap();
        assert_eq!(upstream.len(), 2);
        assert!(upstream.iter().any(|s| s.id == service_a));
        assert!(upstream.iter().any(|s| s.id == service_b));
    }

    #[test]
    fn test_blast_radius() {
        let mut graph = ServiceGraph::new(None);
        let service_db = ServiceId::new();
        let service_api = ServiceId::new();
        let service_frontend = ServiceId::new();

        graph
            .add_service(create_test_service(service_db, "database"))
            .unwrap();
        graph
            .add_service(create_test_service(service_api, "api"))
            .unwrap();
        graph
            .add_service(create_test_service(service_frontend, "frontend"))
            .unwrap();

        // Frontend -> API -> Database
        graph
            .add_dependency(
                service_frontend,
                service_api,
                create_dependency_edge(DependencyType::Calls),
            )
            .unwrap();
        graph
            .add_dependency(
                service_api,
                service_db,
                create_dependency_edge(DependencyType::Calls),
            )
            .unwrap();

        let radius = graph.calculate_blast_radius(&service_db, 5).unwrap();
        assert_eq!(radius.total_affected_services, 2);
        assert_eq!(radius.critical_affected_services, 0);
        assert!(radius.score > 0.0);
    }

    #[test]
    fn test_shortest_path() {
        let mut graph = ServiceGraph::new(None);
        let service_a = ServiceId::new();
        let service_b = ServiceId::new();
        let service_c = ServiceId::new();

        graph
            .add_service(create_test_service(service_a, "service-a"))
            .unwrap();
        graph
            .add_service(create_test_service(service_b, "service-b"))
            .unwrap();
        graph
            .add_service(create_test_service(service_c, "service-c"))
            .unwrap();

        // A -> B -> C
        graph
            .add_dependency(
                service_a,
                service_b,
                create_dependency_edge(DependencyType::Calls),
            )
            .unwrap();
        graph
            .add_dependency(
                service_b,
                service_c,
                create_dependency_edge(DependencyType::Calls),
            )
            .unwrap();

        let path = graph.find_shortest_path(&service_a, &service_c).unwrap();
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 3); // A -> B -> C
    }

    fn create_dependency_edge(edge_type: DependencyType) -> DependencyEdge {
        DependencyEdge {
            from: ServiceId::new(), // Will be filled in by caller
            to: ServiceId::new(),   // Will be filled in by caller
            edge_type,
            metadata: HashMap::new(),
        }
    }
}
