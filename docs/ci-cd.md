# CI/CD Pipeline Documentation

This document describes the CI/CD pipelines for the RustOps AIOps platform.

## Overview

The CI/CD system consists of three main pipelines:

1. **PR Pipeline** (`pr.yml`) - Runs on pull requests
2. **Main Branch Pipeline** (`main.yml`) - Runs on pushes to main/develop
3. **Release Pipeline** (`release.yml`) - Runs on version tags

## PR Pipeline

Located in `.github/workflows/pr.yml`

### Jobs

| Job | Description | Timeout |
|-----|-------------|----------|
| format-check | Validates code formatting with rustfmt | 5 min |
| clippy | Runs linter in strict mode | 15 min |
| unit-tests | Runs unit tests on Linux, Windows, macOS | 20 min |
| coverage | Generates code coverage report | 25 min |
| security | Runs cargo-audit and cargo-deny | 15 min |
| benchmarks | Checks for performance regressions | 30 min |
| documentation | Builds and validates docs | 10 min |
| dependency-review | Reviews dependency changes | 5 min |

### Security Checks

- **cargo-audit**: Scans for known vulnerabilities
- **cargo-deny**: Checks advisories, licenses, and banned crates
- **dependency-review-action**: GitHub dependency review

### Coverage

- Uses `cargo-tarpaulin` for coverage reporting
- Uploads to Codecov
- Generates LCOV and XML reports

## Main Branch Pipeline

Located in `.github/workflows/main.yml`

### Jobs

| Job | Description | Timeout |
|-----|-------------|----------|
| build-and-test | Full build and test suite | 30 min |
| docker-build | Builds Docker images for all components | 45 min |
| security-scan | Trivy vulnerability scanning | 30 min |
| deploy-dev | Deploys to dev environment (develop branch) | 20 min |
| smoke-tests | Runs health checks on deployed services | 15 min |
| notify | Sends deployment notifications | - |

### Docker Images

Built for three components:
- `rustops-api` - Main API service
- `rustops-agent` - Agent service
- `rustops-pipeline` - Pipeline processor

### Deployment (Develop Branch Only)

The pipeline deploys to the dev environment when pushing to the `develop` branch:
- Uses Helm for deployment
- Runs smoke tests after deployment
- Performs health checks on all services

## Release Pipeline

Located in `.github/workflows/release.yml`

### Workflow Triggers

- Tag push: `v*.*.*`
- Manual workflow dispatch

### Jobs

| Job | Description | Timeout |
|-----|-------------|----------|
| validate | Validates version and changelog | 10 min |
| build-release | Builds binaries for multiple platforms | 60 min |
| docker-release | Builds and pushes Docker images | 60 min |
| deploy-staging | Deploys to staging environment | 30 min |
| rollout-production | Multi-stage rollout (25% -> 50% -> 100%) | 90 min |
| rollback-on-failure | Automated rollback on failure | 20 min |
| create-release | Creates GitHub release | 15 min |
| post-release | Post-release tasks | 15 min |

### Multi-Stage Rollout

The production deployment uses a gradual rollout strategy:

1. **25%** - Deploy to single replica
2. **50%** - Scale to two replicas
3. **100%** - Full deployment with three replicas

Each stage:
- Waits for rollout completion
- Runs health checks
- Monitors for errors (5 minutes)
- Runs integration tests (final stage)

### Automated Rollback

If any stage fails:
- Automatic Helm rollback
- Kubernetes deployment undo
- Verification of rollback
- Notification sent

## Additional Workflows

### Security Scan (`security-scan.yml`)

Runs daily at 2 AM UTC:
- Security audit
- Dependency review
- License compliance check
- Docker image scanning with Trivy

### Benchmarks (`benchmarks.yml`)

Runs weekly and on code changes:
- Criterion benchmarks
- Performance regression detection
- Flamegraph generation

### Dependencies (`dependencies.yml`)

Runs weekly on Mondays:
- Updates all dependencies
- Creates pull request for review

### Documentation (`docs.yml`)

Runs on changes to docs:
- Builds Rust documentation
- Checks documentation links
- Deploys to GitHub Pages (main branch)

## Dockerfiles

Multi-stage builds located in `/docker/`:

| File | Component | Base Image |
|------|-----------|------------|
| Dockerfile.api | API Service | debian:bookworm-slim |
| Dockerfile.agent | Agent Service | debian:bookworm-slim |
| Dockerfile.pipeline | Pipeline Processor | debian:bookworm-slim |

### Build Stages

1. **Builder**: Compiles Rust code with full toolchain
2. **Runtime**: Minimal runtime image with only binary

### Security Features

- Non-root user (uid 1000)
- Read-only root filesystem
- Dropped capabilities
- Health checks enabled

## Helm Charts

Located in `/helm/rustops/`

### Structure

```
helm/rustops/
├── Chart.yaml              # Chart metadata
├── values.yaml             # Default values
├── values-dev.yaml         # Development overrides
├── values-staging.yaml     # Staging overrides
├── values-prod.yaml        # Production overrides
└── templates/
    ├── _helpers.tpl        # Template helpers
    ├── deployment.yaml     # Deployment resources
    ├── service.yaml        # Service resources
    ├── hpa.yaml            # Horizontal Pod Autoscaler
    ├── ingress.yaml        # Ingress resources
    ├── serviceaccount.yaml # Service account
    ├── rbac.yaml           # RBAC resources
    ├── configmap.yaml      # ConfigMap
    └── secret.yaml         # Secret resources
```

### Environments

| Environment | Namespace | Replicas | Auto-scaling |
|-------------|-----------|----------|--------------|
| dev | rustops-dev | 1 | Disabled |
| staging | rustops-staging | 2 | 2-5 replicas |
| prod | rustops-prod | 3 | 3-20 replicas |

## Composite Actions

Reusable actions located in `.github/composite-actions/`:

### rust-build

Sets up Rust build environment with caching.

```yaml
- uses: ./.github/composite-actions/rust-build
  with:
    rust-version: 'stable'
    components: 'clippy, rustfmt'
```

### docker-build

Builds and pushes Docker images.

```yaml
- uses: ./.github/composite-actions/docker-build
  with:
    component: 'api'
    registry: 'ghcr.io'
    image-name: 'rustops/rustops'
    version: 'v1.0.0'
```

### helm-deploy

Deploys with Helm to Kubernetes.

```yaml
- uses: ./.github/composite-actions/helm-deploy
  with:
    environment: 'production'
    kubeconfig: ${{ secrets.KUBE_CONFIG_PROD }}
    version: 'v1.0.0'
```

## Scripts

### smoke-tests.sh

Runs health checks on deployed services.

```bash
./scripts/smoke-tests.sh <environment>
```

Checks:
- Pod status
- Health endpoints
- Error logs
- Resource usage

### monitor-deployment.sh

Monitors deployment for errors.

```bash
./scripts/monitor-deployment.sh <environment> <duration_seconds>
```

Monitors:
- Pod status
- Crash loops
- Error logs
- Resource usage

### integration-tests.sh

Runs integration tests.

```bash
./scripts/integration-tests.sh <environment>
```

Tests:
- API endpoints
- Service discovery
- Data flow
- Metrics

## Configuration Files

### CODEOWNERS

Defines code ownership for review requirements.

### branch-protection-rules.yml

Documents branch protection rules (apply via GitHub UI).

### deny.toml

Cargo-deny configuration for:
- Security advisories
- License compliance
- Dependency bans

## Secrets

The following secrets must be configured in GitHub:

| Secret | Description | Required By |
|--------|-------------|-------------|
| GITHUB_TOKEN | GitHub token | All workflows |
| CODECOV_TOKEN | Codecov token | Coverage reporting |
| KUBE_CONFIG_DEV | Dev cluster kubeconfig | Main pipeline |
| KUBE_CONFIG_STAGING | Staging cluster kubeconfig | Release pipeline |
| KUBE_CONFIG_PROD | Prod cluster kubeconfig | Release pipeline |

## Concurrency

Workflows use concurrency groups to prevent duplicate runs:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

## Build Optimization

### Cargo Workspace Caching

Uses `Swatinem/rust-cache@v2` for:
- Dependency caching
- Build artifacts
- Incremental compilation

### Docker Layer Caching

Uses GitHub Actions cache for Docker layers:
```yaml
cache-from: type=gha
cache-to: type=gha,mode=max
```

### Parallel Builds

- Matrix builds for multi-platform support
- Parallel job execution
- Independent test runs

## Security Best Practices

1. **No hardcoded secrets**
2. **GITHUB_TOKEN with minimal permissions**
3. **CODEOWNERS for workflow changes**
4. **Environment protection rules**
5. **Security scanning on all builds**
6. **SBOM and provenance for Docker images**
7. **Non-root containers**
8. **Read-only filesystems**
9. **Dropped capabilities**
10. **Regular dependency updates**

## Troubleshooting

### Pipeline Failures

1. Check the Actions tab in GitHub
2. Review job logs for errors
3. Check for security advisories
4. Verify dependency compatibility

### Deployment Failures

1. Check pod logs: `kubectl logs -n <namespace> <pod>`
2. Check events: `kubectl get events -n <namespace>`
3. Verify health endpoints
4. Check resource usage

### Rollback

If a release fails, automatic rollback is triggered. To manually rollback:

```bash
helm rollback rustops-prod -n rustops-prod
```

## Performance Targets

| Metric | Target |
|--------|--------|
| PR Pipeline | < 15 minutes |
| Main Pipeline | < 30 minutes |
| Release Pipeline | < 3 hours |
| Docker Build | < 10 minutes |
| Deployment | < 5 minutes per environment |
