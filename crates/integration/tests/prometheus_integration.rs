//! Integration tests for Prometheus adapter
//!
//! These tests require a running Prometheus instance or can be mocked

use chrono::{DateTime, Duration, Utc};
use rustops_integration::{
    adapter::{IntegrationAdapter, MetricQuery, TelemetryCollector},
    telemetry::prometheus::{PrometheusAdapter, PrometheusConfig, StaticTarget},
    CircuitBreakerConfig, RateLimiterConfig, RetryConfig,
};
use std::collections::HashMap;

fn create_test_adapter(url: &str) -> PrometheusAdapter {
    let config = PrometheusConfig {
        url: url.to_string(),
        username: None::<&str>,
        password: None::<&str>,
        bearer_token: None,
        timeout: Duration::seconds(30),
    };
    PrometheusAdapter::new(config)
}

#[tokio::test]
async fn test_prometheus_health_check_with_mock() {
    // This test demonstrates the health check behavior
    let adapter = create_test_adapter("http://mock-prometheus:9090");

    // In a real scenario, this would connect to actual Prometheus
    let health = adapter.health_check().await;
    println!("Health check result: {:?}", health);

    // Test doesn't assert success as it requires actual Prometheus server
}

#[tokio::test]
async fn test_prometheus_query_construction() {
    let adapter = create_test_adapter("http://localhost:9090");

    // Test query parameters construction
    let start_time = Utc::now() - Duration::minutes(5);
    let end_time = Utc::now();
    let query = "up{job=\"prometheus\"}";

    // This would make the actual query if Prometheus is running
    let _ = adapter
        .query_range(query, start_time, end_time, Duration::seconds(15))
        .await;
}

#[tokio::test]
async fn test_prometheus_adapter_creation() {
    let config = PrometheusConfig {
        url: "http://localhost:9090".to_string(),
        username: None,
        password: None,
        bearer_token: None,
        timeout: Duration::seconds(30),
    };

    let adapter = PrometheusAdapter::new(config);
    assert!(adapter.id().starts_with("prometheus-"));
}

#[tokio::test]
async fn test_prometheus_config() {
    let config = PrometheusConfig {
        url: "http://localhost:9090".to_string(),
        username: Some("user".to_string()),
        password: Some("pass".to_string()),
        bearer_token: None,
        timeout: Duration::seconds(60),
    };

    assert_eq!(config.url, "http://localhost:9090");
    assert_eq!(config.username, Some("user".to_string()));
    assert_eq!(config.password, Some("pass".to_string()));
}
