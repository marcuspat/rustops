# RustOps Integration Strategy

## Overview

This document outlines the comprehensive integration strategy for RustOps AIOps platform, covering all external system connections required for enterprise AIOps functionality.

**Document Information**
| Field | Value |
|-------|-------|
| **Version** | 1.0 |
| **Date** | January 18, 2026 |
| **Status** | Design Draft |
| **Related PRD** | `/plans/research/agenticops.md` |

---

## Table of Contents

1. [Integration Philosophy](#integration-philosophy)
2. [Architecture Overview](#architecture-overview)
3. [Integration Categories](#integration-categories)
4. [Prioritized Roadmap](#prioritized-roadmap)
5. [Technical Implementation](#technical-implementation)
6. [Security & Authentication](#security--authentication)
7. [Testing Strategy](#testing-strategy)
8. [Operational Considerations](#operational-considerations)

---

## Integration Philosophy

### Core Principles

1. **Plugin Architecture**: Every integration is a self-contained plugin with uniform interface
2. **Fail-Safe Design**: External system failures never crash RustOps
3. **Rate Limiting**: Built-in backpressure and rate limiting per integration
4. **Observability**: All integration activities are logged and metered
5. **Extensibility**: New integrations added without core platform changes

### Risk Mitigation

**Addressing "Integration Complexity" Risk** (from PRD Section 12):

| Strategy | Implementation |
|----------|----------------|
| **Prioritize Top 5** | Phase 1 focuses on Prometheus, Kubernetes, ServiceNow, PagerDuty, Slack |
| **Plugin Architecture** | Unified adapter interface isolates integration complexity |
| **Gradual Rollout** | Canary deployments per integration with instant rollback |
| **Mock Testing** | Comprehensive mock servers for offline development |
| **Contract Testing** | API compatibility verified before deployment |

---

## Architecture Overview

### Integration Layer Design

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        RustOps Core Platform                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
│  │   Alert     │  │ Remediation │  │  Anomaly    │  │  Topology   │   │
│  │  Manager    │  │   Engine    │  │  Detection  │  │   Engine    │   │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                     Integration Abstraction Layer                       │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                  Unified Adapter Interface                       │   │
│  │  - TelemetryCollector (metrics, logs, traces)                  │   │
│  │  - ITSMNotifier (incidents, changes)                           │   │
│  │  - InfrastructureMonitor (cloud, k8s)                          │   │
│  │  - CollaborationChannel (notifications, chatops)               │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                   │                                     │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │              Cross-Cutting Services                             │   │
│  │  - Circuit Breaker (failure isolation)                         │   │
│  │  - Rate Limiter (per-integration quotas)                       │   │
│  │  - Retry Logic (exponential backoff)                           │   │
│  │  - Webhook Receiver (event ingestion)                          │   │
│  │  - Credential Manager (secure secrets)                         │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         Integration Plugins                             │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐     │
│  │Prometheus│ │   AWS    │ │ServiceNow│ │PagerDuty │ │  Slack   │     │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐     │
│  │ Datadog  │ │Kubernetes│ │   Jira   │ │   Splunk │ │  Teams   │     │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐     │
│  │ Jaeger   │ │   Azure  │ │OpsGenie  │ │   Loki   │ │Terraform │     │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘     │
│  ... (20+ more plugins supported via plugin architecture)              │
└─────────────────────────────────────────────────────────────────────────┘
```

### Adapter Pattern Implementation

```rust
/// Unified adapter interface for all integrations
#[async_trait]
pub trait IntegrationAdapter: Send + Sync {
    /// Integration identifier
    fn id(&self) -> &str;

    /// Integration type classification
    fn kind(&self) -> IntegrationKind;

    /// Health check for external system
    async fn health_check(&self) -> Result<HealthStatus, AdapterError>;

    /// Initialize connection (with reconnection support)
    async fn initialize(&mut self) -> Result<(), AdapterError>;

    /// Shutdown gracefully
    async fn shutdown(&mut self) -> Result<(), AdapterError>;
}

/// Telemetry collector interface
#[async_trait]
pub trait TelemetryCollector: IntegrationAdapter {
    /// Collect metrics from external system
    async fn collect_metrics(&self, query: MetricQuery) -> Result<Vec<Metric>, AdapterError>;

    /// Collect logs from external system
    async fn collect_logs(&self, query: LogQuery) -> Result<LogStream, AdapterError>;

    /// Collect traces from external system
    async fn collect_traces(&self, query: TraceQuery) -> Result<Vec<Trace>, AdapterError>;

    /// Subscribe to real-time telemetry updates
    async fn subscribe(&self) -> Result<tokio::sync::mpsc::Receiver<TelemetryEvent>, AdapterError>;
}

/// ITSM notifier interface
#[async_trait]
pub trait ITSMNotifier: IntegrationAdapter {
    /// Create or update incident
    async fn create_incident(&self, incident: Incident) -> Result<String, AdapterError>;

    /// Update incident status
    async fn update_incident(&self, id: &str, update: IncidentUpdate) -> Result<(), AdapterError>;

    /// Query incident details
    async fn get_incident(&self, id: &str) -> Result<Incident, AdapterError>;

    /// Sync with CMDB
    async fn sync_cmdb(&self) -> Result<CMDBSyncResult, AdapterError>;
}

/// Infrastructure monitor interface
#[async_trait]
pub trait InfrastructureMonitor: IntegrationAdapter {
    /// List monitored resources
    async fn list_resources(&self, filters: ResourceFilter) -> Result<Vec<Resource>, AdapterError>;

    /// Get resource metrics
    async fn get_resource_metrics(&self, id: &str) -> Result<ResourceMetrics, AdapterError>;

    /// Watch for resource changes (streaming)
    async fn watch_resources(&self) -> Result<tokio::sync::mpsc::Receiver<ResourceEvent>, AdapterError>;

    /// Execute infrastructure action
    async fn execute_action(&self, action: InfraAction) -> Result<ActionResult, AdapterError>;
}
```

---

## Integration Categories

### 1. Monitoring Tool Integrations

| Category | Tools | Priority | Phase | Documentation |
|----------|-------|----------|-------|---------------|
| **Metrics** | Prometheus, CloudWatch, Datadog | Critical | 1 | [`metrics-integrations.md`](./metrics-integrations.md) |
| **Logs** | Elasticsearch, Splunk, Loki, CloudWatch Logs | High | 1 | [`logs-integrations.md`](./logs-integrations.md) |
| **Traces** | Jaeger, Zipkin, AWS X-Ray (OTLP) | High | 2 | [`traces-integrations.md`](./traces-integrations.md) |
| **APM** | New Relic, Dynatrace, AppDynamics | Medium | 3 | [`apm-integrations.md`](./apm-integrations.md) |

### 2. ITSM/Collaboration Integrations

| Tool | Capabilities | Priority | Phase | Documentation |
|------|-------------|----------|-------|---------------|
| **ServiceNow** | Incident creation, CMDB sync, change tracking | Critical | 1 | [`servicenow-integration.md`](./servicenow-integration.md) |
| **Jira** | Issue creation, status sync, sprint tracking | High | 1 | [`jira-integration.md`](./jira-integration.md) |
| **PagerDuty** | Alert routing, escalation, on-call sync | Critical | 1 | [`pagerduty-integration.md`](./pagerduty-integration.md) |
| **Slack** | Notifications, ChatOps, incident channels | High | 1 | [`slack-integration.md`](./slack-integration.md) |
| **Microsoft Teams** | Notifications, adaptive cards, bot commands | Medium | 2 | [`teams-integration.md`](./teams-integration.md) |

### 3. Infrastructure Integrations

| Platform | Capabilities | Priority | Phase | Documentation |
|----------|-------------|----------|-------|---------------|
| **Kubernetes** | Watch API, pods/events/deployments | Critical | 1 | [`kubernetes-integration.md`](./kubernetes-integration.md) |
| **AWS** | CloudWatch, EC2, ECS, Lambda, RDS | High | 1 | [`aws-integration.md`](./aws-integration.md) |
| **Azure** | Monitor, App Insights, AKS, VMs | Medium | 2 | [`azure-integration.md`](./azure-integration.md) |
| **GCP** | Cloud Monitoring, GKE, Compute Engine | Medium | 2 | [`gcp-integration.md`](./gcp-integration.md) |
| **Terraform** | Drift detection, change correlation | Low | 3 | [`terraform-integration.md`](./terraform-integration.md) |

---

## Prioritized Roadmap

### Phase 1: Foundation (Months 1-3) - Top 5 Integrations

**Goal**: Cover 80% of enterprise use cases with 5 critical integrations

| Integration | Rationale | Effort | Value |
|-------------|-----------|--------|-------|
| **Prometheus** | De facto standard for metrics in Kubernetes/cloud-native | Medium | Critical |
| **Kubernetes** | Primary deployment target for containerized workloads | Medium | Critical |
| **ServiceNow** #1 ITSM platform in enterprise (50%+ market share) | High | Critical |
| **PagerDuty** | Standard for on-call management and alert routing | Low | High |
| **Slack** | Ubiquitous collaboration platform for DevOps teams | Low | High |

**Phase 1 Deliverables**:
- All 5 integrations production-ready
- Unified adapter interface finalized
- Circuit breaker and rate limiting implemented
- Integration test suite with mocks
- Deployment playbooks for each integration

### Phase 2: Expansion (Months 4-6) - Next 7 Integrations

| Integration | Rationale |
|-------------|-----------|
| **CloudWatch** | AWS is #1 cloud provider (32% market share) |
| **Jira** | Complementary to ServiceNow for development teams |
| **Elasticsearch** | Most popular log aggregation platform |
| **Splunk** | Enterprise log management standard |
| **Jaeger** | OpenTelemetry-compatible distributed tracing |
| **Microsoft Teams** | Microsoft 365 enterprise penetration |
| **Datadog** | Leading SaaS observability platform |

### Phase 3: Completeness (Months 7-12) - Remaining Integrations

All remaining integrations including Azure, GCP, APM tools, and specialized platforms.

---

## Technical Implementation

### Rust Integration Libraries

```toml
[dependencies]
# HTTP Clients
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
hyper = { version = "0.14", features = ["full"] }
surf = "2.3"  # Alternative HTTP client

# Kubernetes
kube = { version = "0.87", features = ["runtime", "client", "ws"] }
k8s-openapi = { version = "0.20", features = ["v1_26"] }

# gRPC
tonic = "0.10"
prost = "0.12"

# WebSocket
tokio-tungstenite = "0.20"

# Authentication
oauth2 = "4.4"
jsonwebtoken = "8.3"

# Retry Logic
backoff = { version = "0.4", features = ["tokio"] }

# Rate Limiting
governor = "0.6"

# Circuit Breaker
tokio-circuit-breaker = "0.4"
```

### Retry with Exponential Backoff

```rust
use backoff::{ExponentialBackoff, future::retry_notify};
use std::time::Duration;

/// Retry configuration for external API calls
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_interval: Duration,
    pub max_interval: Duration,
    pub multiplier: f64,
    pub randomization_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_interval: Duration::from_secs(1),
            max_interval: Duration::from_secs(60),
            multiplier: 2.0,
            randomization_factor: 0.2,
        }
    }
}

/// Execute operation with retry logic
pub async fn retry_with_backoff<F, Fut, T, E>(
    config: RetryConfig,
    operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let backoff = ExponentialBackoff {
        initial_interval: config.initial_interval,
        max_interval: config.max_interval,
        multiplier: config.multiplier,
        randomization_factor: config.randomization_factor,
        ..Default::default()
    };

    retry_notify(backoff, operation, |err, dur| {
        tracing::warn!(
            error = %err,
            retry_after = ?dur,
            "Operation failed, retrying..."
        )
    })
    .await
}
```

### Circuit Breaker Pattern

```rust
use tokio_circuit_breaker::CircuitBreaker;
use std::sync::Arc;
use std::time::Duration;

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub error_threshold: usize,
    pub success_threshold: usize,
    pub timeout: Duration,
    pub max_calls: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            error_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            max_calls: 100,
        }
    }
}

/// Integration-level circuit breaker
pub struct IntegrationCircuitBreaker {
    breaker: Arc<CircuitBreaker>,
    config: CircuitBreakerConfig,
}

impl IntegrationCircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            breaker: Arc::new(CircuitBreaker::new(
                config.error_threshold,
                config.success_threshold,
                config.timeout,
            )),
            config,
        }
    }

    pub async fn call<F, R, E>(&self, operation: F) -> Result<R, E>
    where
        F: std::future::Future<Output = Result<R, E>>,
    {
        if self.breaker.is_open() {
            tracing::warn!("Circuit breaker is open, rejecting call");
            return Err(/* circuit open error */);
        }

        match operation.await {
            Ok(result) => {
                self.breaker.report_success();
                Ok(result)
            }
            Err(err) => {
                self.breaker.report_failure();
                Err(err)
            }
        }
    }
}
```

### Rate Limiting

```rust
use governor::{Quota, RateLimiter};
use nonzero_ext::nonzero;
use std::num::NonZeroU32;

/// Rate limiter per integration
pub struct IntegrationRateLimiter {
    limiter: RateLimiter<clock::DefaultClock>,
}

impl IntegrationRateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        let quota = Quota::per_second(nonzero!(requests_per_second));
        Self {
            limiter: RateLimiter::direct(quota),
        }
    }

    pub async fn acquire(&self) -> Result<(), RateLimitError> {
        self.limiter.until_ready().await;
        Ok(())
    }
}
```

---

## Security & Authentication

### Authentication Methods per Integration

| Integration | Auth Method | Credential Storage |
|-------------|-------------|-------------------|
| **Prometheus** | None (pull-based) or Basic Auth | Kubernetes Secrets |
| **ServiceNow** | OAuth 2.0 | Vault / AWS Secrets Manager |
| **PagerDuty** | API Token / OAuth | Vault / AWS Secrets Manager |
| **Slack** | Bot Token / OAuth | Vault / AWS Secrets Manager |
| **AWS** | IAM Access Key / IRSA | AWS Secrets Manager |
| **Kubernetes** | Service Account / Certificates | Kubernetes Secrets |
| **Datadog** | API Key / App Key | Vault / AWS Secrets Manager |

### Credential Management

```rust
/// Credential source for integrations
pub enum CredentialSource {
    /// Direct secret (not recommended for production)
    Direct { secret: String },
    /// HashiCorp Vault
    Vault {
        path: String,
        field: String,
        role: String,
    },
    /// AWS Secrets Manager
    AwsSecretsManager {
        secret_id: String,
        region: String,
    },
    /// Kubernetes Secret
    K8sSecret {
        namespace: String,
        name: String,
        key: String,
    },
    /// Environment variable
    EnvVar { name: String },
}

/// Credential manager for integrations
pub struct CredentialManager {
    vault_client: Option<vault::Client>,
    aws_secrets: aws_sdk_secretsmanager::Client,
    k8s_client: kube::Client,
}

impl CredentialManager {
    /// Retrieve credential for integration
    pub async fn get_credential(
        &self,
        source: &CredentialSource,
    ) -> Result<String, CredentialError> {
        match source {
            CredentialSource::Direct { secret } => Ok(secret.clone()),
            CredentialSource::Vault { path, field, role } => {
                self.get_from_vault(path, field, role).await
            }
            CredentialSource::AwsSecretsManager { secret_id, region } => {
                self.get_from_aws_secrets(secret_id, region).await
            }
            CredentialSource::K8sSecret { namespace, name, key } => {
                self.get_from_k8s_secret(namespace, name, key).await
            }
            CredentialSource::EnvVar { name } => {
                std::env::var(name).map_err(|_| CredentialError::NotFound)
            }
        }
    }
}
```

---

## Testing Strategy

### Mock Servers for External Dependencies

```rust
/// Mock server for testing integrations
#[cfg(test)]
pub mod mock_servers {
    use mockito::{Server, Mock};
    use serde_json::json;

    /// Mock ServiceNow instance
    pub struct MockServiceNow {
        server: Server,
    }

    impl MockServiceNow {
        pub fn new() -> Self {
            Self {
                server: Server::new(),
            }
        }

        pub fn url(&self) -> String {
            self.server.url()
        }

        pub fn mock_incident_create(&self) -> Mock {
            self.server
                .mock("POST", "/api/now/table/incident")
                .with_status(201)
                .with_header("content-type", "application/json")
                .with_body(
                    json!({
                        "result": {
                            "sys_id": "test-incident-id",
                            "number": "INC0010001",
                            "priority": "1",
                            "state": "New"
                        }
                    })
                    .to_string(),
                )
                .create()
        }

        pub fn mock_health_check(&self) -> Mock {
            self.server
                .mock("GET", "/api/now/table/sys_properties")
                .with_status(200)
                .create()
        }
    }
}
```

### Contract Testing

```rust
/// Contract tests ensure API compatibility
#[cfg(test)]
mod contract_tests {
    use super::*;

    #[tokio::test]
    async fn test_prometheus_scrape_contract() {
        // Verify Prometheus scrape format compliance
        let mock_prom = setup_mock_prometheus().await;
        let adapter = PrometheusAdapter::new(mock_prom.url()).await;

        let metrics = adapter.collect_metrics(query).await.unwrap();

        // Verify metric format
        assert!(metrics.iter().all(|m| m.name.contains(|c| c.is_alphanumeric() || c == '_')));
    }

    #[tokio::test]
    async fn test_servicenow_incident_contract() {
        // Verify ServiceNow incident API contract
        let mock_sn = MockServiceNow::new();
        let _mock = mock_sn.mock_incident_create();

        let adapter = ServiceNowAdapter::new(mock_sn.url(), "token").await;
        let incident_id = adapter.create_incident(test_incident()).await.unwrap();

        assert_eq!(incident_id, "test-incident-id");
    }
}
```

### Integration Test Environments

```
┌─────────────────────────────────────────────────────────────────┐
│                  Integration Test Environments                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐    ┌─────────────────┐    ┌──────────────┐ │
│  │   Unit Tests    │    │  Integration    │    │    E2E       │ │
│  │                 │    │     Tests       │    │    Tests     │ │
│  │  - Mock servers │───▶│  - Docker comp. │───▶│  - Full env  │ │
│  │  - Contract    │    │  - Test cloud   │    │  - Staging   │ │
│  │    validation  │    │    accounts     │    │    env       │ │
│  │  - Fast (< 1s) │    │  - Real APIs    │    │  - Full      │ │
│  └─────────────────┘    │  - Medium (1m)  │    │    tests     │ │
│                         └─────────────────┘    │  - Slow      │ │
│                                                │  (10m+)     │ │
│                                                └──────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Canary Deployment Strategy

```yaml
# Canary deployment for new integration version
apiVersion: flagger.app/v1beta1
kind: Canary
metadata:
  name: servicenow-integration
spec:
  targetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: rustops-integrations
  service:
    port: 8080
  analysis:
    interval: 1m
    threshold: 5
    maxWeight: 50
    stepWeight: 10
    metrics:
    # Success rate for ServiceNow API calls
    - name: servicenow-success-rate
      thresholdRange:
        min: 99
      interval: 1m
    # Alert latency
    - name: servicenow-latency
      thresholdRange:
        max: 500
      interval: 1m
  webhooks:
    # Pre-canary check
    - name: servicenov-connection-check
      url: http://rustops-integrations:8080/health/servicenow
      timeout: 5s
```

---

## Operational Considerations

### Per-Integration Configuration

```yaml
# Integration configuration example
integrations:
  servicenow:
    enabled: true
    instance_url: "https://dev12345.service-now.com"
    auth:
      method: oauth2
      client_id: "${SERVICENOW_CLIENT_ID}"
      client_secret: "${SERVICENOW_CLIENT_SECRET}"  # From Vault
    rate_limit:
      requests_per_second: 10
      burst: 20
    circuit_breaker:
      error_threshold: 5
      timeout: 60s
    retry:
      max_attempts: 3
      initial_backoff: 1s
      max_backoff: 30s
    webhook:
      enabled: true
      path: /webhooks/servicenow
      secret: "${SERVICENOW_WEBHOOK_SECRET}"

  prometheus:
    enabled: true
    scrape_configs:
    - job_name: 'kubernetes-pods'
      kubernetes_sd_configs:
      - role: pod
      scrape_interval: 15s
      scrape_timeout: 10s
    rate_limit:
      requests_per_second: 100  # Higher for scraping

  slack:
    enabled: true
    workspace_url: "https://acme-corp.slack.com"
    auth:
      bot_token: "${SLACK_BOT_TOKEN}"
    notification_channels:
      incidents: "#ops-incidents"
      alerts: "#ops-alerts"
      status_updates: "#ops-status"
```

### Monitoring Integrations

```rust
/// Integration health metrics
#[derive(Debug, Clone)]
pub struct IntegrationHealth {
    pub integration_id: String,
    pub status: HealthStatus,
    pub last_successful_call: Option<DateTime<Utc>>,
    pub error_rate: f64,
    pub avg_latency: Duration,
    pub circuit_breaker_open: bool,
    pub rate_limit_hits: u64,
}

/// Integration monitoring service
pub struct IntegrationMonitor {
    registry: prometheus::Registry,
}

impl IntegrationMonitor {
    pub fn record_metrics(&self, integration: &str, outcome: &CallOutcome) {
        // Record success/failure
        self.integration_calls_total
            .with_label_values(&[integration, &outcome.status.to_string()])
            .inc();

        // Record latency
        self.integration_latency_seconds
            .with_label_values(&[integration])
            .observe(outcome.latency.as_secs_f64());

        // Update circuit breaker status
        if outcome.circuit_breaker_open {
            self.circuit_breaker_open
                .with_label_values(&[integration])
                .set(1);
        }
    }
}
```

### Rollback Strategy

```bash
#!/bin/bash
# Integration rollback playbook

set -euo pipefail

INTEGRATION_NAME="${1:?Integration name required}"
PREVIOUS_VERSION="${2:?Target version required}"

echo "Rolling back ${INTEGRATION_NAME} to ${PREVIOUS_VERSION}"

# 1. Scale down current version
kubectl scale deployment/rustops-${INTEGRATION_NAME} --replicas=0

# 2. Restore previous version
kubectl rollout undo deployment/rustops-${INTEGRATION_NAME}

# 3. Verify health
./scripts/wait-for-healthy.sh rustops-${INTEGRATION_NAME}

# 4. Run smoke tests
cargo test --test integration_${INTEGRATION_NAME} -- --ignored

# 5. Check metrics
./scripts/check-integration-metrics.sh ${INTEGRATION_NAME}

echo "Rollback complete"
```

---

## Appendix

### Related Documents

- [PRD: AIOps Agent for IT Operations](/plans/research/agenticops.md)
- [Architecture Decision Records](/plans/adrs/)
- [Implementation Roadmap](/plans/roadmap.md)

### Integration Quick Reference

| Integration | Auth Type | Rate Limit | Retry | Circuit Breaker | Webhook |
|-------------|-----------|------------|-------|-----------------|---------|
| Prometheus | Basic Auth / TLS | 100 req/s | Yes | Yes | No |
| ServiceNow | OAuth 2.0 | 10 req/s | Yes | Yes | Yes |
| PagerDuty | API Token | 20 req/s | Yes | Yes | Yes |
| Slack | Bot Token | 50 req/s | Yes | Yes | Events API |
| Kubernetes | Service Account | N/A (Watch API) | Yes | Yes | Webhook |
| AWS | IAM Key | Varies per service | Yes | Yes | EventBridge |

### Support Matrix

| Integration | Community | Enterprise Support | SLA |
|-------------|-----------|-------------------|-----|
| Prometheus | ✓ | RustOps support | Best effort |
| ServiceNow | ✓ | Official vendor | 99.9% |
| PagerDuty | ✓ | Official vendor | 99.99% |
| Slack | ✓ | Official vendor | 99.99% |
| Kubernetes | ✓ | RustOps support | Best effort |
| AWS | ✓ | AWS Support | 99.99% |

---

**Document Version**: 1.0
**Last Updated**: 2026-01-18
**Next Review**: 2026-02-01
