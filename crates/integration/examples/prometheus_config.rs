//! Configuration example for Prometheus integration
//!
//! This example shows how to configure the Prometheus adapter
//! with various options including authentication, resilience settings,
//! and service discovery configurations.

use rustops_integration::{
    adapter::IntegrationConfig,
    prometheus::{
        AlertRule, KubernetesSDConfig, PrometheusAdapter, RelabelAction, RelabelConfig,
        ServiceDiscoveryConfig, StaticTarget,
    },
    CircuitBreakerConfig, RateLimiterConfig, RetryConfig,
};
use std::collections::HashMap;

/// Example configuration for production environment
pub fn production_prometheus_config() -> IntegrationConfig {
    IntegrationConfig {
        id: "prometheus-production".to_string(),
        kind: crate::IntegrationKind::TelemetryCollector,
        enabled: true,
        circuit_breaker: CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: std::time::Duration::from_secs(30),
            expected_duration: std::time::Duration::from_secs(5),
        },
        rate_limiter: RateLimiterConfig {
            limit: 1000,
            window: std::time::Duration::from_secs(60),
        },
        retry: RetryConfig {
            max_attempts: 5,
            base_delay: std::time::Duration::from_millis(100),
            max_delay: std::time::Duration::from_secs(10),
            backoff_factor: 2.0,
            jitter: true,
        },
        timeout: std::time::Duration::from_secs(30),
    }
}

/// Create Prometheus adapter with production configuration
pub fn create_production_adapter() -> PrometheusAdapter {
    PrometheusAdapter::new(
        "prometheus-production",
        "https://prometheus-prod.example.com:9090",
        Some(("admin", "secure-password")),
        CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: std::time::Duration::from_secs(30),
            expected_duration: std::time::Duration::from_secs(5),
        },
        RateLimiterConfig {
            limit: 1000,
            window: std::time::Duration::from_secs(60),
        },
        RetryConfig {
            max_attempts: 5,
            base_delay: std::time::Duration::from_millis(100),
            max_delay: std::time::Duration::from_secs(10),
            backoff_factor: 2.0,
            jitter: true,
        },
    )
}

/// Example service discovery configuration for Kubernetes
pub fn kubernetes_service_discovery() -> ServiceDiscoveryConfig {
    ServiceDiscoveryConfig {
        kubernetes_sd: Some(KubernetesSDConfig {
            namespaces: vec![
                "default".to_string(),
                "kube-system".to_string(),
                "monitoring".to_string(),
            ],
            selectors: {
                let mut selectors = HashMap::new();
                selectors.insert("app".to_string(), "prometheus".to_string());
                selectors
            },
        }),
        static_configs: None,
        relabel_configs: Some(vec![
            RelabelConfig {
                source_labels: vec!["__meta_kubernetes_pod_label_app".to_string()],
                separator: Some("|".to_string()),
                regex: Some("prometheus".to_string()),
                modulus: None,
                replacement: "".to_string(),
                action: RelabelAction::Keep,
                target_label: None,
            },
            RelabelConfig {
                source_labels: vec!["__meta_kubernetes_pod_name".to_string()],
                separator: Some("_".to_string()),
                regex: None,
                modulus: None,
                replacement: "".to_string(),
                action: RelabelAction::Replace,
                target_label: Some("instance".to_string()),
            },
        ]),
    }
}

/// Example static service discovery configuration
pub fn static_service_discovery() -> ServiceDiscoveryConfig {
    ServiceDiscoveryConfig {
        kubernetes_sd: None,
        static_configs: Some(vec![
            StaticTarget {
                targets: vec![
                    "prometheus-1:9090".to_string(),
                    "prometheus-2:9090".to_string(),
                ],
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("cluster".to_string(), "east".to_string());
                    labels.insert("env".to_string(), "production".to_string());
                    labels
                },
            },
            StaticTarget {
                targets: vec![
                    "node-exporter-1:9100".to_string(),
                    "node-exporter-2:9100".to_string(),
                ],
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("job".to_string(), "node-exporter".to_string());
                    labels.insert("env".to_string(), "production".to_string());
                    labels
                },
            },
        ]),
        relabel_configs: None,
    }
}

/// Example alert rules for production monitoring
pub fn production_alert_rules() -> Vec<AlertRule> {
    vec![
        AlertRule {
            name: "prometheus_down".to_string(),
            expression: "up{job=\"prometheus\"} == 0".to_string(),
            duration: "5m".to_string(),
            labels: {
                let mut labels = HashMap::new();
                labels.insert("severity".to_string(), "critical".to_string());
                labels.insert("category".to_string(), "availability".to_string());
                labels.insert("team".to_string(), "platform".to_string());
                labels
            },
            annotations: {
                let mut annotations = HashMap::new();
                annotations.insert(
                    "summary".to_string(),
                    "Prometheus server is down".to_string(),
                );
                annotations.insert(
                    "description".to_string(),
                    "Prometheus server has been down for more than 5 minutes".to_string(),
                );
                annotations.insert(
                    "runbook_url".to_string(),
                    "https://runbooks.example.com/prometheus-down".to_string(),
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
                labels.insert("team".to_string(), "platform".to_string());
                labels
            },
            annotations: {
                let mut annotations = HashMap::new();
                annotations.insert(
                    "summary".to_string(),
                    "High error rate detected".to_string(),
                );
                annotations.insert(
                    "description".to_string(),
                    "HTTP error rate has been above 10% for 10 minutes".to_string(),
                );
                annotations
            },
        },
        AlertRule {
            name: "prometheus_tsdb_compaction_failed".to_string(),
            expression: "prometheus_tsdb_compension_failed_total > 0".to_string(),
            duration: "15m".to_string(),
            labels: {
                let mut labels = HashMap::new();
                labels.insert("severity".to_string(), "critical".to_string());
                labels.insert("category".to_string(), "data_integrity".to_string());
                labels.insert("team".to_string(), "platform".to_string());
                labels
            },
            annotations: {
                let mut annotations = HashMap::new();
                annotations.insert("summary".to_string(), "TSDB compaction failed".to_string());
                annotations.insert(
                    "description".to_string(),
                    "Prometheus TSDB compaction has failed".to_string(),
                );
                annotations.insert(
                    "runbook_url".to_string(),
                    "https://runbooks.example.com/prometheus-tsdb-issues".to_string(),
                );
                annotations
            },
        },
    ]
}

/// Development configuration
pub fn development_prometheus_config() -> IntegrationConfig {
    IntegrationConfig {
        id: "prometheus-development".to_string(),
        kind: crate::IntegrationKind::TelemetryCollector,
        enabled: true,
        circuit_breaker: CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout: std::time::Duration::from_secs(10),
            expected_duration: std::time::Duration::from_secs(2),
        },
        rate_limiter: RateLimiterConfig {
            limit: 100,
            window: std::time::Duration::from_secs(30),
        },
        retry: RetryConfig {
            max_attempts: 3,
            base_delay: std::time::Duration::from_millis(100),
            max_delay: std::time::Duration::from_secs(5),
            backoff_factor: 2.0,
            jitter: false,
        },
        timeout: std::time::Duration::from_secs(10),
    }
}

/// Create Prometheus adapter with development configuration
pub fn create_development_adapter() -> PrometheusAdapter {
    PrometheusAdapter::new(
        "prometheus-development",
        "http://prometheus-dev:9090",
        None, // No auth in dev
        CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout: std::time::Duration::from_secs(10),
            expected_duration: std::time::Duration::from_secs(2),
        },
        RateLimiterConfig {
            limit: 100,
            window: std::time::Duration::from_secs(30),
        },
        RetryConfig {
            max_attempts: 3,
            base_delay: std::time::Duration::from_millis(100),
            max_delay: std::time::Duration::from_secs(5),
            backoff_factor: 2.0,
            jitter: false,
        },
    )
}

/// Configuration for federated Prometheus setup
pub fn federated_prometheus_config() -> ServiceDiscoveryConfig {
    ServiceDiscoveryConfig {
        kubernetes_sd: None,
        static_configs: Some(vec![
            StaticTarget {
                targets: vec!["prometheus-primary:9090".to_string()],
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("role".to_string(), "primary".to_string());
                    labels
                },
            },
            StaticTarget {
                targets: vec![
                    "prometheus-remote-1:9090".to_string(),
                    "prometheus-remote-2:9090".to_string(),
                ],
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("role".to_string(), "remote".to_string());
                    labels
                },
            },
        ]),
        relabel_configs: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_config() {
        let config = production_prometheus_config();
        assert_eq!(config.id, "prometheus-production");
        assert!(config.enabled);
        assert_eq!(config.retry.max_attempts, 5);
    }

    #[test]
    fn test_development_config() {
        let config = development_prometheus_config();
        assert_eq!(config.id, "prometheus-development");
        assert_eq!(config.retry.max_attempts, 3);
    }

    #[test]
    fn test_alert_rules() {
        let rules = production_alert_rules();
        assert_eq!(rules.len(), 3);
        assert!(rules.iter().all(|r| r.labels.contains_key("severity")));
    }

    #[test]
    fn test_service_discovery_configs() {
        let k8s_config = kubernetes_service_discovery();
        assert!(k8s_config.kubernetes_sd.is_some());

        let static_config = static_service_discovery();
        assert!(static_config.static_configs.is_some());
    }
}
