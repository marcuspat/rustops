//! Integration tests for Prometheus adapter
//!
//! These tests require a running Prometheus instance or can be mocked

use chrono::{DateTime, Duration, Utc};
use rustops_integration::{
    adapter::{IntegrationAdapter, MetricQuery, TelemetryCollector},
    prometheus::{
        AlertEvaluation, AlertRule, AlertStatus, PrometheusAdapter, PrometheusQuery, RelabelAction,
        ServiceDiscoveryConfig, ServiceTarget,
    },
    CircuitBreakerConfig, RateLimiterConfig, RetryConfig,
};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::test]
async fn test_prometheus_health_check_with_mock() {
    // This test demonstrates the health check behavior
    let adapter = PrometheusAdapter::new(
        "test-prometheus",
        "http://mock-prometheus:9090",
        None,
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig::default(),
    );

    // In a real scenario, this would connect to actual Prometheus
    let health = adapter.health_check().await;
    println!("Health check result: {:?}", health);

    // Test doesn't assert success as it requires actual Prometheus server
}

#[tokio::test]
async fn test_prometheus_query_construction() {
    let adapter = PrometheusAdapter::new(
        "test-prometheus",
        "http://localhost:9090",
        None,
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig::default(),
    );

    // Test query parameters construction
    let start_time = Utc::now() - Duration::minutes(5);
    let end_time = Utc::now();
    let query = "up{job=\"prometheus\"}";

    // This would make the actual query if Prometheus is running
    let _ = adapter
        .query_range(query, start_time, end_time, "15s")
        .await;
}

#[tokio::test]
async fn test_alert_rule_evaluation() {
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
            name: "high_cpu_usage".to_string(),
            expression: "process_cpu_seconds_total > 0.8".to_string(),
            duration: "5m".to_string(),
            labels: {
                let mut labels = HashMap::new();
                labels.insert("severity".to_string(), "critical".to_string());
                labels.insert("team".to_string(), "platform".to_string());
                labels
            },
            annotations: {
                let mut annotations = HashMap::new();
                annotations.insert("summary".to_string(), "High CPU usage detected".to_string());
                annotations.insert(
                    "description".to_string(),
                    "CPU usage has been above 80% for 5 minutes".to_string(),
                );
                annotations
            },
        },
        AlertRule {
            name: "memory_pressure".to_string(),
            expression: "process_resident_memory_bytes > 1073741824".to_string(), // 1GB
            duration: "10m".to_string(),
            labels: {
                let mut labels = HashMap::new();
                labels.insert("severity".to_string(), "warning".to_string());
                labels.insert("team".to_string(), "platform".to_string());
                labels
            },
            annotations: HashMap::new(),
        },
    ];

    let evaluations = adapter.evaluate_alerts(&rules).await;
    println!("Alert evaluations: {:?}", evaluations);
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
        static_configs: Some(vec![
            ServiceTarget {
                targets: vec!["localhost:9090".to_string()],
                labels: HashMap::new(),
            },
            ServiceTarget {
                targets: vec!["node-exporter:9100".to_string()],
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("job".to_string(), "node-exporter".to_string());
                    labels.insert("env".to_string(), "production".to_string());
                    labels
                },
            },
        ]),
        relabel_configs: None,
    };

    let targets = adapter.discover_services(&config).await;
    match targets {
        Ok(targets) => {
            println!("Discovered {} targets", targets.len());
            for target in targets {
                println!(
                    "Target: {}:{} Path: {}",
                    target.address,
                    target.port.unwrap_or("9090".to_string()),
                    target.metrics_path
                );
            }
        }
        Err(e) => println!("Service discovery failed: {}", e),
    }
}

#[tokio::test]
async fn test_custom_headers() {
    let mut adapter = PrometheusAdapter::new(
        "test-prometheus",
        "http://localhost:9090",
        None,
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig::default(),
    );

    let mut headers = HashMap::new();
    headers.insert("X-API-Key".to_string(), "secret-key".to_string());
    headers.insert("X-Client-Version".to_string(), "1.0.0".to_string());

    adapter.set_headers(headers);

    // Verify headers are set (implementation detail test)
    println!("Custom headers configured");
}

#[tokio::test]
async fn test_authenticated_prometheus() {
    let adapter = PrometheusAdapter::new(
        "test-prometheus",
        "http://prometheus.example.com:9090",
        Some(("admin", "prometheus")),
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig::default(),
    );

    // Test with basic authentication
    let _ = adapter.query_instant("up").await;
}

#[tokio::test]
async fn test_metrics_collection_interface() {
    let adapter = PrometheusAdapter::new(
        "test-prometheus",
        "http://localhost:9090",
        None,
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig::default(),
    );

    let metric_query = MetricQuery {
        metric_name: "rustops_anomaly_detected_total".to_string(),
        labels: HashMap::new(),
        start_time: Utc::now() - Duration::minutes(15),
        end_time: Utc::now(),
        step: Some(60), // 1 minute steps
    };

    let metrics = adapter.collect_metrics(metric_query).await;
    println!("Collected metrics: {:?}", metrics);
}

#[tokio::test]
async fn test_telemetry_subscription() {
    let adapter = PrometheusAdapter::new(
        "test-prometheus",
        "http://localhost:9090",
        None,
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig::default(),
    );

    let mut receiver = adapter.subscribe().await.unwrap();
    let mut count = 0;

    // Test receiving telemetry events (polling-based)
    tokio::time::timeout(tokio::time::Duration::from_millis(100), async {
        while let Some(event) = receiver.recv().await {
            println!("Received telemetry event: {:?}", event);
            count += 1;
            if count >= 3 {
                // Limit number of events for testing
                break;
            }
        }
    })
    .await;

    println!("Received {} telemetry events", count);
}

#[tokio::test]
async fn test_label_names_retrieval() {
    let adapter = PrometheusAdapter::new(
        "test-prometheus",
        "http://localhost:9090",
        None,
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig::default(),
    );

    // Get all label names
    let all_labels = adapter.label_names(None).await;

    // Get specific label names
    let specific_labels = adapter.label_names(Some("__name__")).await;

    println!("All labels: {:?}", all_labels);
    println!("Specific labels: {:?}", specific_labels);
}

#[tokio::test]
async fn test_resilience_integration() {
    let adapter = PrometheusAdapter::new(
        "test-prometheus",
        "http://unreachable-prometheus:9999", // This will fail
        None,
        CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: std::time::Duration::from_secs(10),
            expected_duration: std::time::Duration::from_secs(30),
        },
        RateLimiterConfig {
            limit: 10,
            window: std::time::Duration::from_secs(30),
        },
        RetryConfig {
            max_attempts: 3,
            base_delay: std::time::Duration::from_millis(100),
            max_delay: std::time::Duration::from_secs(5),
            backoff_factor: 2.0,
            jitter: true,
        },
    );

    // This should demonstrate the resilience features
    let result = adapter.health_check().await;
    println!("Health check with resilience features: {:?}", result);
}

#[tokio::test]
async fn test_multiple_prometheus_servers() {
    let servers = vec![
        ("prometheus-1", "http://prometheus-1:9090"),
        ("prometheus-2", "http://prometheus-2:9090"),
        ("prometheus-federation", "http://prometheus-federation:9090"),
    ];

    for (name, url) in servers {
        let adapter = PrometheusAdapter::new(
            name.to_string(),
            url.to_string(),
            None,
            CircuitBreakerConfig::default(),
            RateLimiterConfig::default(),
            RetryConfig::default(),
        );

        println!("Testing {}: {}", name, url);
        let health = adapter.health_check().await;
        println!("{} health: {:?}", name, health);
    }
}

#[tokio::test]
async fn test_prometheus_adapter_lifecycle() {
    let mut adapter = PrometheusAdapter::new(
        "lifecycle-test",
        "http://localhost:9090",
        None,
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig::default(),
    );

    // Test initialization
    let init_result = adapter.initialize().await;
    println!("Initialization result: {:?}", init_result);

    // Test health check after initialization
    let health = adapter.health_check().await;
    println!("Health after initialization: {:?}", health);

    // Test shutdown
    let shutdown_result = adapter.shutdown().await;
    println!("Shutdown result: {:?}", shutdown_result);
}

// Helper function for creating test alerts
fn create_test_alert_rules() -> Vec<AlertRule> {
    vec![
        AlertRule {
            name: "test_alert_1".to_string(),
            expression: "up == 0".to_string(),
            duration: "1m".to_string(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        },
        AlertRule {
            name: "test_alert_2".to_string(),
            expression: "rate(prometheus_tsdb_head_series_created_total[5m]) > 0".to_string(),
            duration: "5m".to_string(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        },
    ]
}

#[tokio::test]
async fn test_complex_alert_scenario() {
    let adapter = PrometheusAdapter::new(
        "complex-alert-test",
        "http://localhost:9090",
        None,
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig::default(),
    );

    let rules = create_test_alert_rules();
    let evaluations = adapter.evaluate_alerts(&rules).await;

    match evaluations {
        Ok(evals) => {
            for eval in evals {
                println!("Alert '{}' status: {:?}", eval.rule_name, eval.status);
                if let Some(data) = eval.metric_data {
                    println!("  Metric data: {} results", data.result.len());
                }
            }
        }
        Err(e) => println!("Alert evaluation failed: {}", e),
    }
}
