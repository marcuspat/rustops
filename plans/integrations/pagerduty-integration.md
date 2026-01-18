# PagerDuty Integration Guide

**Integration Type**: Alert Routing / On-Call Management
**Priority**: Critical (Phase 1)
**Status**: Design

---

## Overview

PagerDuty is the de facto standard for on-call management and alert routing. RustOps integrates with PagerDuty for intelligent alert escalation, on-call synchronization, and bi-directional incident management.

### Integration Capabilities

| Capability | Description | Use Case |
|------------|-------------|----------|
| **Alert Creation** | Create alerts from correlated incidents | Critical alert escalation |
| **Incident Creation** | Create incidents from high-severity alerts | Major incident workflow |
| **On-Call Lookup** | Query current on-call engineer | Auto-assignment |
| **Escalation Policy** | Respect PagerDuty escalation policies | Smart routing |
| **Alert Acknowledge** | Sync acknowledgment status | Coordination |
| **Alert Resolution** | Resolve alerts on remediation | Close the loop |
| **Webhook Events** | Receive status updates from PagerDuty | Bi-directional sync |

### Why PagerDuty is Top Priority

- **Standard for on-call** management in DevOps
- **Rich REST API** for incident/escalation management
- **Webhook support** for bi-directional events
- **Escalation policies** that integrate with alert severity
- **Mobile app** for instant engineer notification

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    RustOps - PagerDuty Integration                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                        PagerDuty API                             │   │
│  │  ┌───────────────────────────────────────────────────────────┐  │   │
│  │  │                   REST API / v2                            │  │   │
│  │  │  - Alerts (create, acknowledge, resolve)                  │  │   │
│  │  │  - Incidents (create, update, query)                      │  │   │
│  │  │  - On-call (lookup schedules, rotations)                  │  │   │
│  │  │  - Escalation Policies (query, apply)                     │  │   │
│  │  │  - Services (list, query)                                 │  │   │
│  │  └───────────────────────────────────────────────────────────┘  │   │
│  │                          │                                       │   │
│  │              API Token │ OAuth 2.0                              │   │
│  └────────────────────────┼───────────────────────────────────────┘   │
│                            │                                             │
│                            ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                 RustOps PagerDuty Adapter                       │   │
│  │  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────┐  │   │
│  │  │ Alert Manager    │  │ On-Call Lookup   │  │ Webhook      │  │   │
│  │  │  - Create alert  │  │  - Who's on call │  │  Handler     │  │   │
│  │  │  - Acknowledge   │  │  - Get contact   │  │  - Status    │  │   │
│  │  │  - Resolve       │  │  - Escalate      │  │    updates   │  │   │
│  │  └──────────────────┘  └──────────────────┘  └──────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                           │                                             │
│                           ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │              RustOps Alert → PagerDuty Pipeline                  │   │
│  │  1. Correlate Alert                                             │   │
│  │  2. Determine Severity & Routing                                 │   │
│  │  3. Query On-Call Engineer                                       │   │
│  │  4. Create/Update PagerDuty Incident                             │   │
│  │  5. Monitor Acknowledge/Resolution                               │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation

### Rust Dependencies

```toml
[dependencies]
# HTTP client
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Time handling
chrono = { version = "0.4", features = ["serde"] }
```

### PagerDuty Client Setup

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// PagerDuty adapter configuration
#[derive(Debug, Clone)]
pub struct PagerDutyConfig {
    pub api_token: String,
    pub base_url: String,
    pub default_service_id: Option<String>,
    pub default_escalation_policy_id: Option<String>,
    pub timeout: std::time::Duration,
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_concurrent: usize,
    pub requests_per_second: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 20,
            requests_per_second: 20,
        }
    }
}

/// PagerDuty adapter
pub struct PagerDutyAdapter {
    config: PagerDutyConfig,
    client: Client,
    rate_limiter: Arc<tokio::sync::Semaphore>,
}

impl PagerDutyAdapter {
    /// Create new PagerDuty adapter
    pub fn new(config: PagerDutyConfig) -> Result<Self, PagerDutyError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| PagerDutyError::Client(e.to_string()))?;

        let rate_limiter = Arc::new(tokio::sync::Semaphore::new(config.rate_limit.max_concurrent));

        Ok(Self {
            config,
            client,
            rate_limiter,
        })
    }

    /// Build API URL
    fn api_url(&self, path: &str) -> String {
        format!("{}/{}", self.config.base_url.trim_end_matches('/'), path)
    }

    /// Get request headers with auth
    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Accept",
            reqwest::header::HeaderValue::from_static("application/vnd.pagerduty+json;version=2"),
        );
        headers.insert(
            "Content-Type",
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "Authorization",
            reqwest::header::HeaderValue::from_str(&format!("Token token={}", self.config.api_token))
                .unwrap(),
        );

        headers
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthStatus, PagerDutyError> {
        let url = self.api_url("users?limit=1");

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers();

        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| PagerDutyError::Request(e.to_string()))?;

        if response.status().is_success() {
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Unhealthy)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PagerDutyError {
    #[error("Client error: {0}")]
    Client(String),

    #[error("Request error: {0}")]
    Request(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Rate limit exceeded")]
    RateLimit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}
```

### Alert Management

```rust
/// PagerDuty alert payload
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PagerDutyAlert {
    #[serde(rename = "routing_key")]
    pub routing_key: String,  // Events API v2 integration key
    #[serde(rename = "event_action")]
    pub event_action: EventAction,
    #[serde(rename = "dedup_key")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedup_key: Option<String>,  // For updating existing alerts
    pub payload: AlertPayload,
    #[serde(rename = "client")]
    pub client: String,
    #[serde(rename = "client_url")]
    pub client_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum EventAction {
    Trigger,
    Acknowledge,
    Resolve,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlertPayload {
    pub summary: String,
    pub severity: AlertSeverity,
    pub source: String,
    pub timestamp: String,
    #[serde(rename = "custom_details")]
    pub custom_details: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Critical,
    Error,
    Warning,
    Info,
}

/// RustOps alert for PagerDuty
#[derive(Debug, Clone)]
pub struct RustOpsAlert {
    pub title: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub source: String,
    pub service: String,
    pub correlation_id: String,
    pub metadata: AlertMetadata,
    pub dedup_key: Option<String>,  // For updating existing alerts
}

#[derive(Debug, Clone)]
pub struct AlertMetadata {
    pub alert_id: String,
    pub affected_resources: Vec<String>,
    pub detection_time: chrono::DateTime<chrono::Utc>,
    pub first_seen: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub occurrences: u32,
    pub root_cause_hypothesis: Option<String>,
    pub auto_remediation_attempted: bool,
}

/// Alert management operations
impl PagerDutyAdapter {
    /// Create or update alert via Events API v2
    pub async fn send_alert(
        &self,
        routing_key: &str,
        alert: RustOpsAlert,
        action: EventAction,
    ) -> Result<String, PagerDutyError> {
        let url = "https://events.pagerduty.com/v2/enqueue";

        let pd_alert = PagerDutyAlert {
            routing_key: routing_key.to_string(),
            event_action: action,
            dedup_key: alert.dedup_key.clone(),
            payload: AlertPayload {
                summary: alert.title,
                severity: alert.severity,
                source: alert.source,
                timestamp: alert.metadata.last_seen.to_rfc3339(),
                custom_details: self.build_custom_details(&alert),
                group: Some(alert.service),
            },
            client: "RustOps AIOps".to_string(),
            client_url: Some(format!(
                "https://rustops.example.com/alerts/{}",
                alert.correlation_id
            )),
        };

        let _permit = self.rate_limiter.acquire().await;

        let response = self.client
            .post(url)
            .json(&pd_alert)
            .send()
            .await
            .map_err(|e| PagerDutyError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| PagerDutyError::Api(e.to_string()))?;

            let dedup_key = result
                .get("dedup_key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| PagerDutyError::Api("No dedup_key returned".into()))?;

            tracing::info!(
                "PagerDuty alert sent: {} (dedup_key: {})",
                alert.title,
                dedup_key
            );

            Ok(dedup_key.to_string())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(PagerDutyError::Api(format!("{}: {}", status, body)))
        }
    }

    /// Create incident via REST API (for more complex scenarios)
    pub async fn create_incident(
        &self,
        incident: RustOpsIncident,
    ) -> Result<PagerDutyIncident, PagerDutyError> {
        let url = self.api_url("incidents");

        let pd_incident = serde_json::json!({
            "incident": {
                "type": "incident",
                "title": incident.title,
                "service": {
                    "id": incident.service_id,
                    "type": "service_reference"
                },
                "priority": incident.priority_id.map(|id| serde_json::json!({
                    "id": id,
                    "type": "priority_reference"
                })),
                "urgency": incident.urgency,
                "body": {
                    "type": "incident_body",
                    "details": incident.description
                },
                "escalation_policy": incident.escalation_policy_id.map(|id| serde_json::json!({
                    "id": id,
                    "type": "escalation_policy_reference"
                }))
            }
        });

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers();

        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&pd_incident)
            .send()
            .await
            .map_err(|e| PagerDutyError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| PagerDutyError::Api(e.to_string()))?;

            let incident = result.get("incident")
                .ok_or_else(|| PagerDutyError::Api("No incident returned".into()))?;

            Ok(PagerDutyIncident {
                id: incident.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                title: incident.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                status: incident.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            })
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(PagerDutyError::Api(format!("{}: {}", status, body)))
        }
    }

    /// Acknowledge alert
    pub async fn acknowledge_alert(
        &self,
        routing_key: &str,
        dedup_key: &str,
    ) -> Result<(), PagerDutyError> {
        // Send acknowledge event with same dedup_key
        let alert = RustOpsAlert {
            title: String::new(),  // Not used for acknowledge
            description: String::new(),
            severity: AlertSeverity::Info,
            source: "RustOps".to_string(),
            service: String::new(),
            correlation_id: String::new(),
            metadata: AlertMetadata {
                alert_id: String::new(),
                affected_resources: vec![],
                detection_time: chrono::Utc::now(),
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                occurrences: 0,
                root_cause_hypothesis: None,
                auto_remediation_attempted: false,
            },
            dedup_key: Some(dedup_key.to_string()),
        };

        self.send_alert(routing_key, alert, EventAction::Acknowledge).await?;
        Ok(())
    }

    /// Resolve alert
    pub async fn resolve_alert(
        &self,
        routing_key: &str,
        dedup_key: &str,
    ) -> Result<(), PagerDutyError> {
        let alert = RustOpsAlert {
            title: String::new(),
            description: String::new(),
            severity: AlertSeverity::Info,
            source: "RustOps".to_string(),
            service: String::new(),
            correlation_id: String::new(),
            metadata: AlertMetadata {
                alert_id: String::new(),
                affected_resources: vec![],
                detection_time: chrono::Utc::now(),
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                occurrences: 0,
                root_cause_hypothesis: None,
                auto_remediation_attempted: false,
            },
            dedup_key: Some(dedup_key.to_string()),
        };

        self.send_alert(routing_key, alert, EventAction::Resolve).await?;
        Ok(())
    }

    /// Build custom details for alert
    fn build_custom_details(&self, alert: &RustOpsAlert) -> serde_json::Value {
        serde_json::json!({
            "correlation_id": alert.correlation_id,
            "alert_id": alert.metadata.alert_id,
            "service": alert.service,
            "affected_resources": alert.metadata.affected_resources,
            "detection_time": alert.metadata.detection_time.to_rfc3339(),
            "first_seen": alert.metadata.first_seen.to_rfc3339(),
            "last_seen": alert.metadata.last_seen.to_rfc3339(),
            "occurrences": alert.metadata.occurrences,
            "root_cause_hypothesis": alert.metadata.root_cause_hypothesis,
            "auto_remediation_attempted": alert.metadata.auto_remediation_attempted,
            "description": alert.description,
        })
    }
}

/// PagerDuty incident for REST API creation
#[derive(Debug, Clone)]
pub struct RustOpsIncident {
    pub title: String,
    pub description: String,
    pub service_id: String,
    pub priority_id: Option<String>,
    pub urgency: IncidentUrgency,
    pub escalation_policy_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum IncidentUrgency {
    High,
    Low,
}

/// PagerDuty incident response
#[derive(Debug, Clone)]
pub struct PagerDutyIncident {
    pub id: String,
    pub title: String,
    pub status: String,
}
```

### On-Call Lookup

```rust
/// On-call information
#[derive(Debug, Clone, Deserialize)]
pub struct OnCallInfo {
    pub user: User,
    pub schedule: Option<Schedule>,
    pub escalation_policy: Option<EscalationPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EscalationPolicy {
    pub id: String,
    pub name: String,
}

/// On-call operations
impl PagerDutyAdapter {
    /// Get current on-call for a service
    pub async fn get_on_call_for_service(
        &self,
        service_id: &str,
    ) -> Result<Vec<OnCallInfo>, PagerDutyError> {
        let url = self.api_url(&format!(
            "oncalls?service_ids[]={}&include[]=users&include[]=schedules&include[]=escalation_policies",
            service_id
        ));

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers();

        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| PagerDutyError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| PagerDutyError::Api(e.to_string()))?;

            let oncalls = result.get("oncalls")
                .and_then(|v| v.as_array())
                .ok_or_else(|| PagerDutyError::Api("Invalid response".into()))?;

            let mut oncall_info = Vec::new();

            for oncall in oncalls {
                if let Some(user) = oncall.get("user") {
                    let user_obj = User {
                        id: user.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        name: user.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        email: user.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    };

                    let schedule = oncall.get("schedule").map(|s| Schedule {
                        id: s.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        name: s.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    });

                    let escalation_policy = oncall.get("escalation_policy").map(|ep| EscalationPolicy {
                        id: ep.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        name: ep.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    });

                    oncall_info.push(OnCallInfo {
                        user: user_obj,
                        schedule,
                        escalation_policy,
                    });
                }
            }

            Ok(oncall_info)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(PagerDutyError::Api(body))
        }
    }

    /// Get all on-call engineers
    pub async fn get_all_on_call(
        &self,
    ) -> Result<Vec<OnCallInfo>, PagerDutyError> {
        let url = self.api_url(
            "oncalls?include[]=users&include[]=schedules&include[]=escalation_policies"
        );

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers();

        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| PagerDutyError::Request(e.to_string()))?;

        if response.status().is_success() {
            // Parse similar to get_on_call_for_service
            unimplemented!()
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(PagerDutyError::Api(body))
        }
    }

    /// Get services
    pub async fn get_services(
        &self,
    ) -> Result<Vec<PagerDutyService>, PagerDutyError> {
        let url = self.api_url("services?limit=100");

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers();

        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| PagerDutyError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| PagerDutyError::Api(e.to_string()))?;

            let services = result.get("services")
                .and_then(|v| v.as_array())
                .ok_or_else(|| PagerDutyError::Api("Invalid response".into()))?;

            let mut service_list = Vec::new();

            for service in services {
                service_list.push(PagerDutyService {
                    id: service.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    name: service.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    status: service.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                });
            }

            Ok(service_list)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(PagerDutyError::Api(body))
        }
    }
}

#[derive(Debug, Clone)]
pub struct PagerDutyService {
    pub id: String,
    pub name: String,
    pub status: String,
}
```

### Webhook Handler

```rust
use axum::{Json, extract::State};
use serde::Deserialize;

/// PagerDuty webhook event
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PagerDutyWebhookEvent {
    IncidentTriggered { data: WebhookData },
    IncidentAcknowledged { data: WebhookData },
    IncidentResolved { data: WebhookData },
    IncidentAssigned { data: WebhookData },
    IncidentEscalated { data: WebhookData },
}

#[derive(Debug, Deserialize)]
pub struct WebhookData {
    pub incident: serde_json::Value,
}

/// Webhook handler for PagerDuty events
pub async fn handle_pagerduty_webhook(
    State(adapter): State<Arc<PagerDutyAdapter>>,
    Json(event): Json<PagerDutyWebhookEvent>,
) -> Result<Json<serde_json::Value>, PagerDutyError> {
    match event {
        PagerDutyWebhookEvent::IncidentAcknowledged { data } => {
            tracing::info!("PagerDuty incident acknowledged: {:?}", data.incident);
            // Update RustOps incident state
        }
        PagerDutyWebhookEvent::IncidentResolved { data } => {
            tracing::info!("PagerDuty incident resolved: {:?}", data.incident);
            // Update RustOps incident state
        }
        PagerDutyWebhookEvent::IncidentAssigned { data } => {
            tracing::info!("PagerDuty incident assigned: {:?}", data.incident);
            // Update assignment in RustOps
        }
        _ => {
            tracing::debug!("Unhandled PagerDuty event type");
        }
    }

    Ok(Json(serde_json::json!({
        "status": "accepted"
    })))
}
```

---

## Configuration

### PagerDuty Configuration

```yaml
integrations:
  pagerduty:
    enabled: true

    # API authentication
    api:
      token: "${PAGERDUTY_API_TOKEN}"  # From Vault
      base_url: "https://api.pagerduty.com"

    # Events API v2 integration keys
    events:
      # Default routing key (can be overridden per service)
      default_routing_key: "${PAGERDUTY_ROUTING_KEY}"

    # Service mappings
    services:
      - name: "critical-services"
        service_id: "P123456"
        routing_key: "${PAGERDUTY_CRITICAL_ROUTING_KEY}"
        escalation_policy_id: "P789012"
      - name: "backend-services"
        service_id: "P234567"
        routing_key: "${PAGERDUTY_BACKEND_ROUTING_KEY}"
      - name: "frontend-services"
        service_id: "P345678"
        routing_key: "${PAGERDUTY_FRONTEND_ROUTING_KEY}"

    # Severity mapping
    severity_mapping:
      critical:
        pagerduty_severity: "critical"
        urgency: "high"
        priority_id: "P456789"
      high:
        pagerduty_severity: "error"
        urgency: "high"
      moderate:
        pagerduty_severity: "warning"
        urgency: "low"
      low:
        pagerduty_severity: "info"
        urgency: "low"

    # Rate limiting
    rate_limit:
      max_concurrent: 20
      requests_per_second: 20

    # Webhook for bi-directional sync
    webhook:
      enabled: true
      path: /webhooks/pagerduty
      secret: "${PAGERDUTY_WEBHOOK_SECRET}"
      # Event types to subscribe to
      events:
        - incident.acknowledged
        - incident.resolved
        - incident.assigned
        - incident.escalated
        - incident.reassigned
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_severity_serialization() {
        let critical = AlertSeverity::Critical;
        let json = serde_json::to_string(&critical).unwrap();
        assert_eq!(json, "\"critical\"");
    }

    #[test]
    fn test_custom_details_build() {
        let config = PagerDutyConfig {
            api_token: "test".into(),
            base_url: "https://api.pagerduty.com".into(),
            default_service_id: None,
            default_escalation_policy_id: None,
            timeout: std::time::Duration::from_secs(30),
            rate_limit: RateLimitConfig::default(),
        };

        let adapter = PagerDutyAdapter::new(config).unwrap();

        let alert = RustOpsAlert {
            title: "Test Alert".into(),
            description: "Test description".into(),
            severity: AlertSeverity::Critical,
            source: "RustOps".into(),
            service: "test-service".into(),
            correlation_id: "test-123".into(),
            metadata: AlertMetadata {
                alert_id: "alert-1".into(),
                affected_resources: vec!["resource-1".into()],
                detection_time: chrono::Utc::now(),
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                occurrences: 5,
                root_cause_hypothesis: Some("Hypothesis".into()),
                auto_remediation_attempted: true,
            },
            dedup_key: None,
        };

        let details = adapter.build_custom_details(&alert);

        assert_eq!(details["correlation_id"], "test-123");
        assert_eq!(details["service"], "test-service");
        assert_eq!(details["occurrences"], 5);
        assert_eq!(details["auto_remediation_attempted"], true);
    }
}
```

---

## References

- [PagerDuty REST API](https://developer.pagerduty.com/api-reference/)
- [PagerDuty Events API v2](https://developer.pagerduty.com/docs/ZG9jOjExMDI5NTgw-events-api-v2-overview)
- [PagerDuty Webhooks](https://developer.pagerduty.com/docs/ZG9jOjExMDI5NTgy-webhooks-overview)

---

**Version**: 1.0
**Last Updated**: 2026-01-18
**Integration Phase**: Phase 1 (Foundation)
