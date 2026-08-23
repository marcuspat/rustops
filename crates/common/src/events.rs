//! # Domain Events
//!
//! Domain events represent significant business events that occur within the system.
//! Following Domain-Driven Design (DDD) and Event Sourcing patterns.

use crate::{AlertId, AnomalyId, IncidentId, MetricId, ServiceId, TraceId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A domain event that occurred in the system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainEvent {
    /// Unique event ID
    pub event_id: Uuid,
    /// Event type for routing/handling
    pub event_type: EventType,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
    /// Causation ID - what caused this event
    pub causation_id: Option<Uuid>,
    /// Correlation ID - groups related events
    pub correlation_id: Option<Uuid>,
    /// Event version for schema evolution
    pub version: u32,
    /// Event payload
    pub payload: EventPayload,
}

/// Types of domain events
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Telemetry received
    TelemetryReceived,
    /// Anomaly detected
    AnomalyDetected,
    /// Alert created
    AlertCreated,
    /// Alert updated
    AlertUpdated,
    /// Alert resolved
    AlertResolved,
    /// Incident created
    IncidentCreated,
    /// Incident updated
    IncidentUpdated,
    /// Incident resolved
    IncidentResolved,
    /// Service added to topology
    ServiceAdded,
    /// Service removed from topology
    ServiceRemoved,
    /// Service dependency discovered
    DependencyDiscovered,
}

/// Event payload containing event-specific data
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPayload {
    /// Anomaly was detected
    AnomalyDetected {
        /// Identifier of the detected anomaly.
        anomaly_id: AnomalyId,
        /// Metric this event refers to.
        metric_id: MetricId,
        /// Anomaly score in [0, 1].
        score: f64,
        /// Detector confidence in [0, 1].
        confidence: f64,
        /// Human-readable explanation.
        explanation: String,
    },
    /// Alert was created
    AlertCreated {
        /// Identifier of the alert.
        alert_id: AlertId,
        /// Short human-readable title.
        title: String,
        /// Severity classification.
        severity: Severity,
        /// Service this event concerns.
        service_id: ServiceId,
    },
    /// Alert was updated
    AlertUpdated {
        /// Identifier of the alert.
        alert_id: AlertId,
        /// Descriptions of what changed.
        changes: Vec<String>,
    },
    /// Alert was resolved
    AlertResolved {
        /// Identifier of the alert.
        alert_id: AlertId,
        /// How it was resolved.
        resolution: String,
    },
    /// Incident was created
    IncidentCreated {
        /// Identifier of the incident.
        incident_id: IncidentId,
        /// Short human-readable title.
        title: String,
        /// Severity classification.
        severity: Severity,
        /// Alerts grouped into this incident.
        alert_ids: Vec<AlertId>,
    },
    /// Incident was updated
    IncidentUpdated {
        /// Identifier of the incident.
        incident_id: IncidentId,
        /// Descriptions of what changed.
        changes: Vec<String>,
    },
    /// Incident was resolved
    IncidentResolved {
        /// Identifier of the incident.
        incident_id: IncidentId,
        /// How it was resolved.
        resolution: String,
        /// Time to resolution, in seconds.
        mttr_seconds: u64,
    },
    /// Service dependency discovered
    DependencyDiscovered {
        /// Dependent (calling) service.
        from_service: ServiceId,
        /// Depended-on (called) service.
        to_service: ServiceId,
        /// Kind of dependency.
        dependency_type: DependencyType,
    },
    /// Trace analysis completed
    TraceAnalysisCompleted {
        /// Identifier of the trace.
        trace_id: TraceId,
        /// Number of spans in the trace.
        span_count: usize,
        /// Number of spans that errored.
        error_count: usize,
        /// End-to-end trace latency in milliseconds.
        latency_ms: u64,
    },
    /// Metric threshold breached
    MetricThresholdBreached {
        /// Metric this event refers to.
        metric_id: MetricId,
        /// Configured threshold that was crossed.
        threshold: f64,
        /// Observed value that crossed the threshold.
        actual_value: f64,
        /// Service this event concerns.
        service_id: ServiceId,
    },
    /// Unknown event payload for forward compatibility
    Unknown {
        /// Name of the custom event type.
        type_name: String,
        /// Arbitrary event payload.
        data: serde_json::Value,
    },
}

/// Severity levels for alerts and incidents
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    /// Informational - no action required
    Info = 0,
    /// Warning - potential issue
    Warning = 1,
    /// Minor - degraded performance
    Minor = 2,
    /// Major - significant impact
    Major = 3,
    /// Critical - severe impact
    Critical = 4,
}

impl Severity {
    /// Convert from integer
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Severity::Info),
            1 => Some(Severity::Warning),
            2 => Some(Severity::Minor),
            3 => Some(Severity::Major),
            4 => Some(Severity::Critical),
            _ => None,
        }
    }

    /// Convert to integer
    pub fn to_i32(self) -> i32 {
        self as i32
    }

    /// Check if severity requires immediate action
    pub fn requires_immediate_action(self) -> bool {
        self >= Severity::Major
    }
}

/// Type of service dependency
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    /// Synchronous HTTP call
    Http,
    /// Asynchronous message
    MessageQueue,
    /// Database connection
    Database,
    /// Cache connection
    Cache,
    /// External service
    External,
}

impl DomainEvent {
    /// Create a new domain event
    pub fn new(event_type: EventType, payload: EventPayload) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type,
            timestamp: Utc::now(),
            causation_id: None,
            correlation_id: None,
            version: 1,
            payload,
        }
    }

    /// Set the causation ID (what caused this event)
    pub fn with_causation(mut self, causation_id: Uuid) -> Self {
        self.causation_id = Some(causation_id);
        self
    }

    /// Set the correlation ID (groups related events)
    pub fn with_correlation(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Create an event for testing
    #[cfg(test)]
    pub fn test_event(event_type: EventType) -> Self {
        Self::new(
            event_type,
            EventPayload::MetricThresholdBreached {
                metric_id: MetricId::new(),
                threshold: 100.0,
                actual_value: 150.0,
                service_id: ServiceId::new(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_event_creation() {
        let event = DomainEvent::new(
            EventType::AnomalyDetected,
            EventPayload::AnomalyDetected {
                anomaly_id: AnomalyId::new(),
                metric_id: MetricId::new(),
                score: 0.95,
                confidence: 0.9,
                explanation: "CPU spike detected".to_string(),
            },
        );

        assert_eq!(event.event_type, EventType::AnomalyDetected);
        assert_eq!(event.version, 1);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::Major);
        assert!(Severity::Warning < Severity::Minor);
    }

    #[test]
    fn test_severity_requires_action() {
        assert!(Severity::Critical.requires_immediate_action());
        assert!(Severity::Major.requires_immediate_action());
        assert!(!Severity::Warning.requires_immediate_action());
    }

    #[test]
    fn test_event_serialization() {
        let event = DomainEvent::test_event(EventType::AnomalyDetected);
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.event_id, deserialized.event_id);
    }
}
