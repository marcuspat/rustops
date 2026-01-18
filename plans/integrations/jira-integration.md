# Jira Integration Guide

**Integration Type**: Issue Tracking / Project Management
**Priority**: High (Phase 1)
**Status**: Design**

---

## Overview

Jira is the standard for issue tracking and project management in development teams. RustOps integrates with Jira for creating and syncing issues, sprint tracking, and linking incidents to development work.

### Integration Capabilities

| Capability | Description | Use Case |
|------------|-------------|----------|
| **Issue Creation** | Create issues from incidents | Bug tracking |
| **Issue Updates** | Sync status and comments | Bi-directional sync |
| **Sprint Tracking** | Query sprint progress | Reporting |
| **Search** | Find existing issues | Deduplication |
| **Link Management** | Link incidents to Jira issues | Traceability |

---

## Implementation

### Rust Dependencies

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Jira Client

```rust
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct JiraConfig {
    pub base_url: String,
    pub username: String,
    pub api_token: String,
}

pub struct JiraAdapter {
    config: JiraConfig,
    client: Client,
}

impl JiraAdapter {
    pub fn new(config: JiraConfig) -> Result<Self, JiraError> {
        let client = Client::new();
        Ok(Self { config, client })
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Authorization",
            reqwest::header::HeaderValue::from_str(&format!(
                "Basic {}",
                base64::encode(format!("{}:{}", self.config.username, self.config.api_token))
            )).unwrap()
        );
        headers.insert("Content-Type", reqwest::header::HeaderValue::from_static("application/json"));
        headers
    }

    /// Create Jira issue
    pub async fn create_issue(&self, issue: JiraIssue) -> Result<String, JiraError> {
        let url = format!("{}/rest/api/3/issue", self.config.base_url);

        let payload = serde_json::json!({
            "fields": {
                "project": { "key": issue.project_key },
                "summary": issue.summary,
                "description": {
                    "type": "doc",
                    "version": 1,
                    "content": [
                        {
                            "type": "paragraph",
                            "content": [
                                {
                                    "type": "text",
                                    "text": issue.description
                                }
                            ]
                        }
                    ]
                },
                "issuetype": { "name": issue.issue_type }
            }
        });

        let response = self.client
            .post(&url)
            .headers(self.headers())
            .json(&payload)
            .send()
            .await
            .map_err(|e| JiraError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await.unwrap();
            let key = result["key"].as_str().unwrap_or("");
            Ok(key.to_string())
        } else {
            Err(JiraError::Api(response.text().await.unwrap_or_default()))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JiraError {
    #[error("Request error: {0}")]
    Request(String),
    #[error("API error: {0}")]
    Api(String),
}

#[derive(Debug, Clone)]
pub struct JiraIssue {
    pub project_key: String,
    pub summary: String,
    pub description: String,
    pub issue_type: String,
}
```

---

## Configuration

```yaml
integrations:
  jira:
    enabled: true
    base_url: "https://your-domain.atlassian.net"
    username: "${JIRA_USERNAME}"
    api_token: "${JIRA_API_TOKEN}"
    project_key: "OPS"
    default_issue_type: "Bug"
```

---

**Version**: 1.0
**Last Updated**: 2026-01-18
**Integration Phase**: Phase 1 (Foundation)
