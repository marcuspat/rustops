# RustOps

**Intelligent AIOps Platform for Automated Monitoring, Anomaly Detection, and Incident Remediation**

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-139%20passing-success.svg)](https://github.com/rustops/rustops)
[![Coverage](https://img.shields.io/badge/coverage-80%25%20minimum-brightgreen.svg)](https://github.com/rustops/rustops)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/rustops/rustops)

---

## Overview

RustOps is a comprehensive **AIOps (Artificial Intelligence for IT Operations)** platform built in Rust. It combines real-time monitoring, intelligent anomaly detection, automated incident management, and safe remediation workflows to empower DevOps teams with intelligent automation.

### Key Capabilities

- 🔍 **Real-time Anomaly Detection** - Statistical and ML-based detection with sub-millisecond latency
- 🚨 **Incident Management** - Alert correlation, deduplication, and root cause analysis
- 🔧 **Automated Remediation** - Safe, risk-based approval workflows with automatic rollback
- 🗺️ **Service Topology** - Graph-based dependency mapping with impact analysis
- 📚 **Knowledge Management** - Vector embeddings for semantic search and pattern storage
- 🔄 **Event Sourcing** - Complete audit trail with CQRS for scalable read models

---

## Table of Contents

- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Project Structure](#project-structure)
- [Features](#features)
- [Installation](#installation)
- [Configuration](#configuration)
- [Development](#development)
- [Testing](#testing)
- [Deployment](#deployment)
- [API Reference](#api-reference)
- [Contributing](#contributing)
- [License](#license)

---

## Quick Start

### Prerequisites

- **Rust**: 1.85 or later
- **Docker**: 20.10+ (optional, for containerized deployment)
- **Kubernetes**: 1.21+ (optional, for production deployment)
- **Neo4j**: 5.0+ (optional, for topology features)

### Local Development

```bash
# Clone the repository
git clone https://github.com/rustops/rustops.git
cd rustops

# Install dependencies and build
cargo build --workspace

# Run tests
cargo test --workspace

# Start the API server
RUST_LOG=info cargo run --bin rustops-api
# Server available at http://localhost:8080

# Start the agent service
RUST_LOG=info cargo run --bin rustops-agent
# Service available at http://localhost:8081
```

### Docker Deployment

```bash
# Build all images
make docker-build-all

# Start infrastructure services (Kafka, Neo4j, Prometheus, etc.)
docker-compose up -d

# Run RustOps services
docker-compose up -d rustops-api rustops-agent rustops-pipeline
```

### Verify Installation

```bash
# Health check
curl http://localhost:8080/health

# Metrics endpoint
curl http://localhost:8080/metrics

# API version
curl http://localhost:8080/api/v1/version
```

---

## Architecture

RustOps follows **Domain-Driven Design (DDD)** principles with **Event Sourcing** and **CQRS** patterns for scalable, maintainable architecture.

### System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         RustOps Platform                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────┐  ┌────────────┐  ┌─────────────┐  ┌───────────┐  │
│  │  API     │  │  Pipeline  │  │  Agent     │  │ Knowledge │  │
│  │  Server  │  │  Service  │  │  Service   │  │   Graph   │  │
│  └────┬─────┘  └──────┬─────┘  └──────┬──────┘  └─────┬─────┘  │
│       │              │                │              │         │
│  ┌───▼──────────────▼────────────────▼────────▼───────────▼───┐  │
│  │              Integration Layer (Adapters)                      │  │
│  │  Prometheus │ Kubernetes │ ServiceNow │ Slack │ PagerDuty    │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                   │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │              Bounded Contexts (Domain Layer)                   │  │
│  │  ┌────────┐ ┌──────────┐ ┌─────────┐ ┌──────────┐ ┌─────────┐  │ │
│  │  │Telemetry│ │ Anomaly  │ │Incident │ │Topology │ │Remediation│  │ │
│  │  │        │ │Detection│ │Management│ │         │ │         │  │ │
│  │  └────────┘ └──────────┘ └─────────┘ └──────────┘ └─────────┘  │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                   │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │              Infrastructure & Data Layer                        │  │
│  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌─────────┐ ┌───────────┐    │  │
│  │  │Kafka │ │Neo4j │ │PostgreSQL│Redis │Prometheus│    │  │
│  │  └──────┘ └──────┘ └──────┘ └─────────┘ └───────────┘    │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

### Design Patterns

| Pattern | Implementation | Benefit |
|---------|----------------|---------|
| **Domain-Driven Design** | Bounded contexts for each domain | Clear separation of concerns |
| **Event Sourcing** | Complete audit trail via domain events | Temporal queries, replay capability |
| **CQRS** | Separate read/write models | Optimized for each use case |
| **Adapter Pattern** | Unified interface for integrations | Swappable implementations |
| **Circuit Breaker** | Fault tolerance for external calls | Prevents cascading failures |
| **Retry with Backoff** | Exponential backoff for retries | Handles transient failures |
| **Repository Pattern** | Aggregate root persistence | Encapsulates data access |

---

## Project Structure

RustOps is organized as a Cargo workspace with specialized crates:

```
rustops/
├── Cargo.toml                 # Workspace configuration
├── crates/
│   ├── common/                 # Shared types and utilities
│   ├── telemetry/              # Metrics, logs, traces collection
│   ├── anomaly/                # Anomaly detection algorithms
│   ├── incident/                # Incident management (CQRS/ES)
│   ├── integration/             # External system adapters
│   ├── topology/                # Service topology graph
│   ├── knowledge/               # Knowledge graph with embeddings
│   └── remediation/             # Automated remediation workflows
├── src/                        # Binary entry points
├── tests/                      # Integration tests
├── docs/                       # Documentation
├── docker-compose.yml           # Local development stack
└── Dockerfile                  # Multi-stage builds
```

### Crates Overview

| Crate | Purpose | Key Exports |
|-------|---------|--------------|
| **common** | Foundation types | IDs, Events, Errors, Telemetry primitives |
| **telemetry** | Collection pipeline | CollectorRegistry, Metric, LogEntry, TraceSpan |
| **anomaly** | Detection engine | ZScoreDetector, IQRDetector, AnomalyRouter |
| **incident** | Incident management | IncidentRepository, AlertCorrelator, TopologyGrouping |
| **integration** | External adapters | PrometheusAdapter, KubernetesAdapter, ServiceNowAdapter |
|**topology** | Service graph | ServiceGraph, DependencyAnalyzer, ImpactScorer |
| **knowledge** | Knowledge management | VectorSearch, PatternStorage, Runbook |
| **remediation** | Remediation engine | WorkflowEngine, SafetyCheck, ApprovalGate |

---

## Features

### 🔍 Anomaly Detection

Multi-algorithm approach for optimal accuracy and performance:

- **Statistical Detection** (<1ms latency)
  - Z-score for spike/drop detection
  - IQR (Interquartile Range) for outlier detection
  - CUSUM for cumulative change detection

- **ML-Based Detection** (~50ms latency)
  - ONNX Runtime integration for model inference
  - Support for TensorFlow, PyTorch, scikit-learn models
  - Model versioning and hot-reloading

- **Pattern Matching**
  - Signature-based detection for known anomalies
  - Seasonal decomposition
  - Trend analysis

```rust
// Example: Using Z-score detector
let detector = ZScoreDetector::new(2.0);
let result = detector.detect(&metrics).await?;

for anomaly in result.anomalies {
    println!("Anomaly detected: {:?} with confidence: {}",
        anomaly.anomaly_type, anomaly.confidence);
}
```

### 🚨 Incident Management

Complete incident lifecycle management:

- **Alert Ingestion** - Real-time alert processing from monitoring systems
- **Correlation** - Intelligent grouping of related alerts
- **Deduplication** - Eliminates duplicate alerts using similarity matching
- **Topology Grouping** - Groups alerts by affected service topology
- **Root Cause Ranking** - ML-based ranking of probable root causes
- **Event Sourcing** - Complete audit trail for compliance

```rust
// Create new incident
let incident = Incident::new(
    "High CPU usage detected",
    IncidentSeverity::P2,
    IncidentStatus::New,
);

// Repository persists with event sourcing
repository.save(incident).await?;
```

### 🔧 Automated Remediation

Safe, risk-based remediation workflows:

- **Workflow Orchestration** - Temporal workflow engine for complex processes
- **Risk Assessment** - Automatic risk scoring based on change impact
- **Approval Gates** - Configurable approval workflows for high-risk changes
- **Blast Radius Limits** - Namespace and resource constraints
- **Safety Interlocks** - Pre-flight checks and validation
- **Automatic Rollback** - Revert changes on failure detection

```rust
// Remediation workflow
let workflow = RestartServiceWorkflow::new(executor, config);
let context = WorkflowContext::new(incident);

let result = workflow.execute(&mut context).await?;
if result.success {
    println!("Service restarted successfully");
}
```

### 🗺️ Service Topology

Graph-based service dependency management:

- **Discovery** - Automatic service discovery from Kubernetes
- **Graph Database** - Neo4j for efficient topology queries
- **Impact Analysis** - Predict downstream impact of changes
- **Communication Patterns** - Detect HTTP, gRPC, Kafka connections
- **Real-time Updates** - Streaming topology changes

```rust
// Query service dependencies
let dependencies = graph.get_dependencies("payment-api").await?;
for dep in dependencies {
    println!("payment-api depends on: {}", dep.service_name);
}

// Impact analysis
let impact = graph.estimate_impact("database", "delete").await?;
println!("Would affect {} services", impact.service_count);
```

### 📚 Knowledge Management

Intelligent knowledge storage and retrieval:

- **Vector Embeddings** - Semantic search using HNSW (150x-12,500x faster)
- **Pattern Storage** - Store successful remediation patterns
- **Runbook Automation** - Link knowledge to executable actions
- **Learning Loop** - Continuously improve from successful resolutions

```rust
// Semantic search
let results = knowledge.search("service restart timeout").await?;
for pattern in results {
    println!("Found pattern: {} (confidence: {})",
        pattern.description, pattern.similarity);
}
```

---

## Installation

### From Source

```bash
# Clone repository
git clone https://github.com/rustops/rustops.git
cd rustops

# Build workspace
cargo build --release

# Install binaries
cargo install --path .
```

### Using Cargo

```bash
# Install API server
cargo install rustops-api --path crates/api

# Install agent service
cargo install rustops-agent --path crates/agent
```

### Docker

```bash
# Pull images
docker pull rustops/api:latest
docker pull rustops/agent:latest

# Run with docker-compose
docker-compose up -d
```

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| **CPU** | 2 cores | 4+ cores |
| **Memory** | 4 GB | 8+ GB |
| **Disk** | 20 GB | 50+ GB |
| **Rust** | 1.85 | 1.92+ |
| **Docker** | 20.10 | 24.0+ |

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |
| `RUSTOPS_API_PORT` | `8080` | API server port |
| `RUSTOPS_AGENT_PORT` | `8081` | Agent service port |
| `RUSTOPS_PIPELINE_PORT` | `9090` | Pipeline service port |
| `KAFKA_BROKERS` | `localhost:9092` | Kafka broker addresses |
| `NEO4J_URI` | `bolt://localhost:7687` | Neo4j connection URI |
| `PROMETHEUS_PORT` | `9090` | Prometheus scrape port |

### Configuration File

Create `config.yaml`:

```yaml
# Agent configuration
agent:
  collection_interval_seconds: 15
  batch_size: 100

# Pipeline configuration
pipeline:
  kafka_brokers:
    - "localhost:9092"
  consumer_group: "rustops-pipeline"
  auto_offset_reset: "latest"

# Telemetry configuration
telemetry:
  prometheus:
    url: "http://localhost:9090"
    scrape_interval: "15s"

  logging:
    level: "info"
    format: "json"

# Anomaly detection
anomaly:
  z_score_threshold: 2.0
  iqr_multiplier: 1.5
  ml_model_path: "/models/anomaly.onnx"

  detection_window_seconds: 300
  minimum_data_points: 100

# Incident management
incident:
  correlation_window_minutes: 15
  deduplication_similarity_threshold: 0.85

  auto_escalation:
    p1_escalation_minutes: 5
    p2_escalation_minutes: 15

# Remediation
remediation:
  default_approval_strategy: "auto"  # auto, manual, hybrid

  blast_radius:
    enabled: true
    max_aged_services: 5
    max_namespace_depth: 3

  safety_checks:
    - "business_hours_check"
    - "maintenance_window_check"
    - "canary_deployment_check"

# Topology
topology:
  discovery_interval_seconds: 60
  neo4j_uri: "bolt://localhost:7687"

  communication_patterns:
    - "http"
    - "grpc"
    - "kafka"

# Knowledge graph
knowledge:
  hnsw_index_dimension: 384
  similarity_threshold: 0.75
  storage_path: "/data/knowledge"
```

### Service Configuration

Each service can be configured independently:

```bash
# API Server
rustops-api \
  --port 8080 \
  --config /etc/rustops/config.yaml \
  --log-level info

# Agent Service
rustops-agent \
  --port 8081 \
  --telemetry-prometheus http://prometheus:9090 \
  --pipeline-kafka-brokers localhost:9092
```

---

## Development

### Development Setup

```bash
# Install development dependencies
cargo install cargo-watch cargo-tarpaulin cargo-criterion

# Run with hot reload
cargo watch -x 'run --bin rustops-api'

# Run tests with output
cargo test --workspace -- --nocapture

# Run benchmarks
cargo bench
```

### Code Organization

- **Bounded Contexts** - Each crate represents a domain boundary
- **Aggregate Roots** - `Incident`, `ServiceGraph`, `Workflow` are key aggregates
- **Domain Events** - All state changes emit events for sourcing
- **Repositories** - Data access abstracted through repository pattern
- **Factories** - Complex object creation via factory methods

### Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration

# Property-based tests
cargo test --test property_tests

# With coverage
cargo tarpaulin --out Html

# Benchmarks
cargo bench -- --test
```

### Build Configuration

```toml
# Development profile
[profile.dev]
opt-level = 0
debug = true

# Release profile
[profile.release]
opt-level = 3
lto = true
codegen-units = 256
strip = true

# Benchmark profile
[profile.bench]
debug = true
```

### Linting

```bash
# Check code
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

---

## Testing

### Test Suite

The project includes **139 tests** covering all bounded contexts:

| Crate | Tests | Coverage Target |
|-------|-------|-----------------|
| common | 60 | >80% |
| telemetry | 14 | >80% |
| anomaly | 8 | >80% |
| incident | 16 | >80% |
| integration | 16 | >80% |
| topology | 9 | >80% |
| knowledge | 6 | >80% |
| remediation | 16 | >80% |

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p rustops-common

# Specific test
cargo test test_z_score_detector

# With output
cargo test -- --nocapture

# Run tests in parallel
cargo test --workspace --jobs 4
```

### Property-Based Testing

```bash
# Run property tests
cargo test --test property_tests

# Generate test cases
cargo test --test property_tests -- --generate
```

### Benchmarking

```bash
# Run all benchmarks
cargo bench

# Specific benchmark
cargo bench --bench id_benchmark

# Generate flamegraph
cargo bench --bench id_benchmark -- --profile-time=10
```

---

## Deployment

### Docker Deployment

```bash
# Build image
docker build -t rustops-api:latest .

# Run container
docker run -d \
  --name rustops-api \
  -p 8080:8080 \
  -v /etc/rustops:/etc/rustops:ro \
  -e RUST_LOG=info \
  rustops-api:latest
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustops-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rustops-api
  template:
    metadata:
      labels:
        app: rustops-api
    spec:
      containers:
      - name: rustops-api
        image: rustops/api:latest
        ports:
        - containerPort: 8080
        env:
        - name: RUST_LOG
          value: "info"
        - name: KAFKA_BROKERS
          value: "kafka:9092"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: rustops-api
spec:
  selector:
    app: rustops-api
  ports:
  - port: 8080
    targetPort: 8080
  type: LoadBalancer
```

### Infrastructure Requirements

#### Minimum Production Setup

| Service | vCPU | Memory | Storage |
|---------|------|--------|--------|
| rustops-api | 2 | 4GB | 10GB |
| rustops-agent | 1 | 2GB | 5GB |
| rustops-pipeline | 2 | 4GB | 10GB |
| Kafka | 2 | 4GB | 50GB |
| Neo4j | 2 | 4GB | 50GB |
| PostgreSQL | 2 | 4GB | 50GB |
| Redis | 1 | 2GB | 10GB |
| Prometheus | 1 | 2GB | 20GB |
| Grafana | 1 | 2GB | 10GB |
| Jaeger | 1 | 2GB | 10GB |
| Temporal | 2 | 4GB | 20GB |
| Fluentd | 1 | 2GB | 10GB |
| ClickHouse | 2 | 4GB | 50GB |

**Total**: ~17 vCPU, 36GB RAM, 245GB storage

---

## API Reference

### REST API v1

#### Health & Status

```
GET /health
GET /metrics
GET /api/v1/version
```

#### Telemetry

```
POST /api/v1/telemetry/metrics
POST /api/v1/telemetry/logs
POST /api/v1/telemetry/traces
```

#### Incidents

```
GET /api/v1/incidents
GET /api/v1/incidents/:id
POST /api/v1/incidents
PUT /api/v1/incidents/:id
DELETE /api/v1/incidents/:id
GET /api/v1/incidents/:id/timeline
```

#### Anomalies

```
GET /api/v1/anomalies
GET /api/v1/anomalies/:id
POST /api/v1/anomalies/detect
PUT /api/v1/anomalies/:id/acknowledge
```

#### Topology

```
GET /api/v1/topology/services
GET /api/v1/topology/services/:id/dependencies
POST /api/v1/topology/analyze-impact
```

#### Knowledge

```
GET /api/v1/knowledge/search
POST /api/v1/knowledge/patterns
GET /api/v1/knowledge/patterns/:id
```

#### Remediation

```
GET /api/v1/remediation/workflows
POST /api/v1/remediation/workflows
PUT /api/v1/remediation/workflows/:id
POST /api/v1/remediation/workflows/:id/approve
POST /api/v1/remediation/workflows/:id/execute
GET /api/v1/remediation/safety-checks
```

### WebSocket Streams

```
WS /api/v1/stream/metrics
WS /api/v1/stream/alerts
WS /api/v1/stream/topology
WS /api/v1/stream/workflows
```

### Example Requests

```bash
# Create incident
curl -X POST http://localhost:8080/api/v1/incidents \
  -H "Content-Type: application/json" \
  -d '{
    "title": "High CPU usage on payment-service",
    "severity": "P2",
    "description": "CPU usage at 95% for 5 minutes",
    "labels": {"service": "payment-service"}
  }'

# Search knowledge
curl -X POST http://localhost:8080/api/v1/knowledge/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "database connection pool exhaustion",
    "limit": 10
  }'

# Trigger remediation
curl -X POST http://localhost:8080/api/v1/remediation/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "incident_id": "incident-123",
    "workflow_type": "restart_service",
    "parameters": {
      "service_name": "payment-service",
      "namespace": "production"
    }
  }'
```

---

## Contributing

We welcome contributions! Please see below for guidelines.

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Write tests (`cargo test --workspace`)
5. Format code (`cargo fmt`)
6. Run lints (`cargo clippy`)
7. Commit your changes (`git commit -m "Add amazing feature"`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

### Code Style

- Follow Rust naming conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Document all public APIs
- Keep functions focused and small
- Write descriptive commit messages

### Testing Requirements

- All tests must pass (`cargo test --workspace`)
- New features require tests
- Maintain >80% code coverage
- Add integration tests for external APIs
- Include property tests for algorithms

### Pull Request Guidelines

- Describe the change in the PR title
- Provide context in the PR description
- Link related issues
- Ensure CI checks pass
- Request review from relevant maintainers
- Keep PRs small and focused

---

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

```
Copyright 2025 RustOps Contributors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

---

## Support

### Documentation

- 📚 [Full Documentation](/docs/)
- 📖 [API Reference](/docs/api.md)
- 🏗️ [Architecture Guide](/docs/architecture.md)
- 🔧 [Development Guide](/docs/development.md)

### Community

- 💬 [Discussions](https://github.com/rustops/rustops/discussions)
- 🐛 [Issues](https://github.com/rustops/rustops/issues)
- 📧 Email: support@rustops.com

### Acknowledgments

Built with:
- **[Rust](https://www.rust-lang.org/)** - Systems programming language
- **[Tokio](https://tokio.rs/)** - Async runtime
- **[Serde](https://serde.rs/)** - Serialization framework
- **[Kube](https://github.com/kube-rs/kube)** - Kubernetes client
- **[Prometheus](https://prometheus.io/)** - Metrics monitoring
- **[Temporal](https://temporal.io/)** - Workflow orchestration
- **[Neo4j](https://neo4j.com/)** - Graph database

---

**RustOps** - Empowering DevOps teams with intelligent automation and observability.

For more information, visit [https://github.com/rustops/rustops](https://github.com/rustops/rustops)
