# Monorepo Structure

Complete directory structure for the RustOps AIOps platform monorepo.

## Directory Tree

```
rustops/
├── crates/                          # Rust workspace crates
│   ├── agent/                       # Telemetry collection agent
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs              # Agent entry point
│   │       ├── collectors/          # Metric, log, trace collectors
│   │       ├── config/              # Configuration management
│   │       └── transport/           # Kafka/pipeline client
│   │
│   ├── pipeline/                    # Telemetry processing pipeline
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── consumer.rs          # Kafka consumer
│   │       ├── processor.rs         # Stream processing
│   │       ├── router.rs            # Data routing
│   │       └── buffer/              # Batching and buffering
│   │
│   ├── anomaly/                     # ML anomaly detection
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── models/              # ONNX model loading
│   │       ├── detectors/           # Statistical, ML detectors
│   │       ├── scoring.rs           # Anomaly scoring
│   │       └── feedback.rs          # Learning feedback loop
│   │
│   ├── correlation/                 # Alert correlation engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── dedup.rs             # Deduplication logic
│   │       ├── grouper.rs           # Time/topology grouping
│   │       ├── enricher.rs          # Context enrichment
│   │       └── causal.rs            # Causal inference
│   │
│   ├── remediation/                 # Remediation workflow engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── engine.rs            # Workflow engine
│   │       ├── actions/             # Remediation actions
│   │       ├── approval.rs          # Approval gates
│   │       └── execution.rs         # Execution tracking
│   │
│   ├── topology/                    # Service topology discovery
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── discovery.rs         # Service discovery
│   │       ├── graph.rs             # Dependency graph
│   │       ├── mapper.rs            # K8s/cloud mapping
│   │       └── impact.rs            # Impact analysis
│   │
│   ├── api/                         # REST/GraphQL API gateway
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── routes/              # API routes
│   │       ├── handlers/            # Request handlers
│   │       ├── middleware/          # Auth, logging, etc.
│   │       └── graphql/             # GraphQL schema
│   │
│   └── common/                      # Shared utilities
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── config.rs            # Configuration structs
│           ├── error.rs             # Error types
│           ├── telemetry.rs         # Telemetry utils
│           └── crypto.rs            # Cryptographic utilities
│
├── integrations/                     # External system adapters
│   ├── prometheus/                  # Prometheus integration
│   ├── datadog/                     # Datadog integration
│   ├── cloudwatch/                  # AWS CloudWatch
│   ├── kubernetes/                  # Kubernetes adapter
│   ├── servicenow/                  # ServiceNow ITSM
│   ├── pagerduty/                   # PagerDuty integration
│   └── slack/                       # Slack integration
│
├── ml/                              # Machine learning models
│   ├── models/                      # Trained ONNX models
│   │   ├── time_series_anomaly.onnx
│   │   ├── log_clustering.onnx
│   │   └── root_cause.onnx
│   ├── training/                    # Model training scripts
│   │   ├── train_anomaly.py
│   │   ├── train_clustering.py
│   │   └── evaluate.py
│   ├── data/                        # Training datasets
│   └── notebooks/                   # Jupyter notebooks
│
├── web/                             # Dashboard UI
│   ├── package.json
│   ├── tsconfig.json
│   ├── src/
│   │   ├── components/              # React components
│   │   ├── pages/                   # Page components
│   │   ├── hooks/                   # Custom hooks
│   │   ├── services/                # API clients
│   │   └── types/                   # TypeScript types
│   ├── public/
│   └── tests/                       # Frontend tests
│
├── deploy/                          # Deployment configurations
│   ├── kubernetes/                  # K8s manifests
│   │   ├── base/
│   │   ├── overlays/
│   │   │   ├── dev/
│   │   │   ├── staging/
│   │   │   └── production/
│   │   └── helm/                    # Helm charts
│   ├── terraform/                   # Infrastructure as code
│   ├── docker/                      # Dockerfiles
│   └── scripts/                     # Deployment scripts
│
├── tests/                           # Integration tests
│   ├── integration/                 # Integration test suites
│   ├── e2e/                         # End-to-end tests
│   ├── fixtures/                    # Test fixtures and data
│   ├── mocks/                       # Mock servers and services
│   └── performance/                 # Performance test suites
│
├── docs/                            # Documentation
│   ├── architecture/                # Architecture documentation
│   │   ├── adr/                     # Architecture Decision Records
│   │   ├── diagrams/                # Architecture diagrams
│   │   └── api/                     # API documentation
│   ├── runbooks/                    # Operational runbooks
│   ├── guides/                      # User and developer guides
│   └── api/                         # Generated rustdoc
│
├── plans/                           # Project plans and research
│   ├── development/                 # Development documentation (this file)
│   ├── research/                    # Research documents
│   ├── adrs/                        # ADRs
│   └── integrations/                # Integration plans
│
├── config/                          # Configuration files
│   ├── examples/                    # Example configurations
│   └── schemas/                     # JSON schemas for validation
│
├── scripts/                         # Utility scripts
│   ├── setup/                       # Development setup
│   ├── build/                       # Build helpers
│   └── release/                     # Release automation
│
├── .github/                         # GitHub-specific files
│   ├── workflows/                   # CI/CD workflows
│   ├── ISSUE_TEMPLATE/              # Issue templates
│   ├── PULL_REQUEST_TEMPLATE.md     # PR template
│   └── dependabot.yml               # Dependency updates
│
├── Cargo.toml                       # Workspace root
├── Cargo.lock                       # Lock file
├── Makefile                         # Build automation
├── Justfile                         # Alternative to Makefile
├── .gitignore
├── .dockerignore
├── clippy.toml                      # Clippy configuration
├── rustfmt.toml                     # Rustfmt configuration
└── README.md
```

## Directory Purposes

### `/crates`

Rust workspace containing all application crates. Each crate is a separate compilation unit with clear responsibilities.

- **agent**: Lightweight telemetry collection agent deployed on monitored nodes
- **pipeline**: High-throughput data processing pipeline
- **anomaly**: ML-based anomaly detection with ONNX Runtime
- **correlation**: Alert correlation and deduplication engine
- **remediation**: Self-healing workflow orchestration
- **topology**: Service discovery and dependency mapping
- **api**: Public API gateway for integrations
- **common**: Shared utilities and types

### `/integrations`

Adapters for external systems. Each integration is a separate crate that implements a common trait interface.

### `/ml`

Machine learning models, training scripts, and data. ONNX models for runtime inference.

### `/web`

Dashboard UI built with React/TypeScript. Uses Vite for fast development and builds.

### `/deploy`

All deployment configurations:
- Kubernetes manifests (kustomize)
- Helm charts for production deployments
- Dockerfiles for all services
- Terraform for infrastructure provisioning

### `/tests`

Test code not included in individual crates:
- Integration tests spanning multiple crates
- End-to-end scenario tests
- Performance and load tests
- Test fixtures and mocks

### `/docs`

Project documentation:
- Architecture Decision Records (ADRs)
- API documentation (rustdoc + hand-written)
- Operational runbooks
- User and developer guides

### `/plans`

Project planning documents:
- Development guides
- Research documents
- Architecture decisions
- Integration plans

### `/config`

Configuration files and schemas:
- Example configurations for different environments
- JSON schemas for validation
- Default configurations

### `/scripts`

Development and deployment scripts:
- Environment setup
- Build helpers
- Release automation

## File Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Rust files | `snake_case.rs` | `telemetry_collector.rs` |
| Modules | `snake_case` | `mod telemetry;` |
| Tests | `<module>_test.rs` | `collector_test.rs` |
| Config | `<name>.yaml` | `agent.config.yaml` |
| Docs | `kebab-case.md` | `api-reference.md` |
| Scripts | `kebab-case.sh` | `setup-dev.sh` |

## Workspace Organization Principles

1. **Single Responsibility**: Each crate has one clear purpose
2. **Minimal Coupling**: Crates depend only on what they need
3. **Clear Boundaries**: Integrations are isolated in their own crates
4. **Shared Code**: Common utilities go in `common` crate
5. **Test Visibility**: Integration tests at workspace level
6. **Deployment Ready**: All deployment configs in `/deploy`

## Adding a New Crate

1. Create directory under `/crates`
2. Add `Cargo.toml` with workspace inheritance
3. Add to root `Cargo.toml` workspace members
4. Follow naming convention: `rustops-<crate-name>`
5. Update this document with purpose

## Adding an Integration

1. Create directory under `/integrations`
2. Implement integration trait
3. Add feature flag to relevant crate
4. Add integration tests
5. Update documentation

## Adding Documentation

1. Architecture docs go in `/docs/architecture`
2. ADRs go in `/docs/architecture/adr`
3. Runbooks go in `/docs/runbooks`
4. API docs are generated from source code
