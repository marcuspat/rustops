# RustOps Development Guide

Comprehensive monorepo structure, CI/CD pipeline, and development workflow documentation.

## Quick Start

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
cargo install cargo-watch cargo-nextest

# Build workspace
make build

# Run tests
make test

# Run linters
make lint
```

## Documentation Index

| Document | Description |
|----------|-------------|
| [Monorepo Structure](./01-monorepo-structure.md) | Workspace layout and crate organization |
| [Cargo Workspace](./02-cargo-workspace.md) | Workspace TOML and dependency management |
| [Testing Strategy](./03-testing-strategy.md) | Unit, integration, property-based tests |
| [CI/CD Pipeline](./04-ci-cd-pipeline.md) | GitHub Actions workflows |
| [Quality Gates](./05-quality-gates.md) | Linting, security, and coverage standards |
| [Documentation Standards](./06-documentation-standards.md) | API docs, ADRs, runbooks |
| [Build Configuration](./07-build-config.md) | Makefile, Justfile, build scripts |
| [Development Workflow](./08-development-workflow.md) | Branching, commits, PR process |

## Architecture

```
rustops/
├── crates/              # Rust workspace
│   ├── agent/          # Telemetry collection
│   ├── pipeline/       # Data processing
│   ├── anomaly/        # ML detection
│   ├── correlation/    # Alert correlation
│   ├── remediation/    # Self-healing
│   ├── topology/       # Service discovery
│   ├── api/            # API gateway
│   └── common/         # Shared utilities
├── web/                # Dashboard UI
├── deploy/             # K8s manifests, Docker
├── tests/              # Integration tests
└── docs/               # Documentation
```

## Quality Metrics

| Metric | Target |
|--------|--------|
| Code Coverage | 80% |
| Clippy Warnings | 0 |
| Security Vulnerabilities | 0 |
| Performance Regression | 0% |
| Documentation Coverage | 100% public |

## CI/CD Pipeline

```
PR: lint → test → security → docs
     ↓
Merge: build → package → deploy-dev
     ↓
Release: build-containers → deploy-staging → deploy-prod
```

## Development Workflow

1. Create feature branch: `git checkout -b feature/my-feature`
2. Make changes and test: `make test`
3. Format code: `make fmt`
4. Lint: `make lint`
5. Push and create PR
6. Address reviews
7. Squash merge to develop

## Support

- Documentation: https://docs.rustops.dev
- Issues: https://github.com/rustops/rustops/issues
- Slack: https://rustops-dev.slack.com
