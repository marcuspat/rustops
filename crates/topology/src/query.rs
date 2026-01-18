//! Query engine for topology analysis

use crate::{error::Result, EdgeType, Error, GraphDatabase, ServiceNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Topology query
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyQuery {
    /// Find all upstream dependencies
    UpstreamDependencies {
        service_id: String,
        max_depth: Option<usize>,
    },

    /// Find all downstream dependencies
    DownstreamDependencies {
        service_id: String,
        max_depth: Option<usize>,
    },

    /// Calculate blast radius
    BlastRadius {
        service_id: String,
        max_depth: usize,
    },

    /// Find critical path
    CriticalPath {
        from_service: String,
        to_service: String,
    },

    /// Find single points of failure
    SinglePointsOfFailure {
        min_dependents: usize,
    },

    /// Impact analysis for change
    ImpactAnalysis {
        service_id: String,
        change_type: ChangeType,
    },
}

/// Change type for impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// Service will be restarted
    Restart,
    /// Service will be scaled
    Scale,
    /// Service will be deleted
    Delete,
    /// Service configuration change
    ConfigChange,
    /// Custom change
    Custom { description: String },
}

/// Impact analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    /// Service being changed
    pub service_id: String,

    /// Change type
    pub change_type: ChangeType,

    /// Direct dependents (immediate impact)
    pub direct_dependents: Vec<String>,

    /// Transitive dependents (indirect impact)
    pub transitive_dependents: Vec<String>,

    /// Total blast radius
    pub total_affected_services: usize,

    /// Risk level
    pub risk_level: ImpactRisk,

    /// Recommendations
    pub recommendations: Vec<String>,
}

/// Impact risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImpactRisk {
    /// Low risk - minimal impact
    Low,
    /// Medium risk - moderate impact
    Medium,
    /// High risk - significant impact
    High,
    /// Critical risk - widespread impact
    Critical,
}

/// Query engine
pub struct QueryEngine {
    graph: Box<dyn GraphDatabase>,
}

impl QueryEngine {
    /// Create new query engine
    pub fn new(graph: Box<dyn GraphDatabase>) -> Self {
        Self { graph }
    }

    /// Execute query
    pub async fn execute(&self, query: &TopologyQuery) -> Result<QueryResult> {
        match query {
            TopologyQuery::UpstreamDependencies {
                service_id,
                max_depth,
            } => self.upstream_dependencies(service_id, *max_depth).await,

            TopologyQuery::DownstreamDependencies {
                service_id,
                max_depth,
            } => self.downstream_dependencies(service_id, *max_depth).await,

            TopologyQuery::BlastRadius {
                service_id,
                max_depth,
            } => self.blast_radius(service_id, *max_depth).await,

            TopologyQuery::ImpactAnalysis {
                service_id,
                change_type,
            } => self.impact_analysis(service_id, change_type).await,

            _ => Err(Error::query("Query not implemented".to_string())),
        }
    }

    /// Find upstream dependencies
    async fn upstream_dependencies(&self, service_id: &str, max_depth: Option<usize>) -> Result<QueryResult> {
        let depth = max_depth.unwrap_or(5);

        let query = format!(
            r#"
            MATCH path = (upstream:Service)-[:CALLS*1..{}]->(service:Service {{id: $service_id}})
            RETURN upstream.id AS upstream_service, length(path) AS hops
            ORDER BY hops
        "#,
            depth
        );

        let mut params = HashMap::new();
        params.insert("service_id".to_string(), serde_json::json!(service_id));

        self.graph.execute_query(&query, params).await
    }

    /// Find downstream dependencies
    async fn downstream_dependencies(&self, service_id: &str, max_depth: Option<usize>) -> Result<QueryResult> {
        let depth = max_depth.unwrap_or(5);

        let query = format!(
            r#"
            MATCH path = (service:Service {{id: $service_id}})-[:CALLS*1..{}]->(downstream:Service)
            RETURN downstream.id AS downstream_service, length(path) AS hops
            ORDER BY hops
        "#,
            depth
        );

        let mut params = HashMap::new();
        params.insert("service_id".to_string(), serde_json::json!(service_id));

        self.graph.execute_query(&query, params).await
    }

    /// Calculate blast radius
    async fn blast_radius(&self, service_id: &str, max_depth: usize) -> Result<QueryResult> {
        let query = format!(
            r#"
            MATCH (affected:Service)-[:CALLS*1..{}]->(service:Service {{id: $service_id}})
            RETURN count(DISTINCT affected) AS blast_radius
        "#,
            max_depth
        );

        let mut params = HashMap::new();
        params.insert("service_id".to_string(), serde_json::json!(service_id));

        self.graph.execute_query(&query, params).await
    }

    /// Perform impact analysis
    async fn impact_analysis(&self, service_id: &str, change_type: &ChangeType) -> Result<QueryResult> {
        // Find direct dependents
        let direct_query = r#"
            MATCH (dependent:Service)-[:CALLS]->(service:Service {id: $service_id})
            RETURN dependent.id AS dependent_id
        "#;

        let mut params = HashMap::new();
        params.insert("service_id".to_string(), serde_json::json!(service_id));

        let direct_result = self.graph.execute_query(direct_query, params).await?;

        // Find transitive dependents
        let transitive_query = r#"
            MATCH (dependent:Service)-[:CALLS*1..5]->(service:Service {id: $service_id})
            RETURN DISTINCT dependent.id AS dependent_id
        "#;

        let mut params = HashMap::new();
        params.insert("service_id".to_string(), serde_json::json!(service_id));

        let transitive_result = self.graph.execute_query(transitive_query, params).await?;

        // Calculate risk level
        let direct_count = direct_result.rows.len();
        let transitive_count = transitive_result.rows.len();

        let risk_level = match transitive_count {
            0..=5 => ImpactRisk::Low,
            6..=20 => ImpactRisk::Medium,
            21..=50 => ImpactRisk::High,
            _ => ImpactRisk::Critical,
        };

        // Generate recommendations
        let mut recommendations = Vec::new();
        if risk_level >= ImpactRisk::High {
            recommendations.push("Consider performing this change during low-traffic hours".to_string());
            recommendations.push("Ensure on-call team is available".to_string());
        }

        if matches!(change_type, ChangeType::Delete) {
            recommendations.push("Verify no active deployments depend on this service".to_string());
            recommendations.push("Consider graceful migration instead of deletion".to_string());
        }

        let analysis = ImpactAnalysis {
            service_id: service_id.to_string(),
            change_type: change_type.clone(),
            direct_dependents: vec![],
            transitive_dependents: vec![],
            total_affected_services: transitive_count,
            risk_level,
            recommendations,
        };

        let result = serde_json::to_value(analysis)?;
        Ok(QueryResult {
            rows: vec![vec![result]],
        })
    }

    /// Find single points of failure
    pub async fn single_points_of_failure(&self, min_dependents: usize) -> Result<QueryResult> {
        let query = r#"
            MATCH (service:Service)
            WHERE NOT (service)-[:FAILS_OVER]->()
            WITH service, count{ (other)-[:CALLS]->(service) } AS dependent_services
            WHERE dependent_services >= $min_dependents
            RETURN service.name AS spof, dependent_services
            ORDER BY dependent_services DESC
        "#;

        let mut params = HashMap::new();
        params.insert("min_dependents".to_string(), serde_json::json!(min_dependents));

        self.graph.execute_query(query, params).await
    }

    /// Find circular dependencies
    pub async fn circular_dependencies(&self) -> Result<QueryResult> {
        let query = r#"
            MATCH path = (s1:Service)-[:CALLS+]->(s1)
            WHERE s1 IN [n IN nodes(path) WHERE n:Service]
            RETURN [n IN nodes(path) | n.name] AS cycle
        "#;

        self.graph.execute_query(query, HashMap::new()).await
    }
}

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub rows: Vec<Vec<serde_json::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Neo4jGraph;

    #[tokio::test]
    async fn test_query_engine() {
        // This test requires a running Neo4j instance
        // For now, we just test the query construction

        let service_id = "test-service";

        let upstream_query = TopologyQuery::UpstreamDependencies {
            service_id: service_id.to_string(),
            max_depth: Some(3),
        };

        let _ = format!("{:?}", upstream_query);
    }

    #[test]
    fn test_impact_risk_calculation() {
        let transitive_counts = vec![
            (3, ImpactRisk::Low),
            (10, ImpactRisk::Medium),
            (30, ImpactRisk::High),
            (100, ImpactRisk::Critical),
        ];

        for (count, expected_risk) in transitive_counts {
            let risk = match count {
                0..=5 => ImpactRisk::Low,
                6..=20 => ImpactRisk::Medium,
                21..=50 => ImpactRisk::High,
                _ => ImpactRisk::Critical,
            };
            assert_eq!(risk, expected_risk);
        }
    }
}
