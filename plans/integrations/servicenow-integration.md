# ServiceNow Integration Guide

**Integration Type**: ITSM / Incident Management
**Priority**: Critical (Phase 1)
**Status**: Design

---

## Overview

ServiceNow is the #1 ITSM platform with 50%+ enterprise market share. RustOps integrates with ServiceNow for bi-directional incident management, CMDB synchronization, and change tracking.

### Integration Capabilities

| Capability | Description | Use Case |
|------------|-------------|----------|
| **Incident Creation** | Create incidents from alerts | Alert → ITSM workflow |
| **Incident Updates** | Sync incident status and comments | Bi-directional sync |
| **CMDB Sync** | Synchronize service topology | Dependency mapping |
| **Change Tracking** | Track remediation as changes | Audit trail |
| **Worknote Updates** | Add investigation notes | Collaboration |
| **User Lookup** | Resolve assignee from contact | Auto-assignment |

### Why ServiceNow is Top Priority

- **50%+** enterprise ITSM market share
- **Rich REST API** for incident/CMDB operations
- **CMDB integration** enables topology mapping
- **Change management** for audit compliance
- **Enterprise requirement** for most organizations

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    RustOps - ServiceNow Integration                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                        ServiceNow Instance                       │   │
│  │  ┌───────────────────────────────────────────────────────────┐  │   │
│  │  │                   ServiceNow REST API                      │  │   │
│  │  │  - Incident Table (incident)                              │  │   │
│  │  │  - CMDB Table (cmdb_ci)                                   │  │   │
│  │  │  - Change Request Table (change_request)                  │  │   │
│  │  │  - User Table (sys_user)                                  │  │   │
│  │  └───────────────────────────────────────────────────────────┘  │   │
│  │                          │                                       │   │
│  │              OAuth 2.0 │ Basic Auth                             │   │
│  └────────────────────────┼───────────────────────────────────────┘   │
│                            │                                             │
│                            ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                 RustOps ServiceNow Adapter                      │   │
│  │  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────┐  │   │
│  │  │ Incident Manager │  │   CMDB Sync      │  │ Change       │  │   │
│  │  │  - Create        │  │  - Topology sync │  │  Tracker     │  │   │
│  │  │  - Update        │  │  - CI discovery  │  │  - Remediat. │  │   │
│  │  │  - Query         │  │  - Dependencies  │  │    tracking  │  │   │
│  │  └──────────────────┘  └──────────────────┘  └──────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                           │                                             │
│                           ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │              RustOps Alert → Incident Pipeline                   │   │
│  │  1. Alert Correlation                                           │   │
│  │  2. Deduplication (check existing incidents)                     │   │
│  │  3. Enrichment (add context from CMDB)                           │   │
│  │  4. Incident Creation                                            │   │
│  │  5. Bi-directional Sync Loop                                     │   │
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

# Authentication
oauth2 = "4.4"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Time handling
chrono = { version = "0.4", features = ["serde"] }
```

### ServiceNow Client Setup

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// ServiceNow adapter configuration
#[derive(Debug, Clone)]
pub struct ServiceNowConfig {
    pub instance_url: String,
    pub auth: ServiceNowAuth,
    pub timeout: std::time::Duration,
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone)]
pub enum ServiceNowAuth {
    OAuth2 {
        client_id: String,
        client_secret: String,
        username: String,
        password: String,
    },
    BasicAuth {
        username: String,
        password: String,
    },
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_concurrent: usize,
    pub requests_per_second: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            requests_per_second: 10,
        }
    }
}

/// ServiceNow adapter
pub struct ServiceNowAdapter {
    config: ServiceNowConfig,
    client: Client,
    rate_limiter: Arc<tokio::sync::Semaphore>,
}

impl ServiceNowAdapter {
    /// Create new ServiceNow adapter
    pub async fn new(config: ServiceNowConfig) -> Result<Self, ServiceNowError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| ServiceNowError::Client(e.to_string()))?;

        let rate_limiter = Arc::new(tokio::sync::Semaphore::new(config.rate_limit.max_concurrent));

        Ok(Self {
            config,
            client,
            rate_limiter,
        })
    }

    /// Build API URL
    fn api_url(&self, path: &str) -> String {
        format!("{}/api/now/{}", self.config.instance_url.trim_end_matches('/'), path)
    }

    /// Get request headers with auth
    async fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Accept",
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "Content-Type",
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        match &self.config.auth {
            ServiceNowAuth::OAuth2 { client_id, .. } => {
                // In real implementation, get access token from cache
                if let Ok(token) = self.get_cached_token().await {
                    headers.insert(
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
                            .unwrap(),
                    );
                }
            }
            ServiceNowAuth::BasicAuth { username, password } => {
                let creds = format!("{}:{}", username, password);
                let encoded = base64::encode(creds);
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Basic {}", encoded))
                        .unwrap(),
                );
            }
        }

        headers
    }

    async fn get_cached_token(&self) -> Result<String, ServiceNowError> {
        // Token cache implementation
        unimplemented!()
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthStatus, ServiceNowError> {
        let url = self.api_url("table/incident?sysparm_limit=1");

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers().await;

        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ServiceNowError::Request(e.to_string()))?;

        if response.status().is_success() {
            Ok(HealthStatus::Healthy)
        } else {
            Ok(HealthStatus::Unhealthy)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceNowError {
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

### Incident Management

```rust
/// ServiceNow incident model
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceNowIncident {
    #[serde(rename = "sys_id")]
    pub sys_id: Option<String>,
    #[serde(rename = "number")]
    pub number: Option<String>,
    #[serde(rename = "short_description")]
    pub short_description: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "priority")]
    pub priority: Option<i32>,
    #[serde(rename = "severity")]
    pub severity: Option<String>,
    #[serde(rename = "state")]
    pub state: Option<String>,
    #[serde(rename = "impact")]
    pub impact: Option<i32>,
    #[serde(rename = "urgency")]
    pub urgency: Option<i32>,
    #[serde(rename = "assignment_group")]
    pub assignment_group: Option<String>,
    #[serde(rename = "assigned_to")]
    pub assigned_to: Option<String>,
    #[serde(rename = "configuration_item")]
    pub configuration_item: Option<String>,
    #[serde(rename = "correlation_id")]
    pub correlation_id: Option<String>,
    #[serde(rename = "correlation_display")]
    pub correlation_display: Option<String>,
}

/// RustOps incident for ServiceNow creation
#[derive(Debug, Clone)]
pub struct RustOpsIncident {
    pub title: String,
    pub description: String,
    pub severity: IncidentSeverity,
    pub impact: IncidentImpact,
    pub urgency: IncidentUrgency,
    pub configuration_item: Option<String>,
    pub assignment_group: Option<String>,
    pub correlation_id: String,
    pub metadata: IncidentMetadata,
}

#[derive(Debug, Clone, Copy)]
pub enum IncidentSeverity {
    Critical = 1,
    High = 2,
    Moderate = 3,
    Low = 4,
}

#[derive(Debug, Clone, Copy)]
pub enum IncidentImpact {
    Critical = 1,
    High = 2,
    Medium = 3,
    Low = 4,
}

#[derive(Debug, Clone, Copy)]
pub enum IncidentUrgency {
    Critical = 1,
    High = 2,
    Moderate = 3,
    Low = 4,
}

#[derive(Debug, Clone)]
pub struct IncidentMetadata {
    pub alert_id: String,
    pub service_name: String,
    pub affected_resources: Vec<String>,
    pub detection_time: chrono::DateTime<chrono::Utc>,
    pub root_cause_hypothesis: Option<String>,
    pub auto_remediation_attempted: bool,
}

/// Incident management operations
impl ServiceNowAdapter {
    /// Create new incident
    pub async fn create_incident(
        &self,
        incident: RustOpsIncident,
    ) -> Result<ServiceNowIncident, ServiceNowError> {
        let url = self.api_url("table/incident");

        // Convert RustOps incident to ServiceNow format
        let sn_incident = ServiceNowIncident {
            sys_id: None,
            number: None,
            short_description: incident.title,
            description: self.format_description(&incident),
            priority: Some(Self::calculate_priority(
                incident.severity,
                incident.impact,
                incident.urgency,
            )),
            severity: Some(format!("{:?}", incident.severity)),
            state: Some("New".to_string()),
            impact: Some(incident.impact as i32),
            urgency: Some(incident.urgency as i32),
            assignment_group: incident.assignment_group,
            assigned_to: None,
            configuration_item: incident.configuration_item,
            correlation_id: Some(incident.correlation_id.clone()),
            correlation_display: Some(format!("RustOps: {}", incident.correlation_id)),
        };

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers().await;

        let response = self.client
            .post(&url)
            .headers(headers)
            .json(&sn_incident)
            .send()
            .await
            .map_err(|e| ServiceNowError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: ServiceNowIncident = response
                .json()
                .await
                .map_err(|e| ServiceNowError::Api(e.to_string()))?;

            tracing::info!(
                "Created ServiceNow incident {} ({})",
                result.number.as_ref().unwrap_or(&"?".to_string()),
                result.sys_id.as_ref().unwrap_or(&"?".to_string())
            );

            Ok(result)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(ServiceNowError::Api(format!("{}: {}", status, body)))
        }
    }

    /// Update existing incident
    pub async fn update_incident(
        &self,
        sys_id: &str,
        update: IncidentUpdate,
    ) -> Result<(), ServiceNowError> {
        let url = self.api_url(&format!("table/incident/{}", sys_id));

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers().await;

        let response = self.client
            .patch(&url)
            .headers(headers)
            .json(&update)
            .send()
            .await
            .map_err(|e| ServiceNowError::Request(e.to_string()))?;

        if response.status().is_success() {
            tracing::info!("Updated ServiceNow incident {}", sys_id);
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(ServiceNowError::Api(format!("{}: {}", status, body)))
        }
    }

    /// Add worknote to incident
    pub async fn add_worknote(
        &self,
        sys_id: &str,
        note: &str,
    ) -> Result<(), ServiceNowError> {
        let url = self.api_url(&format!("table/incident/{}", sys_id));

        let update = serde_json::json!({
            "work_notes": note
        });

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers().await;

        let response = self.client
            .patch(&url)
            .headers(headers)
            .json(&update)
            .send()
            .await
            .map_err(|e| ServiceNowError::Request(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(ServiceNowError::Api(body))
        }
    }

    /// Query incident by correlation ID
    pub async fn get_incident_by_correlation(
        &self,
        correlation_id: &str,
    ) -> Result<Option<ServiceNowIncident>, ServiceNowError> {
        let url = self.api_url(&format!(
            "table/incident?correlation_id={}&sysparm_limit=1",
            correlation_id
        ));

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers().await;

        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ServiceNowError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: ServiceNowListResponse<ServiceNowIncident> = response
                .json()
                .await
                .map_err(|e| ServiceNowError::Api(e.to_string()))?;

            Ok(result.result.into_iter().next())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(ServiceNowError::Api(body))
        }
    }

    /// Format incident description with metadata
    fn format_description(&self, incident: &RustOpsIncident) -> String {
        format!(
            "## RustOps Detected Incident\n\n\
            **Detection Time**: {}\n\
            **Service**: {}\n\
            **Severity**: {:?}\n\
            **Impact**: {:?}\n\
            **Correlation ID**: {}\n\n\
            ### Description\n{}\n\n\
            ### Affected Resources\n{}\n\n\
            ### Root Cause Hypothesis\n{}\n\n\
            ### Auto-Remediation\n{}\n\n\
            ---\n\
            *This incident was automatically created by RustOps AIOps platform*",
            incident.metadata.detection_time.format("%Y-%m-%d %H:%M:%S UTC"),
            incident.metadata.service_name,
            incident.severity,
            incident.impact,
            incident.correlation_id,
            incident.description,
            incident.metadata.affected_resources.join("\n"),
            incident.metadata.root_cause_hypothesis.as_ref().map(|s| s.as_str()).unwrap_or("Not yet determined"),
            if incident.metadata.auto_remediation_attempted {
                "Attempted"
            } else {
                "Not attempted"
            }
        )
    }

    /// Calculate priority from severity, impact, urgency
    fn calculate_priority(severity: IncidentSeverity, impact: IncidentImpact, urgency: IncidentUrgency) -> i32 {
        let sum = severity as i32 + impact as i32 + urgency as i32;
        match sum {
            3..=5 => 1,  // Critical
            6..=8 => 2,  // High
            9..=11 => 3, // Moderate
            _ => 4,      // Low
        }
    }
}

/// Incident update payload
#[derive(Debug, Serialize)]
pub struct IncidentUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignment_group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
}

/// ServiceNow list response wrapper
#[derive(Debug, Deserialize)]
pub struct ServiceNowListResponse<T> {
    pub result: Vec<T>,
}
```

### CMDB Integration

```rust
/// ServiceNow Configuration Item (CI)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigurationItem {
    #[serde(rename = "sys_id")]
    pub sys_id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "sys_class_name")]
    pub sys_class_name: String,
    #[serde(rename = "category")]
    pub category: Option<String>,
    #[serde(rename = "subcategory")]
    pub subcategory: Option<String>,
    #[serde(rename = "ip_address")]
    pub ip_address: Option<String>,
    #[serde(rename = "fqdn")]
    pub fqdn: Option<String>,
    #[serde(rename = "cmdb_ci")]
    pub cmdb_ci: Option<String>,
}

/// CMDB synchronization
impl ServiceNowAdapter {
    /// Query CI by name
    pub async fn get_ci_by_name(
        &self,
        name: &str,
    ) -> Result<Option<ConfigurationItem>, ServiceNowError> {
        let url = self.api_url(&format!(
            "table/cmdb_ci?name={}&sysparm_limit=1",
            urlencoding::encode(name)
        ));

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers().await;

        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ServiceNowError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: ServiceNowListResponse<ConfigurationItem> = response
                .json()
                .await
                .map_err(|e| ServiceNowError::Api(e.to_string()))?;

            Ok(result.result.into_iter().next())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(ServiceNowError::Api(body))
        }
    }

    /// Get CI dependencies
    pub async fn get_ci_dependencies(
        &self,
        sys_id: &str,
    ) -> Result<Vec<ConfigurationItem>, ServiceNowError> {
        let url = self.api_url(&format!(
            "table/cmdb_rel_ci?child={}&sysparm_fields=parent",
            sys_id
        ));

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers().await;

        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ServiceNowError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: ServiceNowListResponse<serde_json::Value> = response
                .json()
                .await
                .map_err(|e| ServiceNowError::Api(e.to_string()))?;

            // Extract parent CIs
            let dependencies = result.result
                .into_iter()
                .filter_map(|v| v.get("parent").and_then(|p| p.as_str()))
                .filter_map(|sys_id| {
                    // Fetch each CI
                    tokio::runtime::Handle::try_current()
                        .unwrap()
                        .block_on(self.get_ci_by_sys_id(sys_id))
                })
                .collect::<Vec<_>>();

            Ok(dependencies)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(ServiceNowError::Api(body))
        }
    }

    /// Get CI by sys_id
    pub async fn get_ci_by_sys_id(
        &self,
        sys_id: &str,
    ) -> Result<Option<ConfigurationItem>, ServiceNowError> {
        let url = self.api_url(&format!("table/cmdb_ci/{}", sys_id));

        let _permit = self.rate_limiter.acquire().await;
        let headers = self.headers().await;

        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ServiceNowError::Request(e.to_string()))?;

        if response.status().is_success() {
            let ci: Option<ConfigurationItem> = response
                .json()
                .await
                .map_err(|e| ServiceNowError::Api(e.to_string()))?;

            Ok(ci)
        } else {
            Ok(None)
        }
    }

    /// Sync service topology to CMDB
    pub async fn sync_topology(
        &self,
        services: Vec<ServiceTopology>,
    ) -> Result<CMDBSyncResult, ServiceNowError> {
        let mut created = 0;
        let mut updated = 0;
        let mut failed = 0;

        for service in services {
            match self.sync_service(&service).await {
                Ok(CIAction::Created) => created += 1,
                Ok(CIAction::Updated) => updated += 1,
                Err(_) => failed += 1,
            }
        }

        Ok(CMDBSyncResult {
            created,
            updated,
            failed,
        })
    }

    /// Sync single service to CMDB
    async fn sync_service(
        &self,
        service: &ServiceTopology,
    ) -> Result<CIAction, ServiceNowError> {
        // Check if CI exists
        let existing = self.get_ci_by_name(&service.name).await?;

        if let Some(mut ci) = existing {
            // Update existing CI
            ci.fqdn = Some(service.fqdn.clone());
            ci.ip_address = service.ip_address.clone();

            let url = self.api_url(&format!("table/cmdb_ci/{}", ci.sys_id));
            let _permit = self.rate_limiter.acquire().await;
            let headers = self.headers().await;

            let _ = self.client
                .patch(&url)
                .headers(headers)
                .json(&ci)
                .send()
                .await;

            Ok(CIAction::Updated)
        } else {
            // Create new CI
            let new_ci = ConfigurationItem {
                sys_id: String::new(), // Generated by ServiceNow
                name: service.name.clone(),
                sys_class_name: service.ci_type.clone(),
                category: Some("Application".to_string()),
                subcategory: Some(service.service_type.clone()),
                ip_address: service.ip_address.clone(),
                fqdn: Some(service.fqdn.clone()),
                cmdb_ci: None,
            };

            let url = self.api_url("table/cmdb_ci");
            let _permit = self.rate_limiter.acquire().await;
            let headers = self.headers().await;

            let response = self.client
                .post(&url)
                .headers(headers)
                .json(&new_ci)
                .send()
                .await
                .map_err(|e| ServiceNowError::Request(e.to_string()))?;

            if response.status().is_success() {
                Ok(CIAction::Created)
            } else {
                Err(ServiceNowError::Api(response.text().await.unwrap_or_default()))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceTopology {
    pub name: String,
    pub fqdn: String,
    pub ip_address: Option<String>,
    pub ci_type: String,
    pub service_type: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum CIAction {
    Created,
    Updated,
}

#[derive(Debug, Clone)]
pub struct CMDBSyncResult {
    pub created: usize,
    pub updated: usize,
    pub failed: usize,
}
```

---

## Configuration

### ServiceNow Configuration

```yaml
integrations:
  servicenow:
    enabled: true

    # ServiceNow instance
    instance:
      url: "https://dev12345.service-now.com"
      # Instance name for identification
      name: "production"

    # Authentication
    auth:
      method: oauth2  # oauth2 or basic
      oauth2:
        client_id: "${SERVICENOW_CLIENT_ID}"
        client_secret: "${SERVICENOW_CLIENT_SECRET}"  # From Vault
        username: "${SERVICENOW_USERNAME}"
        password: "${SERVICENOW_PASSWORD}"  # From Vault
      basic:
        username: "${SERVICENOW_USERNAME}"
        password: "${SERVICENOW_PASSWORD}"

    # Rate limiting
    rate_limit:
      max_concurrent: 10
      requests_per_second: 10

    # Incident settings
    incidents:
      enabled: true
      # Default assignment group
      default_assignment_group: "DevOps"
      # Priority mapping
      priority_mapping:
        critical:
          severity: 1
          impact: 1
          urgency: 1
        high:
          severity: 2
          impact: 2
          urgency: 2
        moderate:
          severity: 3
          impact: 3
          urgency: 3
        low:
          severity: 4
          impact: 4
          urgency: 4
      # Auto-close settings
      auto_close:
        enabled: true
        after_resolved: true
        close_code: "Resolved by RustOps"

    # CMDB synchronization
    cmdb:
      enabled: true
      sync_interval: 3600s  # 1 hour
      # Auto-create CIs
      auto_create: true
      # CI type mapping
      ci_types:
        kubernetes_deployment: "cmdb_ci_app_server"
        kubernetes_service: "cmdb_ci_service"
        aws_instance: "cmdb_ci_compute_instance"
        database: "cmdb_ci_db"

    # Webhook for ServiceNow events
    webhook:
      enabled: true
      path: /webhooks/servicenow
      secret: "${SERVICENOW_WEBHOOK_SECRET}"
      # Event types to subscribe to
      events:
        - incident.updated
        - incident.assigned
        - incident.resolved
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_calculation() {
        // Critical + Critical + Critical = Priority 1
        let priority = ServiceNowAdapter::calculate_priority(
            IncidentSeverity::Critical,
            IncidentImpact::Critical,
            IncidentUrgency::Critical,
        );
        assert_eq!(priority, 1);

        // Low + Low + Low = Priority 4
        let priority = ServiceNowAdapter::calculate_priority(
            IncidentSeverity::Low,
            IncidentImpact::Low,
            IncidentUrgency::Low,
        );
        assert_eq!(priority, 4);
    }

    #[test]
    fn test_description_formatting() {
        let config = ServiceNowConfig {
            instance_url: "https://test.service-now.com".into(),
            auth: ServiceNowAuth::BasicAuth {
                username: "test".into(),
                password: "test".into(),
            },
            timeout: std::time::Duration::from_secs(30),
            rate_limit: RateLimitConfig::default(),
        };

        let adapter = ServiceNowAdapter::new(config).await.unwrap();

        let incident = RustOpsIncident {
            title: "Test Incident".into(),
            description: "Test description".into(),
            severity: IncidentSeverity::High,
            impact: IncidentImpact::High,
            urgency: IncidentUrgency::High,
            configuration_item: None,
            assignment_group: None,
            correlation_id: "test-123".into(),
            metadata: IncidentMetadata {
                alert_id: "alert-1".into(),
                service_name: "test-service".into(),
                affected_resources: vec!["resource-1".into()],
                detection_time: chrono::Utc::now(),
                root_cause_hypothesis: Some("Hypothesis".into()),
                auto_remediation_attempted: true,
            },
        };

        let description = adapter.format_description(&incident);

        assert!(description.contains("Test Incident"));
        assert!(description.contains("test-service"));
        assert!(description.contains("High"));
        assert!(description.contains("test-123"));
    }
}
```

### Mock ServiceNow Server

```rust
#[cfg(test)]
pub mod mock_servicenow {
    use mockito::{Server, Mock};

    pub struct MockServiceNow {
        server: Server,
    }

    impl MockServiceNow {
        pub fn new() -> Self {
            Self {
                server: Server::new(),
            }
        }

        pub fn url(&self) -> String {
            self.server.url()
        }

        pub fn mock_incident_create(&self) -> Mock {
            self.server
                .mock("POST", "/api/now/table/incident")
                .with_status(201)
                .with_header("content-type", "application/json")
                .with_body(r#"{
                    "result": {
                        "sys_id": "test-sys-id",
                        "number": "INC0010001",
                        "priority": "1",
                        "state": "New"
                    }
                }"#)
                .create()
        }

        pub fn mock_incident_query(&self) -> Mock {
            self.server
                .mock("GET", "/api/now/table/incident")
                .match_query(mockito::Matcher::Regex("correlation_id=.*".into()))
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(r#"{
                    "result": []
                }"#)
                .create()
        }

        pub fn mock_ci_query(&self) -> Mock {
            self.server
                .mock("GET", "/api/now/table/cmdb_ci")
                .match_query(mockito::Matcher::Regex("name=.*".into()))
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(r#"{
                    "result": []
                }"#)
                .create()
        }
    }
}
```

---

## Deployment

### Kubernetes Secrets

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: servicenow-credentials
  namespace: rustops
type: Opaque
stringData:
  client_id: "your-client-id"
  client_secret: "your-client-secret"
  username: "rustops-service-account"
  password: "your-password"
  webhook_secret: "random-webhook-secret"
```

---

## References

- [ServiceNow REST API Reference](https://docs.servicenow.com/bundle/servicenow-platform/page/integrate/inbound-rest/concept/c_RESTAPI.html)
- [ServiceNow Table API](https://docs.servicenow.com/bundle/servicenow-platform/page/integrate/inbound-rest/concept/Table-API.html)
- [ServiceNow OAuth Setup](https://docs.servicenow.com/bundle/servicenow-platform/page/administer/oauth/concept/oauth.html)

---

**Version**: 1.0
**Last Updated**: 2026-01-18
**Integration Phase**: Phase 1 (Foundation)
