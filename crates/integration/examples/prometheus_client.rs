//! Example of using the Prometheus adapter for RustOps integration
//!
//! This example demonstrates how to:
//! - Create a Prometheus adapter
//! - Query metrics
//! - Evaluate alert rules
//! - Discover services
//! - Collect telemetry data

use chrono::{DateTime, Duration, Utc};
use rustops_integration::{
    adapter::{IntegrationAdapter, MetricQuery, TelemetryCollector, TelemetryEvent},
    prometheus::{
        AlertEvaluation, AlertRule, AlertStatus, KubernetesSDConfig, PrometheusAdapter,
        PrometheusQuery, RelabelAction, RelabelConfig, ServiceDiscoveryConfig, ServiceTarget,
    },
    CircuitBreakerConfig, RateLimiterConfig, RetryConfig,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 RustOps Prometheus Integration Example");

    // Create Prometheus adapter with configuration
    let prometheus = PrometheusAdapter::new(
        "rustops-prometheus",    // Unique adapter ID
        "http://localhost:9090", // Prometheus URL
        None,                    // Basic auth (None for no auth)
        CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: std::time::Duration::from_secs(10),
            expected_duration: std::time::Duration::from_secs(30),
        },
        RateLimiterConfig {
            limit: 100,                                 // 100 requests per window
            window: std::time::Duration::from_secs(60), // 60-second window
        },
        RetryConfig {
            max_attempts: 3,
            base_delay: std::time::Duration::from_millis(100),
            max_delay: std::time::Duration::from_secs(5),
            backoff_factor: 2.0,
            jitter: true,
        },
    );

    // Initialize the adapter
    println!("📡 Initializing Prometheus adapter...");
    match prometheus.initialize().await {
        Ok(_) => println!("✅ Prometheus adapter initialized successfully"),
        Err(e) => {
            println!("❌ Failed to initialize: {}", e);
            return Ok(());
        }
    }

    // 1. Health Check
    println!("\n🏥 Checking Prometheus health...");
    match prometheus.health_check().await {
        Ok(health) => println!("✅ Health status: {:?}", health),
        Err(e) => println!("❌ Health check failed: {}", e),
    }

    // 2. Query metrics
    println!("\n📊 Querying Prometheus metrics...");
    match prometheus.query_instant("up").await {
        Ok(response) => {
            println!(
                "✅ Query successful - Found {} metrics",
                response.data.result.len()
            );
            for metric in response.data.result {
                println!("  - Metric: {:?}", metric.metric);
            }
        }
        Err(e) => println!("❌ Query failed: {}", e),
    }

    // 3. Range query
    println!("\n📈 Performing range query...");
    let start = Utc::now() - Duration::minutes(5);
    let end = Utc::now();

    match prometheus
        .query_range("prometheus_build_info", start, end, "1m")
        .await
    {
        Ok(response) => {
            println!("✅ Range query successful");
            for metric in response.data.result {
                if let Some(values) = metric.values {
                    println!("  - Data points: {}", values.len());
                } else if let Some(value) = metric.value {
                    println!("  - Value: {:?}", value);
                }
            }
        }
        Err(e) => println!("❌ Range query failed: {}", e),
    }

    // 4. Alert rule evaluation
    println!("\n🚨 Evaluating alert rules...");
    let alert_rules = vec![
        AlertRule {
            name: "prometheus_down".to_string(),
            expression: "up == 0".to_string(),
            duration: "5m".to_string(),
            labels: {
                let mut labels = HashMap::new();
                labels.insert("severity".to_string(), "critical".to_string());
                labels.insert("category".to_string(), "availability".to_string());
                labels
            },
            annotations: {
                let mut annotations = HashMap::new();
                annotations.insert("summary".to_string(), "Prometheus server down".to_string());
                annotations.insert(
                    "description".to_string(),
                    "Prometheus server has been down for more than 5 minutes".to_string(),
                );
                annotations
            },
        },
        AlertRule {
            name: "high_error_rate".to_string(),
            expression: "rate(prometheus_http_requests_total{status=~\"5..\"}[5m]) > 0.1"
                .to_string(),
            duration: "10m".to_string(),
            labels: {
                let mut labels = HashMap::new();
                labels.insert("severity".to_string(), "warning".to_string());
                labels.insert("category".to_string(), "errors".to_string());
                labels
            },
            annotations: HashMap::new(),
        },
    ];

    match prometheus.evaluate_alerts(&alert_rules).await {
        Ok(evaluations) => {
            println!("✅ Alert evaluation completed");
            for eval in evaluations {
                println!("  - Alert '{}': {:?}", eval.rule_name, eval.status);
                if eval.status == AlertStatus::Firing {
                    println!("    🚨 Firing!");
                }
            }
        }
        Err(e) => println!("❌ Alert evaluation failed: {}", e),
    }

    // 5. Service discovery
    println!("\n🔍 Discovering services...");
    let sd_config = ServiceDiscoveryConfig {
        kubernetes_sd: Some(KubernetesSDConfig {
            namespaces: vec!["default".to_string(), "kube-system".to_string()],
            selectors: HashMap::new(),
        }),
        static_configs: Some(vec![
            ServiceTarget {
                targets: vec!["localhost:9090".to_string()],
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("job".to_string(), "prometheus".to_string());
                    labels
                },
            },
            ServiceTarget {
                targets: vec!["node-exporter:9100".to_string()],
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("job".to_string(), "node-exporter".to_string());
                    labels
                },
            },
        ]),
        relabel_configs: Some(vec![RelabelConfig {
            source_labels: vec!["__meta_kubernetes_pod_label_app".to_string()],
            separator: Some("|".to_string()),
            regex: None,
            modulus: None,
            replacement: "".to_string(),
            action: RelabelAction::Keep,
            target_label: None,
        }]),
    };

    match prometheus.discover_services(&sd_config).await {
        Ok(targets) => {
            println!("✅ Discovered {} targets", targets.len());
            for target in targets {
                println!(
                    "  - {}:{} Path: {}",
                    target.address,
                    target.port.as_ref().unwrap_or(&"9090".to_string()),
                    target.metrics_path
                );
                if let Some(error) = target.error {
                    println!("    ⚠️  Error: {}", error);
                }
            }
        }
        Err(e) => println!("❌ Service discovery failed: {}", e),
    }

    // 6. Telemetry collection
    println!("\n📡 Collecting telemetry...");
    let metrics_query = MetricQuery {
        metric_name: "prometheus_build_info".to_string(),
        labels: HashMap::new(),
        start_time: Utc::now() - Duration::minutes(1),
        end_time: Utc::now(),
        step: None,
    };

    match prometheus.collect_metrics(metrics_query).await {
        Ok(metrics) => {
            println!("✅ Collected {} metrics", metrics.len());
            for metric in metrics {
                println!(
                    "  - {}: {} @ {:?}",
                    metric.name, metric.value, metric.timestamp
                );
            }
        }
        Err(e) => println!("❌ Metrics collection failed: {}", e),
    }

    // 7. Telemetry subscription
    println!("\n📻 Subscribing to telemetry events...");
    let mut receiver = match prometheus.subscribe().await {
        Ok(receiver) => receiver,
        Err(e) => {
            println!("❌ Failed to subscribe: {}", e);
            return Ok(());
        }
    };

    // Listen for events for a short time
    println!("Listening for events for 5 seconds...");
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(event) = receiver.recv().await {
            match event {
                TelemetryEvent::Metric(metric) => {
                    println!(
                        "📊 Metric: {} = {} @ {}",
                        metric.name, metric.value, metric.timestamp
                    );
                }
                TelemetryEvent::Alert(alert) => {
                    println!("🚨 Alert: {} - {:?}", alert.rule_name, alert.status);
                }
            }
        }
    })
    .await?;

    println!("\n✅ Example completed successfully!");
    Ok(())
}

// Helper function to format duration
fn format_duration(duration: Duration) -> String {
    let secs = duration.num_seconds();
    let mins = secs / 60;
    let hours = mins / 60;

    if hours > 0 {
        format!("{}h{}m", hours, mins % 60)
    } else {
        format!("{}m{}s", mins, secs % 60)
    }
}
