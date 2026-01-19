//! # Alert correlation
//!
//! Groups related alerts into incidents using topological analysis.

use crate::deduplication::NormalizedAlert;
use chrono::{DateTime, Utc};
use petgraph::graph::{DiGraph, NodeIndex};
use rustops_common::{AlertId, IncidentId, ServiceId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Configuration for alert correlation
#[derive(Clone, Debug)]
pub struct CorrelationConfig {
    /// Maximum time window for grouping alerts (seconds)
    pub time_window_secs: u64,
    /// Maximum group size
    pub max_group_size: usize,
    /// Minimum severity for creating incidents
    pub min_severity: rustops_common::Severity,
}

impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            time_window_secs: 300, // 5 minutes
            max_group_size: 50,
            min_severity: rustops_common::Severity::Warning,
        }
    }
}

/// A group of related alerts
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertGroup {
    pub group_id: IncidentId,
    pub alerts: Vec<NormalizedAlert>,
    pub root_cause_candidate: AlertId,
    /// Service names affected by this group
    pub affected_services: Vec<String>,
    pub impact_score: f64,
    pub created_at: DateTime<Utc>,
}

/// Service graph for topological analysis
#[derive(Clone)]
pub struct ServiceGraph {
    graph: DiGraph<ServiceNode, DependencyEdge>,
    service_indices: HashMap<String, NodeIndex>,
}

/// Node in service graph
#[derive(Clone, Debug)]
pub struct ServiceNode {
    id: ServiceId,
    name: String,
}

/// Edge in service graph
#[derive(Clone, Debug)]
struct DependencyEdge {
    dependency_type: DependencyType,
}

impl ServiceGraph {
    /// Create a new empty service graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            service_indices: HashMap::new(),
        }
    }

    /// Add a service to the graph
    pub fn add_service(&mut self, id: ServiceId, name: String) {
        if !self.service_indices.contains_key(&name) {
            let node = ServiceNode {
                id,
                name: name.clone(),
            };
            let idx = self.graph.add_node(node);
            self.service_indices.insert(name, idx);
        }
    }

    /// Add a dependency between services
    pub fn add_dependency(&mut self, from: &str, to: &str, dep_type: DependencyType) {
        let from_idx = *self
            .service_indices
            .get(from)
            .unwrap_or_else(|| panic!("Service not found: {}", from));
        let to_idx = *self
            .service_indices
            .get(to)
            .unwrap_or_else(|| panic!("Service not found: {}", to));

        self.graph.update_edge(
            from_idx,
            to_idx,
            DependencyEdge {
                dependency_type: dep_type,
            },
        );
    }

    /// Get dependencies of a service
    pub fn get_dependencies(&self, service: &str) -> Vec<&ServiceNode> {
        if let Some(&idx) = self.service_indices.get(service) {
            self.graph
                .neighbors(idx)
                .map(|neighbor_idx| &self.graph[neighbor_idx])
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if service2 depends on service1 (directly or transitively)
    pub fn has_dependency_path(&self, from: &str, to: &str) -> bool {
        if let (Some(&from_idx), Some(&to_idx)) =
            (self.service_indices.get(from), self.service_indices.get(to))
        {
            petgraph::algo::has_path_connecting(&self.graph, from_idx, to_idx, None)
        } else {
            false
        }
    }
}

impl Default for ServiceGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of service dependency
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyType {
    Http,
    MessageQueue,
    Database,
    Cache,
    External,
}

/// Alert correlator - groups related alerts
pub struct AlertCorrelator {
    config: CorrelationConfig,
    service_graph: ServiceGraph,
}

impl AlertCorrelator {
    /// Create a new alert correlator
    pub fn new(config: CorrelationConfig) -> Self {
        Self {
            config,
            service_graph: ServiceGraph::new(),
        }
    }

    /// Create with default config
    pub fn default() -> Self {
        Self::new(CorrelationConfig::default())
    }

    /// Set the service graph
    pub fn with_service_graph(mut self, graph: ServiceGraph) -> Self {
        self.service_graph = graph;
        self
    }

    /// Correlate alerts into groups
    pub fn correlate(&self, alerts: Vec<NormalizedAlert>) -> Vec<AlertGroup> {
        let mut groups = Vec::new();
        let mut grouped: HashSet<AlertId> = HashSet::new();

        for alert in &alerts {
            if grouped.contains(&alert.alert_id) {
                continue;
            }

            let group = self.create_group(alert, &alerts, &grouped);
            for alert_in_group in &group.alerts {
                grouped.insert(alert_in_group.alert_id);
            }
            groups.push(group);
        }

        groups
    }

    /// Create an alert group from a seed alert
    fn create_group(
        &self,
        seed: &NormalizedAlert,
        all_alerts: &[NormalizedAlert],
        already_grouped: &HashSet<AlertId>,
    ) -> AlertGroup {
        let mut group_alerts = vec![seed.clone()];
        let mut affected_services = HashSet::new();
        affected_services.insert(seed.service.clone());

        // Find related alerts
        for other in all_alerts {
            if already_grouped.contains(&other.alert_id) || other.alert_id == seed.alert_id {
                continue;
            }

            // Check time window
            let time_diff = (other.timestamp - seed.timestamp).num_seconds().abs();
            if time_diff > self.config.time_window_secs as i64 {
                continue;
            }

            // Check if related via service graph
            if self.are_related(seed, other) {
                group_alerts.push(other.clone());
                affected_services.insert(other.service.clone());

                if group_alerts.len() >= self.config.max_group_size {
                    break;
                }
            }
        }

        // Calculate impact score
        let impact_score = self.calculate_impact(&group_alerts);

        // Select root cause candidate (earliest, highest severity)
        let root_cause = group_alerts
            .iter()
            .min_by_key(|a| (a.timestamp, std::cmp::Reverse(a.severity)))
            .map(|a| a.alert_id)
            .unwrap_or(seed.alert_id);

        AlertGroup {
            group_id: IncidentId::new(),
            alerts: group_alerts,
            root_cause_candidate: root_cause,
            affected_services: affected_services.into_iter().collect(),
            impact_score,
            created_at: Utc::now(),
        }
    }

    /// Check if two alerts are related
    fn are_related(&self, a: &NormalizedAlert, b: &NormalizedAlert) -> bool {
        // Same service
        if a.service == b.service {
            return true;
        }

        // Check service graph
        if self
            .service_graph
            .has_dependency_path(&a.service, &b.service)
            || self
                .service_graph
                .has_dependency_path(&b.service, &a.service)
        {
            return true;
        }

        false
    }

    /// Calculate impact score for a group
    fn calculate_impact(&self, alerts: &[NormalizedAlert]) -> f64 {
        if alerts.is_empty() {
            return 0.0;
        }

        // Impact = (severity weight * 0.6) + (service count * 0.4)
        let max_severity = alerts.iter().map(|a| a.severity as i32).max().unwrap_or(0);
        let severity_score = max_severity as f64 / 4.0; // Normalize to 0-1

        let service_count = alerts
            .iter()
            .map(|a| &a.service)
            .collect::<HashSet<_>>()
            .len();
        let service_score = (service_count as f64 / 10.0).min(1.0);

        severity_score * 0.6 + service_score * 0.4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_alert(
        service: &str,
        alert_type: &str,
        severity: rustops_common::Severity,
        timestamp: DateTime<Utc>,
    ) -> NormalizedAlert {
        NormalizedAlert {
            alert_id: AlertId::new(),
            timestamp,
            service: service.to_string(),
            alert_type: alert_type.to_string(),
            severity,
            title: "Test alert".to_string(),
            resource: Some("test-resource".to_string()),
            metric_name: Some("cpu_usage".to_string()),
            metric_value: Some(95.0),
            threshold: Some(80.0),
            labels: HashMap::new(),
        }
    }

    #[test]
    fn test_service_graph() {
        let mut graph = ServiceGraph::new();

        let svc1 = ServiceId::new();
        let svc2 = ServiceId::new();
        let svc3 = ServiceId::new();

        graph.add_service(svc1, "api".to_string());
        graph.add_service(svc2, "database".to_string());
        graph.add_service(svc3, "cache".to_string());

        graph.add_dependency("api", "database", DependencyType::Database);
        graph.add_dependency("api", "cache", DependencyType::Cache);

        assert!(graph.has_dependency_path("api", "database"));
        assert!(!graph.has_dependency_path("database", "api"));
    }

    #[test]
    fn test_alert_correlation() {
        let mut graph = ServiceGraph::new();

        let api_svc = ServiceId::new();
        let db_svc = ServiceId::new();

        graph.add_service(api_svc, "api".to_string());
        graph.add_service(db_svc, "database".to_string());
        graph.add_dependency("api", "database", DependencyType::Database);

        let correlator = AlertCorrelator::default().with_service_graph(graph);

        let now = Utc::now();
        let alerts = vec![
            create_test_alert("database", "high_cpu", rustops_common::Severity::Major, now),
            create_test_alert(
                "api",
                "slow_requests",
                rustops_common::Severity::Warning,
                now,
            ),
        ];

        let groups = correlator.correlate(alerts);

        // Should be grouped due to service dependency
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].alerts.len(), 2);
    }
}
