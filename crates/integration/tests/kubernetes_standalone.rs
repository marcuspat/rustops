// Standalone test for Kubernetes adapter
//
// This test verifies the Kubernetes adapter implementation works correctly

use rustops_integration::infrastructure::kubernetes::{KubernetesAdapter, KubernetesConfig};
use rustops_integration::adapter::IntegrationKind;
use std::time::Duration;

#[tokio::test]
async fn test_kubernetes_adapter_creation() {
    let config = KubernetesConfig::default();
    let adapter = KubernetesAdapter::new(config);

    assert!(adapter.id().starts_with("kubernetes-"));
    assert_eq!(adapter.kind(), IntegrationKind::InfrastructureMonitor);
}

#[tokio::test]
async fn test_kubernetes_adapter_config() {
    let config = KubernetesConfig {
        context: Some("test-context".to_string()),
        namespace: Some("test-namespace".to_string()),
        resync_interval: Duration::from_secs(120),
        max_watch_streams: 50,
        exec_timeout: Duration::from_secs(60),
        log_line_limit: 5000,
    };

    let adapter = KubernetesAdapter::new(config);
    assert!(adapter.id().starts_with("kubernetes-"));
}

#[tokio::test]
fn test_namespace_inference() {
    // Test without env var
    let ns1 = KubernetesAdapter::infer_namespace();
    assert!(ns1.is_none());

    // Test with env var
    std::env::set_var("NAMESPACE", "test-namespace");
    let ns2 = KubernetesAdapter::infer_namespace();
    assert_eq!(ns2, Some("test-namespace".to_string()));
    std::env::remove_var("NAMESPACE");
}

#[test]
fn test_kubernetes_config_defaults() {
    let config = KubernetesConfig::default();
    assert_eq!(config.resync_interval, Duration::from_secs(60));
    assert_eq!(config.max_watch_streams, 100);
    assert_eq!(config.exec_timeout, Duration::from_secs(30));
    assert_eq!(config.log_line_limit, 10000);
}