# CI/CD Pipeline

GitHub Actions workflows for continuous integration and deployment.

## Pipeline Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                         Pull Request                                 │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                   Quality Checks (5-10 min)                   │   │
│  ├──────────────────────────────────────────────────────────────┤   │
│  │  ✓ Format Check              │ rustfmt --check               │   │
│  │  ✓ Lint                      │ clippy (strict)               │   │
│  │  ✓ Unit Tests                │ cargo test --workspace        │   │
│  │  ✓ Integration Tests         │ cargo test --workspace        │   │
│  │  ✓ Property Tests            │ proptest (100+ iterations)    │   │
│  │  ✓ Security Audit            │ cargo-audit, cargo-deny       │   │
│  │  ✓ License Check             │ cargo-deny licenses           │   │
│  │  ✓ Documentation             │ cargo doc --no-deps           │   │
│  │  ✓ Coverage Report           │ tarpaulin (80% minimum)       │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                         Merge to Main                                │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                  Build & Package (10-15 min)                  │   │
│  ├──────────────────────────────────────────────────────────────┤   │
│  │  ✓ Release Build            │ cargo build --release          │   │
│  │  ✓ Cross-compile            │ Linux, macOS, Windows          │   │
│  │  ✓ Build Containers         │ Docker multi-arch              │   │
│  │  ✓ Generate Artifacts       │ Binaries, archives             │   │
│  │  ✓ Performance Benchmarks   │ Criterion (no regression)      │   │
│  │  ✓ Security Scan            │ Trivy, Grype                   │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                   Deploy to Dev (5 min)                       │   │
│  ├──────────────────────────────────────────────────────────────┤   │
│  │  ✓ Push Containers         │ Container registry              │   │
│  │  ✓ Deploy to Dev           │ Kubernetes (dev namespace)      │   │
│  │  ✓ Smoke Tests             │ Health checks, basic tests      │   │
│  │  ✓ Notify Team             │ Slack success message           │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                     Release Tag (vX.Y.Z)                             │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                 Production Build (10-15 min)                  │   │
│  ├──────────────────────────────────────────────────────────────┤   │
│  │  ✓ Tag Validation          │ Semver check                    │   │
│  │  ✓ Changelog Generation    │ From commits                    │   │
│  │  ✓ Production Containers   │ Hardened, scanned               │   │
│  │  ✓ Publish Release         │ GitHub releases                 │   │
│  │  ✓ Deploy to Staging       │ 25% of production               │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              Production Rollout (Staged, 1 hr)                │   │
│  ├──────────────────────────────────────────────────────────────┤   │
│  │  ✓ Stage 1: 25%            │ Monitor for 15 min              │   │
│  │  ✓ Stage 2: 50%            │ Monitor for 15 min              │   │
│  │  ✓ Stage 3: 100%           │ Monitor for 30 min              │   │
│  │  ✓ Verification            │ SLO checks, metrics             │   │
│  │  ✓ Rollback on Failure     │ Automatic if thresholds breach  │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

## Workflow Files

### 1. Pull Request Workflow

```yaml
# .github/workflows/pr.yml
name: Pull Request

on:
  pull_request:
    types: [opened, synchronize, reopened]
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  # Quick checks first - fail fast
  quick-checks:
    name: Quick Checks
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

  # Security scanning
  security:
    name: Security Scan
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Install cargo-deny
        run: cargo install cargo-deny

      - name: Run security audit
        run: cargo audit

      - name: Check advisories
        run: cargo deny check advisories

      - name: Check licenses
        run: cargo deny check licenses

      - name: Check bans
        run: cargo deny check bans

  # Unit tests
  unit-tests:
    name: Unit Tests
    runs-on: ${{ matrix.os }}
    timeout-minutes: 20
    strategy:
      fail-fast: false
      matrix:
        rust: [stable, beta]
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Run unit tests
        run: cargo test --workspace --lib --verbose

  # Integration tests
  integration-tests:
    name: Integration Tests
    runs-on: ubuntu-latest
    timeout-minutes: 30
    services:
      kafka:
        image: bitnami/kafka:latest
        ports:
          - 9092:9092
        env:
          KAFKA_CFG_ZOOKEEPER_CONNECT: zookeeper:2181
      clickhouse:
        image: clickhouse/clickhouse-server:latest
        ports:
          - 8123:8123
      redis:
        image: redis:alpine
        ports:
          - 6379:6379
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Run integration tests
        run: cargo test --workspace --test '*'
        env:
          KAFKA_BROKERS: localhost:9092
          CLICKHOUSE_URL: http://localhost:8123
          REDIS_URL: redis://localhost:6379

  # Property-based tests
  property-tests:
    name: Property Tests
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Run property tests
        run: |
          cargo test --workspace --features test-tests \
            -- --test-threads=1 \
            --nocapture \
            property

  # Code coverage
  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin

      - name: Generate coverage report
        run: |
          cargo tarpaulin --workspace \
            --exclude-files "*/tests/*" \
            --exclude-files "*/benches/*" \
            --timeout 300 \
            --out Lcov \
            --output-dir ./coverage \
            -- --test-threads=1

      - name: Upload to Codecov
        uses: codecov/codecov-action@v4
        with:
          files: ./coverage/lcov.info
          fail_ci_if_error: true
          minimum_coverage: 80

  # Documentation
  docs:
    name: Documentation
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Build documentation
        run: cargo doc --workspace --no-deps

      - name: Check docs links
        run: cargo doc --workspace --no-deps --document-private-items

  # Performance regression check
  benchmarks:
    name: Benchmarks
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Run benchmarks
        run: cargo bench --workspace -- --save-baseline pr

      - name: Compare baselines
        run: cargo bench --workspace -- --baseline pr

      - name: Upload benchmark results
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/report/index.html
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: false
```

### 2. Main Branch Workflow

```yaml
# .github/workflows/main.yml
name: Main Branch

on:
  push:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  build:
    name: Build Release
    runs-on: ${{ matrix.os }}
    timeout-minutes: 30
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Build release
        run: cargo build --workspace --release --target ${{ matrix.target }}

      - name: Strip binaries (Linux)
        if: matrix.os == 'ubuntu-latest'
        run: |
          for bin in target/*/release/rustops-*; do
            strip "$bin" || true
          done

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: binaries-${{ matrix.target }}
          path: |
            target/${{ matrix.target }}/release/rustops-*
            !target/${{ matrix.target }}/release/*.d
            !target/${{ matrix.target }}/release/*.rlib

  build-containers:
    name: Build Containers
    runs-on: ubuntu-latest
    timeout-minutes: 45
    permissions:
      contents: read
      packages: write
    strategy:
      matrix:
        component:
          - agent
          - pipeline
          - anomaly
          - correlation
          - remediation
          - topology
          - api
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}/${{ matrix.component }}
          tags: |
            type=ref,event=branch
            type=sha,prefix={{branch}}-
            type=raw,value=latest,enable={{is_default_branch}}

      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          file: ./deploy/docker/${{ matrix.component }}.Dockerfile
          platforms: linux/amd64,linux/arm64
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

      - name: Scan image
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}/${{ matrix.component }}:latest
          format: 'sarif'
          output: 'trivy-results.sarif'

      - name: Upload Trivy results
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: 'trivy-results.sarif'

  deploy-dev:
    name: Deploy to Dev
    runs-on: ubuntu-latest
    needs: [build, build-containers]
    timeout-minutes: 15
    environment:
      name: dev
      url: https://dev.rustops.internal
    steps:
      - uses: actions/checkout@v4

      - name: Configure kubectl
        uses: azure/k8s-set-context@v4
        with:
          method: kubeconfig
          kubeconfig: ${{ secrets.KUBE_CONFIG_DEV }}

      - name: Deploy to dev
        run: |
          kubectl set image deployment/rustops-api \
            rustops-api=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}/api:latest \
            -n rustops-dev

          kubectl rollout status deployment/rustops-api -n rustops-dev --timeout=5m

      - name: Smoke tests
        run: |
          kubectl run smoke-test --rm -i --restart=Never --image=curlimages/curl \
            -n rustops-dev -- curl -f http://rustops-api:8080/health || exit 1

      - name: Notify Slack
        uses: slackapi/slack-github-action@v1
        with:
          webhook-url: ${{ secrets.SLACK_WEBHOOK }}
          payload: |
            {
              "text": "Deployed to dev: ${{ github.sha }}",
              "blocks": [
                {
                  "type": "section",
                  "text": {
                    "type": "mrkdwn",
                    "text": "✅ Deployed to dev\n*Commit:* ${{ github.sha }}\n*Author:* ${{ github.actor }}"
                  }
                }
              ]
            }
```

### 3. Release Workflow

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

env:
  CARGO_TERM_COLOR: always
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

permissions:
  contents: write
  packages: write

jobs:
  validate:
    name: Validate Release
    runs-on: ubuntu-latest
    timeout-minutes: 10
    outputs:
      version: ${{ steps.parse.outputs.version }}
    steps:
      - uses: actions/checkout@v4

      - name: Parse version
        id: parse
        run: |
          VERSION=${GITHUB_REF#refs/tags/v}
          echo "version=$VERSION" >> $GITHUB_OUTPUT

          if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
            echo "Invalid semver version: $VERSION"
            exit 1
          fi

  changelog:
    name: Generate Changelog
    runs-on: ubuntu-latest
    needs: validate
    timeout-minutes: 10
    outputs:
      changelog: ${{ steps.changelog.outputs.changelog }}
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Generate changelog
        id: changelog
        run: |
          PREV_TAG=$(git describe --tags --abbrev=0 HEAD^)
          CHANGELOG=$(git log ${PREV_TAG}..HEAD --oneline --format="- %s")
          echo "changelog<<EOF" >> $GITHUB_OUTPUT
          echo "$CHANGELOG" >> $GITHUB_OUTPUT
          echo "EOF" >> $GITHUB_OUTPUT

  build-release:
    name: Build Release Binaries
    needs: validate
    runs-on: ${{ matrix.os }}
    timeout-minutes: 45
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: rustops-x86_64-linux.tar.gz
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            artifact: rustops-aarch64-linux.tar.gz
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: rustops-x86_64-macos.tar.gz
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: rustops-aarch64-macos.tar.gz
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: rustops-x86_64-windows.zip
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build release
        run: cargo build --workspace --release --target ${{ matrix.target }}

      - name: Package binaries
        shell: bash
        run: |
          mkdir -p dist
          for bin in target/${{ matrix.target }}/release/rustops-*; do
            if [ -f "$bin" ] && [ -x "$bin" ]; then
              cp "$bin" dist/
            fi
          done

          cd dist
          if [[ "${{ runner.os }}" == "Windows" ]]; then
            7z a ../${{ matrix.artifact }} *
          else
            tar czf ../${{ matrix.artifact }} *
          fi

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: ${{ matrix.artifact }}

  publish-containers:
    name: Publish Containers
    needs: validate
    runs-on: ubuntu-latest
    timeout-minutes: 60
    strategy:
      matrix:
        component:
          - agent
          - pipeline
          - anomaly
          - correlation
          - remediation
          - topology
          - api
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}/${{ matrix.component }}
          tags: |
            type=semver,pattern={{version}},value=${{ needs.validate.outputs.version }}
            type=semver,pattern={{major}}.{{minor}},value=${{ needs.validate.outputs.version }}
            type=raw,value=latest

      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          file: ./deploy/docker/${{ matrix.component }}.Dockerfile
          platforms: linux/amd64,linux/arm64
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          build-args: |
            VERSION=${{ needs.validate.outputs.version }}
            BUILD_DATE=$(date -u +'%Y-%m-%dT%H:%M:%SZ')
            VCS_REF=${{ github.sha }}

  create-release:
    name: Create GitHub Release
    needs: [changelog, build-release, publish-containers]
    runs-on: ubuntu-latest
    timeout-minutes: 15
    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts
        uses: actions/download-artifact@v4

      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ github.ref_name }}
          name: Release ${{ github.ref_name }}
          body: |
            ## What's Changed

            ${{ needs.changelog.outputs.changelog }}

            ## Installation

            See [documentation](https://docs.rustops.dev) for installation instructions.
          files: |
            rustops-*
          draft: false
          prerelease: false
          generate_release_notes: true

  deploy-staging:
    name: Deploy to Staging
    needs: [publish-containers]
    runs-on: ubuntu-latest
    timeout-minutes: 20
    environment:
      name: staging
      url: https://staging.rustops.internal
    steps:
      - uses: actions/checkout@v4

      - name: Configure kubectl
        uses: azure/k8s-set-context@v4
        with:
          method: kubeconfig
          kubeconfig: ${{ secrets.KUBE_CONFIG_STAGING }}

      - name: Deploy to staging (25%)
        run: |
          kubectl set image deployment/rustops-api \
            rustops-api=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}/api:${{ needs.validate.outputs.version }} \
            -n rustops-staging

          kubectl rollout status deployment/rustops-api -n rustops-staging --timeout=5m

      - name: Health checks
        run: |
          sleep 30
          kubectl run smoke-test --rm -i --restart=Never --image=curlimages/curl \
            -n rustops-staging -- curl -f http://rustops-api:8080/health || exit 1
```

### 4. Dependency Update Workflow

```yaml
# .github/workflows/dependencies.yml
name: Dependencies

on:
  schedule:
    - cron: '0 6 * * 1'  # Every Monday at 6 AM
  workflow_dispatch:

permissions:
  contents: write
  pull-requests: write

jobs:
  update-dependencies:
    name: Update Dependencies
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Update cargo dependencies
        run: cargo update

      - name: Update npm dependencies (web)
        run: |
          cd web
          npm update

      - name: Check for changes
        id: changes
        run: |
          if git diff --quiet Cargo.lock; then
            echo "changed=false" >> $GITHUB_OUTPUT
          else
            echo "changed=true" >> $GITHUB_OUTPUT
          fi

      - name: Run tests
        if: steps.changes.outputs.changed == 'true'
        run: cargo test --workspace --lib

      - name: Create PR
        if: steps.changes.outputs.changed == 'true'
        uses: peter-evans/create-pull-request@v6
        with:
          title: 'chore: Update dependencies'
          body: |
            Automated dependency update
          branch: deps/update-dependencies
          commit-message: 'chore: update dependencies'
          labels: dependencies
```

## Status Badges

```markdown
# RustOps

[![CI/CD](https://github.com/rustops/rustops/actions/workflows/pr.yml/badge.svg)](https://github.com/rustops/rustops/actions/workflows/pr.yml)
[![codecov](https://codecov.io/gh/rustops/rustops/branch/main/graph/badge.svg)](https://codecov.io/gh/rustops/rustops)
[![Security Audit](https://github.com/rustops/rustops/actions/workflows/security.yml/badge.svg)](https://github.com/rustops/rustops/actions/workflows/security.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
```
