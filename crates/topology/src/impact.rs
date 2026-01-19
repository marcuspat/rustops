//! # Impact Analysis
//!
//! Provides impact analysis capabilities for service topology changes.
//! Calculates blast radius, identifies critical paths, and assesses change risks.

use crate::{
    events::{TopologyEvent, TopologyEventStore},
    graph::ServiceGraph,
    model::{DependencyEdge, DependencyType, HealthStatus, ServiceNode, ServiceType},
};
use rustops_common::{Result, ServiceId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use tracing::{debug, error, info, warn};

/// Impact analysis result for a service change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    /// Service ID that changed
    pub source_service: ServiceId,
    /// Analysis timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Blast radius summary
    pub blast_radius: BlastRadiusAnalysis,
    /// Critical path analysis
    pub critical_paths: Vec<CriticalPath>,
    /// Affected services by severity
    pub affected_services: AffectedServices,
    /// Risk assessment
    pub risk_assessment: RiskAssessment,
    /// Recommended actions
    pub recommendations: Vec<Recommendation>,
}

/// Blast radius analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadiusAnalysis {
    /// Number of directly affected services
    pub direct_affected: usize,
    /// Number of indirectly affected services (2+ hops)
    pub indirect_affected: usize,
    /// Total affected services
    pub total_affected: usize,
    /// Critical services affected
    pub critical_services_affected: usize,
    /// List of affected service IDs
    pub affected_services: Vec<ServiceId>,
    /// Business impact score (0-100)
    pub business_impact_score: u8,
    /// Technical impact score (0-100)
    pub technical_impact_score: u8,
    /// Recovery time estimate (in minutes)
    pub estimated_recovery_minutes: Option<u32>,
}

/// Critical path in the service topology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalPath {
    /// Path ID
    pub id: String,
    /// Source service ID
    pub source: ServiceId,
    /// Target service ID
    pub target: ServiceId,
    /// Path length in hops
    pub hops: usize,
    /// Total traffic volume on this path
    pub traffic_volume: f64,
    /// Criticality score (0-100)
    pub criticality_score: u8,
    /// Path type
    pub path_type: PathType,
    /// Bottleneck services
    pub bottlenecks: Vec<ServiceId>,
    /// Alternative paths available
    pub alternative_paths: usize,
}

/// Path type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PathType {
    /// Critical business flow
    BusinessCritical,
    /// Data flow
    Data,
    /// Communication flow
    Communication,
    /// Monitoring/Telemetry
    Monitoring,
    /// Auxiliary flow
    Auxiliary,
}

/// Affected services grouped by severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedServices {
    /// Critical services (high impact)
    pub critical: Vec<AffectedService>,
    /// High impact services
    pub high: Vec<AffectedService>,
    /// Medium impact services
    pub medium: Vec<AffectedService>,
    /// Low impact services
    pub low: Vec<AffectedService>,
    /// External services
    pub external: Vec<AffectedService>,
}

impl AffectedServices {
    /// Get total services count
    pub fn total_services(&self) -> usize {
        self.critical.len()
            + self.high.len()
            + self.medium.len()
            + self.low.len()
            + self.external.len()
    }
}

/// Individual affected service details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedService {
    /// Service ID
    pub id: ServiceId,
    /// Service name
    pub name: String,
    /// Namespace
    pub namespace: String,
    /// Impact severity
    pub severity: ImpactSeverity,
    /// Number of direct dependencies from source
    pub dependency_hops: usize,
    /// Estimated downtime (minutes)
    pub estimated_downtime_minutes: Option<u32>,
    /// Customer impact description
    pub customer_impact: Option<String>,
    /// Mitigation available
    pub mitigation_available: bool,
    /// Health status before impact
    pub previous_health: HealthStatus,
    /// Health status after impact
    pub current_health: HealthStatus,
}

/// Impact severity enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImpactSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Overall risk level
    pub risk_level: RiskLevel,
    /// Risk factors
    pub risk_factors: Vec<RiskFactor>,
    /// Mitigation opportunities
    pub mitigation_opportunities: Vec<MitigationOpportunity>,
    /// Containment strategies
    pub containment_strategies: Vec<ContainmentStrategy>,
    /// Total risk score (0-100)
    pub total_risk_score: u8,
}

/// Risk level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// Risk factor identified during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    /// Factor name
    pub name: String,
    /// Factor description
    pub description: String,
    /// Risk score (0-100)
    pub score: u8,
    /// Risk category
    pub category: RiskCategory,
}

/// Risk category enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskCategory {
    Technical,
    Business,
    Security,
    Compliance,
    Operational,
}

/// Mitigation opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitigationOpportunity {
    /// Opportunity description
    pub description: String,
    /// Implementation complexity
    pub complexity: ImplementationComplexity,
    /// Estimated effectiveness (0-100%)
    pub effectiveness: u8,
    /// Implementation time estimate
    pub implementation_time_minutes: u32,
    /// Required resources
    pub required_resources: Vec<String>,
}

/// Implementation complexity enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImplementationComplexity {
    Trivial,
    Low,
    Medium,
    High,
    Complex,
}

/// Containment strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainmentStrategy {
    /// Strategy name
    pub name: String,
    /// Strategy description
    pub description: String,
    /// Containment level
    pub containment_level: ContainmentLevel,
    /// Required actions
    pub actions: Vec<String>,
    /// Prerequisites
    pub prerequisites: Vec<String>,
}

/// Containment level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainmentLevel {
    None,
    Service,
    Namespace,
    Cluster,
    Region,
}

/// Recommendation for addressing the impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Recommendation ID
    pub id: String,
    /// Recommendation type
    pub recommendation_type: RecommendationType,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Priority
    pub priority: Priority,
    /// Implementation guidance
    pub implementation: ImplementationGuidance,
    /// Estimated impact of recommendation
    pub estimated_impact: RecommendationImpact,
    /// Dependencies
    pub dependencies: Vec<String>,
}

/// Recommendation type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecommendationType {
    ImmediateAction,
    ShortTermFix,
    LongTermSolution,
    Prevention,
    Monitoring,
}

/// Priority enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
    Emergency,
}

/// Implementation guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationGuidance {
    /// Steps to implement
    pub steps: Vec<String>,
    /// Required approvals
    pub approvals: Vec<String>,
    /// Rollback plan
    pub rollback_plan: Option<String>,
    /// Testing required
    pub testing_requirements: Vec<String>,
}

/// Recommendation impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationImpact {
    /// Reduction in affected services
    pub affected_services_reduction: Option<usize>,
    /// Reduction in blast radius
    pub blast_radius_reduction: Option<f64>,
    /// Implementation risk
    pub implementation_risk: RiskLevel,
    /// Cost estimate
    pub cost_estimate: Option<f64>,
}

/// Impact analyzer for service topology changes
pub struct ImpactAnalyzer {
    /// Service graph for analysis
    graph: ServiceGraph,
    /// Event store for topology events
    event_store: Option<Box<dyn TopologyEventStore>>,
    /// Configuration
    config: ImpactAnalyzerConfig,
}

/// Impact analyzer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalyzerConfig {
    /// Maximum hops for blast radius calculation
    pub max_blast_radius_hops: usize,
    /// Enable critical path analysis
    pub enable_critical_path_analysis: bool,
    /// Enable risk assessment
    pub enable_risk_assessment: bool,
    /// Enable business impact calculation
    pub enable_business_impact: bool,
    /// External service impact weight
    pub external_service_impact_weight: f64,
    /// Critical service threshold
    pub critical_service_threshold: f64,
    /// Default service recovery time (minutes)
    pub default_service_recovery_minutes: u32,
}

impl Default for ImpactAnalyzerConfig {
    fn default() -> Self {
        Self {
            max_blast_radius_hops: 5,
            enable_critical_path_analysis: true,
            enable_risk_assessment: true,
            enable_business_impact: true,
            external_service_impact_weight: 0.7,
            critical_service_threshold: 0.8,
            default_service_recovery_minutes: 30,
        }
    }
}

impl ImpactAnalyzer {
    /// Create new impact analyzer
    pub fn new(graph: ServiceGraph, event_store: Option<Box<dyn TopologyEventStore>>) -> Self {
        Self {
            graph,
            event_store,
            config: ImpactAnalyzerConfig::default(),
        }
    }

    /// Configure the analyzer
    pub fn with_config(mut self, config: ImpactAnalyzerConfig) -> Self {
        self.config = config;
        self
    }

    /// Analyze impact of a service change
    pub async fn analyze_service_impact(&self, service_id: &ServiceId) -> Result<ImpactAnalysis> {
        info!("Analyzing impact for service: {}", service_id);

        // Get blast radius analysis
        let blast_radius = self.analyze_blast_radius(service_id).await?;

        // Get critical paths
        let critical_paths = if self.config.enable_critical_path_analysis {
            self.analyze_critical_paths(service_id).await?
        } else {
            Vec::new()
        };

        // Get affected services
        let affected_services = self
            .analyze_affected_services(service_id, &blast_radius)
            .await?;

        // Get risk assessment
        let risk_assessment = if self.config.enable_risk_assessment {
            self.assess_risk(
                service_id,
                &blast_radius,
                &critical_paths,
                &affected_services,
            )
            .await?
        } else {
            RiskAssessment {
                risk_level: RiskLevel::None,
                risk_factors: Vec::new(),
                mitigation_opportunities: Vec::new(),
                containment_strategies: Vec::new(),
                total_risk_score: 0,
            }
        };

        // Generate recommendations
        let recommendations = self
            .generate_recommendations(
                service_id,
                &blast_radius,
                &critical_paths,
                &affected_services,
                &risk_assessment,
            )
            .await?;

        Ok(ImpactAnalysis {
            source_service: *service_id,
            timestamp: chrono::Utc::now(),
            blast_radius,
            critical_paths,
            affected_services,
            risk_assessment,
            recommendations,
        })
    }

    /// Analyze blast radius for a service change
    async fn analyze_blast_radius(&self, service_id: &ServiceId) -> Result<BlastRadiusAnalysis> {
        let blast_radius = self
            .graph
            .calculate_blast_radius(service_id, self.config.max_blast_radius_hops)?;

        // Calculate direct and indirect affected services
        let mut direct_affected = 0;
        let mut indirect_affected = 0;
        let mut critical_services = 0;

        for hops in blast_radius.hops_distribution.keys() {
            match hops {
                1 => direct_affected += blast_radius.hops_distribution[hops],
                _ => indirect_affected += blast_radius.hops_distribution[hops],
            }
        }

        // Count critical services
        for service_id in &blast_radius.affected_services {
            if self
                .graph
                .get_service(service_id)
                .and_then(|s| s.labels.get("criticality"))
                .map(|s| s.as_str())
                == Some("high")
            {
                critical_services += 1;
            }
        }

        // Calculate impact scores
        let total_services = self.graph.service_count();
        let technical_impact =
            ((blast_radius.total_affected as f64 / total_services as f64) * 100.0) as u8;

        let business_impact = if self.config.enable_business_impact {
            self.calculate_business_impact(service_id).await?
        } else {
            0
        };

        // Estimate recovery time
        // Estimate recovery time
        let estimated_recovery_minutes = self.estimate_recovery_time(&blast_radius).await?;

        Ok(BlastRadiusAnalysis {
            direct_affected,
            indirect_affected,
            total_affected: blast_radius.total_affected,
            critical_services_affected: critical_services,
            affected_services: blast_radius.affected_services,
            business_impact_score: business_impact,
            technical_impact_score: technical_impact,
            estimated_recovery_minutes,
        })
    }

    /// Analyze critical paths
    async fn analyze_critical_paths(&self, service_id: &ServiceId) -> Result<Vec<CriticalPath>> {
        let mut critical_paths = Vec::new();

        // Find all services that depend on the source service
        let upstream_services = self.graph.find_upstream_dependencies(service_id)?;

        for upstream_service in upstream_services {
            // Find shortest path
            if let Some(path) = self
                .graph
                .find_shortest_path(&upstream_service.id, service_id)?
            {
                if path.len() > 1 {
                    let criticality_score = self.calculate_path_criticality(&path).await?;

                    // Identify bottlenecks
                    let bottlenecks = self.identify_path_bottlenecks(&path).await?;

                    // Check for alternative paths
                    let alternative_paths = self
                        .count_alternative_paths(&upstream_service.id, service_id)
                        .await?;

                    let critical_path = CriticalPath {
                        id: format!("path_{}_{}_{}", path[0].id, service_id, path.len()),
                        source: upstream_service.id,
                        target: *service_id,
                        hops: path.len() - 1,
                        traffic_volume: self.calculate_path_traffic(&path).await?,
                        criticality_score,
                        path_type: self.determine_path_type(&path).await?,
                        bottlenecks,
                        alternative_paths,
                    };

                    critical_paths.push(critical_path);
                }
            }
        }

        // Sort by criticality
        critical_paths.sort_by(|a, b| b.criticality_score.cmp(&a.criticality_score));

        Ok(critical_paths)
    }

    /// Analyze affected services
    async fn analyze_affected_services(
        &self,
        service_id: &ServiceId,
        blast_radius: &BlastRadiusAnalysis,
    ) -> Result<AffectedServices> {
        let mut affected = AffectedServices {
            critical: Vec::new(),
            high: Vec::new(),
            medium: Vec::new(),
            low: Vec::new(),
            external: Vec::new(),
        };

        for affected_service_id in &blast_radius.affected_services {
            if affected_service_id == service_id {
                continue;
            }

            if let Some(service) = self.graph.get_service(affected_service_id) {
                let severity = self
                    .calculate_service_severity(service_id, affected_service_id, blast_radius)
                    .await?;

                let dependency_hops = self
                    .calculate_dependency_hops(service_id, affected_service_id)
                    .await?;

                let downtime_estimate = self
                    .estimate_service_downtime(&service, dependency_hops)
                    .await?;

                let customer_impact = self.get_customer_impact_description(&service).await?;

                let mitigation = self
                    .check_mitigation_available(service_id, affected_service_id)
                    .await?;

                let previous_health = service.health;

                let affected_service = AffectedService {
                    id: service.id,
                    name: service
                        .name
                        .clone()
                        .unwrap_or_else(|| "<unnamed>".to_string()),
                    namespace: service.namespace.clone(),
                    severity,
                    dependency_hops,
                    estimated_downtime_minutes: downtime_estimate,
                    customer_impact,
                    mitigation_available: mitigation,
                    previous_health,
                    current_health: HealthStatus::Unknown, // Would be determined after impact
                };

                match severity {
                    ImpactSeverity::Critical => affected.critical.push(affected_service),
                    ImpactSeverity::High => affected.high.push(affected_service),
                    ImpactSeverity::Medium => affected.medium.push(affected_service),
                    ImpactSeverity::Low => affected.low.push(affected_service),
                }
            }
        }

        Ok(affected)
    }

    /// Assess risk for the impact
    async fn assess_risk(
        &self,
        service_id: &ServiceId,
        blast_radius: &BlastRadiusAnalysis,
        critical_paths: &[CriticalPath],
        affected_services: &AffectedServices,
    ) -> Result<RiskAssessment> {
        let mut risk_factors = Vec::new();
        let mut mitigation_opportunities = Vec::new();
        let mut containment_strategies = Vec::new();
        let mut total_score = 0;

        // Risk factor: Blast radius size
        if blast_radius.total_affected > 10 {
            risk_factors.push(RiskFactor {
                name: "Large Blast Radius".to_string(),
                description: "Change affects many services".to_string(),
                score: 80,
                category: RiskCategory::Technical,
            });
            total_score += 80;
        }

        // Risk factor: Critical services affected
        if blast_radius.critical_services_affected > 0 {
            risk_factors.push(RiskFactor {
                name: "Critical Services Affected".to_string(),
                description: "Critical business services will be impacted".to_string(),
                score: 90,
                category: RiskCategory::Business,
            });
            total_score += 90;
        }

        // Risk factor: High impact critical paths
        for path in critical_paths {
            if path.criticality_score > 80 && path.alternative_paths == 0 {
                risk_factors.push(RiskFactor {
                    name: "Critical Path with No Alternatives".to_string(),
                    description: "Critical business flow has no redundancy".to_string(),
                    score: 85,
                    category: RiskCategory::Business,
                });
                total_score += 85;
            }
        }

        // Mitigation opportunities
        if affected_services.critical.len() > 0 {
            mitigation_opportunities.push(MitigationOpportunity {
                description: "Implement circuit breakers for critical services".to_string(),
                complexity: ImplementationComplexity::Medium,
                effectiveness: 70,
                implementation_time_minutes: 60,
                required_resources: vec!["Engineering team".to_string()],
            });
        }

        // Containment strategies
        if blast_radius.total_affected > 5 {
            containment_strategies.push(ContainmentStrategy {
                name: "Namespace Isolation".to_string(),
                description: "Change can be contained within specific namespaces".to_string(),
                containment_level: ContainmentLevel::Namespace,
                actions: vec![
                    "Identify affected namespaces".to_string(),
                    "Implement namespace-level circuit breakers".to_string(),
                ],
                prerequisites: vec![
                    "Namespace-level monitoring".to_string(),
                    "Namespace-level service discovery".to_string(),
                ],
            });
        }

        // Determine overall risk level
        let risk_level = if total_score >= 200 {
            RiskLevel::Critical
        } else if total_score >= 150 {
            RiskLevel::High
        } else if total_score >= 100 {
            RiskLevel::Medium
        } else if total_score > 0 {
            RiskLevel::Low
        } else {
            RiskLevel::None
        };

        Ok(RiskAssessment {
            risk_level,
            risk_factors,
            mitigation_opportunities,
            containment_strategies,
            total_risk_score: (total_score as f64 / 3.0).min(100.0) as u8,
        })
    }

    /// Generate recommendations for addressing the impact
    async fn generate_recommendations(
        &self,
        service_id: &ServiceId,
        blast_radius: &BlastRadiusAnalysis,
        critical_paths: &[CriticalPath],
        affected_services: &AffectedServices,
        risk_assessment: &RiskAssessment,
    ) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // High priority recommendations for critical impact
        if blast_radius.critical_services_affected > 0
            || risk_assessment.risk_level == RiskLevel::Critical
        {
            recommendations.push(Recommendation {
                id: "immediate-pause".to_string(),
                recommendation_type: RecommendationType::ImmediateAction,
                title: "Pause Deployment".to_string(),
                description: "Consider pausing the deployment to assess the full impact"
                    .to_string(),
                priority: Priority::Emergency,
                implementation: ImplementationGuidance {
                    steps: vec![
                        "Pause deployment pipeline".to_string(),
                        "Assemble incident response team".to_string(),
                        "Communicate with stakeholders".to_string(),
                    ],
                    approvals: vec!["Incident Commander".to_string()],
                    rollback_plan: Some("Resume paused deployment".to_string()),
                    testing_requirements: vec!["Impact reassessment".to_string()],
                },
                estimated_impact: RecommendationImpact {
                    affected_services_reduction: Some(blast_radius.total_affected / 2),
                    blast_radius_reduction: Some(0.5),
                    implementation_risk: RiskLevel::Low,
                    cost_estimate: Some(0.0),
                },
                dependencies: vec!["Incident Response Team".to_string()],
            });
        }

        // Monitoring recommendations
        if affected_services.total_services() > 5 {
            recommendations.push(Recommendation {
                id: "enhanced-monitoring".to_string(),
                recommendation_type: RecommendationType::Monitoring,
                title: "Enhanced Monitoring Setup".to_string(),
                description: "Set up enhanced monitoring for affected services".to_string(),
                priority: Priority::High,
                implementation: ImplementationGuidance {
                    steps: vec![
                        "Enable detailed logging".to_string(),
                        "Set up alerting thresholds".to_string(),
                        "Prepare rollback metrics".to_string(),
                    ],
                    approvals: vec!["Platform Team".to_string()],
                    rollback_plan: None,
                    testing_requirements: vec!["Alert validation".to_string()],
                },
                estimated_impact: RecommendationImpact {
                    affected_services_reduction: None,
                    blast_radius_reduction: None,
                    implementation_risk: RiskLevel::Low,
                    cost_estimate: Some(1000.0),
                },
                dependencies: vec!["Monitoring Platform".to_string()],
            });
        }

        // Mitigation recommendations
        for service in &affected_services.critical {
            if service.mitigation_available {
                recommendations.push(Recommendation {
                    id: format!("mitigation-{}", service.id),
                    recommendation_type: RecommendationType::ShortTermFix,
                    title: format!("Mitigation for {}", service.name),
                    description: "Implement available mitigation for critical service".to_string(),
                    priority: Priority::High,
                    implementation: ImplementationGuidance {
                        steps: vec![
                            "Enable mitigation strategy".to_string(),
                            "Validate mitigation effectiveness".to_string(),
                            "Update service configuration".to_string(),
                        ],
                        approvals: vec!["Service Owner".to_string()],
                        rollback_plan: Some("Disable mitigation strategy".to_string()),
                        testing_requirements: vec!["Service functionality test".to_string()],
                    },
                    estimated_impact: RecommendationImpact {
                        affected_services_reduction: Some(1),
                        blast_radius_reduction: Some(0.1),
                        implementation_risk: RiskLevel::Medium,
                        cost_estimate: Some(5000.0),
                    },
                    dependencies: vec![format!("Service: {}", service.name)],
                });
            }
        }

        Ok(recommendations)
    }

    /// Helper methods

    /// Calculate business impact score
    async fn calculate_business_impact(&self, _service_id: &ServiceId) -> Result<u8> {
        // This is a stub implementation
        // In a real implementation, this would consider:
        // - Critical services affected
        // - External services affected
        // - Customer impact potential

        let business_impact = 50; // Default medium impact

        // Cap at 100
        Ok(business_impact.min(100))
    }

    /// Estimate recovery time
    async fn estimate_recovery_time(
        &self,
        blast_radius: &crate::graph::BlastRadius,
    ) -> Result<Option<u32>> {
        // Base recovery time affected by:
        // - Number of affected services
        // - Critical services
        // - Complexity of dependencies

        if blast_radius.total_affected == 0 {
            return Ok(None);
        }

        let base_time = self.config.default_service_recovery_minutes;
        let multiplier = 1.0 + (blast_radius.total_affected as f64 * 0.1);
        let critical_multiplier = if blast_radius.critical_affected_services > 0 {
            2.0
        } else {
            1.0
        };

        let estimated_time = (base_time as f64 * multiplier * critical_multiplier) as u32;
        Ok(Some(estimated_time))
    }

    /// Calculate path criticality
    async fn calculate_path_criticality(&self, path: &[ServiceNode]) -> Result<u8> {
        let mut criticality = 0;

        for service in path {
            // Check if service is critical
            let crit = service.labels.get("criticality").map(|s| s.as_str());
            if crit == Some("high") {
                criticality += 30;
            } else if crit == Some("medium") {
                criticality += 15;
            }

            // Check service type
            match service.service_type {
                ServiceType::Deployment => criticality += 5,
                ServiceType::StatefulSet => criticality += 10,
                ServiceType::DaemonSet => criticality += 5,
                ServiceType::External => criticality += 15,
            }
        }

        Ok(criticality.min(100))
    }

    /// Identify path bottlenecks
    async fn identify_path_bottlenecks(&self, path: &[ServiceNode]) -> Result<Vec<ServiceId>> {
        let mut bottlenecks = Vec::new();

        for service in path {
            // Check for bottleneck indicators
            if service.replicas == 0 {
                bottlenecks.push(service.id);
            }

            // Check labels for bottleneck indicators
            if service.labels.contains_key("bottleneck") {
                bottlenecks.push(service.id);
            }
        }

        Ok(bottlenecks)
    }

    /// Count alternative paths
    async fn count_alternative_paths(&self, from: &ServiceId, to: &ServiceId) -> Result<usize> {
        // This would implement a path finding algorithm to count alternative routes
        // For now, return 0 (no alternatives found)
        Ok(0)
    }

    /// Calculate path traffic volume
    async fn calculate_path_traffic(&self, _path: &[ServiceNode]) -> Result<f64> {
        // This would integrate with metrics systems to calculate actual traffic
        // For now, return an estimate
        Ok(1000.0) // 1000 requests per minute
    }

    /// Determine path type
    async fn determine_path_type(&self, path: &[ServiceNode]) -> Result<PathType> {
        // Determine path type based on service labels and types
        for service in path {
            let labels = &service.labels;
            if labels.contains_key("business-critical") {
                return Ok(PathType::BusinessCritical);
            } else if labels.contains_key("data") {
                return Ok(PathType::Data);
            }
        }

        // Default to communication
        Ok(PathType::Communication)
    }

    /// Calculate service severity
    async fn calculate_service_severity(
        &self,
        _source_id: &ServiceId,
        affected_id: &ServiceId,
        _blast_radius: &BlastRadiusAnalysis,
    ) -> Result<ImpactSeverity> {
        if let Some(service) = self.graph.get_service(affected_id) {
            // Check if service is critical
            if let Some(criticality) = service.labels.get("criticality") {
                match criticality.as_str() {
                    "high" => return Ok(ImpactSeverity::Critical),
                    "medium" => return Ok(ImpactSeverity::High),
                    _ => (),
                }
            }

            // Check service type
            match service.service_type {
                ServiceType::External => return Ok(ImpactSeverity::High),
                ServiceType::StatefulSet => return Ok(ImpactSeverity::Medium),
                _ => (),
            }
        }

        Ok(ImpactSeverity::Medium)
    }

    /// Calculate dependency hops
    async fn calculate_dependency_hops(&self, from: &ServiceId, to: &ServiceId) -> Result<usize> {
        if let Some(path) = self.graph.find_shortest_path(from, to)? {
            Ok(path.len() - 1)
        } else {
            Ok(0)
        }
    }

    /// Estimate service downtime
    async fn estimate_service_downtime(
        &self,
        _service: &ServiceNode,
        _dependency_hops: usize,
    ) -> Result<Option<u32>> {
        // This would estimate downtime based on service type, dependencies, etc.
        Ok(Some(30)) // 30 minutes default
    }

    /// Get customer impact description
    async fn get_customer_impact_description(
        &self,
        _service: &ServiceNode,
    ) -> Result<Option<String>> {
        // This would look up customer-facing impact descriptions
        Ok(None)
    }

    /// Check if mitigation is available
    async fn check_mitigation_available(&self, _from: &ServiceId, _to: &ServiceId) -> Result<bool> {
        // This would check for available mitigation strategies
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ServiceGraph;
    use rustops_common::ServiceId;

    #[tokio::test]
    async fn test_impact_analysis() {
        let graph = ServiceGraph::new(None);
        let analyzer = ImpactAnalyzer::new(graph, None);

        let service_id = ServiceId::new();
        // This test would require actual graph data
        // For now, we just test the creation
        assert_eq!(analyzer.config.max_blast_radius_hops, 5);
    }

    #[test]
    fn test_impact_severity_ordering() {
        assert!(ImpactSeverity::Critical > ImpactSeverity::High);
        assert!(ImpactSeverity::High > ImpactSeverity::Medium);
        assert!(ImpactSeverity::Medium > ImpactSeverity::Low);
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Critical > RiskLevel::High);
        assert!(RiskLevel::High > RiskLevel::Medium);
        assert!(RiskLevel::Medium > RiskLevel::Low);
        assert!(RiskLevel::Low > RiskLevel::None);
    }
}
