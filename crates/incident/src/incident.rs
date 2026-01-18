//! # Incident domain model and repository
//!
//! Core incident types and CQRS repository pattern.

use crate::events::IncidentEvent;
use chrono::{DateTime, Utc};
use rustops_common::{AlertId, IncidentId, Result, ServiceId, Severity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Incident status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Open,
    Acknowledged,
    Investigating,
    Resolved,
    Closed,
}

/// Incident - represents an ongoing or resolved incident
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Incident {
    /// Unique incident ID
    pub id: IncidentId,
    /// Incident title
    pub title: String,
    /// Description
    pub description: Option<String>,
    /// Current status
    pub status: IncidentStatus,
    /// Severity level
    pub severity: Severity,
    /// Associated alerts
    pub alert_ids: Vec<AlertId>,
    /// Affected services
    pub affected_services: Vec<ServiceId>,
    /// Root cause analysis (optional)
    pub root_cause: Option<String>,
    /// Resolution details (if resolved)
    pub resolution: Option<String>,
    /// Incident commander
    pub commander: Option<String>,
    /// Custom labels/attributes
    pub labels: HashMap<String, String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Resolved timestamp (if resolved)
    pub resolved_at: Option<DateTime<Utc>>,
}

impl Incident {
    /// Create a new incident
    pub fn new(
        title: impl Into<String>,
        severity: Severity,
        alert_ids: Vec<AlertId>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: IncidentId::new(),
            title: title.into(),
            description: None,
            status: IncidentStatus::Open,
            severity,
            alert_ids,
            affected_services: Vec::new(),
            root_cause: None,
            resolution: None,
            commander: None,
            labels: HashMap::new(),
            created_at: now,
            updated_at: now,
            resolved_at: None,
        }
    }

    /// Acknowledge the incident
    pub fn acknowledge(&mut self, commander: impl Into<String>) {
        self.status = IncidentStatus::Acknowledged;
        self.commander = Some(commander.into());
        self.updated_at = Utc::now();
    }

    /// Start investigation
    pub fn start_investigation(&mut self) {
        self.status = IncidentStatus::Investigating;
        self.updated_at = Utc::now();
    }

    /// Resolve the incident
    pub fn resolve(&mut self, resolution: impl Into<String>) {
        self.status = IncidentStatus::Resolved;
        self.resolution = Some(resolution.into());
        self.resolved_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Close the incident
    pub fn close(&mut self) {
        self.status = IncidentStatus::Closed;
        self.updated_at = Utc::now();
    }

    /// Add a label
    pub fn add_label(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.labels.insert(key.into(), value.into());
        self.updated_at = Utc::now();
    }

    /// Update root cause
    pub fn set_root_cause(&mut self, root_cause: impl Into<String>) {
        self.root_cause = Some(root_cause.into());
        self.updated_at = Utc::now();
    }

    /// Check if incident is active (not resolved or closed)
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            IncidentStatus::Open
                | IncidentStatus::Acknowledged
                | IncidentStatus::Investigating
        )
    }

    /// Calculate MTTR (Mean Time To Resolve)
    pub fn mttr(&self) -> Option<chrono::Duration> {
        match (self.resolved_at, self.created_at) {
            (Some(resolved), _) => Some(resolved - self.created_at),
            _ => None,
        }
    }

    /// Apply an event to update the incident
    pub fn apply_event(&mut self, event: IncidentEvent) {
        match event {
            IncidentEvent::Created {
                id,
                title,
                severity,
                alert_ids,
            } => {
                self.id = id;
                self.title = title;
                self.severity = severity;
                self.alert_ids = alert_ids;
            }
            IncidentEvent::Acknowledged { commander } => {
                self.acknowledge(commander);
            }
            IncidentEvent::InvestigationStarted => {
                self.start_investigation();
            }
            IncidentEvent::Resolved { resolution } => {
                self.resolve(resolution);
            }
            IncidentEvent::Closed => {
                self.close();
            }
            IncidentEvent::RootCauseIdentified { root_cause } => {
                self.set_root_cause(root_cause);
            }
            IncidentEvent::LabelAdded { key, value } => {
                self.add_label(key, value);
            }
        }
    }
}

/// Incident repository (write model)
#[async_trait::async_trait]
pub trait IncidentRepository: Send + Sync {
    /// Save a new incident
    async fn save(&self, incident: &Incident) -> Result<()>;

    /// Update an existing incident
    async fn update(&self, incident: &Incident) -> Result<()>;

    /// Get incident by ID
    async fn get(&self, id: IncidentId) -> Result<Option<Incident>>;

    /// List all active incidents
    async fn list_active(&self) -> Result<Vec<Incident>>;

    /// List incidents by status
    async fn list_by_status(&self, status: IncidentStatus) -> Result<Vec<Incident>>;

    /// List incidents for a service
    async fn list_by_service(&self, service_id: ServiceId) -> Result<Vec<Incident>>;
}

/// Severity re-export from common
pub use rustops_common::Severity;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incident_creation() {
        let incident = Incident::new(
            "Database connectivity issues",
            Severity::Major,
            vec![AlertId::new()],
        );

        assert_eq!(incident.status, IncidentStatus::Open);
        assert_eq!(incident.severity, Severity::Major);
        assert!(incident.is_active());
    }

    #[test]
    fn test_incident_acknowledge() {
        let mut incident = Incident::new("Test", Severity::Major, vec![]);
        incident.acknowledge("alice");

        assert_eq!(incident.status, IncidentStatus::Acknowledged);
        assert_eq!(incident.commander, Some("alice".to_string()));
    }

    #[test]
    fn test_incident_resolve() {
        let mut incident = Incident::new("Test", Severity::Major, vec![]);
        incident.resolve("Fixed the database connection");

        assert_eq!(incident.status, IncidentStatus::Resolved);
        assert_eq!(incident.resolution, Some("Fixed the database connection".to_string()));
        assert!(incident.resolved_at.is_some());
        assert!(!incident.is_active());
    }

    #[test]
    fn test_incident_mttr() {
        let mut incident = Incident::new("Test", Severity::Major, vec![]);
        incident.resolve("Fixed");

        let mttr = incident.mttr();
        assert!(mttr.is_some());
        assert!(mttr.unwrap().num_seconds() >= 0);
    }
}
