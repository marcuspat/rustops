//! Integration tests for the Prometheus adapter.
//!
//! These run against a hermetic wiremock server — no live Prometheus needed
//! — and exercise the real HTTP request/response path of the adapter.

use std::collections::HashMap;

use chrono::Utc;
use rustops_integration::{
    adapter::{IntegrationAdapter, MetricQuery, TelemetryCollector},
    telemetry::prometheus::{PrometheusAdapter, PrometheusConfig},
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn config_for(url: &str) -> PrometheusConfig {
    PrometheusConfig {
        url: url.to_string(),
        username: None,
        password: None,
        bearer_token: None,
        timeout: std::time::Duration::from_secs(5),
    }
}

#[tokio::test]
async fn adapter_id_is_prometheus_scoped() {
    let adapter = PrometheusAdapter::new(config_for("http://localhost:9090"));
    assert!(adapter.id().starts_with("prometheus-"));
}

#[tokio::test]
async fn config_round_trips_credentials() {
    let config = PrometheusConfig {
        url: "http://localhost:9090".to_string(),
        username: Some("user".to_string()),
        password: Some("pass".to_string()),
        bearer_token: None,
        timeout: std::time::Duration::from_secs(60),
    };

    assert_eq!(config.url, "http://localhost:9090");
    assert_eq!(config.username.as_deref(), Some("user"));
    assert_eq!(config.password.as_deref(), Some("pass"));
}

#[tokio::test]
async fn health_check_reports_unhealthy_when_unreachable() {
    // Point at a port that is not listening: the health check must complete
    // with an unhealthy/error outcome rather than hanging or panicking.
    let adapter = PrometheusAdapter::new(config_for("http://127.0.0.1:9"));
    let health = adapter.health_check().await;
    assert!(
        health.is_err() || !matches!(health, Ok(rustops_integration::HealthStatus::Healthy)),
        "unreachable server must not report healthy: {health:?}"
    );
}

#[tokio::test]
async fn collect_metrics_end_to_end_against_mock_server() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/query_range"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "success",
            "data": {
                "resultType": "matrix",
                "result": [{
                    "metric": {"__name__": "up", "job": "prometheus"},
                    "values": [[1234567890.0, "1.0"], [1234567950.0, "0.0"]]
                }]
            }
        })))
        .mount(&mock_server)
        .await;

    let adapter = PrometheusAdapter::new(config_for(&mock_server.uri()));

    let query = MetricQuery {
        metric_name: "up".to_string(),
        labels: HashMap::new(),
        start_time: Utc::now() - chrono::Duration::minutes(5),
        end_time: Utc::now(),
        step: Some(15),
    };

    let metrics = adapter
        .collect_metrics(query)
        .await
        .expect("query against mock server");
    assert!(!metrics.is_empty());
}
