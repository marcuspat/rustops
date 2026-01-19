//! # Domain Events for Topology Changes
//!
//! Implements event sourcing for service topology changes following DDD patterns.
//! Provides event store and event handlers for topology management.

use crate::{
    graph::ServiceGraph,
    model::{DependencyEdge, DependencyType, ServiceNode, ServiceType},
};
use chrono::{DateTime, Utc};
use rustops_common::{Result, ServiceId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::{debug, info, warn};

/// Domain event for topology changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TopologyEvent {
    /// Service was added to topology
    ServiceAdded {
        /// Service ID
        service_id: ServiceId,
        /// Service name
        service_name: Option<String>,
        /// Service type
        service_type: ServiceType,
    },
    /// Service was removed from topology
    ServiceRemoved {
        /// Service ID
        service_id: ServiceId,
    },
    /// Service was updated
    ServiceUpdated {
        /// Service ID
        service_id: ServiceId,
        /// Previous service info (for change detection)
        previous_service: Option<ServiceNode>,
        /// Updated service info
        current_service: ServiceNode,
    },
    /// Dependency was added between services
    DependencyAdded {
        /// Source service ID
        from_service_id: ServiceId,
        /// Target service ID
        to_service_id: ServiceId,
        /// Dependency type
        edge_type: DependencyType,
    },
    /// Dependency was removed between services
    DependencyRemoved {
        /// Source service ID
        from_service_id: ServiceId,
        /// Target service ID
        to_service_id: ServiceId,
        /// Dependency type
        edge_type: DependencyType,
    },
    /// Topology was synchronized
    TopologySynchronized {
        /// Timestamp of synchronization
        timestamp: DateTime<Utc>,
        /// Number of services
        service_count: usize,
        /// Number of dependencies
        dependency_count: usize,
    },
    /// Error occurred during topology operation
    TopologyError {
        /// Error message
        message: String,
        /// Error context
        context: HashMap<String, String>,
    },
}

impl TopologyEvent {
    /// Get event timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        // In a real implementation, this would be stored with the event
        Utc::now()
    }

    /// Get event type
    pub fn event_type(&self) -> &'static str {
        match self {
            TopologyEvent::ServiceAdded { .. } => "ServiceAdded",
            TopologyEvent::ServiceRemoved { .. } => "ServiceRemoved",
            TopologyEvent::ServiceUpdated { .. } => "ServiceUpdated",
            TopologyEvent::DependencyAdded { .. } => "DependencyAdded",
            TopologyEvent::DependencyRemoved { .. } => "DependencyRemoved",
            TopologyEvent::TopologySynchronized { .. } => "TopologySynchronized",
            TopologyEvent::TopologyError { .. } => "TopologyError",
        }
    }

    /// Check if event is a service event
    pub fn is_service_event(&self) -> bool {
        matches!(
            self,
            TopologyEvent::ServiceAdded { .. }
                | TopologyEvent::ServiceRemoved { .. }
                | TopologyEvent::ServiceUpdated { .. }
        )
    }

    /// Check if event is a dependency event
    pub fn is_dependency_event(&self) -> bool {
        matches!(
            self,
            TopologyEvent::DependencyAdded { .. } | TopologyEvent::DependencyRemoved { .. }
        )
    }

    /// Check if event is an error event
    pub fn is_error_event(&self) -> bool {
        matches!(self, TopologyEvent::TopologyError { .. })
    }

    /// Get service ID if applicable
    pub fn service_id(&self) -> Option<&ServiceId> {
        match self {
            TopologyEvent::ServiceAdded { service_id, .. } => Some(service_id),
            TopologyEvent::ServiceRemoved { service_id, .. } => Some(service_id),
            TopologyEvent::ServiceUpdated { service_id, .. } => Some(service_id),
            _ => None,
        }
    }

    /// Validate event data
    pub fn validate(&self) -> Result<()> {
        match self {
            TopologyEvent::ServiceAdded {
                service_id,
                service_name,
                ..
            } => {
                if service_id.to_string().is_empty() {
                    return Err(rustops_common::Error::InvalidInput {
                        message: "Service ID cannot be empty".to_string(),
                    });
                }
                if service_name.is_none() {
                    warn!("Service added without name");
                }
                Ok(())
            }
            TopologyEvent::ServiceRemoved { service_id } => {
                if service_id.to_string().is_empty() {
                    return Err(rustops_common::Error::InvalidInput {
                        message: "Service ID cannot be empty".to_string(),
                    });
                }
                Ok(())
            }
            TopologyEvent::ServiceUpdated { service_id, .. } => {
                if service_id.to_string().is_empty() {
                    return Err(rustops_common::Error::InvalidInput {
                        message: "Service ID cannot be empty".to_string(),
                    });
                }
                Ok(())
            }
            TopologyEvent::DependencyAdded {
                from_service_id,
                to_service_id,
                ..
            } => {
                if from_service_id.to_string().is_empty() {
                    return Err(rustops_common::Error::InvalidInput {
                        message: "From service ID cannot be empty".to_string(),
                    });
                }
                if to_service_id.to_string().is_empty() {
                    return Err(rustops_common::Error::InvalidInput {
                        message: "To service ID cannot be empty".to_string(),
                    });
                }
                if from_service_id == to_service_id {
                    return Err(rustops_common::Error::InvalidInput {
                        message: "Service cannot depend on itself".to_string(),
                    });
                }
                Ok(())
            }
            TopologyEvent::DependencyRemoved {
                from_service_id,
                to_service_id,
                ..
            } => {
                if from_service_id.to_string().is_empty() {
                    return Err(rustops_common::Error::InvalidInput {
                        message: "From service ID cannot be empty".to_string(),
                    });
                }
                if to_service_id.to_string().is_empty() {
                    return Err(rustops_common::Error::InvalidInput {
                        message: "To service ID cannot be empty".to_string(),
                    });
                }
                if from_service_id == to_service_id {
                    return Err(rustops_common::Error::InvalidInput {
                        message: "Service cannot depend on itself".to_string(),
                    });
                }
                Ok(())
            }
            TopologyEvent::TopologySynchronized {
                service_count,
                dependency_count,
                ..
            } => {
                if *service_count == 0 && *dependency_count == 0 {
                    warn!("Topology synchronized with no services or dependencies");
                }
                Ok(())
            }
            TopologyEvent::TopologyError { message, .. } => {
                if message.is_empty() {
                    return Err(rustops_common::Error::Config {
                        message: "Error message cannot be empty".to_string(),
                    });
                }
                Ok(())
            }
        }
    }
}

/// Event store for topology changes
pub trait TopologyEventStore: Send + Sync {
    /// Store an event
    fn store_event(&self, event: TopologyEvent) -> Result<()>;

    /// Store multiple events
    fn store_events(&self, events: Vec<TopologyEvent>) -> Result<()> {
        for event in events {
            self.store_event(event)?;
        }
        Ok(())
    }

    /// Get events for a service
    fn get_service_events(&self, service_id: &ServiceId) -> Result<Vec<TopologyEvent>>;

    /// Get events for a time range
    fn get_events_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<TopologyEvent>>;

    /// Get all events
    fn get_all_events(&self) -> Result<Vec<TopologyEvent>>;

    /// Get recent events (last N)
    fn get_recent_events(&self, count: usize) -> Result<Vec<TopologyEvent>>;

    /// Replay events to rebuild state
    fn replay_events(&self, graph: &mut ServiceGraph) -> Result<()>;

    /// Get event statistics
    fn get_statistics(&self) -> Result<EventStatistics>;

    /// Clean up old events
    fn cleanup_old_events(&self, retention_days: i64) -> Result<()>;
}

/// In-memory event store implementation
pub struct InMemoryEventStore {
    events: RwLock<Vec<TopologyEvent>>,
    service_index: RwLock<HashMap<ServiceId, Vec<usize>>>,
}

impl Clone for InMemoryEventStore {
    fn clone(&self) -> Self {
        let events = self.events.read().unwrap();
        let service_index = self.service_index.read().unwrap();
        Self {
            events: RwLock::new(events.clone()),
            service_index: RwLock::new(service_index.clone()),
        }
    }
}

impl InMemoryEventStore {
    /// Create new in-memory event store
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
            service_index: RwLock::new(HashMap::new()),
        }
    }

    /// Create with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: RwLock::new(Vec::with_capacity(capacity)),
            service_index: RwLock::new(HashMap::new()),
        }
    }
}

impl TopologyEventStore for InMemoryEventStore {
    fn store_event(&self, event: TopologyEvent) -> Result<()> {
        event.validate()?;

        let mut events = self
            .events
            .write()
            .map_err(|_| rustops_common::Error::Config {
                message: "Failed to acquire write lock for events".to_string(),
            })?;
        let event_index = events.len();
        events.push(event.clone());

        // Update service index for service events
        if let Some(service_id) = event.service_id() {
            let mut service_index =
                self.service_index
                    .write()
                    .map_err(|_| rustops_common::Error::Config {
                        message: "Failed to acquire write lock for service index".to_string(),
                    })?;
            service_index
                .entry(service_id.clone())
                .or_insert_with(Vec::new)
                .push(event_index);
        }

        debug!("Stored topology event: {}", event.event_type());
        Ok(())
    }

    fn get_service_events(&self, service_id: &ServiceId) -> Result<Vec<TopologyEvent>> {
        let service_index =
            self.service_index
                .read()
                .map_err(|_| rustops_common::Error::Config {
                    message: "Failed to acquire read lock for service index".to_string(),
                })?;
        let events = self
            .events
            .read()
            .map_err(|_| rustops_common::Error::Config {
                message: "Failed to acquire read lock for events".to_string(),
            })?;

        Ok(service_index
            .get(service_id)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&i| events.get(i).cloned())
                    .collect()
            })
            .unwrap_or_default())
    }

    fn get_events_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<TopologyEvent>> {
        let events = self
            .events
            .read()
            .map_err(|_| rustops_common::Error::Config {
                message: "Failed to acquire read lock for events".to_string(),
            })?;

        Ok(events
            .iter()
            .filter(|event| {
                let timestamp = event.timestamp();
                timestamp >= start && timestamp <= end
            })
            .cloned()
            .collect())
    }

    fn get_all_events(&self) -> Result<Vec<TopologyEvent>> {
        let events = self
            .events
            .read()
            .map_err(|_| rustops_common::Error::Config {
                message: "Failed to acquire read lock for events".to_string(),
            })?;
        Ok(events.clone())
    }

    fn get_recent_events(&self, count: usize) -> Result<Vec<TopologyEvent>> {
        let events = self
            .events
            .read()
            .map_err(|_| rustops_common::Error::Config {
                message: "Failed to acquire read lock for events".to_string(),
            })?;
        let total = events.len();
        let start = if count >= total { 0 } else { total - count };

        Ok(events[start..].to_vec())
    }

    fn replay_events(&self, graph: &mut ServiceGraph) -> Result<()> {
        let events = self
            .events
            .read()
            .map_err(|_| rustops_common::Error::Config {
                message: "Failed to acquire read lock for events".to_string(),
            })?;
        info!("Replaying {} topology events", events.len());

        for event in events.iter() {
            match event {
                TopologyEvent::ServiceAdded {
                    service_id,
                    service_name,
                    service_type,
                } => {
                    let now = Utc::now();
                    let service = ServiceNode {
                        id: *service_id,
                        name: service_name.clone(),
                        namespace: "default".to_string(), // Would be stored in event
                        cluster: "default".to_string(),   // Would be stored in event
                        service_type: *service_type,
                        replicas: 1,                 // Would be stored in event
                        labels: HashMap::new(),      // Would be stored in event
                        annotations: HashMap::new(), // Would be stored in event
                        health: crate::model::HealthStatus::Unknown,
                        created_at: now,
                        updated_at: now,
                    };

                    if let Err(e) = graph.add_service(service) {
                        warn!("Failed to replay service add event: {}", e);
                    }
                }
                TopologyEvent::ServiceRemoved { service_id } => {
                    if let Err(e) = graph.remove_service(service_id) {
                        warn!("Failed to replay service remove event: {}", e);
                    }
                }
                TopologyEvent::ServiceUpdated {
                    current_service, ..
                } => {
                    if let Err(e) = graph.add_service(current_service.clone()) {
                        warn!("Failed to replay service update event: {}", e);
                    }
                }
                TopologyEvent::DependencyAdded {
                    from_service_id,
                    to_service_id,
                    edge_type,
                } => {
                    let edge = DependencyEdge {
                        from: *from_service_id,
                        to: *to_service_id,
                        edge_type: *edge_type,
                        metadata: HashMap::new(), // Would be stored in event
                    };

                    if let Err(e) = graph.add_dependency(*from_service_id, *to_service_id, edge) {
                        warn!("Failed to replay dependency add event: {}", e);
                    }
                }
                TopologyEvent::DependencyRemoved {
                    from_service_id,
                    to_service_id,
                    edge_type,
                } => {
                    if let Err(e) =
                        graph.remove_dependency(*from_service_id, *to_service_id, *edge_type)
                    {
                        warn!("Failed to replay dependency remove event: {}", e);
                    }
                }
                _ => {
                    // Skip other events for replay
                }
            }
        }

        info!("Topology replay completed");
        Ok(())
    }

    fn get_statistics(&self) -> Result<EventStatistics> {
        let events = self
            .events
            .read()
            .map_err(|_| rustops_common::Error::Config {
                message: "Failed to acquire read lock for events".to_string(),
            })?;

        let mut service_events = 0;
        let mut dependency_events = 0;
        let mut error_events = 0;
        let mut timestamp = None;

        for event in events.iter() {
            match event {
                TopologyEvent::ServiceAdded { .. }
                | TopologyEvent::ServiceRemoved { .. }
                | TopologyEvent::ServiceUpdated { .. } => {
                    service_events += 1;
                }
                TopologyEvent::DependencyAdded { .. } | TopologyEvent::DependencyRemoved { .. } => {
                    dependency_events += 1;
                }
                TopologyEvent::TopologyError { .. } => {
                    error_events += 1;
                }
                TopologyEvent::TopologySynchronized { timestamp: ts, .. } => {
                    timestamp = Some(*ts);
                }
            }
        }

        let latest_event = events.last().map(|e| e.timestamp());

        Ok(EventStatistics {
            total_events: events.len(),
            service_events,
            dependency_events,
            error_events,
            latest_event,
            earliest_event: events.first().map(|e| e.timestamp()),
            last_sync_timestamp: timestamp,
        })
    }

    fn cleanup_old_events(&self, _retention_days: i64) -> Result<()> {
        // In-memory store doesn't persist, so no cleanup needed
        debug!("Skipping event cleanup for in-memory store");
        Ok(())
    }
}

/// Event statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStatistics {
    /// Total number of events
    pub total_events: usize,
    /// Number of service events
    pub service_events: usize,
    /// Number of dependency events
    pub dependency_events: usize,
    /// Number of error events
    pub error_events: usize,
    /// Timestamp of latest event
    pub latest_event: Option<DateTime<Utc>>,
    /// Timestamp of earliest event
    pub earliest_event: Option<DateTime<Utc>>,
    /// Last sync timestamp
    pub last_sync_timestamp: Option<DateTime<Utc>>,
}

impl EventStatistics {
    /// Check if any events exist
    pub fn has_events(&self) -> bool {
        self.total_events > 0
    }

    /// Get error rate
    pub fn error_rate(&self) -> f64 {
        if self.total_events == 0 {
            0.0
        } else {
            self.error_events as f64 / self.total_events as f64
        }
    }

    /// Get service to dependency ratio
    pub fn service_dependency_ratio(&self) -> f64 {
        if self.dependency_events == 0 {
            0.0
        } else {
            self.service_events as f64 / self.dependency_events as f64
        }
    }
}

/// Event emitter for topology changes
pub struct EventEmitter {
    event_store: Box<dyn TopologyEventStore>,
}

impl EventEmitter {
    /// Create new event emitter
    pub fn new(event_store: Box<dyn TopologyEventStore>) -> Self {
        Self { event_store }
    }

    /// Emit service added event
    pub fn emit_service_added(
        &self,
        service_id: ServiceId,
        service_name: Option<String>,
        service_type: ServiceType,
    ) -> Result<()> {
        let event = TopologyEvent::ServiceAdded {
            service_id,
            service_name,
            service_type,
        };
        self.event_store.store_event(event)
    }

    /// Emit service removed event
    pub fn emit_service_removed(&self, service_id: ServiceId) -> Result<()> {
        let event = TopologyEvent::ServiceRemoved { service_id };
        self.event_store.store_event(event)
    }

    /// Emit service updated event
    pub fn emit_service_updated(
        &self,
        service_id: ServiceId,
        previous_service: Option<ServiceNode>,
        current_service: ServiceNode,
    ) -> Result<()> {
        let event = TopologyEvent::ServiceUpdated {
            service_id,
            previous_service,
            current_service,
        };
        self.event_store.store_event(event)
    }

    /// Emit dependency added event
    pub fn emit_dependency_added(
        &self,
        from_service_id: ServiceId,
        to_service_id: ServiceId,
        edge_type: DependencyType,
    ) -> Result<()> {
        let event = TopologyEvent::DependencyAdded {
            from_service_id,
            to_service_id,
            edge_type,
        };
        self.event_store.store_event(event)
    }

    /// Emit dependency removed event
    pub fn emit_dependency_removed(
        &self,
        from_service_id: ServiceId,
        to_service_id: ServiceId,
        edge_type: DependencyType,
    ) -> Result<()> {
        let event = TopologyEvent::DependencyRemoved {
            from_service_id,
            to_service_id,
            edge_type,
        };
        self.event_store.store_event(event)
    }

    /// Emit topology synchronized event
    pub fn emit_topology_synchronized(
        &self,
        service_count: usize,
        dependency_count: usize,
    ) -> Result<()> {
        let event = TopologyEvent::TopologySynchronized {
            timestamp: Utc::now(),
            service_count,
            dependency_count,
        };
        self.event_store.store_event(event)
    }

    /// Emit error event
    pub fn emit_error(&self, message: String, context: HashMap<String, String>) -> Result<()> {
        let event = TopologyEvent::TopologyError { message, context };
        self.event_store.store_event(event)
    }

    /// Get event statistics
    pub fn get_statistics(&self) -> Result<EventStatistics> {
        self.event_store.get_statistics()
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_validation() {
        let valid_event = TopologyEvent::ServiceAdded {
            service_id: ServiceId::new(),
            service_name: Some("test-service".to_string()),
            service_type: ServiceType::Deployment,
        };

        assert!(valid_event.validate().is_ok());

        // Test invalid event
        let invalid_event = TopologyEvent::ServiceAdded {
            service_id: ServiceId::new(),
            service_name: None,
            service_type: ServiceType::Deployment,
        };

        // This should be Ok (warning, not error)
        assert!(invalid_event.validate().is_ok());
    }

    #[test]
    fn test_event_type_detection() {
        let service_event = TopologyEvent::ServiceAdded {
            service_id: ServiceId::new(),
            service_name: None,
            service_type: ServiceType::Deployment,
        };
        assert!(service_event.is_service_event());
        assert!(!service_event.is_dependency_event());

        let dependency_event = TopologyEvent::DependencyAdded {
            from_service_id: ServiceId::new(),
            to_service_id: ServiceId::new(),
            edge_type: DependencyType::Calls,
        };
        assert!(!dependency_event.is_service_event());
        assert!(dependency_event.is_dependency_event());
    }

    #[tokio::test]
    async fn test_event_store() {
        let store = InMemoryEventStore::new();

        // Store events
        let service_id = ServiceId::new();
        store
            .store_event(TopologyEvent::ServiceAdded {
                service_id,
                service_name: Some("test".to_string()),
                service_type: ServiceType::Deployment,
            })
            .unwrap();

        store
            .store_event(TopologyEvent::ServiceRemoved { service_id })
            .unwrap();

        // Get service events
        let service_events = store.get_service_events(&service_id).unwrap();
        assert_eq!(service_events.len(), 2);

        // Get all events
        let all_events = store.get_all_events().unwrap();
        assert_eq!(all_events.len(), 2);

        // Get statistics
        let stats = store.get_statistics().unwrap();
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.service_events, 2);
    }

    #[tokio::test]
    async fn test_event_emitter() {
        let store = InMemoryEventStore::new();
        let emitter = EventEmitter::new(Box::new(store));

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

        let stats = emitter.get_statistics().unwrap();
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.service_events, 1);
        assert_eq!(stats.dependency_events, 1);
    }
}
