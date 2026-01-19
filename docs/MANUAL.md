# RustOps: Complete Technical Manual & Walkthrough

## Table of Contents

1. [Introduction](#introduction)
2. [Architecture Overview](#architecture-overview)
3. [Core Components](#core-components)
4. [Installation & Setup](#installation--setup)
5. [Configuration Guide](#configuration-guide)
6. [Usage Guide](#usage-guide)
7. [API Reference](#api-reference)
8. [Development Guide](#development-guide)
9. [Deployment](#deployment)
10. [Troubleshooting](#troubleshooting)

---

## Introduction

### What is RustOps?

RustOps is an **Intelligent AIOps (Artificial Intelligence for IT Operations) Platform** built in Rust that combines:

- **Real-time Monitoring**: Collects metrics, logs, and traces from your infrastructure
- **Intelligent Anomaly Detection**: Hybrid statistical + ML-based detection
- **Automated Incident Management**: Alert correlation, deduplication, and root cause analysis
- **Safe Remediation Workflows**: Risk-based approval gates with automatic rollback

### Key Design Philosophy

RustOps follows **Domain-Driven Design (DDD)** principles with:
- **Event Sourcing** for complete audit trails
- **CQRS** for scalable read/write models
- **Bounded Contexts** for clear domain separation
- **Anti-Corruption Layers** for clean integrations

### Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust 1.70+ |
| Async Runtime | Tokio |
| Event Streaming | Apache Kafka |
| Graph Database | Neo4j |
| Relational Database | PostgreSQL |
| Cache | Redis |
| Time-Series Storage | ClickHouse |
| Metrics | Prometheus |
| Workflow Engine | Temporal |
| ML Inference | ONNX Runtime |
| Vector Search | HNSW (150x-12,500x faster) |

---

## Architecture Overview

### System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Application Layer                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐              │
│  │  API Server  │  │Agent Service │  │Pipeline Svc  │              │
│  │  HTTP/WS     │  │  Collection  │  │ Processing   │              │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘              │
└─────────┼──────────────────┼──────────────────┼─────────────────────┘
          │                  │                  │
┌─────────┼──────────────────┼──────────────────┼─────────────────────┐
│         │         Integration Layer (Adapters)                   │
│  ┌──────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐  ┌──────────┐  │
│  │ Prometheus  │  │ Kubernetes  │  │ ServiceNow  │  │  Slack   │  │
│  │   Adapter   │  │   Adapter   │  │   Adapter   │  │  Adapter │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  └──────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                   │
┌──────────────────────────────────▼───────────────────────────────────┐
│                        Domain Layer (Bounded Contexts)              │
│  ┌───────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐ │
│  │Telemetry  │ │ Anomaly  │ │ Incident │ │Topology  │ │Knowledge│ │
│  │ Collection│ │Detection │ │Management│ │Management│ │         │ │
│  └───────────┘ └──────────┘ └──────────┘ └──────────┘ └─────────┘ │
│  ┌──────────────────────────────────────────────────────────┐       │
│  │              Remediation (Safe Automation)                │       │
│  └──────────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────────┘
                                   │
┌──────────────────────────────────▼───────────────────────────────────┐
│                      Infrastructure Layer                            │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────────┐          │
│  │ Kafka│ │Neo4j │ │PgSQL │ │Redis │ │Tempo │ │Prometheus│          │
│  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────────┘          │
└─────────────────────────────────────────────────────────────────────┘
```

### Bounded Contexts

RustOps is organized into 7 bounded contexts, each with its own domain model:

| Context | Responsibility | Events Published |
|---------|---------------|------------------|
| **Telemetry** | Collect metrics/logs/traces | `TelemetryDataCollected` |
| **Anomaly Detection** | Detect anomalies | `AnomalyDetected` |
| **Incident Management** | Manage incidents | `IncidentCreated`, `IncidentUpdated` |
| **Service Topology** | Map dependencies | `TopologyDiscovered`, `TopologyChanged` |
| **Knowledge Management** | Store patterns/runbooks | `PatternStored`, `RunbookExecuted` |
| **Remediation** | Execute fixes safely | `RemediationStarted`, `RemediationCompleted` |
| **Integration** | External system adapters | Various integration-specific events |

### Event Flow

1. **Telemetry Collection**: Agents collect data → Publish `TelemetryDataCollected`
2. **Anomaly Detection**: Subscribe to telemetry → Analyze → Publish `AnomalyDetected`
3. **Incident Creation**: Subscribe to anomalies → Correlate → Create incidents
4. **Topology Analysis**: Subscribe to incidents → Group by service dependencies
5. **Remediation**: Subscribe to incidents → Execute safe workflows

---

## Core Components

### 1. Common Crate (`crates/common/`)

Foundation types and utilities shared across all crates.

#### Type-Safe IDs

```rust
// Prevents ID mixing at compile time
newtype_id!(IncidentId);
newtype_id!(ServiceId);
newtype_id!(AlertId);

// This would NOT compile - catching errors at compile time!
fn process(id: IncidentId) { }
process(service_id); // Compile error!
```

#### Domain Events

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub id: Uuid,
    pub aggregate_id: String,
    pub event_type: String,
    pub data: JsonValue,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<Uuid>,
}
```

### 2. Telemetry Crate (`crates/telemetry/`)

Collects and normalizes telemetry data from various sources.

#### Supported Formats

```rust
pub enum TelemetryData {
    Metric(MetricData),
    Log(LogData),
    Trace(TraceData),
}

pub struct MetricData {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}
```

#### Collector Registry

```rust
// Register a collector
let registry = CollectorRegistry::new();
registry.register(Box::new(PrometheusCollector::new(config)));
registry.register(Box::new(FluentdLogCollector::new(config)));
registry.register(Box::new(OtelTraceCollector::new(config)));

// Start collection
registry.start_all().await?;
```

### 3. Anomaly Detection Crate (`crates/anomaly/`)

Hybrid detection approach for optimal accuracy/performance.

#### Statistical Detection (<1ms latency)

```rust
use anomaly::detectors::statistical::ZScoreDetector;

let detector = ZScoreDetector::new(threshold: 3.0);
let result = detector.detect(&metric_series);

if result.is_anomaly {
    println!("Anomaly detected! Z-score: {}", result.z_score);
}
```

**Algorithms available:**
- **Z-Score**: Detects outliers based on standard deviations
- **IQR (Interquartile Range)**: Robust to extreme outliers
- **CUSUM**: Detects small, persistent shifts

#### ML-Based Detection (~50ms latency)

```rust
use anomaly::detectors::ml::OnnxDetector;

let detector = OnnxDetector::load("models/anomaly_detector.onnx")?;
let result = detector.detect(&metric_series)?;

if result.is_anomaly {
    println!("ML Anomaly! Confidence: {}", result.confidence);
}
```

#### Pattern Matching

```rust
use anomaly::detectors::pattern::SignatureDetector;

let detector = SignatureDetector::new();
detector.add_pattern(AnomalyPattern {
    name: "Memory Leak".to_string(),
    signature: Signature::TrendingUp { duration: Duration::from_hours(1) },
});

let result = detector.detect(&metric_series);
```

### 4. Incident Management Crate (`crates/incident/`)

Manages the complete incident lifecycle.

#### Alert Correlation

```rust
use incident::correlation::AlertCorrelator;

let correlator = AlertCorrelator::new(config);
let incident_id = correlator
    .correlate_alert(alert)
    .await?;

// Similar alerts are grouped into the same incident
// within a 5-minute deduplication window
```

#### Topological Grouping

```rust
use incident::grouping::TopologyGrouper;

let grouper = TopologyGrouper::new(topology_graph);
let groups = grouper.group_incidents(&incidents);

// Incidents are grouped by service dependencies
// e.g., all incidents caused by Database failure
```

#### Root Cause Ranking

```rust
use incident::ranking::RootCauseRanker;

let ranker = RootCauseRanker::new();
let ranked = ranker.rank(&incidents, &topology);

// Returns incidents ranked by likelihood of being root cause
```

### 5. Service Topology Crate (`crates/topology/`)

Graph-based service dependency management.

#### Automatic Discovery

```rust
use topology::discovery::KubernetesDiscovery;

let discovery = KubernetesDiscovery::new(kube_config);
let topology = discovery.discover().await?;

// Automatically discovers services and dependencies
// from Kubernetes annotations and labels
```

#### Impact Analysis

```rust
use topology::analysis::ImpactAnalyzer;

let analyzer = ImpactAnalyzer::new(&topology);
let impact = analyzer.analyze_change(service_id, change_type);

// Returns downstream services that would be affected
```

#### Neo4j Integration

```rust
use topology::store::Neo4jStore;

let store = Neo4jStore::new("bolt://localhost:7687");
store.save_topology(&topology).await?;

// Efficiently query complex dependency relationships
let downstream = store.find_downstream(service_id, depth: 3).await?;
```

### 6. Integration Crate (`crates/integration/`)

Adapter pattern for unified external system interfaces.

#### Adapter Pattern

```rust
use integration::Adapter;

// All adapters implement the same trait
trait Adapter {
    async fn fetch_metrics(&self) -> Result<Vec<Metric>>;
    async fn send_alert(&self, alert: &Alert) -> Result<()>;
    async fn health_check(&self) -> Result<HealthStatus>;
}

// Use any adapter interchangeably
let adapter: Box<dyn Adapter> = Box::new(PrometheusAdapter::new(config));
let metrics = adapter.fetch_metrics().await?;
```

#### Circuit Breaker

```rust
use integration::circuit_breaker::CircuitBreaker;

let breaker = CircuitBreaker::new(threshold: 5, timeout: Duration::from_secs(60));

breaker.call(|| async {
    // This call is protected by circuit breaker
    external_api_call().await
}).await?;
```

### 7. Knowledge Management Crate (`crates/knowledge/`)

Vector embeddings for semantic search and pattern storage.

#### Semantic Search

```rust
use knowledge::search::SemanticSearch;

let search = SemanticSearch::new()?;

// Search for similar past incidents
let results = search
    .search("database connection timeout", limit: 5)
    .await?;

// Results ranked by semantic similarity
for result in results {
    println!("{} (similarity: {})", result.pattern, result.score);
}
```

#### Pattern Storage

```rust
use knowledge::patterns::PatternStore;

let store = PatternStore::new()?;
store.store_pattern(Pattern {
    name: "Database Connection Pool Exhaustion".to_string(),
    description: "High connection count, increasing response times".to_string(),
    remediation: remediation_steps,
    embedding: generate_embedding(&description),
}).await?;
```

### 8. Remediation Crate (`crates/remediation/`)

Safe automated remediation with approval gates.

#### Risk-Based Approval

```rust
use remediation::approval::{ApprovalGate, RiskLevel};

let gate = ApprovalGate::new();

match remediation.risk_level() {
    RiskLevel::Low => {
        // Auto-approve (e.g., restart single service)
        gate.auto_approve(remediation).execute().await?;
    }
    RiskLevel::Medium => {
        // Require 1 approver (e.g., cluster scaling)
        gate.require_approvers(1).request_approval(remediation).await?;
    }
    RiskLevel::High => {
        // Require 2+ approvers (e.g., multi-cluster changes)
        gate.require_approvers(2).request_approval(remediation).await?;
    }
}
```

#### Blast Radius Limits

```rust
use remediation::safety::BlastRadius;

let radius = BlastRadius::new()
    .max_namespaces(1)
    .max_clusters(1)
    .max_services(10);

radius.validate(&remediation)?;
```

#### Automatic Rollback

```rust
use remediation::rollback::AutoRollback;

let remediation = AutoRollback::new(
    remediation,
    rollback_plan,
    rollback_triggers,
);

remediation.execute_with_rollback().await?;
// Automatically rolls back if:
// - New anomalies detected
// - Error rate increases
// - Manual abort triggered
```

---

## Installation & Setup

### Prerequisites

- **Rust**: 1.70 or later
- **Docker**: For infrastructure components
- **Kubernetes**: Optional, for production deployment
- **kubectl**: If using Kubernetes

### Local Development Setup

#### 1. Clone Repository

```bash
git clone https://github.com/your-org/rustops.git
cd rustops
```

#### 2. Start Infrastructure

```bash
docker-compose up -d
```

This starts:
- Kafka (9092)
- Neo4j (7474, 7687)
- PostgreSQL (5432)
- Redis (6379)
- Prometheus (9090)
- Grafana (3000)

#### 3. Build Project

```bash
cargo build
```

#### 4. Run Tests

```bash
cargo test --all
```

#### 5. Start Services

```bash
# Terminal 1: API Server
cargo run --bin api-server

# Terminal 2: Agent Service
cargo run --bin agent-service

# Terminal 3: Pipeline Service
cargo run --bin pipeline-service
```

### Production Setup (Kubernetes)

#### 1. Create Namespace

```bash
kubectl create namespace rustops
```

#### 2. Install Infrastructure

```bash
kubectl apply -f deploy/infrastructure/
```

#### 3. Deploy RustOps

```bash
kubectl apply -f deploy/rustops/
```

#### 4. Verify Deployment

```bash
kubectl get pods -n rustops
```

---

## Configuration Guide

### Configuration File Structure

Configuration is managed through `config/` directory:

```
config/
├── base/           # Base configuration
├── development/    # Development overrides
├── staging/        # Staging overrides
└── production/     # Production overrides
```

### Example Configuration

```yaml
# config/base/rustops.yaml
server:
  host: "0.0.0.0"
  port: 8080

kafka:
  brokers:
    - "localhost:9092"
  group_id: "rustops"

neo4j:
  uri: "bolt://localhost:7687"
  username: "neo4j"
  password: "password"

postgresql:
  url: "postgresql://localhost:5432/rustops"

redis:
  url: "redis://localhost:6379"

anomaly_detection:
  statistical:
    z_score_threshold: 3.0
    iqr_multiplier: 1.5
  ml:
    model_path: "/models/anomaly_detector.onnx"
    confidence_threshold: 0.8

remediation:
  auto_approval:
    max_risk_level: "low"
  blast_radius:
    max_namespaces: 1
    max_clusters: 1
```

### Environment Variables

```bash
# Override configuration with environment variables
export RUSTOPS_SERVER_PORT=9090
export RUSTOPS_KAFKA_BROKERS="kafka:9092"
export RUSTOPS_NEO4J_URI="bolt://neo4j:7687"
export RUSTOPS_LOG_LEVEL="debug"
```

---

## Usage Guide

### Collecting Telemetry

#### Start an Agent

```bash
cargo run --bin agent -- --config config/development/agent.yaml
```

#### Agent Configuration

```yaml
# agent.yaml
collector:
  type: "prometheus"
  interval: 15s
  sources:
    - url: "http://localhost:9090"
      scrape_config: |
        scrape_configs:
          - job_name: 'kubernetes-pods'
            kubernetes_sd_configs:
              - role: pod
```

### Detecting Anomalies

#### API Request

```bash
curl -X POST http://localhost:8080/api/v1/anomalies/detect \
  -H "Content-Type: application/json" \
  -d '{
    "metric": "http_request_duration_seconds",
    "values": [0.1, 0.12, 0.11, 0.15, 0.45, 0.52],
    "timestamps": [1, 2, 3, 4, 5, 6]
  }'
```

#### Response

```json
{
  "anomaly_detected": true,
  "confidence": 0.95,
  "algorithm": "z_score",
  "details": {
    "z_score": 3.2,
    "threshold": 3.0
  }
}
```

### Managing Incidents

#### Create Incident

```bash
curl -X POST http://localhost:8080/api/v1/incidents \
  -H "Content-Type: application/json" \
  -d '{
    "title": "High error rate in checkout service",
    "severity": "high",
    "service": "checkout",
    "description": "Error rate exceeded 5% threshold"
  }'
```

#### List Incidents

```bash
curl http://localhost:8080/api/v1/incidents?status=open
```

#### Update Incident

```bash
curl -X PATCH http://localhost:8080/api/v1/incidents/{id} \
  -H "Content-Type: application/json" \
  -d '{
    "status": "investigating",
    "assignee": "john.doe"
  }'
```

### Executing Remediations

#### Submit Remediation

```bash
curl -X POST http://localhost:8080/api/v1/remediations \
  -H "Content-Type: application/json" \
  -d '{
    "incident_id": "uuid",
    "action": "restart_service",
    "target": {
      "service": "checkout",
      "namespace": "production"
    },
    "rollback_plan": {
      "action": "restore_previous_replicas"
    }
  }'
```

#### Approve Remediation

```bash
curl -X POST http://localhost:8080/api/v1/remediations/{id}/approve \
  -H "Content-Type: application/json" \
  -d '{
    "approver": "jane.doe",
    "comment": "Approved - safe to proceed"
  }'
```

### Querying Topology

```bash
curl "http://localhost:8080/api/v1/topology/services/{service_id}/impact"
```

Response:

```json
{
  "service_id": "checkout",
  "downstream_services": [
    {
      "service": "payment",
      "impact_level": "critical",
      "affected_endpoints": 1500
    },
    {
      "service": "inventory",
      "impact_level": "high",
      "affected_endpoints": 500
    }
  ]
}
```

---

## API Reference

### REST API Endpoints

#### Telemetry

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/telemetry/metrics` | List all metrics |
| GET | `/api/v1/telemetry/logs` | Query logs |
| GET | `/api/v1/telemetry/traces` | Query traces |

#### Anomalies

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/anomalies/detect` | Detect anomalies |
| GET | `/api/v1/anomalies` | List detected anomalies |
| GET | `/api/v1/anomalies/{id}` | Get anomaly details |

#### Incidents

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/incidents` | List incidents |
| POST | `/api/v1/incidents` | Create incident |
| GET | `/api/v1/incidents/{id}` | Get incident |
| PATCH | `/api/v1/incidents/{id}` | Update incident |
| DELETE | `/api/v1/incidents/{id}` | Delete incident |

#### Remediations

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/remediations` | List remediations |
| POST | `/api/v1/remediations` | Submit remediation |
| GET | `/api/v1/remediations/{id}` | Get remediation |
| POST | `/api/v1/remediations/{id}/approve` | Approve remediation |
| POST | `/api/v1/remediations/{id}/reject` | Reject remediation |
| POST | `/api/v1/remediations/{id}/rollback` | Trigger rollback |

#### Topology

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/topology` | Get full topology |
| GET | `/api/v1/topology/services/{id}` | Get service details |
| GET | `/api/v1/topology/services/{id}/impact` | Analyze impact |
| POST | `/api/v1/topology/discover` | Trigger discovery |

#### Knowledge

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/knowledge/patterns` | Store pattern |
| GET | `/api/v1/knowledge/patterns` | List patterns |
| POST | `/api/v1/knowledge/search` | Semantic search |
| GET | `/api/v1/knowledge/runbooks/{id}` | Get runbook |

### WebSocket API

#### Subscribe to Events

```javascript
const ws = new WebSocket('ws://localhost:8080/api/v1/events');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);

  switch (data.type) {
    case 'anomaly.detected':
      console.log('New anomaly:', data.payload);
      break;
    case 'incident.created':
      console.log('New incident:', data.payload);
      break;
    case 'remediation.completed':
      console.log('Remediation complete:', data.payload);
      break;
  }
};
```

---

## Development Guide

### Project Structure

```
rustops/
├── crates/
│   ├── common/          # Shared types and utilities
│   ├── telemetry/       # Telemetry collection
│   ├── anomaly/         # Anomaly detection
│   ├── incident/        # Incident management
│   ├── topology/        # Service topology
│   ├── integration/     # External integrations
│   ├── knowledge/       # Knowledge management
│   └── remediation/     # Remediation workflows
├── config/              # Configuration files
├── deploy/              # Deployment manifests
├── docs/                # Documentation
└── scripts/             # Utility scripts
```

### Adding a New Integration

1. **Create Adapter**

```rust
use integration::Adapter;

pub struct MyIntegrationAdapter {
    client: Client,
    config: MyConfig,
}

#[async_trait]
impl Adapter for MyIntegrationAdapter {
    async fn fetch_metrics(&self) -> Result<Vec<Metric>> {
        // Implementation
    }

    async fn send_alert(&self, alert: &Alert) -> Result<()> {
        // Implementation
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        // Implementation
    }
}
```

2. **Register Adapter**

```rust
use integration::Registry;

let registry = Registry::new();
registry.register("my_integration", Box::new(adapter));
```

3. **Add Configuration**

```yaml
integrations:
  my_integration:
    enabled: true
    config:
      api_key: "${MY_INTEGRATION_API_KEY}"
      endpoint: "https://api.example.com"
```

### Adding a New Anomaly Detection Algorithm

1. **Implement Detector Trait**

```rust
use anomaly::Detector;

pub struct MyDetector {
    config: MyConfig,
}

impl Detector for MyDetector {
    fn detect(&self, series: &TimeSeries) -> Result<DetectionResult> {
        // Your algorithm implementation
        Ok(DetectionResult {
            is_anomaly: true,
            confidence: 0.95,
            details: json!({}),
        })
    }
}
```

2. **Register Detector**

```rust
use anomaly::Registry;

let registry = Registry::new();
registry.register("my_detector", Box::new(detector));
```

### Running Tests

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# With coverage
cargo tarpaulin --out Html
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings

# Audit dependencies
cargo audit
```

---

## Deployment

### Docker Deployment

#### Build Image

```bash
docker build -t rustops:latest .
```

#### Run Container

```bash
docker run -d \
  --name rustops \
  -p 8080:8080 \
  -v $(pwd)/config:/app/config \
  rustops:latest
```

### Kubernetes Deployment

#### Helm Chart

```bash
# Add Helm repository
helm repo add rustops https://charts.rustops.io

# Install
helm install rustops rustops/rustops \
  --namespace rustops \
  --create-namespace \
  --values values.yaml
```

#### Values Example

```yaml
# values.yaml
replicaCount: 3

image:
  repository: rustops/rustops
  tag: "1.0.0"
  pullPolicy: Always

resources:
  limits:
    cpu: 500m
    memory: 512Mi
  requests:
    cpu: 250m
    memory: 256Mi

autoscaling:
  enabled: true
  minReplicas: 3
  maxReplicas: 10
  targetCPUUtilizationPercentage: 80

# Infrastructure dependencies
kafka:
  enabled: true

neo4j:
  enabled: true

postgresql:
  enabled: true

redis:
  enabled: true
```

### Monitoring

#### Prometheus Metrics

RustOps exposes metrics at `/metrics`:

- `rustops_incidents_total` - Total incidents
- `rustops_anomalies_detected_total` - Total anomalies detected
- `rustops_remediations_total` - Total remediations executed
- `rustops_remediation_duration_seconds` - Remediation duration

#### Grafana Dashboards

Import the provided dashboard from `deploy/grafana/dashboards/` to visualize:
- Incident trends
- Anomaly detection accuracy
- Remediation success rates
- System health

---

## Troubleshooting

### Common Issues

#### Kafka Connection Failed

**Symptom**: `Failed to connect to Kafka`

**Solution**:
```bash
# Check Kafka is running
docker ps | grep kafka

# Check Kafka logs
docker logs kafka

# Verify connectivity
telnet localhost 9092
```

#### Neo4j Connection Timeout

**Symptom**: `Connection timeout to Neo4j`

**Solution**:
```bash
# Check Neo4j is running
docker ps | grep neo4j

# Check Neo4j logs
docker logs neo4j

# Verify connectivity
curl http://localhost:7474
```

#### High Memory Usage

**Symptom**: RustOps process using excessive memory

**Solution**:
```yaml
# Adjust configuration
anomaly_detection:
  ml:
    batch_size: 100  # Reduce batch size
    max_cached_models: 3  # Limit cached models

remediation:
  max_concurrent: 5  # Limit concurrent remediations
```

#### Slow Anomaly Detection

**Symptom**: Anomaly detection takes > 100ms

**Solution**:
```yaml
# Disable ML detection for faster response
anomaly_detection:
  ml:
    enabled: false
  statistical:
    enabled: true
```

### Debug Mode

Enable debug logging:

```bash
export RUSTOPS_LOG_LEVEL=debug
export RUST_LOG=debug
cargo run
```

### Health Check

```bash
curl http://localhost:8080/health
```

Response:

```json
{
  "status": "healthy",
  "components": {
    "api": "healthy",
    "kafka": "healthy",
    "neo4j": "healthy",
    "postgresql": "healthy",
    "redis": "healthy"
  },
  "version": "1.0.0"
}
```

---

## Appendix

### ADR (Architecture Decision Records)

See `docs/adr/` for detailed architecture decisions:

- ADR-001: Event Sourcing
- ADR-002: CQRS Pattern
- ADR-003: Bounded Contexts
- ADR-004: Type-Safe IDs
- ADR-005: Hybrid Anomaly Detection
- ADR-006: Risk-Based Remediation Approval

### Glossary

| Term | Definition |
|------|------------|
| **AIOps** | Artificial Intelligence for IT Operations |
| **Bounded Context** | A distinct part of the domain logic |
| **CQRS** | Command Query Responsibility Segregation |
| **Event Sourcing** | Persisting state as a sequence of events |
| **Aggregate** | A cluster of domain objects treated as a unit |
| **Anti-Corruption Layer** | Isolating domain from external systems |
| **Blast Radius** | Scope of impact for a change |

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test --all`
5. Submit a pull request

### License

See LICENSE file for details.

### Support

- Documentation: https://docs.rustops.io
- Issues: https://github.com/your-org/rustops/issues
- Slack: https://rustops.slack.com
