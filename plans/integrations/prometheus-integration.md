# Prometheus Integration Guide

**Integration Type**: Metrics Collection
**Priority**: Critical (Phase 1)
**Status**: Design

---

## Overview

Prometheus is the de facto standard for metrics collection in cloud-native and Kubernetes environments. RustOps integrates with Prometheus for both **pull-based scraping** (Prometheus server pulls from RustOps) and **push-based remote write** (RustOps pushes to Prometheus).

### Integration Options

| Option | Description | Use Case | Complexity |
|--------|-------------|----------|------------|
| **Pull Mode** | Prometheus scrapes RustOps `/metrics` endpoint | Standard monitoring | Low |
| **Push Mode** | RustOps pushes to Prometheus via Remote Write | High-volume metrics | Medium |
| **Federation** | RustOps scrapes other Prometheus servers | Multi-cluster aggregation | Medium |
| **Thanos** | Integration with Thanos for long-term storage | Global metrics view | High |

### Why Prometheus is #1 Priority

- **40%+** of Kubernetes deployments use Prometheus
- **Native integration** with Kubernetes service discovery
- **Rich ecosystem** of exporters (node, database, custom)
- **OpenTelemetry compatibility** via Prometheus Remote Write
- **CNCF graduation** ensures long-term stability

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    RustOps - Prometheus Integration                      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                    RustOps Metrics Export                        │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │   │
│  │  │   Agent     │  │  Anomaly    │  │  Remediation│             │   │
│  │  │  Metrics    │  │  Detection  │  │   Metrics   │             │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘             │   │
│  │         │                │                │                      │   │
│  │         └────────────────┼────────────────┘                      │   │
│  │                          ▼                                       │   │
│  │  ┌─────────────────────────────────────────────────────────┐    │   │
│  │  │              Prometheus Metrics Exposer                  │    │   │
│  │  │  - /metrics endpoint (pull mode)                        │    │   │
│  │  │  - Remote Write client (push mode)                      │    │   │
│  │  │  - Histogram/Summary/Gauge/Counter support              │    │   │
│  │  └─────────────────────────────────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                           │                         │                   │
│                    Pull │                         │ Push                │
│                           ▼                         ▼                   │
│  ┌─────────────────────────────────────┐  ┌───────────────────────────┐ │
│  │        Prometheus Server            │  │   Prometheus (Remote      │ │
│  │  - Scrapes /metrics endpoint        │  │   Write)                 │ │
│  │  - Service discovery (K8s, DNS)     │  │  - Receives pushed       │ │
│  │  - Alert evaluation                 │  │    metrics               │ │
│  └─────────────────────────────────────┘  └───────────────────────────┘ │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation

### Rust Dependencies

```toml
[dependencies]
# Prometheus client
prometheus = { version = "0.13", features = ["process"] }
prometheus-static-metric = "0.5"

# Hyper for serving /metrics endpoint
hyper = { version = "0.14", features = ["server", "http1", "tcp"] }
tokio = { version = "1.0", features = ["full"] }

# For Prometheus Remote Write (push mode)
prost = "0.12"
tonic = "0.10"
```

### Pull Mode Implementation

```rust
use prometheus::{
    Encoder, TextEncoder, Registry, Counter, Gauge, Histogram, IntCounter,
    IntGauge, IntCounterVec, GaugeVec, HistogramVec,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Prometheus metrics exporter
pub struct PrometheusExporter {
    registry: Registry,
    metrics: Arc<RustOpsMetrics>,
}

/// RustOps-specific metrics
pub struct RustOpsMetrics {
    // Alert metrics
    pub alerts_received_total: IntCounter,
    pub alerts_correlated_total: IntCounter,
    pub_alerts_false_positive_total: IntCounter,

    // Anomaly detection metrics
    pub anomalies_detected_total: IntCounterVec,
    pub anomaly_detection_duration_seconds: HistogramVec,

    // Remediation metrics
    pub remediations_attempted_total: IntCounterVec,
    pub remediations_succeeded_total: IntCounterVec,
    pub remediations_failed_total: IntCounterVec,

    // Infrastructure metrics
    pub monitored_endpoints: IntGauge,
    pub healthy_endpoints: IntGauge,

    // Performance metrics
    pub event_processing_duration_seconds: Histogram,
    pub storage_operations_duration_seconds: HistogramVec,
}

impl RustOpsMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let anomalies_detected_total = IntCounterVec::new(
            prometheus::Opts {
                namespace: "rustops".into(),
                subsystem: "anomaly".into(),
                name: "detected_total".into(),
                help: "Total number of anomalies detected by type".into(),
            },
            &["type", "severity"],
        )?;

        let anomaly_detection_duration = HistogramVec::new(
            prometheus::HistogramOpts {
                namespace: "rustops".into(),
                subsystem: "anomaly".into(),
                name: "detection_duration_seconds".into(),
                help: "Time taken to detect anomalies".into(),
                buckets: prometheus::exponential_buckets(0.001, 2.0, 10)?
            },
            &["type"],
        )?;

        // ... initialize other metrics ...

        Ok(Self {
            anomalies_detected_total,
            anomaly_detection_duration_seconds: anomaly_detection_duration,
            // ... other metrics ...
        })
    }

    pub fn register(&self, registry: &Registry) -> Result<(), prometheus::Error> {
        registry.register(Box::new(self.anomalies_detected_total.clone()))?;
        registry.register(Box::new(self.anomaly_detection_duration_seconds.clone()))?;
        // ... register other metrics ...
        Ok(())
    }
}

impl PrometheusExporter {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        let metrics = Arc::new(RustOpsMetrics::new()?);
        metrics.register(&registry)?;

        Ok(Self { registry, metrics })
    }

    pub fn metrics(&self) -> Arc<RustOpsMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Generate Prometheus text format metrics
    pub fn encode(&self) -> Result<String, prometheus::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }

    /// Serve /metrics endpoint
    pub async fn serve_metrics(
        &self,
        req: hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<hyper::Body>, hyper::Error> {
        match req.uri().path() {
            "/metrics" => {
                let metrics = self.encode().unwrap_or_else(|e| {
                    format!("# Error gathering metrics: {}", e)
                });

                Ok(hyper::Response::builder()
                    .status(200)
                    .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
                    .body(hyper::Body::from(metrics))
                    .unwrap())
            }
            "/health" => {
                Ok(hyper::Response::builder()
                    .status(200)
                    .body(hyper::Body::from("OK"))
                    .unwrap())
            }
            _ => Ok(hyper::Response::builder()
                .status(404)
                .body(hyper::Body::from("Not Found"))
                .unwrap()),
        }
    }
}

/// Start Prometheus metrics server
pub async fn start_metrics_server(exporter: Arc<PrometheusExporter>, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = ([0, 0, 0, 0], port).into();

    let make_svc = hyper::service::make_service_fn(move |_| {
        let exporter = Arc::clone(&exporter);
        async move {
            Ok::<_, hyper::Error>(hyper::service::service_fn(move |req| {
                exporter.serve_metrics(req)
            }))
        }
    });

    let server = hyper::Server::bind(&addr).serve(make_svc);

    tracing::info!("Prometheus metrics server listening on http://{}", addr);

    server.await?;
    Ok(())
}
```

### Push Mode (Remote Write) Implementation

```rust
use prost::Message;
use std::time::SystemTime;

/// Prometheus Remote Write client for pushing metrics
pub struct PrometheusRemoteWriteClient {
    endpoint: String,
    client: reqwest::Client,
    basic_auth: Option<(String, String)>,
    headers: Vec<(String, String)>,
}

impl PrometheusRemoteWriteClient {
    pub fn new(
        endpoint: String,
        basic_auth: Option<(String, String)>,
    ) -> Self {
        Self {
            endpoint,
            client: reqwest::Client::new(),
            basic_auth,
            headers: Vec::new(),
        }
    }

    /// Write metrics via Remote Write protocol
    pub async fn write_metrics(
        &self,
        metrics: Vec<WriteRequest>,
    ) -> Result<(), RemoteWriteError> {
        // Build protobuf WriteRequest
        let write_request = proto::prometheus::WriteRequest {
            timeseries: metrics.into_iter().map(|m| m.into()).collect(),
            metadata: Vec::new(),
        };

        let mut buf = Vec::new();
        write_request.encode(&mut buf)
            .map_err(|e| RemoteWriteError::Encode(e.to_string()))?;

        // Build HTTP request
        let mut request = self.client
            .post(&self.endpoint)
            .header("Content-Encoding", "snappy")
            .header("Content-Type", "application/x-protobuf")
            .header("X-Prometheus-Remote-Write-Version", "0.1.0");

        // Add basic auth
        if let Some((username, password)) = &self.basic_auth {
            request = request.basic_auth(username, Some(password));
        }

        // Add custom headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        // Compress with snappy
        let compressed = snap::raw::Encoder::new()
            .compress_vec(&buf)
            .map_err(|e| RemoteWriteError::Compression(e.to_string()))?;

        // Send request
        let response = request
            .body(compressed)
            .send()
            .await
            .map_err(|e| RemoteWriteError::Request(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(RemoteWriteError::Server {
                status: status.as_u16(),
                message: body,
            })
        }
    }
}

/// Write request metric
#[derive(Debug, Clone)]
pub struct WriteRequest {
    pub labels: Vec<(String, String)>,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub value: f64,
    pub timestamp: Option<i64>,
}

impl From<WriteRequest> for proto::prometheus::TimeSeries {
    fn from(req: WriteRequest) -> Self {
        proto::prometheus::TimeSeries {
            labels: req.labels.into_iter().map(|(k, v)| proto::prometheus::Label {
                name: k,
                value: v,
            }).collect(),
            samples: req.samples.into_iter().map(|s| proto::prometheus::Sample {
                value: s.value,
                timestamp: s.timestamp.unwrap_or_else(|| {
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64
                }),
            }).collect(),
            exemplars: Vec::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RemoteWriteError {
    #[error("Encode error: {0}")]
    Encode(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Request error: {0}")]
    Request(String),

    #[error("Server error {status}: {message}")]
    Server { status: u16, message: String },
}
```

### Metrics Query Implementation

```rust
use regex::Regex;

/// Query metrics from Prometheus
pub struct PrometheusQueryClient {
    prometheus_url: String,
    client: reqwest::Client,
}

impl PrometheusQueryClient {
    pub fn new(prometheus_url: String) -> Self {
        Self {
            prometheus_url,
            client: reqwest::Client::new(),
        }
    }

    /// Execute instant query
    pub async fn query(
        &self,
        query: &str,
        timestamp: Option<i64>,
    ) -> Result<prometheus::Response, QueryError> {
        let mut url = format!("{}/api/v1/query", self.prometheus_url);

        let mut params = Vec::new();
        params.push(("query", query));
        if let Some(ts) = timestamp {
            params.push(("time", &ts.to_string()));
        }

        let response = self.client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| QueryError::Request(e.to_string()))?;

        if !response.status().is_success() {
            return Err(QueryError::Server {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let body: prometheus::Response = response
            .json()
            .await
            .map_err(|e| QueryError::Parse(e.to_string()))?;

        if body.status != "success" {
            return Err(QueryError::QueryFailed(body.error.unwrap_or_default()));
        }

        Ok(body)
    }

    /// Execute range query
    pub async fn query_range(
        &self,
        query: &str,
        start: i64,
        end: i64,
        step: i64,
    ) -> Result<prometheus::Response, QueryError> {
        let url = format!("{}/api/v1/query_range", self.prometheus_url);

        let response = self.client
            .get(&url)
            .query(&[
                ("query", query),
                ("start", &start.to_string()),
                ("end", &end.to_string()),
                ("step", &step.to_string()),
            ])
            .send()
            .await
            .map_err(|e| QueryError::Request(e.to_string()))?;

        // ... similar error handling ...
        Ok(response.json().await?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Request error: {0}")]
    Request(String),

    #[error("Server error {status}: {message}")]
    Server { status: u16, message: String },

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),
}
```

---

## Configuration

### Prometheus Scrape Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  # Scrape RustOps metrics
  - job_name: 'rustops'
    static_configs:
      - targets: ['rustops:9090']
    metrics_path: /metrics
    scrape_interval: 15s
    scrape_timeout: 10s

  # Scrape RustOps agents
  - job_name: 'rustops-agents'
    kubernetes_sd_configs:
      - role: pod
        namespaces:
          names:
            - rustops
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        action: keep
        regex: rustops-agent
      - source_labels: [__meta_kubernetes_pod_name]
        target_label: pod
      - source_labels: [__meta_kubernetes_pod_node_name]
        target_label: node

  # Scrape other Prometheus servers (federation)
  - job_name: 'prometheus-federation'
    honor_labels: true
    metrics_path: /federate
    params:
      'match[]':
        - '{__name__=~"rustops_.*"}'
    static_configs:
      - targets:
          - 'prometheus-1:9090'
          - 'prometheus-2:9090'
```

### RustOps Integration Configuration

```yaml
integrations:
  prometheus:
    enabled: true

    # Pull mode - expose /metrics endpoint
    pull_mode:
      enabled: true
      listen_address: "0.0.0.0:9090"
      metrics_path: "/metrics"

    # Push mode - Remote Write
    push_mode:
      enabled: false
      endpoint: "http://prometheus:9090/api/v1/write"
      basic_auth:
        username: "${PROMETHEUS_USERNAME}"
        password: "${PROMETHEUS_PASSWORD}"
      batch_size: 1000
      flush_interval: 15s
      retry:
        max_attempts: 3
        initial_backoff: 1s

    # Query mode - for reading metrics
    query_mode:
      enabled: true
      url: "http://prometheus:9090"
      timeout: 30s

    # Metrics to collect
    scrape_configs:
      - job_name: 'kubernetes-pods'
        scrape_interval: 15s
        sample_limit: 10000

      - job_name: 'node-exporter'
        scrape_interval: 30s

      - job_name: 'kube-state-metrics'
        scrape_interval: 30s
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::Counter;

    #[tokio::test]
    async fn test_metrics_exposure() {
        let exporter = PrometheusExporter::new().unwrap();
        let metrics = exporter.encode().unwrap();

        // Verify metrics are in Prometheus format
        assert!(metrics.contains("# HELP"));
        assert!(metrics.contains("# TYPE"));

        // Check for our custom metrics
        assert!(metrics.contains("rustops_anomaly_detected_total"));
        assert!(metrics.contains("rustops_remediation_attempted_total"));
    }

    #[test]
    fn test_metric_labels() {
        let metrics = RustOpsMetrics::new().unwrap();

        // Test label-based metrics
        metrics
            .anomalies_detected_total
            .with_label_values(&["metric_spike", "critical"])
            .inc();

        // Verify metric was incremented
        let val = metrics
            .anomalies_detected_total
            .with_label_values(&["metric_spike", "critical"])
            .get();

        assert_eq!(val, 1);
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let exporter = Arc::new(PrometheusExporter::new().unwrap());

        // Make request to /metrics endpoint
        let req = hyper::Request::builder()
            .uri("/metrics")
            .body(hyper::Body::empty())
            .unwrap();

        let response = exporter.serve_metrics(req).await.unwrap();

        assert_eq!(response.status(), 200);

        // Check content type
        let content_type = response
            .headers()
            .get("Content-Type")
            .and_then(|v| v.to_str().ok());

        assert_eq!(
            content_type,
            Some("text/plain; version=0.0.4; charset=utf-8")
        );
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    #[ignore]  // Run with --ignored flag
    async fn test_prometheus_scrape() {
        // Start metrics server
        let exporter = Arc::new(PrometheusExporter::new().unwrap());
        let port = 19090;

        tokio::spawn(async move {
            start_metrics_server(exporter, port).await.unwrap();
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Scrape metrics
        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://localhost:{}/metrics", port))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        let body = response.text().await.unwrap();
        assert!(body.contains("rustops_"));
    }

    #[tokio::test]
    #[ignore]
    async fn test_prometheus_query() {
        let client = PrometheusQueryClient::new(
            std::env::var("PROMETHEUS_URL").unwrap_or_else(|_| "http://localhost:9090".into())
        );

        let result = client
            .query("up", None)
            .await
            .unwrap();

        assert_eq!(result.status, "success");
    }
}
```

### Mock Prometheus Server

```rust
#[cfg(test)]
pub mod mock_prometheus {
    use mockito::{Server, Mock};

    pub struct MockPrometheus {
        server: Server,
    }

    impl MockPrometheus {
        pub fn new() -> Self {
            Self {
                server: Server::new(),
            }
        }

        pub fn url(&self) -> String {
            self.server.url()
        }

        pub fn mock_query_success(&self) -> Mock {
            self.server
                .mock("GET", "/api/v1/query")
                .match_query(mockito::Matcher::AllOf(vec![
                    mockito::Matcher::UrlEncoded("query".into(), "up".into()),
                ]))
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(r#"{
                    "status": "success",
                    "data": {
                        "resultType": "vector",
                        "result": []
                    }
                }"#)
                .create()
        }

        pub fn mock_query_error(&self) -> Mock {
            self.server
                .mock("GET", "/api/v1/query")
                .with_status(400)
                .with_header("content-type", "application/json")
                .with_body(r#"{
                    "status": "error",
                    "errorType": "bad_data",
                    "error": "invalid query parameter"
                }"#)
                .create()
        }
    }
}
```

---

## Deployment

### Kubernetes Deployment

```yaml
apiVersion: v1
kind: Service
metadata:
  name: rustops-prometheus
  labels:
    app: rustops
spec:
  ports:
    - port: 9090
      targetPort: metrics
      name: metrics
  selector:
    app: rustops
  type: ClusterIP

---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustops
spec:
  replicas: 1
  selector:
    matchLabels:
      app: rustops
  template:
    metadata:
      labels:
        app: rustops
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      containers:
        - name: rustops
          image: rustops:latest
          ports:
            - name: metrics
              containerPort: 9090
          env:
            - name: RUSTOPS_PROMETHEUS_ENABLED
              value: "true"
            - name: RUSTOPS_PROMETHEUS_PORT
              value: "9090"
          readinessProbe:
            httpGet:
              path: /health
              port: 9090
          livenessProbe:
            httpGet:
              path: /health
              port: 9090
```

### ServiceMonitor (for Prometheus Operator)

```yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: rustops
  labels:
    app: rustops
spec:
  selector:
    matchLabels:
      app: rustops
  endpoints:
    - port: metrics
      interval: 15s
      path: /metrics
```

---

## Monitoring & Troubleshooting

### Key Metrics to Monitor

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `rustops_prometheus_scrape_errors_total` | Scrape error count | > 0 in 5m |
| `rustops_prometheus_remote_write_errors_total` | Remote write errors | > 0 in 5m |
| `rustops_prometheus_request_duration_seconds` | Request latency | p99 > 1s |
| `up{job="rustops"}` | Prometheus target status | == 0 |

### Common Issues

| Issue | Symptom | Solution |
|-------|---------|----------|
| **Port conflict** | Connection refused | Change metrics port in config |
| **Missing labels** | Metrics not appearing | Verify Prometheus relabel configs |
| **High cardinality** | Slow queries | Reduce label values |
| **Large metrics** | Scrape timeout | Increase scrape_timeout |

---

## References

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Prometheus Remote Write Protocol](https://prometheus.io/docs/prometheus/latest/configuration/remote_write/)
- [OpenTelemetry to Prometheus](https://opentelemetry.io/docs/reference/specification/protocol/otlp-to-prometheus/)
- [Kubernetes Prometheus Operator](https://github.com/prometheus-operator/prometheus-operator)

---

**Version**: 1.0
**Last Updated**: 2026-01-18
**Integration Phase**: Phase 1 (Foundation)
