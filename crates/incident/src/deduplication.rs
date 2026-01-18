//! # Alert deduplication
//!
//! Removes duplicate alerts using fingerprinting and time windows.

use chrono::{DateTime, Duration, Utc};
use rustops_common::{AlertId, ServiceId, Severity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Duration as StdDuration;

/// Alert fingerprint - unique identifier for alert type
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Fingerprint(u64);

impl Fingerprint {
    /// Create a fingerprint from an alert signature
    pub fn new(
        service: &str,
        alert_type: &str,
        resource: Option<&str>,
        metric_name: Option<&str>,
    ) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        service.hash(&mut hasher);
        alert_type.hash(&mut hasher);
        if let Some(r) = resource {
            r.hash(&mut hasher);
        }
        if let Some(m) = metric_name {
            m.hash(&mut hasher);
        }
        Self(hasher.finish())
    }
}

/// Normalized alert for deduplication
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedAlert {
    pub alert_id: AlertId,
    pub timestamp: DateTime<Utc>,
    pub service: String,
    pub alert_type: String,
    pub severity: Severity,
    pub title: String,
    pub resource: Option<String>,
    pub metric_name: Option<String>,
    pub metric_value: Option<f64>,
    pub threshold: Option<f64>,
    pub labels: HashMap<String, String>,
}

/// Alert deduplicator - removes duplicates within time window
pub struct AlertDeduplicator {
    /// Time window for deduplication (default 5 minutes)
    window: Duration,
    /// Track seen fingerprints and first seen time
    seen: HashMap<Fingerprint, DateTime<Utc>>,
    /// Fingerprinter for creating fingerprints
    fingerprinter: Fingerprinter,
}

impl AlertDeduplicator {
    /// Create a new deduplicator
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            seen: HashMap::new(),
            fingerprinter: Fingerprinter,
        }
    }

    /// Create with default 5-minute window
    pub fn default() -> Self {
        Self::new(Duration::minutes(5))
    }

    /// Deduplicate alerts, returning only unique alerts
    pub fn deduplicate(&mut self, alerts: Vec<NormalizedAlert>) -> Vec<NormalizedAlert> {
        let mut unique = Vec::new();
        let now = Utc::now();

        // Clean up old entries
        self.seen.retain(|_, first_seen| {
            now.signed_duration_since(*first_seen).num_seconds() < self.window.num_seconds()
        });

        for alert in alerts {
            let fp = self.fingerprinter.fingerprint(&alert);

            match self.seen.get(&fp) {
                Some(first_seen) => {
                    let age = alert.timestamp.signed_duration_since(*first_seen);
                    if age.num_seconds() < self.window.num_seconds() {
                        // Duplicate
                        tracing::debug!(
                            "Deduplicated alert: {} (age: {}s)",
                            alert.alert_id,
                            age.num_seconds()
                        );
                        continue;
                    }
                }
                None => {
                    self.seen.insert(fp, alert.timestamp);
                }
            }

            unique.push(alert);
        }

        unique
    }
}

/// Fingerprinter for creating alert fingerprints
pub struct Fingerprinter;

impl Fingerprinter {
    /// Create a fingerprint for an alert
    pub fn fingerprint(&self, alert: &NormalizedAlert) -> Fingerprint {
        Fingerprint::new(
            &alert.service,
            &alert.alert_type,
            alert.resource.as_deref(),
            alert.metric_name.as_deref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_alert(
        service: &str,
        alert_type: &str,
        timestamp: DateTime<Utc>,
    ) -> NormalizedAlert {
        NormalizedAlert {
            alert_id: AlertId::new(),
            timestamp,
            service: service.to_string(),
            alert_type: alert_type.to_string(),
            severity: Severity::Major,
            title: "Test alert".to_string(),
            resource: Some("test-resource".to_string()),
            metric_name: Some("cpu_usage".to_string()),
            metric_value: Some(95.0),
            threshold: Some(80.0),
            labels: HashMap::new(),
        }
    }

    #[test]
    fn test_fingerprinter() {
        let fingerprinter = Fingerprinter;

        let alert1 = create_test_alert("service1", "high_cpu", Utc::now());
        let alert2 = create_test_alert("service1", "high_cpu", Utc::now());

        let fp1 = fingerprinter.fingerprint(&alert1);
        let fp2 = fingerprinter.fingerprint(&alert2);

        // Same service and alert_type should have same fingerprint
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_deduplication() {
        let mut deduplicator = AlertDeduplicator::default();

        let now = Utc::now();
        let alerts = vec![
            create_test_alert("service1", "high_cpu", now),
            create_test_alert("service1", "high_cpu", now + Duration::seconds(1)),
            create_test_alert("service1", "high_cpu", now + Duration::seconds(2)),
            create_test_alert("service2", "high_memory", now),
        ];

        let unique = deduplicator.deduplicate(alerts);

        // Should deduplicate to 2 unique alerts
        assert_eq!(unique.len(), 2);
    }

    #[test]
    fn test_deduplication_window_expires() {
        let mut deduplicator = AlertDeduplicator::new(Duration::seconds(2));

        let now = Utc::now();
        let alerts = vec![
            create_test_alert("service1", "high_cpu", now),
            create_test_alert("service1", "high_cpu", now + Duration::seconds(3)),
        ];

        let unique = deduplicator.deduplicate(alerts);

        // Window expired, both should be unique
        assert_eq!(unique.len(), 2);
    }
}
