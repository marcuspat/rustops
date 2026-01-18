# Datadog Integration Guide

**Integration Type**: Metrics / APM / Logs
**Priority**: High (Phase 2)
**Status**: Design

---

## Overview

Datadog is a leading SaaS observability platform. RustOps integrates with Datadog for pulling metrics, APM traces, and logs.

### Integration Capabilities

| Capability | Description | Use Case |
|------------|-------------|----------|
| **Metrics API** | Query time series metrics | Resource monitoring |
| **APM Traces** | Fetch distributed traces | Performance analysis |
| **Logs API** | Query and stream logs | Log aggregation |
| **Events API** | Send events to Datadog | Event correlation |
| **Monitors** | Query and manage monitors | Alert management |

---

## Implementation

### Datadog Client

```rust
use reqwest::Client;

pub struct DatadogAdapter {
    api_key: String,
    app_key: String,
    base_url: String,
    client: Client,
}

impl DatadogAdapter {
    pub fn new(api_key: String, app_key: String) -> Self {
        Self {
            api_key,
            app_key,
            base_url: "https://api.datadoghq.com/api/v1".to_string(),
            client: Client::new(),
        }
    }

    /// Query metrics
    pub async fn query_metrics(
        &self,
        query: &str,
        from: i64,
        to: i64,
    ) -> Result<Vec<MetricPoint>, DatadogError> {
        let url = format!("{}/query", self.base_url);

        let response = self.client
            .get(&url)
            .query(&[
                ("query", query),
                ("from", &from.to_string()),
                ("to", &to.to_string()),
            ])
            .header("DD-API-KEY", &self.api_key)
            .header("DD-APPLICATION-KEY", &self.app_key)
            .send()
            .await
            .map_err(|e| DatadogError::Request(e.to_string()))?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await.unwrap();
            // Parse metric series
            Ok(vec![])
        } else {
            Err(DatadogError::Api(response.text().await.unwrap_or_default()))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DatadogError {
    #[error("Request error: {0}")]
    Request(String),
    #[error("API error: {0}")]
    Api(String),
}

#[derive(Debug, Clone)]
pub struct MetricPoint {
    pub timestamp: i64,
    pub value: f64,
}
```

---

## Configuration

```yaml
integrations:
  datadog:
    enabled: true
    api_key: "${DATADOG_API_KEY}"
    app_key: "${DATADOG_APP_KEY}"
    site: "datadoghq.com"
```

---

**Version**: 1.0
**Last Updated**: 2026-01-18
**Integration Phase**: Phase 2 (Expansion)
