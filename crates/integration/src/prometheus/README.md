# Prometheus Integration for RustOps

The Prometheus adapter provides comprehensive integration with Prometheus for metrics collection, querying, and alert management. It follows RustOps integration patterns with resilience features and async/await support.

## Features

- ✅ **Query Support**: Instant and range queries for Prometheus metrics
- ✅ **Alert Evaluation**: Evaluate Prometheus alert rules and get firing alerts
- ✅ **Service Discovery**: Kubernetes and static service discovery
- ✅ **Resilience**: Built-in circuit breaker, rate limiting, and retry logic
- ✅ **Async/Await**: Full async support with tokio
- ✅ **Type Safety**: Strongly typed with comprehensive error handling

## Quick Start

```rust
use rustops_integration::prometheus::PrometheusAdapter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create adapter
    let prometheus = PrometheusAdapter::new(
        "my-prometheus",
        "http://localhost:9090",
        None, // No authentication
        CircuitBreakerConfig::default(),
        RateLimiterConfig::default(),
        RetryConfig::default(),
    );

    // Query metrics
    let response = prometheus.query_instant("up").await?;
    println!("Found {} metrics", response.data.result.len());

    Ok(())
}
```

## Configuration

### Basic Configuration

```rust
use rustops_integration::{
    prometheus::PrometheusAdapter,
    CircuitBreakerConfig, RateLimiterConfig, RetryConfig,
};

let adapter = PrometheusAdapter::new(
    "prometheus-integration",  // Unique identifier
    "https://prometheus.example.com:9090",  // URL
    Some(("username", "password")),  // Optional basic auth
    CircuitBreakerConfig::default(),
    RateLimiterConfig::default(),
    RetryConfig::default(),
);
```

### Custom Headers

```rust
let mut adapter = PrometheusAdapter::new(/* ... */);

let mut headers = std::collections::HashMap::new();
headers.insert("X-API-Key".to_string(), "your-api-key".to_string());
adapter.set_headers(headers);
```

## Usage Examples

### Query Metrics

```rust
// Instant query for current values
let response = prometheus.query_instant("up").await?;

// Range query for historical data
let start = chrono::Utc::now() - chrono::Duration::hours(1);
let end = chrono::Utc::now();
let response = prometheus.query_range("cpu_usage", start, end, "1m").await?;
```

### Evaluate Alerts

```rust
use rustops_integration::prometheus::AlertRule;

let rules = vec
![
    AlertRule {
        name: "high_cpu".to_string(),
        expression: "process_cpu_seconds_total > 0.8".to_string(),
        duration: "5m".to_string(),
        labels: std::collections::HashMap::new(),
        annotations: std::collections::HashMap::new(),
    },
];

let evaluations = prometheus.evaluate_alerts(&rules).await?;
for eval in evaluations {
    match eval.status {
        AlertStatus::Firing => println!("🚨 Alert {} is firing!", eval.rule_name),
        _ => println!("✅ Alert {} is OK", eval.rule_name),
    }
}
```

### Service Discovery

```rust
use rustops_integration::prometheus::{ServiceDiscoveryConfig, KubernetesSDConfig};

// Kubernetes service discovery
let k8s_config = ServiceDiscoveryConfig {
    kubernetes_sd: Some(KubernetesSDConfig {
        namespaces: vec
!["default".to_string(), "monitoring".to_string()],
        selectors: std::collections::HashMap::new(),
    }),
    static_configs: None,
    relabel_configs: None,
};

let targets = prometheus.discover_services(&k8s_config).await?;
for target in targets {
    println!("Target: {}:{}/{}",
        target.address,
        target.port.as_ref().unwrap_or(&"9090".to_string()),
        target.metrics_path);
}
```

### Telemetry Collection

```rust
use rustops_integration::adapter::MetricQuery;

let query = MetricQuery {
    metric_name: "rustops_anomaly_detected_total".to_string(),
    labels: std::collections::HashMap::new(),
    start_time: chrono::Utc::now() - chrono::Duration::hours(1),
    end_time: chrono::Utc::now(),
    step: Some(60),  // 1 minute intervals
};

let metrics = prometheus.collect_metrics(query).await?;
for metric in metrics {
    println!("{}: {} @ {}", metric.name, metric.value, metric.timestamp);
}
```

### Real-time Subscription

```rust
use tokio::sync::mpsc;

let mut receiver = prometheus.subscribe().await?;

while let Some(event) = receiver.recv().await {
    match event {
        TelemetryEvent::Metric(metric) => {
            println!("📊 Metric: {} = {}", metric.name, metric.value);
        }
        TelemetryEvent::Alert(alert) => {
            println!("🚨 Alert: {} - {:?}", alert.rule_name, alert.status);
        }
    }
}
```

## Advanced Features

### Custom Retry Strategy

```rust
let retry_config = RetryConfig {
    max_attempts: 5,
    base_delay: std::time::Duration::from_millis(100),
    max_delay: std::time::Duration::from_secs(30),
    backoff_factor: 2.0,
    jitter: true,  // Add randomness to avoid thundering herd
};
```

### Circuit Breaker

```rust
let circuit_config = CircuitBreakerConfig {
    failure_threshold: 3,  // Open after 3 failures
    recovery_timeout: std::time::Duration::from_secs(60),
    expected_duration: std::time::Duration::from_secs(30),
};
```

### Rate Limiting

```rust
let rate_config = RateLimiterConfig {
    limit: 1000,  // 1000 requests per window
    window: std::time::Duration::from_secs(60),  // 60-second window
};
```

## Integration with RustOps

The Prometheus adapter implements the `TelemetryCollector` trait, making it compatible with the RustOps integration framework:

```rust
use rustops_integration::adapter::TelemetryCollector;

impl TelemetryCollector for PrometheusAdapter {
    async fn collect_metrics(&self, query: MetricQuery) -> Result<Vec<Metric>, IntegrationError> {
        // Implementation
    }

    async fn collect_logs(&self, query: LogQuery) -> Result<LogStream, IntegrationError> {
        // Implementation
    }

    async fn collect_traces(&self, query: TraceQuery) -> Result<Vec<Trace>, IntegrationError> {
        // Implementation
    }

    async fn subscribe(&self) -> Result<mpsc::Receiver<TelemetryEvent>, IntegrationError> {
        // Implementation
    }
}
```

## Error Handling

The adapter returns `IntegrationResult<T>` for all operations:

```rust
use rustops_integration::resilience::{IntegrationError, IntegrationResult};

match prometheus.query_instant("up").await {
    Ok(response) => /* Handle success */,
    Err(IntegrationError::Network { message }) => /* Handle network error */,
    Err(IntegrationError::Timeout { duration_ms }) => /* Handle timeout */,
    Err(e) => /* Handle other errors */,
}
```

## Testing

Run the integration tests:

```bash
cargo test prometheus_integration --features integration_tests
```

## Performance Considerations

1. **Connection Pooling**: The underlying HTTP client uses connection pooling
2. **Async Operations**: All operations are non-blocking
3. **Resilience**: Built-in features handle failures gracefully
4. **Rate Limiting**: Prevents overwhelming the Prometheus server

## Security

- Basic authentication support
- Custom headers for API keys
- Circuit breaker protects against DoS
- Secure defaults for timeouts and retries

## Examples

See the examples in `examples/` for complete usage scenarios:

- `prometheus_client.rs` - Basic client example
- `prometheus_config.rs` - Configuration examples

## Troubleshooting

### Common Issues

1. **Connection Refused**: Check Prometheus URL and network connectivity
2. **Authentication Failed**: Verify credentials and permissions
3. **Timeout Errors**: Increase timeout or check network latency
4. **High Error Rate**: Adjust circuit breaker and retry settings

### Debug Mode

Enable debug logging:

```rust
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

## License

This integration is part of RustOps and follows the same license terms.