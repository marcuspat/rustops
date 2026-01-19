//! # CQRS repository pattern for incidents
//!
//! Implements Command Query Responsibility Segregation.

use crate::events::IncidentEvent;
use crate::incident::{Incident, IncidentRepository, IncidentStatus};
use rustops_common::{Error, IncidentId, Result, ServiceId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Write model - handles commands
pub struct WriteModel {
    repository: Arc<dyn IncidentRepository>,
}

impl WriteModel {
    /// Create a new write model
    pub fn new(repository: Arc<dyn IncidentRepository>) -> Self {
        Self { repository }
    }

    /// Create a new incident
    pub async fn create_incident(
        &self,
        title: String,
        severity: rustops_common::Severity,
        alert_ids: Vec<rustops_common::AlertId>,
    ) -> Result<Incident> {
        let incident = Incident::new(title, severity, alert_ids);
        self.repository.save(&incident).await?;
        Ok(incident)
    }

    /// Acknowledge an incident
    pub async fn acknowledge_incident(
        &self,
        id: IncidentId,
        commander: String,
    ) -> Result<Incident> {
        let mut incident = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| Error::not_found("incident", id.to_string()))?;

        incident.acknowledge(commander);
        self.repository.update(&incident).await?;
        Ok(incident)
    }

    /// Resolve an incident
    pub async fn resolve_incident(&self, id: IncidentId, resolution: String) -> Result<Incident> {
        let mut incident = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| Error::not_found("incident", id.to_string()))?;

        incident.resolve(resolution);
        self.repository.update(&incident).await?;
        Ok(incident)
    }

    /// Close an incident
    pub async fn close_incident(&self, id: IncidentId) -> Result<Incident> {
        let mut incident = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| Error::not_found("incident", id.to_string()))?;

        incident.close();
        self.repository.update(&incident).await?;
        Ok(incident)
    }

    /// Update root cause
    pub async fn update_root_cause(&self, id: IncidentId, root_cause: String) -> Result<Incident> {
        let mut incident = self
            .repository
            .get(id)
            .await?
            .ok_or_else(|| Error::not_found("incident", id.to_string()))?;

        incident.set_root_cause(root_cause);
        self.repository.update(&incident).await?;
        Ok(incident)
    }
}

/// Read model - optimized for queries
pub struct ReadModel {
    projections: Arc<RwLock<HashMap<IncidentId, IncidentProjection>>>,
}

/// Projected incident data for queries
#[derive(Clone, Debug)]
pub struct IncidentProjection {
    pub id: IncidentId,
    pub title: String,
    pub status: IncidentStatus,
    pub severity: rustops_common::Severity,
    pub affected_services: Vec<ServiceId>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub mttr_seconds: Option<u64>,
    pub alert_count: usize,
}

impl IncidentProjection {
    /// Create projection from incident
    fn from(incident: &Incident) -> Self {
        Self {
            id: incident.id,
            title: incident.title.clone(),
            status: incident.status,
            severity: incident.severity,
            affected_services: incident.affected_services.clone(),
            created_at: incident.created_at,
            resolved_at: incident.resolved_at,
            mttr_seconds: incident.mttr().map(|d| d.num_seconds() as u64),
            alert_count: incident.alert_ids.len(),
        }
    }
}

impl ReadModel {
    /// Create a new read model
    pub fn new() -> Self {
        Self {
            projections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update projection from incident
    pub async fn update(&self, incident: &Incident) -> Result<()> {
        let mut projections = self.projections.write().await;
        let projection = IncidentProjection::from(incident);
        projections.insert(incident.id, projection);
        Ok(())
    }

    /// Get projection by ID
    pub async fn get(&self, id: IncidentId) -> Result<Option<IncidentProjection>> {
        let projections = self.projections.read().await;
        Ok(projections.get(&id).cloned())
    }

    /// List all active incidents
    pub async fn list_active(&self) -> Result<Vec<IncidentProjection>> {
        let projections = self.projections.read().await;
        Ok(projections
            .values()
            .filter(|p| p.status != IncidentStatus::Resolved && p.status != IncidentStatus::Closed)
            .cloned()
            .collect())
    }

    /// List by severity
    pub async fn list_by_severity(
        &self,
        severity: rustops_common::Severity,
    ) -> Result<Vec<IncidentProjection>> {
        let projections = self.projections.read().await;
        Ok(projections
            .values()
            .filter(|p| p.severity == severity)
            .cloned()
            .collect())
    }

    /// List by service
    pub async fn list_by_service(&self, service_id: ServiceId) -> Result<Vec<IncidentProjection>> {
        let projections = self.projections.read().await;
        Ok(projections
            .values()
            .filter(|p| p.affected_services.contains(&service_id))
            .cloned()
            .collect())
    }
}

impl Default for ReadModel {
    fn default() -> Self {
        Self::new()
    }
}

/// CQRS projection - keeps read model in sync
pub struct CQRSProjection {
    read_model: Arc<ReadModel>,
}

impl CQRSProjection {
    /// Create a new CQRS projection
    pub fn new(read_model: Arc<ReadModel>) -> Self {
        Self { read_model }
    }

    /// Project an event to the read model
    pub async fn project(&self, incident: &Incident) -> Result<()> {
        self.read_model.update(incident).await
    }

    /// Get the read model
    pub fn read_model(&self) -> Arc<ReadModel> {
        Arc::clone(&self.read_model)
    }
}

/// In-memory incident repository for testing
pub struct InMemoryIncidentRepository {
    incidents: Arc<RwLock<HashMap<IncidentId, Incident>>>,
}

impl InMemoryIncidentRepository {
    /// Create a new in-memory repository
    pub fn new() -> Self {
        Self {
            incidents: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryIncidentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl IncidentRepository for InMemoryIncidentRepository {
    async fn save(&self, incident: &Incident) -> Result<()> {
        let mut incidents = self.incidents.write().await;
        incidents.insert(incident.id, incident.clone());
        Ok(())
    }

    async fn update(&self, incident: &Incident) -> Result<()> {
        let mut incidents = self.incidents.write().await;
        incidents.insert(incident.id, incident.clone());
        Ok(())
    }

    async fn get(&self, id: IncidentId) -> Result<Option<Incident>> {
        let incidents = self.incidents.read().await;
        Ok(incidents.get(&id).cloned())
    }

    async fn list_active(&self) -> Result<Vec<Incident>> {
        let incidents = self.incidents.read().await;
        Ok(incidents
            .values()
            .filter(|i| i.is_active())
            .cloned()
            .collect())
    }

    async fn list_by_status(&self, status: IncidentStatus) -> Result<Vec<Incident>> {
        let incidents = self.incidents.read().await;
        Ok(incidents
            .values()
            .filter(|i| i.status == status)
            .cloned()
            .collect())
    }

    async fn list_by_service(&self, service_id: ServiceId) -> Result<Vec<Incident>> {
        let incidents = self.incidents.read().await;
        Ok(incidents
            .values()
            .filter(|i| i.affected_services.contains(&service_id))
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustops_common::Severity;

    #[tokio::test]
    async fn test_write_model() {
        let repo = Arc::new(InMemoryIncidentRepository::new());
        let write_model = WriteModel::new(repo);

        let incident = write_model
            .create_incident("Test incident".to_string(), Severity::Major, vec![])
            .await
            .unwrap();

        assert_eq!(incident.status, IncidentStatus::Open);
    }

    #[tokio::test]
    async fn test_read_model() {
        let read_model = Arc::new(ReadModel::new());

        let incident = Incident::new("Test".to_string(), Severity::Major, vec![]);
        read_model.update(&incident).await.unwrap();

        let projection = read_model.get(incident.id).await.unwrap();
        assert!(projection.is_some());
        assert_eq!(projection.unwrap().title, "Test");
    }

    #[tokio::test]
    async fn test_cqrs_projection() {
        let repo = Arc::new(InMemoryIncidentRepository::new());
        let read_model = Arc::new(ReadModel::new());
        let projection = CQRSProjection::new(read_model.clone());

        let incident = Incident::new("Test".to_string(), Severity::Major, vec![]);
        repo.save(&incident).await.unwrap();
        projection.project(&incident).await.unwrap();

        let retrieved = read_model.get(incident.id).await.unwrap();
        assert!(retrieved.is_some());
    }
}
