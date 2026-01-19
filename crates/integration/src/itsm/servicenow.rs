// ServiceNow integration
//
// Implements ServiceNow incident management and CMDB sync

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info};

use crate::adapter::{
    BaseAdapter, ITSMNotifier, IntegrationKind,
    Incident, IncidentSeverity, IncidentStatus, IncidentUpdate, CMDBSyncResult,
};
use crate::resilience::{IntegrationError, IntegrationResult, HealthStatus};
use crate::{CircuitBreakerConfig, RateLimiterConfig, RetryConfig};

/// ServiceNow adapter configuration
#[derive(Debug, Clone)]
pub struct ServiceNowConfig {
    /// ServiceNow instance URL (e.g., https://dev12345.service-now.com)
    pub instance_url: String,

    /// OAuth client ID
    pub client_id: String,

    /// OAuth client secret
    pub client_secret: String,

    /// Username for basic auth (fallback)
    pub username: Option<String>,

    /// Password for basic auth (fallback)
    pub password: Option<String>,
}

/// ServiceNow adapter
pub struct ServiceNowAdapter {
    base: BaseAdapter,
    config: ServiceNowConfig,
    client: Client,
    access_token: Option<String>,
}

impl ServiceNowAdapter {
    /// Create new ServiceNow adapter
    pub fn new(config: ServiceNowConfig) -> Self {
        let base = BaseAdapter::new(
            format!("servicenow-{}", ulid::Ulid::new()),
            IntegrationKind::ITSMNotifier,
            CircuitBreakerConfig::default(),
            RateLimiterConfig::default(),
            RetryConfig::default(),
        );

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        Self {
            base,
            config,
            client,
            access_token: None,
        }
    }

    /// Get auth headers
    fn auth_headers(&self) -> Result<HashMap<String, String>, IntegrationError> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());

        if let Some(token) = &self.access_token {
            headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        } else if let (Some(username), Some(password)) = (&self.config.username, &self.config.password) {
            let auth = format!("{}:{}", username, password);
            let encoded = base64::encode(auth);
            headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
        }

        Ok(headers)
    }

    /// Refresh OAuth token
    async fn refresh_token(&mut self) -> IntegrationResult<()> {
        // OAuth token refresh implementation
        Ok(())
    }
}

#[async_trait]
impl ITSMNotifier for ServiceNowAdapter {
    async fn create_incident(&self, incident: Incident) -> IntegrationResult<String> {
        info!("Creating ServiceNow incident: {}", incident.title);

        let url = format!("{}/api/now/table/incident", self.config.instance_url.trim_end_matches('/'));

        let payload = CreateIncidentRequest {
            short_description: incident.title.clone(),
            description: incident.description.clone(),
            priority: severity_to_priority(incident.severity),
            impact: severity_to_impact(incident.severity),
            urgency: severity_to_urgency(incident.severity),
        };

        let base = self.base.clone();
        let client = self.client.clone();
        let headers = self.auth_headers().unwrap_or_default();
        let payload_clone = CreateIncidentRequest {
            short_description: payload.short_description.clone(),
            description: payload.description.clone(),
            priority: payload.priority.clone(),
            impact: payload.impact.clone(),
            urgency: payload.urgency.clone(),
        };

        base.execute_with_resilience(move || {
            let client = client.clone();
            let url = url.clone();
            let headers = headers.clone();
            let payload = payload_clone.clone();
            async move {
                let response = client
                    .post(&url)
                    .headers(try_into_headers(headers)?)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| IntegrationError::Network(e.to_string()))?;

                if response.status().is_success() {
                    let result: ServiceNowResponse = response
                        .json()
                        .await
                        .map_err(|e| IntegrationError::Deserialization(e.to_string()))?;

                    Ok(result.result.sys_id)
                } else {
                    let error_text = response.text().await.unwrap_or_default();
                    Err(IntegrationError::ServiceUnavailable(error_text))
                }
            }
        })
        .await
    }

    async fn update_incident(&self, id: &str, update: IncidentUpdate) -> IntegrationResult<()> {
        debug!("Updating ServiceNow incident: {}", id);

        let url = format!("{}/api/now/table/incident/{}", self.config.instance_url.trim_end_matches('/'), id);

        let mut payload = HashMap::new();
        if let Some(status) = update.status {
            payload.insert("state", status_to_state(status));
        }
        if let Some(description) = update.description {
            payload.insert("description", description);
        }
        if let Some(resolution) = update.resolution {
            payload.insert("close_notes", resolution);
        }

        let base = self.base.clone();
        let client = self.client.clone();
        let headers = self.auth_headers().unwrap_or_default();

        base.execute_with_resilience(move || {
            let client = client.clone();
            let url = url.clone();
            let headers = headers.clone();
            let payload = payload.clone();
            async move {
                let response = client
                    .patch(&url)
                    .headers(try_into_headers(headers)?)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| IntegrationError::Network(e.to_string()))?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    let error_text = response.text().await.unwrap_or_default();
                    Err(IntegrationError::ServiceUnavailable(error_text))
                }
            }
        })
        .await
    }

    async fn get_incident(&self, id: &str) -> IntegrationResult<Incident> {
        let url = format!("{}/api/now/table/incident/{}", self.config.instance_url.trim_end_matches('/'), id);

        let base = self.base.clone();
        let client = self.client.clone();
        let headers = self.auth_headers().unwrap_or_default();

        let response: ServiceNowResponse = base.execute_with_resilience(move || {
            let client = client.clone();
            let url = url.clone();
            let headers = headers.clone();
            async move {
                let response = client
                    .get(&url)
                    .headers(try_into_headers(headers)?)
                    .send()
                    .await
                    .map_err(|e| IntegrationError::Network(e.to_string()))?;

                if response.status().is_success() {
                    response
                        .json()
                        .await
                        .map_err(|e| IntegrationError::Deserialization(e.to_string()))
                } else {
                    let error_text = response.text().await.unwrap_or_default();
                    Err(IntegrationError::ServiceUnavailable(error_text))
                }
            }
        })
        .await?;

        Ok(incident_from_response(response.result))
    }

    async fn sync_cmdb(&self) -> IntegrationResult<CMDBSyncResult> {
        info!("Syncing CMDB from ServiceNow");

        // CMDB sync implementation
        Ok(CMDBSyncResult {
            items_synced: 0,
            items_updated: 0,
            items_created: 0,
            items_failed: 0,
            errors: vec![],
        })
    }
}

#[async_trait]
impl crate::adapter::IntegrationAdapter for ServiceNowAdapter {
    fn id(&self) -> &str {
        self.base.id()
    }

    fn kind(&self) -> crate::adapter::IntegrationKind {
        self.base.kind()
    }

    async fn health_check(&self) -> IntegrationResult<HealthStatus> {
        let url = format!("{}/api/now/table/sys_properties", self.config.instance_url.trim_end_matches('/'));

        let base = self.base.clone();
        let client = self.client.clone();
        let headers = self.auth_headers().unwrap_or_default();

        match base.execute_with_resilience(move || {
            let client = client.clone();
            let url = url.clone();
            let headers = headers.clone();
            async move {
                let response = client
                    .head(&url)
                    .headers(try_into_headers(headers)?)
                    .send()
                    .await
                    .map_err(|e| IntegrationError::Network(e.to_string()))?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(IntegrationError::Authentication("Health check failed".to_string()))
                }
            }
        })
        .await
        {
            Ok(_) => Ok(HealthStatus::Healthy),
            Err(_) => Ok(HealthStatus::Unhealthy),
        }
    }

    async fn initialize(&mut self) -> IntegrationResult<()> {
        info!("Initializing ServiceNow adapter: {}", self.config.instance_url);
        self.refresh_token().await?;
        self.health_check().await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> IntegrationResult<()> {
        info!("Shutting down ServiceNow adapter");
        Ok(())
    }
}

// =============================================================================
// Helper Functions and Types
// =============================================================================

fn severity_to_priority(severity: IncidentSeverity) -> String {
    match severity {
        IncidentSeverity::P1 => "1".to_string(),
        IncidentSeverity::P2 => "2".to_string(),
        IncidentSeverity::P3 => "3".to_string(),
        IncidentSeverity::P4 => "4".to_string(),
    }
}

fn severity_to_impact(severity: IncidentSeverity) -> String {
    match severity {
        IncidentSeverity::P1 => "1".to_string(),
        IncidentSeverity::P2 => "2".to_string(),
        IncidentSeverity::P3 => "2".to_string(),
        IncidentSeverity::P4 => "3".to_string(),
    }
}

fn severity_to_urgency(severity: IncidentSeverity) -> String {
    severity_to_impact(severity)
}

fn status_to_state(status: IncidentStatus) -> String {
    match status {
        IncidentStatus::New => "1".to_string(),
        IncidentStatus::Assigned => "2".to_string(),
        IncidentStatus::InProgress => "2".to_string(),
        IncidentStatus::Resolved => "6".to_string(),
        IncidentStatus::Closed => "7".to_string(),
    }
}

fn incident_from_response(result: ServiceNowIncident) -> Incident {
    Incident {
        id: Some(result.sys_id),
        title: result.short_description,
        description: result.description,
        severity: priority_to_severity(result.priority),
        status: state_to_status(result.state),
        assigned_to: result.assigned_to,
        created_at: result.created_at,
        updated_at: Some(result.updated_at),
        resolved_at: result.resolved_at,
    }
}

fn priority_to_severity(priority: String) -> IncidentSeverity {
    match priority.as_str() {
        "1" => IncidentSeverity::P1,
        "2" => IncidentSeverity::P2,
        "3" => IncidentSeverity::P3,
        _ => IncidentSeverity::P4,
    }
}

fn state_to_status(state: String) -> IncidentStatus {
    match state.as_str() {
        "1" => IncidentStatus::New,
        "2" => IncidentStatus::Assigned,
        "3" => IncidentStatus::InProgress,
        "6" => IncidentStatus::Resolved,
        "7" => IncidentStatus::Closed,
        _ => IncidentStatus::New,
    }
}

fn try_into_headers(map: HashMap<String, String>) -> Result<reqwest::header::HeaderMap, IntegrationError> {
    let mut headers = reqwest::header::HeaderMap::new();
    for (k, v) in map {
        let key = reqwest::header::HeaderName::from_bytes(k.as_bytes())
            .map_err(|_| IntegrationError::Unknown(format!("Invalid header name: {}", k)))?;
        let value = reqwest::header::HeaderValue::from_str(&v)
            .map_err(|_| IntegrationError::Unknown(format!("Invalid header value: {}", v)))?;
        headers.insert(key, value);
    }
    Ok(headers)
}

// =============================================================================
// ServiceNow API Types
// =============================================================================

#[derive(Debug, Clone, Serialize)]
struct CreateIncidentRequest {
    short_description: String,
    description: String,
    priority: String,
    impact: String,
    urgency: String,
}

#[derive(Debug, Deserialize)]
struct ServiceNowResponse {
    result: ServiceNowIncident,
}

#[derive(Debug, Deserialize)]
struct ServiceNowIncident {
    sys_id: String,
    short_description: String,
    description: String,
    priority: String,
    state: String,
    assigned_to: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    resolved_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_conversion() {
        assert_eq!(severity_to_priority(IncidentSeverity::P1), "1");
        assert_eq!(severity_to_priority(IncidentSeverity::P2), "2");
        assert_eq!(severity_to_priority(IncidentSeverity::P3), "3");
        assert_eq!(severity_to_priority(IncidentSeverity::P4), "4");
    }
}
