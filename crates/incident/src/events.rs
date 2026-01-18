//! # Incident event sourcing
//!
//! Events for incident lifecycle with event store.

use chrono::{DateTime, Utc};
use rustops_common::{AlertId, IncidentId, Severity};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Domain event for incident
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IncidentEvent {
    /// Incident was created
    Created {
        id: IncidentId,
        title: String,
        severity: Severity,
        alert_ids: Vec<AlertId>,
    },
    /// Incident was acknowledged
    Acknowledged { commander: String },
    /// Investigation started
    InvestigationStarted,
    /// Incident was resolved
    Resolved { resolution: String },
    /// Incident was closed
    Closed,
    /// Root cause was identified
    RootCauseIdentified { root_cause: String },
    /// Label was added
    LabelAdded { key: String, value: String },
}

impl IncidentEvent {
    /// Get event type name
    pub fn event_type(&self) -> &str {
        match self {
            Self::Created { .. } => "created",
            Self::Acknowledged { .. } => "acknowledged",
            Self::InvestigationStarted => "investigation_started",
            Self::Resolved { .. } => "resolved",
            Self::Closed => "closed",
            Self::RootCauseIdentified { .. } => "root_cause_identified",
            Self::LabelAdded { .. } => "label_added",
        }
    }

    /// Get the incident ID
    pub fn incident_id(&self) -> IncidentId {
        match self {
            Self::Created { id, .. } => *id,
            // For other events, we'd need to store the incident ID
            // This is simplified - in practice, events would include the ID
            _ => IncidentId::new(),
        }
    }
}

/// Event store for incidents
pub struct IncidentEventStore {
    events: Arc<RwLock<Vec<StoredIncidentEvent>>>,
}

/// Stored event with metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredIncidentEvent {
    /// Event ID
    pub event_id: uuid::Uuid,
    /// Incident this event belongs to
    pub incident_id: IncidentId,
    /// Event data
    pub event: IncidentEvent,
    /// Event sequence number
    pub sequence: u64,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
}

impl IncidentEventStore {
    /// Create a new event store
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Append events for an incident
    pub async fn append(&self, incident_id: IncidentId, events: Vec<IncidentEvent>) -> Result<()> {
        let mut store = self.events.write().await;
        let sequence = store.len() as u64;

        for (i, event) in events.into_iter().enumerate() {
            let stored = StoredIncidentEvent {
                event_id: uuid::Uuid::new_v4(),
                incident_id,
                event,
                sequence: sequence + i as u64,
                timestamp: Utc::now(),
            };
            store.push(stored);
        }

        Ok(())
    }

    /// Get all events for an incident
    pub async fn get_events(&self, incident_id: IncidentId) -> Result<Vec<StoredIncidentEvent>> {
        let store = self.events.read().await;
        Ok(store
            .iter()
            .filter(|e| e.incident_id == incident_id)
            .cloned()
            .collect())
    }

    /// Get events since a sequence number
    pub async fn get_events_since(&self, sequence: u64) -> Result<Vec<StoredIncidentEvent>> {
        let store = self.events.read().await;
        Ok(store
            .iter()
            .filter(|e| e.sequence > sequence)
            .cloned()
            .collect())
    }

    /// Replay events to reconstruct an incident
    pub async fn replay(&self, incident_id: IncidentId) -> Result<Vec<IncidentEvent>> {
        let events = self.get_events(incident_id).await?;
        Ok(events.into_iter().map(|e| e.event).collect())
    }
}

impl Default for IncidentEventStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type
pub type Result<T> = std::result::Result<T, rustops_common::Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_store() {
        let store = IncidentEventStore::new();
        let incident_id = IncidentId::new();

        let events = vec![IncidentEvent::Created {
            id: incident_id,
            title: "Test incident".to_string(),
            severity: Severity::Major,
            alert_ids: vec![],
        }];

        store.append(incident_id, events).await.unwrap();

        let retrieved = store.get_events(incident_id).await.unwrap();
        assert_eq!(retrieved.len(), 1);
    }

    #[tokio::test]
    async fn test_event_replay() {
        let store = IncidentEventStore::new();
        let incident_id = IncidentId::new();

        let events = vec![
            IncidentEvent::Created {
                id: incident_id,
                title: "Test".to_string(),
                severity: Severity::Major,
                alert_ids: vec![],
            },
            IncidentEvent::Acknowledged {
                commander: "alice".to_string(),
            },
        ];

        store.append(incident_id, events).await.unwrap();

        let replayed = store.replay(incident_id).await.unwrap();
        assert_eq!(replayed.len(), 2);
    }
}
