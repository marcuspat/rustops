// Prometheus integration
//
// Implements Prometheus scrape and Remote Write protocols

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use std::collections::HashMap;
use tracing::{debug, error, info};

use crate::adapter::{
    BaseAdapter, IntegrationKind, LogQuery, LogStream, Metric, MetricQuery, TelemetryCollector,
    TelemetryEvent, Trace, TraceQuery,
};
use crate::resilience::{HealthStatus, IntegrationError, IntegrationResult};
use crate::{CircuitBreakerConfig, RateLimiterConfig, RetryConfig};

/// Prometheus adapter configuration
#[derive(Debug, Clone)]
pub struct PrometheusConfig {
    /// Prometheus URL
    pub url: String,

    /// Basic auth username (optional)
    pub username: Option<String>,

    /// Basic auth password (optional)
    pub password: Option<String>,

    /// Bearer token (optional)
    pub bearer_token: Option<String>,

    /// HTTP client timeout
    pub timeout: std::time::Duration,
}

/// Prometheus adapter
pub struct PrometheusAdapter {
    base: BaseAdapter,
    config: PrometheusConfig,
    client: Client,
}

impl PrometheusAdapter {
    /// Create new Prometheus adapter
    pub fn new(config: PrometheusConfig) -> Self {
        let base = BaseAdapter::new(
            format!("prometheus-{}", ulid::Ulid::new()),
            IntegrationKind::TelemetryCollector,
            CircuitBreakerConfig::default(),
            RateLimiterConfig {
                requests_per_second: 100, // Higher for scraping
                burst: 200,
            },
            RetryConfig::default(),
        );

        let client = Client::builder().timeout(config.timeout).build().unwrap();

        Self {
            base,
            config,
            client,
        }
    }

    /// Query Prometheus API
    async fn query_api<T>(&self, endpoint: &str) -> IntegrationResult<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let url = format!("{}{}", self.config.url.trim_end_matches('/'), endpoint);
        let base = self.base.clone();
        let client = self.client.clone();

        base.execute_with_resilience(move || {
            let client = client.clone();
            let url = url.clone();
            async move {
                let mut request = client.get(&url);

                request
                    .send()
                    .await
                    .map_err(|e| IntegrationError::Network(e.to_string()))?
                    .json::<T>()
                    .await
                    .map_err(|e| IntegrationError::Deserialization(e.to_string()))
            }
        })
        .await
    }

    /// Parse metric value from Prometheus response
    fn parse_metric_value(value: serde_json::Value) -> Option<f64> {
        match value {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }
}

#[async_trait]
impl TelemetryCollector for PrometheusAdapter {
    async fn collect_metrics(&self, query: MetricQuery) -> IntegrationResult<Vec<Metric>> {
        debug!("Collecting metrics: {}", query.metric_name);

        // Build PromQL query
        let mut promql = query.metric_name.clone();
        if !query.labels.is_empty() {
            let labels: Vec<String> = query
                .labels
                .iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect();
            promql = format!("{{{}}}", labels.join(","));
        }

        let start = query.start_time.timestamp();
        let end = query.end_time.timestamp();
        let step = query.step.unwrap_or(60);

        let endpoint = format!(
            "/api/v1/query_range?query={}&start={}&end={}&step={}",
            urlencoding::encode(&promql),
            start,
            end,
            step
        );

        let response: PrometheusResponse = self.query_api(&endpoint).await?;

        match response.data {
            Some(data) => {
                let metric_name = query.metric_name.clone();
                let metrics = data
                    .result
                    .into_iter()
                    .flat_map(|result| {
                        let name = metric_name.clone();
                        match result {
                            PrometheusResult::Matrix(matrix) => matrix
                                .values
                                .into_iter()
                                .map(move |(ts, value)| Metric {
                                    name: name.clone(),
                                    labels: matrix.metric.clone(),
                                    value,
                                    timestamp: DateTime::from_timestamp(ts as i64, 0)
                                        .unwrap_or_default(),
                                })
                                .collect::<Vec<_>>()
                                .into_iter(),
                            PrometheusResult::Vector(vector) => vec![Metric {
                                name: name.clone(),
                                labels: vector.metric,
                                value: vector.value,
                                timestamp: Utc::now(),
                            }]
                            .into_iter(),
                        }
                    })
                    .collect();

                Ok(metrics)
            }
            None => Ok(vec![]),
        }
    }

    async fn collect_logs(&self, _query: LogQuery) -> IntegrationResult<LogStream> {
        // Prometheus doesn't support logs - use Loki
        Err(IntegrationError::InvalidResponse(
            "Prometheus doesn't support logs. Use Loki integration.".to_string(),
        ))
    }

    async fn collect_traces(&self, _query: TraceQuery) -> IntegrationResult<Vec<Trace>> {
        // Prometheus doesn't support traces - use Tempo/Jaeger
        Err(IntegrationError::InvalidResponse(
            "Prometheus doesn't support traces. Use Tempo/Jaeger integration.".to_string(),
        ))
    }

    async fn subscribe(&self) -> IntegrationResult<tokio::sync::mpsc::Receiver<TelemetryEvent>> {
        // Prometheus is pull-based, not push-based
        // Use Remote Write for streaming
        Err(IntegrationError::InvalidResponse(
            "Prometheus is pull-based. Use Remote Write for streaming.".to_string(),
        ))
    }
}

#[async_trait]
impl crate::adapter::IntegrationAdapter for PrometheusAdapter {
    fn id(&self) -> &str {
        self.base.id()
    }

    fn kind(&self) -> crate::adapter::IntegrationKind {
        self.base.kind()
    }

    async fn health_check(&self) -> IntegrationResult<HealthStatus> {
        match self
            .query_api::<serde_json::Value>("/api/v1/status/config")
            .await
        {
            Ok(_) => Ok(HealthStatus::Healthy),
            Err(e) => {
                error!("Prometheus health check failed: {}", e);
                Ok(HealthStatus::Unhealthy)
            }
        }
    }

    async fn initialize(&mut self) -> IntegrationResult<()> {
        info!("Initializing Prometheus adapter: {}", self.config.url);
        self.health_check().await?;
        Ok(())
    }

    async fn shutdown(&mut self) -> IntegrationResult<()> {
        info!("Shutting down Prometheus adapter");
        Ok(())
    }
}

// =============================================================================
// Prometheus API Types
// =============================================================================

#[derive(Debug, serde::Deserialize)]
struct PrometheusResponse {
    pub status: String,
    #[serde(rename = "data")]
    pub data: Option<PrometheusData>,
}

#[derive(Debug, serde::Deserialize)]
struct PrometheusData {
    pub result_type: String,
    pub result: Vec<PrometheusResult>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum PrometheusResult {
    Matrix(PrometheusMatrix),
    Vector(PrometheusVector),
}

#[derive(Debug, serde::Deserialize, Clone)]
struct PrometheusMatrix {
    pub metric: HashMap<String, String>,
    pub values: Vec<(f64, f64)>, // (timestamp, value)
}

#[derive(Debug, serde::Deserialize, Clone)]
struct PrometheusVector {
    pub metric: HashMap<String, String>,
    #[serde(rename = "value")]
    pub value: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::IntegrationAdapter;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_prometheus_query() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "success",
                "data": {
                    "resultType": "vector",
                    "result": [{
                        "metric": {"__name__": "up", "job": "prometheus"},
                        "value": [1234567890.0, "1.0"]
                    }]
                }
            })))
            .mount(&mock_server)
            .await;

        let config = PrometheusConfig {
            url: mock_server.uri(),
            username: None,
            password: None,
            bearer_token: None,
            timeout: std::time::Duration::from_secs(5),
        };

        let adapter = PrometheusAdapter::new(config);
        adapter.initialize().await.unwrap();

        let query = MetricQuery {
            metric_name: "up".to_string(),
            labels: HashMap::new(),
            start_time: Utc::now() - chrono::Duration::hours(1),
            end_time: Utc::now(),
            step: Some(60),
        };

        let result = adapter.collect_metrics(query).await;
        assert!(result.is_ok());

        let metrics = result.unwrap();
        assert!(!metrics.is_empty());
    }

    #[tokio::test]
    async fn test_prometheus_health_check() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/status/config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "status": "success",
                "data": {}
            })))
            .mount(&mock_server)
            .await;

        let config = PrometheusConfig {
            url: mock_server.uri(),
            username: None,
            password: None,
            bearer_token: None,
            timeout: std::time::Duration::from_secs(5),
        };

        let adapter = PrometheusAdapter::new(config);
        let health = adapter.health_check().await.unwrap();
        assert_eq!(health, HealthStatus::Healthy);
    }
}
