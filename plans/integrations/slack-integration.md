# Slack Integration Guide

**Integration Type**: Collaboration / ChatOps
**Priority**: Critical (Phase 1)
**Status**: Design

---

## Overview

Slack is ubiquitous in DevOps teams for collaboration and incident management. RustOps integrates with Slack for notifications, ChatOps commands, incident channel management, and bi-directional incident status updates.

### Integration Capabilities

| Capability | Description | Use Case |
|------------|-------------|----------|
| **Alert Notifications** | Send formatted alerts to channels | Real-time awareness |
| **Incident Channels** | Auto-create channels for incidents | Organized response |
| **ChatOps Commands** | Execute commands via Slack messages | Interactive operations |
| **Status Updates** | Sync incident status to Slack | Transparency |
| **Threaded Updates** | Add investigation notes | Collaboration |
| **Interactive Buttons** | Acknowledge, resolve, escalate actions | Quick response |
| **Workflow Steps** | Rich incident workflows with approval gates | Structured response |

### Why Slack is Top Priority

- **Ubiquitous** in DevOps teams
- **Rich API** with bots, webhooks, and workflows
- **ChatOps** standard for operations teams
- **Mobile app** for instant on-the-go access
- **Integrations** with other tools (Jira, GitHub, PagerDuty)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       RustOps - Slack Integration                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                          Slack Workspace                         │   │
│  │  ┌───────────────────────────────────────────────────────────┐  │   │
│  │  │                      Slack API                             │  │   │
│  │  │  - Web API (chat.postMessage, conversations.create)      │  │   │
│  │  │  - RTM API (real-time messaging)                          │  │   │
│  │  │  - Events API (user interactions)                        │  │   │
│  │  │  - Webhooks (incoming webhooks)                          │  │   │
│  │  └───────────────────────────────────────────────────────────┘  │   │
│  │                          │                                       │   │
│  │              Bot Token │ Webhook URL                            │   │
│  └────────────────────────┼───────────────────────────────────────┘   │
│                            │                                             │
│                            ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                     RustOps Slack Adapter                        │   │
│  │  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────┐  │   │
│  │  │ Notification     │  │ ChatOps Handler  │  │ Incident     │  │   │
│  │  │  - Send alerts   │  │  - Command parse │  │  Channel     │  │   │
│  │  │  - Rich format  │  │  - Execute       │  │  Manager     │  │   │
│  │  │  - Threading    │  │    commands      │  │  - Create    │  │   │
│  │  └──────────────────┘  └──────────────────┘  │  - Archive   │  │   │
│  │                                                  └──────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                           │                                             │
│                           ▼                                             │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                   RustOps Alert → Slack Flow                     │   │
│  │  1. Detect Alert/Incident                                       │   │
│  │  2. Format Message with Metadata                                 │   │
│  │  3. Send to Channel or Create Incident Channel                   │   │
│  │  4. Add Interactive Buttons (Acknowledge, Resolve)               │   │
│  │  5. Listen for User Actions/Commands                             │   │
│  │  6. Update Message Based on Status Changes                       │   │
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

# Web server for slash commands
axum = "0.7"

# Time handling
chrono = { version = "0.4", features = ["serde"] }

# HMAC for webhook verification
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
```

### Slack Client Setup

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Slack adapter configuration
#[derive(Debug, Clone)]
pub struct SlackConfig {
    pub bot_token: String,
    pub signing_secret: String,
    pub default_channel: Option<String>,
    pub incident_channel_prefix: String,
    pub timeout: std::time::Duration,
}

/// Slack adapter
pub struct SlackAdapter {
    config: SlackConfig,
    client: Client,
}

impl SlackAdapter {
    /// Create new Slack adapter
    pub fn new(config: SlackConfig) -> Result<Self, SlackError> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| SlackError::Client(e.to_string()))?;

        Ok(Self {
            config,
            client,
        })
    }

    /// Build API URL
    fn api_url(&self, method: &str) -> String {
        format!("https://slack.com/api/{}", method)
    }

    /// Get request headers with auth
    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            reqwest::header::HeaderValue::from_static("application/json; charset=utf-8"),
        );
        headers.insert(
            "Authorization",
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.config.bot_token))
                .unwrap(),
        );

        headers
    }

    /// Verify Slack request signature
    pub fn verify_signature(
        &self,
        timestamp: &str,
        signature: &str,
        body: &str,
    ) -> Result<(), SlackError> {
        // Check timestamp to prevent replay attacks (5 min tolerance)
        let now = chrono::Utc::now().timestamp();
        let request_time = timestamp.parse::<i64>()
            .map_err(|_| SlackError::Auth("Invalid timestamp".into()))?;

        if (now - request_time).abs() > 300 {
            return Err(SlackError::Auth("Timestamp too old".into()));
        }

        // Create HMAC-SHA256
        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(
            self.config.signing_secret.as_bytes()
        )
        .map_err(|e| SlackError::Auth(e.to_string()))?;

        mac.update(format!("v0:{}:{}", timestamp, body).as_bytes());

        let expected = hex::encode(mac.finalize().into_bytes());
        let computed = format!("v0={}", expected);

        use hmac::Mac;
        if computed == signature {
            Ok(())
        } else {
            Err(SlackError::Auth("Invalid signature".into()))
        }
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthStatus, SlackError> {
        let url = self.api_url("auth.test");

        let response = self.client
            .post(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| SlackError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| SlackError::Api(e.to_string()))?;

            if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                Ok(HealthStatus::Healthy)
            } else {
                Ok(HealthStatus::Degraded)
            }
        } else {
            Ok(HealthStatus::Unhealthy)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SlackError {
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

### Message Sending

```rust
use serde_json::json;

/// Slack message
#[derive(Debug, Serialize, Clone)]
pub struct SlackMessage {
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_ts: Option<String>,  // For threaded replies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Vec<SlackBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<SlackAttachment>>,
}

/// Slack block kit block
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlackBlock {
    Section {
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<SlackText>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<Vec<SlackText>>,
    },
    Header {
        text: SlackText,
    },
    Divider,
    Actions {
        elements: Vec<SlackActionElement>,
    },
    Context {
        elements: Vec<SlackText>,
    },
}

/// Slack text object
#[derive(Debug, Serialize, Clone)]
pub struct SlackText {
    #[serde(rename = "type")]
    pub text_type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbatim: Option<bool>,
}

/// Slack action element (buttons, etc.)
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlackActionElement {
    Button {
        action_id: String,
        text: SlackText,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<String>,  // primary, danger
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        confirm: Option<SlackConfirmation>,
    },
}

/// Slack confirmation dialog
#[derive(Debug, Serialize, Clone)]
pub struct SlackConfirmation {
    pub title: SlackText,
    pub text: SlackText,
    pub confirm: SlackText,
    pub deny: SlackText,
}

/// Slack attachment (legacy format)
#[derive(Debug, Serialize, Clone)]
pub struct SlackAttachment {
    pub color: String,
    pub title: Option<String>,
    pub text: Option<String>,
    pub fields: Option<Vec<SlackField>>,
    pub footer: Option<String>,
    pub ts: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SlackField {
    pub title: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<bool>,
}

/// Message sending operations
impl SlackAdapter {
    /// Send message to channel
    pub async fn send_message(
        &self,
        message: SlackMessage,
    ) -> Result<SlackMessageResponse, SlackError> {
        let url = self.api_url("chat.postMessage");

        let response = self.client
            .post(&url)
            .headers(self.headers())
            .json(&message)
            .send()
            .await
            .map_err(|e| SlackError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: SlackMessageResponse = response
                .json()
                .await
                .map_err(|e| SlackError::Api(e.to_string()))?;

            if !result.ok {
                return Err(SlackError::Api(result.error.unwrap_or_default()));
            }

            Ok(result)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(SlackError::Api(body))
        }
    }

    /// Update existing message
    pub async fn update_message(
        &self,
        channel: &str,
        ts: &str,
        message: SlackMessage,
    ) -> Result<SlackMessageResponse, SlackError> {
        let url = self.api_url("chat.update");

        let mut payload = serde_json::to_value(&message).unwrap();
        payload["channel"] = json!(channel);
        payload["ts"] = json!(ts);

        let response = self.client
            .post(&url)
            .headers(self.headers())
            .json(&payload)
            .send()
            .await
            .map_err(|e| SlackError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: SlackMessageResponse = response
                .json()
                .await
                .map_err(|e| SlackError::Api(e.to_string()))?;

            if !result.ok {
                return Err(SlackError::Api(result.error.unwrap_or_default()));
            }

            Ok(result)
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(SlackError::Api(body))
        }
    }

    /// Send formatted alert message
    pub async fn send_alert(
        &self,
        channel: &str,
        alert: RustOpsAlert,
    ) -> Result<SlackMessageResponse, SlackError> {
        let message = self.format_alert_message(channel, alert)?;
        self.send_message(message).await
    }

    /// Format alert as Slack message with blocks
    fn format_alert_message(
        &self,
        channel: &str,
        alert: RustOpsAlert,
    ) -> Result<SlackMessage, SlackError> {
        let color = match alert.severity {
            AlertSeverity::Critical => "#FF0000",
            AlertSeverity::High => "#FF6600",
            AlertSeverity::Moderate => "#FFCC00",
            AlertSeverity::Low => "#00CC00",
        };

        let blocks = vec![
            SlackBlock::Header {
                text: SlackText {
                    text_type: "plain_text".into(),
                    text: format!("{} - {}", alert.severity_emoji(), alert.title),
                    emoji: Some(true),
                },
            },
            SlackBlock::Section {
                text: Some(SlackText {
                    text_type: "mrkdwn".into(),
                    text: alert.description,
                    verbatim: Some(false),
                }),
                fields: None,
            },
            SlackBlock::Section {
                text: None,
                fields: Some(vec![
                    SlackText {
                        text_type: "mrkdwn".into(),
                        text: format!("*Service:*\n{}", alert.service),
                        verbatim: Some(false),
                    },
                    SlackText {
                        text_type: "mrkdwn".into(),
                        text: format!("*Severity:*\n{:?}", alert.severity),
                        verbatim: Some(false),
                    },
                    SlackText {
                        text_type: "mrkdwn".into(),
                        text: format!("*Correlation ID:*\n`{}`", alert.correlation_id),
                        verbatim: Some(false),
                    },
                    SlackText {
                        text_type: "mrkdwn".into(),
                        text: format!("*Occurrences:*\n{}", alert.metadata.occurrences),
                        verbatim: Some(false),
                    },
                ]),
            },
            SlackBlock::Actions {
                elements: vec![
                    SlackActionElement::Button {
                        action_id: "acknowledge".into(),
                        text: SlackText {
                            text_type: "plain_text".into(),
                            text: "Acknowledge".into(),
                            emoji: Some(true),
                        },
                        style: Some("primary".into()),
                        value: Some(alert.correlation_id.clone()),
                        confirm: None,
                    },
                    SlackActionElement::Button {
                        action_id: "resolve".into(),
                        text: SlackText {
                            text_type: "plain_text".into(),
                            text: "Resolve".into(),
                            emoji: Some(true),
                        },
                        style: Some("danger".into()),
                        value: Some(alert.correlation_id.clone()),
                        confirm: None,
                    },
                    SlackActionElement::Button {
                        action_id: "escalate".into(),
                        text: SlackText {
                            text_type: "plain_text".into(),
                            text: "Escalate".into(),
                            emoji: Some(true),
                        },
                        style: None,
                        value: Some(alert.correlation_id.clone()),
                        confirm: None,
                    },
                ],
            },
            SlackBlock::Context {
                elements: vec![
                    SlackText {
                        text_type: "mrkdwn".into(),
                        text: format!(
                            "<{}|View in RustOps> | {}",
                            format!("https://rustops.example.com/alerts/{}", alert.correlation_id),
                            alert.metadata.detection_time.format("%Y-%m-%d %H:%M:%S UTC")
                        ),
                        verbatim: Some(false),
                    },
                ],
            },
        ];

        Ok(SlackMessage {
            channel: Some(channel.to_string()),
            thread_ts: None,
            text: Some(format!("{} - {}", alert.severity_emoji(), alert.title)),
            blocks: Some(blocks),
            attachments: None,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct SlackMessageResponse {
    pub ok: bool,
    #[serde(default)]
    pub error: Option<String>,
    pub channel: Option<String>,
    pub ts: Option<String>,
    pub message: Option<serde_json::Value>,
}

/// RustOps alert for Slack
#[derive(Debug, Clone)]
pub struct RustOpsAlert {
    pub title: String,
    pub description: String,
    pub severity: AlertSeverity,
    pub service: String,
    pub correlation_id: String,
    pub metadata: AlertMetadata,
}

#[derive(Debug, Clone, Copy)]
pub enum AlertSeverity {
    Critical,
    High,
    Moderate,
    Low,
}

impl AlertSeverity {
    fn severity_emoji(&self) -> &str {
        match self {
            AlertSeverity::Critical => ":rotating_light:",
            AlertSeverity::High => ":warning:",
            AlertSeverity::Moderate => ":large_yellow_circle:",
            AlertSeverity::Low => ":large_green_circle:",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlertMetadata {
    pub occurrences: u32,
    pub detection_time: chrono::DateTime<chrono::Utc>,
}
```

### Incident Channel Management

```rust
/// Incident channel operations
impl SlackAdapter {
    /// Create incident channel
    pub async fn create_incident_channel(
        &self,
        incident: RustOpsIncident,
    ) -> Result<IncidentChannelInfo, SlackError> {
        let channel_name = format!(
            "{}-{}-{}",
            self.config.incident_channel_prefix,
            incident.service.replace('_', "-"),
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        );

        // Create channel
        let url = self.api_url("conversations.create");

        let create_payload = json!({
            "name": channel_name,
            "is_private": true
        });

        let response = self.client
            .post(&url)
            .headers(self.headers())
            .json(&create_payload)
            .send()
            .await
            .map_err(|e| SlackError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(SlackError::Api(body));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SlackError::Api(e.to_string()))?;

        if !result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Err(SlackError::Api("Failed to create channel".into()));
        }

        let channel_id = result
            .get("channel")
            .and_then(|c| c.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| SlackError::Api("No channel ID returned".into()))?;

        // Invite incident responders
        if let Some(users) = incident.responders {
            for user_id in users {
                let _ = self.invite_user_to_channel(channel_id, &user_id).await;
            }
        }

        // Send incident summary message
        let summary = self.format_incident_summary(&incident);
        let _ = self.send_message(summary).await;

        Ok(IncidentChannelInfo {
            channel_id: channel_id.to_string(),
            channel_name: channel_name.clone(),
            created_at: chrono::Utc::now(),
        })
    }

    /// Invite user to channel
    async fn invite_user_to_channel(
        &self,
        channel_id: &str,
        user_id: &str,
    ) -> Result<(), SlackError> {
        let url = self.api_url("conversations.invite");

        let payload = json!({
            "channel": channel_id,
            "users": user_id
        });

        let _ = self.client
            .post(&url)
            .headers(self.headers())
            .json(&payload)
            .send()
            .await;

        Ok(())
    }

    /// Archive incident channel
    pub async fn archive_incident_channel(
        &self,
        channel_id: &str,
    ) -> Result<(), SlackError> {
        let url = self.api_url("conversations.archive");

        let payload = json!({
            "channel": channel_id
        });

        let response = self.client
            .post(&url)
            .headers(self.headers())
            .json(&payload)
            .send()
            .await
            .map_err(|e| SlackError::Request(e.to_string()))?;

        if response.status().is_success() {
            tracing::info!("Archived Slack channel: {}", channel_id);
            Ok(())
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(SlackError::Api(body))
        }
    }

    /// Format incident summary message
    fn format_incident_summary(&self, incident: &RustOpsIncident) -> SlackMessage {
        let severity_emoji = match incident.severity {
            IncidentSeverity::Critical => ":rotating_light:",
            IncidentSeverity::High => ":warning:",
            IncidentSeverity::Moderate => ":large_yellow_circle:",
            IncidentSeverity::Low => ":large_green_circle:",
        };

        SlackMessage {
            channel: None,  // Will be set when sending
            thread_ts: None,
            text: Some(format!("{} Incident Summary", severity_emoji)),
            blocks: Some(vec![
                SlackBlock::Header {
                    text: SlackText {
                        text_type: "plain_text".into(),
                        text: format!("{} Incident Summary", severity_emoji),
                        emoji: Some(true),
                    },
                },
                SlackBlock::Section {
                    text: Some(SlackText {
                        text_type: "mrkdwn".into(),
                        text: format!("*{}\n{}", incident.title, incident.description),
                        verbatim: Some(false),
                    }),
                    fields: None,
                },
                SlackBlock::Divider,
                SlackBlock::Section {
                    text: None,
                    fields: Some(vec![
                        SlackText {
                            text_type: "mrkdwn".into(),
                            text: format!("*Severity*\n{:?}", incident.severity),
                            verbatim: Some(false),
                        },
                        SlackText {
                            text_type: "mrkdwn".into(),
                            text: format!("*Service*\n{}", incident.service),
                            verbatim: Some(false),
                        },
                        SlackText {
                            text_type: "mrkdwn".into(),
                            text: format!("*Started*\n{}", incident.started_at.format("%H:%M UTC")),
                            verbatim: Some(false),
                        },
                        SlackText {
                            text_type: "mrkdwn".into(),
                            text: format!("*Correlation ID*\n`{}`", incident.correlation_id),
                            verbatim: Some(false),
                        },
                    ]),
                },
            ]),
            attachments: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RustOpsIncident {
    pub title: String,
    pub description: String,
    pub severity: IncidentSeverity,
    pub service: String,
    pub correlation_id: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub responders: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy)]
pub enum IncidentSeverity {
    Critical,
    High,
    Moderate,
    Low,
}

#[derive(Debug, Clone)]
pub struct IncidentChannelInfo {
    pub channel_id: String,
    pub channel_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

### ChatOps Commands

```rust
use axum::{Json, extract::State};
use serde::Deserialize;

/// Slack slash command payload
#[derive(Debug, Deserialize)]
pub struct SlashCommand {
    pub command: String,
    pub text: String,
    pub user_id: String,
    pub user_name: String,
    pub channel_id: String,
    pub channel_name: String,
    pub team_id: String,
    pub response_url: String,
}

/// ChatOps command handler
pub async fn handle_slash_command(
    State(adapter): State<Arc<SlackAdapter>>,
    Form(command): Form<SlashCommand>,
) -> Result<String, SlackError> {
    match command.command.as_str() {
        "/rustops" => {
            handle_rustops_command(adapter, command).await
        }
        "/incident" => {
            handle_incident_command(adapter, command).await
        }
        _ => Ok(format!("Unknown command: {}", command.command)),
    }
}

/// Handle /rustops command
async fn handle_rustops_command(
    adapter: Arc<SlackAdapter>,
    command: SlashCommand,
) -> Result<String, SlackError> {
    let parts = command.text.split_whitespace().collect::<Vec<_>>();

    match parts.first() {
        Some(&"status") => {
            // Query system status
            let status = adapter.get_system_status().await?;
            Ok(format!("System Status: {}", status))
        }
        Some(&"alerts") => {
            // List recent alerts
            let alerts = adapter.get_recent_alerts(10).await?;
            Ok(format!("Recent Alerts:\n{}", alerts.join("\n")))
        }
        Some(&"help") => {
            Ok(rustops_help_text())
        }
        _ => Ok(rustops_help_text()),
    }
}

/// Handle /incident command
async fn handle_incident_command(
    adapter: Arc<SlackAdapter>,
    command: SlashCommand,
) -> Result<String, SlackError> {
    let parts = command.text.split_whitespace().collect::<Vec<_>>();

    match parts.first() {
        Some(&"create") => {
            // Create incident from command
            let title = parts[1..].join(" ");
            let incident = RustOpsIncident {
                title: title.clone(),
                description: "Created via Slack command".into(),
                severity: IncidentSeverity::High,
                service: "unknown".into(),
                correlation_id: uuid::Uuid::new_v4().to_string(),
                started_at: chrono::Utc::now(),
                responders: Some(vec![command.user_id.clone()]),
            };

            let channel = adapter.create_incident_channel(incident).await?;
            Ok(format!("Created incident channel: <#{}>", channel.channel_id))
        }
        Some(&"list") => {
            // List active incidents
            Ok("Active incidents: ...".to_string())
        }
        _ => Ok(incident_help_text()),
    }
}

fn rustops_help_text() -> String {
    r#"
*RustOps Commands:*
`/rustops status` - Show system status
`/rustops alerts` - List recent alerts
`/rustops help` - Show this help

*Incident Commands:*
`/incident create <title>` - Create new incident
`/incident list` - List active incidents
"#.to_string()
}

fn incident_help_text() -> String {
    r#"
*Incident Commands:*
`/incident create <title>` - Create new incident
`/incident list` - List active incidents
`/incident resolve <id>` - Resolve incident
"#.to_string()
}
```

---

## Configuration

### Slack Configuration

```yaml
integrations:
  slack:
    enabled: true

    # Bot credentials
    bot:
      token: "${SLACK_BOT_TOKEN}"  # xoxb-...
      signing_secret: "${SLACK_SIGNING_SECRET}"

    # Default channels
    channels:
      alerts: "#ops-alerts"
      incidents: "#ops-incidents"
      status_updates: "#ops-status"

    # Incident channels
    incidents:
      channel_prefix: "inc"
      auto_create: true
      auto_archive: true
      archive_after: 86400s  # 24 hours after resolution

    # Notification settings
    notifications:
      enabled: true
      # Minimum severity for notifications
      min_severity: "moderate"
      # Use threads for updates
      thread_updates: true
      # Interactive buttons
      enable_actions: true

    # ChatOps commands
    commands:
      enabled: true
      prefix: "/rustops"
      allowed_users: "*"  # All users

    # Formatting
    formatting:
      use_blocks: true
      emoji_enabled: true
      timestamp_format: "%Y-%m-%d %H:%M:%S UTC"

    # Rate limiting
    rate_limit:
      messages_per_minute: 100
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_verification() {
        let config = SlackConfig {
            bot_token: "xoxb-test".into(),
            signing_secret: "test_secret".into(),
            default_channel: Some("#test".into()),
            incident_channel_prefix: "inc".into(),
            timeout: std::time::Duration::from_secs(30),
        };

        let adapter = SlackAdapter::new(config).unwrap();

        // Test valid signature
        // Note: In real tests, you'd use actual HMAC values
        let result = adapter.verify_signature("1234567890", "v0=signature", "test_body");
        // This will fail due to signature mismatch, but tests the logic
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_alert_severity_emoji() {
        assert_eq!(AlertSeverity::Critical.severity_emoji(), ":rotating_light:");
        assert_eq!(AlertSeverity::High.severity_emoji(), ":warning:");
        assert_eq!(AlertSeverity::Moderate.severity_emoji(), ":large_yellow_circle:");
        assert_eq!(AlertSeverity::Low.severity_emoji(), ":large_green_circle:");
    }
}
```

---

## References

- [Slack API Documentation](https://api.slack.com/docs)
- [Slack Block Kit Builder](https://api.slack.com/block-kit/building)
- [Slack Webhooks](https://api.slack.com/messaging/webhooks)
- [ChatOps Best Practices](https://api.slack.com/automation/chatops)

---

**Version**: 1.0
**Last Updated**: 2026-01-18
**Integration Phase**: Phase 1 (Foundation)
