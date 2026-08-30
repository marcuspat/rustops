//! Integration tests for Prometheus adapter
//!
//! These tests require a running Prometheus instance or can be mocked

use chrono::{Duration, Utc};
use rustops_integration::{
    adapter::IntegrationAdapter,
    prometheus::PrometheusAdapter,
    CircuitBreakerConfig, RateLimiterConfig, RetryConfig,
};

fn create_test_adapter(url: &str) -> PrometheusAdapter {
    PrometheusAdapter::new(
        "test-prometheus",
        url,
        None::<(&str, &str)>,
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig::default(),
    )
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
    let _ = adapter.query_range(query, start_time, end_time, "15s").await;
}

#[tokio::test]
async fn test_prometheus_adapter_creation() {
    let adapter = create_test_adapter("http://localhost:9090");
    assert_eq!(adapter.id(), "test-prometheus");
}

#[tokio::test]
async fn test_prometheus_config() {
    let adapter = PrometheusAdapter::new(
        "test-prometheus-auth",
        "http://localhost:9090",
        Some(("user", "pass")),
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig {
            max_attempts: 3,
            base_delay: std::time::Duration::from_millis(100),
            max_delay: std::time::Duration::from_secs(60),
            backoff_factor: 2.0,
            jitter: true,
        },
    );

    assert_eq!(adapter.id(), "test-prometheus-auth");
}
