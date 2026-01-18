# Elasticsearch Integration Guide

**Integration Type**: Log Aggregation
**Priority**: High (Phase 2)
**Status**: Design

---

## Overview

Elasticsearch (ELK Stack) is the most popular open-source log aggregation platform. RustOps integrates with Elasticsearch for querying logs and streaming new log entries.

### Integration Capabilities

| Capability | Description | Use Case |
|------------|-------------|----------|
| **Log Query** | Search logs via Elasticsearch API | Log analysis |
| **Log Streaming** | Follow log indices | Real-time monitoring |
| **Aggregations** | Aggregate log data | Statistics |

---

## Implementation

### Elasticsearch Client

```rust
use reqwest::Client;

pub struct ElasticsearchAdapter {
    base_url: String,
    username: Option<String>,
    password: Option<String>,
    client: Client,
}

impl ElasticsearchAdapter {
    pub fn new(base_url: String, username: Option<String>, password: Option<String>) -> Self {
        Self {
            base_url,
            username,
            password,
            client: Client::new(),
        }
    }

    /// Search logs
    pub async fn search_logs(
        &self,
        index: &str,
        query: serde_json::Value,
    ) -> Result<Vec<LogEntry>, ElasticsearchError> {
        let url = format!("{}/{}/_search", self.base_url, index);

        let mut request = self.client.post(&url).json(&query);

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            request = request.basic_auth(username, Some(password));
        }

        let response = request
            .send()
            .await
            .map_err(|e| ElasticsearchError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await.unwrap();
            Ok(vec![])  // Parse results
        } else {
            Err(ElasticsearchError::Api(response.text().await.unwrap_or_default()))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ElasticsearchError {
    #[error("Request error: {0}")]
    Request(String),
    #[error("API error: {0}")]
    Api(String),
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub message: String,
    pub level: String,
}
```

---

## Configuration

```yaml
integrations:
  elasticsearch:
    enabled: true
    base_url: "http://elasticsearch:9200"
    username: "${ELASTICSEARCH_USERNAME}"
    password: "${ELASTICSEARCH_PASSWORD}"
    indices:
      - "logs-*"
      - "application-*"
```

---

**Version**: 1.0
**Last Updated**: 2026-01-18
**Integration Phase**: Phase 2 (Expansion)
