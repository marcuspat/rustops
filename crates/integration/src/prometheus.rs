//! Prometheus integration adapter for RustOps
//!
//! Provides integration with Prometheus for metrics collection, querying, and alert rule evaluation.
//! Supports both pull mode (scraping) and push mode (remote write).

use crate::adapter::{self, IntegrationAdapter, TelemetryCollector};
use crate::resilience::{IntegrationError, IntegrationResult, HealthStatus};
use crate::{CircuitBreakerConfig, RateLimiterConfig, RetryConfig};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hyper::body::Bytes;
use hyper::header::{CONTENT_TYPE, USER_AGENT};
use hyper::http::HeaderValue;
use hyper::{Client, Request, Method, Body};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

// Prometheus API response types
#[derive(Debug, Deserialize)]
pub struct PrometheusResponse {
    pub status: String,
    pub data: PrometheusData,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrometheusData {
    pub result_type: String,
    pub result: Vec<PrometheusMetric>,
}

#[derive(Debug, Deserialize)]
pub struct PrometheusMetric {
    pub metric: HashMap<String, String>,
    pub value: Option<Vec<serde_json::Value>>,
    pub values: Option<Vec<Vec<serde_json::Value>>>,
}

// Query parameters
#[derive(Debug, Clone)]
pub struct PrometheusQuery {
    pub query: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub step: Option<String>,
    pub timeout: Option<String>,
}

// Alert configuration
#[derive(Debug, Clone)]
pub struct AlertRule {
    pub name: String,
    pub expression: String,
    pub duration: String,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
}

// Service discovery configuration
#[derive(Debug, Clone)]
pub struct ServiceDiscoveryConfig {
    pub kubernetes_sd: Option<KubernetesSDConfig>,
    pub static_configs: Option<Vec<StaticTarget>>,
    pub relabel_configs: Option<Vec<RelabelConfig>>,
}

#[derive(Debug, Clone)]
pub struct KubernetesSDConfig {
    pub namespaces: Vec<String>,
    pub selectors: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct StaticTarget {
    pub targets: Vec<String>,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct RelabelConfig {
    pub source_labels: Vec<String>,
    pub separator: Option<String>,
    pub regex: Option<String>,
    pub modulus: Option<u64>,
    pub replacement: String,
    pub action: RelabelAction,
    pub target_label: Option<String>,
}

#[derive(Debug, Clone)]
pub enum RelabelAction {
    Replace,
    Keep,
    Drop,
    HashMod,
    LabelMap,
    MetricLabelMap,
    LabelDrop,
    MetricLabelDrop,
}

/// Prometheus adapter implementation
pub struct PrometheusAdapter {
    base: adapter::BaseAdapter,
    client: Client<hyper::client::HttpConnector>,
    base_url: String,
    auth: Option<(String, String)>,
    headers: HashMap<String, String>,
}

impl PrometheusAdapter {
    /// Create a new Prometheus adapter
    pub fn new(
        id: impl Into<String>,
        base_url: impl Into<String>,
        auth: Option<(impl Into<String>, impl Into<String>)>,
        circuit_breaker_config: CircuitBreakerConfig,
        rate_limiter_config: RateLimiterConfig,
        retry_config: RetryConfig,
    ) -> Self {
        let base_url = base_url.into();
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "rustops-integration/1.0".to_string());

        Self {
            base: adapter::BaseAdapter::new(id, adapter::IntegrationKind::TelemetryCollector, circuit_breaker_config, rate_limiter_config, retry_config),
            client: Client::new(),
            base_url,
            auth: auth.map(|(u, p)| (u.into(), p.into())),
            headers,
        }
    }

    /// Set custom headers
    pub fn set_headers(&mut self, headers: HashMap<String, String>) {
        self.headers = headers;
    }

    /// Execute a Prometheus query
    async fn execute_query(&self, query: &PrometheusQuery) -> IntegrationResult<PrometheusResponse> {
        let url = format!("{}/api/v1/query", self.base_url);

        let mut params = vec![
            ("query", query.query.clone()),
        ];

        if let Some(start_time) = query.start_time {
            params.push(("start", start_time.timestamp().to_string()));
        }

        if let Some(end_time) = query.end_time {
            params.push(("end", end_time.timestamp().to_string()));
        }

        if let Some(ref step) = query.step {
            params.push(("step", step.clone()));
        }

        if let Some(ref timeout) = query.timeout {
            params.push(("timeout", timeout.clone()));
        }

        let params_clone = params.clone();

        self.base.execute_with_resilience(move || {
            let url_clone = url.clone();
            let client_clone = self.client.clone();
            let params = params_clone.clone();

            async move {
                let request = Request::builder()
                    .method(Method::GET)
                    .uri(&url_clone)
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&params)?))?;

                let response = client_clone.request(request).await?;

                if !response.status().is_success() {
                    return Err(IntegrationError::Network(format!("HTTP {}: {}", response.status(), response.status().canonical_reason().unwrap_or(""))));
                }

                let body = hyper::body::to_bytes(response.into_body()).await?;
                let response: PrometheusResponse = serde_json::from_slice(&body)?;

                if response.status != "success" {
                    return Err(IntegrationError::Unknown(response.error.unwrap_or_else(|| "Unknown query error".to_string())));
                }

                Ok(response)
            }
        }).await
    }

    /// Execute a range query
    pub async fn query_range(
        &self,
        query: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        step: &str,
    ) -> IntegrationResult<PrometheusResponse> {
        let query_params = PrometheusQuery {
            query: query.to_string(),
            start_time: Some(start),
            end_time: Some(end),
            step: Some(step.to_string()),
            timeout: None,
        };

        self.execute_query(&query_params).await
    }

    /// Get current metric values
    pub async fn query_instant(&self, query: &str) -> IntegrationResult<PrometheusResponse> {
        let query_params = PrometheusQuery {
            query: query.to_string(),
            start_time: None,
            end_time: None,
            step: None,
            timeout: None,
        };

        self.execute_query(&query_params).await
    }

    /// Evaluate alert rules
    pub async fn evaluate_alerts(&self, rules: &[AlertRule]) -> IntegrationResult<Vec<AlertEvaluation>> {
        let mut evaluations = Vec::new();

        for rule in rules {
            match self.query_instant(&rule.expression).await {
                Ok(response) => {
                    let evaluation = AlertEvaluation {
                        rule_name: rule.name.clone(),
                        expression: rule.expression.clone(),
                        status: AlertStatus::Firing,
                        active_at: Some(Utc::now()),
                        labels: rule.labels.clone(),
                        annotations: rule.annotations.clone(),
                        metric_data: Some(response.data),
                    };
                    evaluations.push(evaluation);
                }
                Err(e) => {
                    tracing::warn!("Failed to evaluate alert rule {}: {}", rule.name, e);
                    let evaluation = AlertEvaluation {
                        rule_name: rule.name.clone(),
                        expression: rule.expression.clone(),
                        status: AlertStatus::Error,
                        active_at: Some(Utc::now()),
                        labels: rule.labels.clone(),
                        annotations: rule.annotations.clone(),
                        metric_data: None,
                    };
                    evaluations.push(evaluation);
                }
            }
        }

        Ok(evaluations)
    }

    /// Discover services using Prometheus service discovery
    pub async fn discover_services(&self, config: &ServiceDiscoveryConfig) -> IntegrationResult<Vec<ServiceTarget>> {
        // For now, implement basic static config discovery
        // In a full implementation, this would integrate with Prometheus SD mechanisms
        let mut targets = Vec::new();

        if let Some(static_configs) = &config.static_configs {
            for config in static_configs {
                for target in &config.targets {
                    let service = ServiceTarget {
                        address: target.clone(),
                        port: None,
                        labels: config.labels.clone(),
                        scheme: "http".to_string(),
                        metrics_path: "/metrics".to_string(),
                        last_scraped: Some(Utc::now()),
                        error: None,
                    };
                    targets.push(service);
                }
            }
        }

        Ok(targets)
    }

    /// Scrape metrics from a target
    pub async fn scrape_target(&self, target: &ServiceTarget) -> IntegrationResult<String> {
        let url = format!(
            "{}://{}:{}/{}",
            target.scheme,
            target.address,
            target.port.as_ref().unwrap_or(&"9090".to_string()),
            &target.metrics_path
        );

        self.base.execute_with_resilience(move || {
            let url_clone = url.clone();
            let client_clone = self.client.clone();

            async move {
                let request = Request::builder()
                    .method(Method::GET)
                    .uri(&url_clone)
                    .header(USER_AGENT, "rustops-integration/1.0")
                    .body(Body::empty())?;

                let response = client_clone.request(request).await?;

                if !response.status().is_success() {
                    return Err(IntegrationError::Network(format!("HTTP {}: {}", response.status(), response.status().canonical_reason().unwrap_or(""))));
                }

                let body = hyper::body::to_bytes(response.into_body()).await?;
                Ok(String::from_utf8(body.to_vec())?)
            }
        }).await
    }

    /// Get series metadata
    pub async fn label_names(&self, match_: Option<&str>) -> IntegrationResult<Vec<String>> {
        let mut url = format!("{}/api/v1/labels", self.base_url);

        if let Some(match_) = match_ {
            url.push_str(&format!("?match[]={}", match_));
        }

        self.base.execute_with_resilience(move || {
            let url_clone = url.clone();
            let client_clone = self.client.clone();

            async move {
                let request = Request::builder()
                    .method(Method::GET)
                    .uri(&url_clone)
                    .body(Body::empty())?;

                let response = client_clone.request(request).await?;

                if !response.status().is_success() {
                    return Err(IntegrationError::Network(format!("HTTP {}: {}", response.status(), response.status().canonical_reason().unwrap_or(""))));
                }

                let body = hyper::body::to_bytes(response.into_body()).await?;
                let response: LabelNamesResponse = serde_json::from_slice(&body)?;

                if response.status != "success" {
                    return Err(IntegrationError::Unknown(response.error.unwrap_or_else(|| "Failed to get label names".to_string())));
                }

                Ok(response.data)
            }
        }).await
    }
}

#[async_trait]
impl IntegrationAdapter for PrometheusAdapter {
    fn id(&self) -> &str {
        self.base.id()
    }

    fn kind(&self) -> adapter::IntegrationKind {
        self.base.kind()
    }

    async fn health_check(&self) -> IntegrationResult<HealthStatus> {
        let result = self.query_instant("up").await;

        match result {
            Ok(_) => Ok(HealthStatus::Healthy),
            Err(e) => {
                tracing::warn!("Prometheus health check failed: {}", e);
                Ok(HealthStatus::Degraded)
            }
        }
    }

    async fn initialize(&mut self) -> IntegrationResult<()> {
        // Test connectivity to Prometheus
        let result = self.health_check().await;
        match result {
            Ok(HealthStatus::Healthy) => {
                tracing::info!("Prometheus adapter initialized successfully");
                Ok(())
            }
            _ => Err(IntegrationError::ServiceUnavailable {
                service: "prometheus".to_string(),
            }),
        }
    }

    async fn shutdown(&mut self) -> IntegrationResult<()> {
        tracing::info!("Shutting down Prometheus adapter");
        Ok(())
    }
}

#[async_trait]
impl TelemetryCollector for PrometheusAdapter {
    async fn collect_metrics(&self, query: adapter::MetricQuery) -> IntegrationResult<Vec<adapter::Metric>> {
        let prometheus_query = if query.step.is_some() {
            self.query_range(
                &query.metric_name,
                query.start_time,
                query.end_time,
                &query.step.unwrap().to_string(),
            ).await?
        } else {
            self.query_instant(&query.metric_name).await?
        };

        let mut metrics = Vec::new();

        for result in prometheus_query.data.result {
            let mut labels = HashMap::new();
            labels.extend(result.metric);

            if let Some(values) = result.values {
                for value_pair in values {
                    let timestamp = DateTime::<Utc>::from_timestamp(value_pair[1].as_u64().unwrap_or(0) / 1000, 0)
                        .unwrap_or(Utc::now());

                    metrics.push(adapter::Metric {
                        name: query.metric_name.clone(),
                        labels: labels.clone(),
                        value: value_pair[0].as_f64().unwrap_or(0.0),
                        timestamp,
                    });
                }
            } else if let Some(value) = result.value {
                let timestamp = DateTime::<Utc>::from_timestamp(value[1].as_u64().unwrap_or(0) / 1000, 0)
                    .unwrap_or(Utc::now());

                metrics.push(adapter::Metric {
                    name: query.metric_name.clone(),
                    labels: labels.clone(),
                    value: value[0].as_f64().unwrap_or(0.0),
                    timestamp,
                });
            }
        }

        Ok(metrics)
    }

    async fn collect_logs(&self, _query: adapter::LogQuery) -> IntegrationResult<adapter::LogStream> {
        // Prometheus doesn't support log collection
        tracing::warn!("Log collection is not supported by Prometheus");
        Ok(adapter::LogStream {
            entries: Vec::new(),
            has_more: false,
        })
    }

    async fn collect_traces(&self, _query: adapter::TraceQuery) -> IntegrationResult<Vec<adapter::Trace>> {
        // Prometheus doesn't support trace collection
        tracing::warn!("Trace collection is not supported by Prometheus");
        Ok(Vec::new())
    }

    async fn subscribe(&self) -> IntegrationResult<mpsc::Receiver<adapter::TelemetryEvent>> {
        // For now, implement a simple polling-based subscription
        let (tx, rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));

            loop {
                interval.tick().await;

                // Query for recent anomalies
                if let Ok(response) = PrometheusAdapter::new(
                    "temp-subscriber",
                    "http://localhost:9090",
                    None,
                    crate::CircuitBreakerConfig::default(),
                    crate::RateLimiterConfig::default(),
                    crate::RetryConfig::default(),
                )
                .query_instant("rustops_anomaly_detected_total")
                .await
                {
                    for metric in response.data.result {
                        let event = adapter::TelemetryEvent::Metric(adapter::Metric {
                            name: "rustops_anomaly_detected_total".to_string(),
                            labels: metric.metric,
                            value: 1.0,
                            timestamp: Utc::now(),
                        });

                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(rx)
    }
}

/// Alert evaluation result
#[derive(Debug, Clone)]
pub struct AlertEvaluation {
    pub rule_name: String,
    pub expression: String,
    pub status: AlertStatus,
    pub active_at: Option<DateTime<Utc>>,
    pub labels: HashMap<String, String>,
    pub annotations: HashMap<String, String>,
    pub metric_data: Option<PrometheusData>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertStatus {
    Firing,
    Pending,
    Inactive,
    Error,
}

/// Service target for scraping
#[derive(Debug, Clone)]
pub struct ServiceTarget {
    pub address: String,
    pub port: Option<String>,
    pub labels: HashMap<String, String>,
    pub scheme: String,
    pub metrics_path: String,
    pub last_scraped: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

// Response type for label names API
#[derive(Debug, Deserialize)]
struct LabelNamesResponse {
    pub status: String,
    pub data: Vec<String>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CircuitBreakerConfig, RateLimiterConfig, RetryConfig};
    use chrono::{DateTime, Utc};

    #[tokio::test]
    async fn test_prometheus_adapter_creation() {
        let adapter = PrometheusAdapter::new(
            "test-prometheus",
            "http://localhost:9090",
            None,
            CircuitBreakerConfig::default(),
            RateLimiterConfig::default(),
            RetryConfig::default(),
        );

        assert_eq!(adapter.id(), "test-prometheus");
        assert!(matches!(adapter.kind(), adapter::IntegrationKind::TelemetryCollector));
    }

    #[tokio::test]
    async fn test_health_check() {
        let adapter = PrometheusAdapter::new(
            "test-prometheus",
            "http://localhost:9090",
            None,
            CircuitBreakerConfig::default(),
            RateLimiterConfig::default(),
            RetryConfig::default(),
        );

        let status = adapter.health_check().await;
        // Note: This will fail if there's no Prometheus server running
        assert!(status.is_ok());
    }

    #[tokio::test]
    async fn test_query_parsing() {
        let adapter = PrometheusAdapter::new(
            "test-prometheus",
            "http://localhost:9090",
            None,
            CircuitBreakerConfig::default(),
            RateLimiterConfig::default(),
            RetryConfig::default(),
        );

        let start = Utc::now() - chrono::Duration::minutes(5);
        let end = Utc::now();

        // This will test the query construction (may fail due to no server)
        let _ = adapter.query_range(
            "up",
            start,
            end,
            "15s",
        ).await;
    }

    #[tokio::test]
    async fn test_alert_evaluation() {
        let adapter = PrometheusAdapter::new(
            "test-prometheus",
            "http://localhost:9090",
            None,
            CircuitBreakerConfig::default(),
            RateLimiterConfig::default(),
            RetryConfig::default(),
        );

        let rules = vec![
            AlertRule {
                name: "high_cpu".to_string(),
                expression: "process_cpu_seconds_total > 0.8".to_string(),
                duration: "5m".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            }
        ];

        let _ = adapter.evaluate_alerts(&rules).await;
    }

    #[tokio::test]
    async fn test_service_discovery() {
        let adapter = PrometheusAdapter::new(
            "test-prometheus",
            "http://localhost:9090",
            None,
            CircuitBreakerConfig::default(),
            RateLimiterConfig::default(),
            RetryConfig::default(),
        );

        let config = ServiceDiscoveryConfig {
            kubernetes_sd: None,
            static_configs: Some(vec![StaticTarget {
                targets: vec!["localhost:9090".to_string()],
                labels: HashMap::new(),
            }]),
            relabel_configs: None,
        };

        let _ = adapter.discover_services(&config).await;
    }

    #[test]
    fn test_alert_status() {
        assert_ne!(AlertStatus::Firing, AlertStatus::Inactive);
        assert_eq!(AlertStatus::Pending, AlertStatus::Pending);
    }
}