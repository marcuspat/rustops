# Build Configuration

Makefile, Justfile, and build scripts for the RustOps platform.

## Makefile

```makefile
# RustOps Makefile
# Primary build automation file

.PHONY: help build test clean lint fmt docs install dev-setup

# Default target
.DEFAULT_GOAL := help

# Variables
CARGO := cargo
RUSTUP := rustup
DOCKER := docker
KUBECTL := kubectl
PROJECT_NAME := rustops
WORKSPACE := .

# Colors for output
BLUE := \033[0;34m
GREEN := \033[0;32m
RED := \033[0;31m
NC := \033[0m # No Color

## help: Show this help message
help:
	@echo '$(PROJECT_NAME) - Build Commands'
	@echo ''
	@echo 'Usage:'
	@echo '  make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}' $(MAKEFILE_LIST)

## dev-setup: Set up development environment
dev-setup: install-rust install-tools install-hooks
	@echo "$(BLUE)Setting up development environment...$(NC)"
	@echo "$(GREEN)Development environment ready!$(NC)"

## install-rust: Install Rust toolchain
install-rust:
	@echo "$(BLUE)Installing Rust toolchain...$(NC)"
	@if ! command -v rustup &> /dev/null; then \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable; \
	fi
	$(RUSTUP) toolchain install stable beta nightly
	$(RUSTUP) component add rust-analyzer clippy rustfmt

## install-tools: Install development tools
install-tools:
	@echo "$(BLUE)Installing development tools...$(NC)"
	$(CARGO) install cargo-watch
	$(CARGO) install cargo-nextest
	$(CARGO) install cargo-tarpaulin
	$(CARGO) install cargo-audit
	$(CARGO) install cargo-deny
	$(CARGO) install cargo-binstall
	$(CARGO) install flamegraph
	$(CARGO) install cargo-release
	npm install -g @mermaid-js/mermaid-cli

## install-hooks: Install git hooks
install-hooks:
	@echo "$(BLUE)Installing git hooks...$(NC)"
	@chmod +x scripts/hooks/*
	@ln -sf $$(pwd)/scripts/hooks/pre-commit .git/hooks/pre-commit || true
	@ln -sf $$(pwd)/scripts/hooks/pre-push .git/hooks/pre-push || true

## build: Build all workspace crates
build:
	@echo "$(BLUE)Building workspace...$(NC)"
	$(CARGO) build --workspace

## build-release: Build optimized release binaries
build-release:
	@echo "$(BLUE)Building release binaries...$(NC)"
	$(CARGO) build --workspace --release

## build-all: Build for all targets
build-all:
	@echo "$(BLUE)Building for all targets...$(NC)"
	$(CARGO) build --workspace --all-targets

## test: Run all tests
test:
	@echo "$(BLUE)Running tests...$(NC)"
	$(CARGO) test --workspace

## test-unit: Run unit tests only
test-unit:
	@echo "$(BLUE)Running unit tests...$(NC)"
	$(CARGO) test --workspace --lib

## test-integration: Run integration tests
test-integration:
	@echo "$(BLUE)Running integration tests...$(NC)"
	$(CARGO) test --workspace --test '*'

## test-nextest: Run tests with nextest
test-nextest:
	@echo "$(BLUE)Running tests with nextest...$(NC)"
	cargo nextest run --workspace

## test-coverage: Generate coverage report
test-coverage:
	@echo "$(BLUE)Generating coverage report...$(NC)"
	cargo tarpaulin --workspace \
		--exclude-files "*/tests/*" \
		--exclude-files "*/benches/*" \
		--timeout 300 \
		--out Lcov \
		--output-dir ./coverage \
		-- --test-threads=1

## lint: Run linters
lint: lint-clippy lint-fmt
	@echo "$(GREEN)Linting passed!$(NC)"

## lint-clippy: Run clippy
lint-clippy:
	@echo "$(BLUE)Running clippy...$(NC)"
	$(CARGO) clippy --workspace --all-targets -- -D warnings

## lint-fmt: Check code formatting
lint-fmt:
	@echo "$(BLUE)Checking code formatting...$(NC)"
	$(CARGO) fmt --all -- --check

## fmt: Format code
fmt:
	@echo "$(BLUE)Formatting code...$(NC)"
	$(CARGO) fmt --all

## docs: Build documentation
docs:
	@echo "$(BLUE)Building documentation...$(NC)"
	$(CARGO) doc --workspace --no-deps

## docs-open: Build and open documentation
docs-open:
	@echo "$(BLUE)Building and opening documentation...$(NC)"
	$(CARGO) doc --workspace --no-deps --open

## audit: Run security audit
audit:
	@echo "$(BLUE)Running security audit...$(NC)"
	$(CARGO) audit

## deny: Run cargo-deny checks
deny:
	@echo "$(BLUE)Running cargo-deny checks...$(NC)"
	$(CARGO) deny check all

## clean: Clean build artifacts
clean:
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	$(CARGO) clean
	@rm -rf coverage/
	@rm -rf target/criterion/

## bench: Run benchmarks
bench:
	@echo "$(BLUE)Running benchmarks...$(NC)"
	$(CARGO) bench --workspace

## flamegraph: Generate flamegraph
flamegraph:
	@echo "$(BLUE)Generating flamegraph...$(NC)"
	$(CARGO) flamegraph --bin rustops-agent

## dev: Start development environment
dev:
	@echo "$(BLUE)Starting development environment...$(NC)"
	$(DOCKER) compose -f tests/docker-compose.yml up -d
	@echo "$(GREEN)Development environment started!$(NC)"

## dev-stop: Stop development environment
dev-stop:
	@echo "$(BLUE)Stopping development environment...$(NC)"
	$(DOCKER) compose -f tests/docker-compose.yml down

## dev-logs: Show development logs
dev-logs:
	$(DOCKER) compose -f tests/docker-compose.yml logs -f

## docker-build: Build Docker images
docker-build:
	@echo "$(BLUE)Building Docker images...$(NC)"
	$(DOCKER) build -t $(PROJECT_NAME)/agent:dev -f deploy/docker/agent.Dockerfile .
	$(DOCKER) build -t $(PROJECT_NAME)/api:dev -f deploy/docker/api.Dockerfile .

## docker-push: Push Docker images
docker-push:
	@echo "$(BLUE)Pushing Docker images...$(NC)"
	$(DOCKER) push $(PROJECT_NAME)/agent:dev
	$(DOCKER) push $(PROJECT_NAME)/api:dev

## k8s-deploy-dev: Deploy to dev cluster
k8s-deploy-dev:
	@echo "$(BLUE)Deploying to dev cluster...$(NC)"
	$(KUBECTL) apply -f deploy/kubernetes/base/

## k8s-logs: Show Kubernetes logs
k8s-logs:
	$(KUBECTL) logs -n rustops-dev deployment/rustops-api -f

## ci: Run CI checks locally
ci: lint test-unit test-integration audit deny
	@echo "$(GREEN)CI checks passed!$(NC)"

## release: Prepare release
release:
	@echo "$(BLUE)Preparing release...$(NC)"
	$(CARGO) release --dry-run

## update-deps: Update dependencies
update-deps:
	@echo "$(BLUE)Updating dependencies...$(NC)"
	$(CARGO) update
	cd web && npm update

## check: Quick check (format + clippy)
check:
	$(CARGO) fmt --all -- --check
	$(CARGO) clippy --workspace --all-targets -- -D warnings

## watch: Watch for changes and rebuild
watch:
	@echo "$(BLUE)Watching for changes...$(NC)"
	$(CARGO) watch -x 'check --workspace'

## test-watch: Watch and run tests
test-watch:
	@echo "$(BLUE)Watching and running tests...$(NC)"
	$(CARGO) watch -x 'test --workspace'
```

## Justfile

```toml
# Justfile for RustOps
# Alternative to Makefile with better UX

# Variables
cargo := "cargo"
docker := "docker"
kubectl := "kubectl"
project := "rustops"

# Default recipe
default:
    @just --list

# Development setup
dev-setup:
    @echo "Setting up development environment..."
    install-rust
    install-tools
    install-hooks

# Install Rust toolchain
install-rust:
    #!/usr/bin/env bash
    if ! command -v rustup &> /dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    fi
    rustup toolchain install stable beta nightly
    rustup component add rust-analyzer clippy rustfmt

# Install development tools
install-tools:
    cargo install cargo-watch cargo-nextest cargo-tarpaulin
    cargo install cargo-audit cargo-deny cargo-binstall

# Install git hooks
install-hooks:
    @chmod +x scripts/hooks/*
    @ln -sf $PWD/scripts/hooks/pre-commit .git/hooks/pre-commit || true
    @ln -sf $PWD/scripts/hooks/pre-push .git/hooks/pre-push || true

# Build workspace
build:
    cargo build --workspace

# Build release
build-release:
    cargo build --workspace --release

# Run all tests
test:
    cargo test --workspace

# Run unit tests
test-unit:
    cargo test --workspace --lib

# Run integration tests
test-integration *args:
    docker compose -f tests/docker-compose.yml up -d
    cargo test --workspace --test '*'
    docker compose -f tests/docker-compose.yml down

# Run tests with nextest
test-nextest:
    cargo nextest run --workspace

# Generate coverage
test-coverage:
    cargo tarpaulin --workspace \
        --exclude-files "*/tests/*" \
        --exclude-files "*/benches/*" \
        --timeout 300 \
        --out Lcov \
        --output-dir ./coverage

# Lint code
lint: lint-clippy lint-fmt

# Run clippy
lint-clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Check formatting
lint-fmt:
    cargo fmt --all -- --check

# Format code
fmt:
    cargo fmt --all

# Build documentation
docs:
    cargo doc --workspace --no-deps

# Open documentation
docs-open:
    cargo doc --workspace --no-deps --open

# Security audit
audit:
    cargo audit

# Run cargo-deny
deny:
    cargo deny check all

# Clean build artifacts
clean:
    cargo clean
    rm -rf coverage/ target/criterion/

# Run benchmarks
bench:
    cargo bench --workspace

# Start dev environment
dev:
    docker compose -f tests/docker-compose.yml up -d

# Stop dev environment
dev-stop:
    docker compose -f tests/docker-compose.yml down

# Build Docker images
docker-build:
    docker build -t {{project}}/agent:dev -f deploy/docker/agent.Dockerfile .
    docker build -t {{project}}/api:dev -f deploy/docker/api.Dockerfile .

# Deploy to Kubernetes
k8s-deploy:
    kubectl apply -f deploy/kubernetes/base/

# Run CI locally
ci: lint test-unit audit deny

# Update dependencies
update-deps:
    cargo update
    cd web && npm update

# Watch mode
watch:
    cargo watch -x 'check --workspace'

# Test watch mode
test-watch:
    cargo watch -x 'test --workspace'
```

## Build Scripts

### Pre-commit Hook

```bash
#!/bin/bash
# scripts/hooks/pre-commit

set -e

echo "Running pre-commit checks..."

# Check formatting
echo "Checking formatting..."
cargo fmt --all -- --check

# Run clippy
echo "Running clippy..."
cargo clippy --workspace --all-targets -- -D warnings

# Run unit tests
echo "Running unit tests..."
cargo test --workspace --lib --quiet

# Check documentation
echo "Checking documentation..."
cargo doc --workspace --no-deps --quiet

echo "Pre-commit checks passed!"
```

### Pre-push Hook

```bash
#!/bin/bash
# scripts/hooks/pre-push

set -e

echo "Running pre-push checks..."

# Run all tests
echo "Running tests..."
cargo test --workspace

# Run security audit
echo "Running security audit..."
cargo audit

# Check licenses
echo "Checking licenses..."
cargo deny check licenses

echo "Pre-push checks passed!"
```

### Release Script

```bash
#!/bin/bash
# scripts/release.sh

set -e

VERSION=$1

if [ -z "$VERSION" ]; then
    echo "Usage: ./release.sh <version>"
    exit 1
fi

echo "Preparing release $VERSION..."

# Update version in Cargo.toml
echo "Updating version..."
sed -i "s/^version = .*/version = \"$VERSION\"/" Cargo.toml

# Generate changelog
echo "Generating changelog..."
git log $(git describe --tags --abbrev=0 HEAD^)..HEAD --oneline > CHANGELOG.md.new

# Commit changes
echo "Committing changes..."
git add Cargo.toml CHANGELOG.md
git commit -m "chore: release $VERSION"

# Create tag
echo "Creating tag..."
git tag -a "v$VERSION" -m "Release $VERSION"

# Push
echo "Pushing..."
git push origin main
git push origin "v$VERSION"

echo "Release $VERSION prepared!"
```

### Docker Build Script

```bash
#!/bin/bash
# scripts/docker-build.sh

set -e

REGISTRY=${REGISTRY:-ghcr.io/rustops}
VERSION=${VERSION:-$(git describe --tags --always --dirty)}
PLATFORMS=${PLATFORMS:-linux/amd64,linux/arm64}

echo "Building Docker images for $REGISTRY:$VERSION"

# Build multi-platform images
for component in agent pipeline anomaly correlation remediation topology api; do
    echo "Building $component..."
    docker buildx build \
        --platform "$PLATFORMS" \
        -t "$REGISTRY/$component:$VERSION" \
        -t "$REGISTRY/$component:latest" \
        -f "deploy/docker/$component.Dockerfile" \
        --push \
        .
done

echo "Docker images built and pushed!"
```

### Coverage Script

```bash
#!/bin/bash
# scripts/coverage.sh

set -e

echo "Generating coverage report..."

# Run tarpaulin
cargo tarpaulin --workspace \
    --exclude-files "*/tests/*" \
    --exclude-files "*/benches/*" \
    --exclude-files "*/examples/*" \
    --timeout 300 \
    --out Html \
    --out Lcov \
    --output-dir ./coverage \
    -- --test-threads=1

# Check coverage threshold
COVERAGE=$(cargo tarpaulin --workspace --exclude-files "*/tests/*" --output-dir ./coverage 2>&1 | grep "Overall" | awk '{print $2}' | sed 's/%//')

echo "Coverage: $COVERAGE%"

if (( $(echo "$COVERAGE < 80" | bc -l) )); then
    echo "ERROR: Coverage below 80%"
    exit 1
fi

echo "Coverage check passed!"

# Open in browser (if available)
if command -v xdg-open &> /dev/null; then
    xdg-open coverage/index.html
fi
```

## Configuration Files

### clippy.toml

```toml
# clippy.toml
array-local-expr-threshold = 512000
array-size-threshold = 512000
vec-box-size-threshold = 512000
stack-size-threshold = 512000
enum-variant-name-threshold = 3
enum-variant-size-threshold = 200
literal-representation-threshold = 10

blacklisted-names = ["foo", "bar", "baz", "quux"]
doc-valid-idents = ["RustOps", "AIOps", "SLO", "MTTR", "MTTD", "Kubernetes"]

too-many-arguments-threshold = 7
type-complexity-threshold = 250
```

### rustfmt.toml

```toml
# rustfmt.toml
edition = "2024"
max_width = 100
hard_tabs = false
tab_spaces = 4
indent_style = "Block"
fn_single_line = false
where_single_line = false

imports_indent = "Block"
imports_layout = "Mixed"
group_imports = "StdExternalCrate"
reorder_imports = true
reorder_modules = true

comment_width = 100
wrap_comments = true
normalize_comments = true
normalize_doc_attributes = true

format_code_in_doc_comments = true
format_strings = true
format_macro_matchers = true
format_macro_bodies = true

use_field_init_shorthand = true
use_try_shorthand = true
use_small_heuristics = "Default"
merge_derives = true
overflow_delimited_expr = true

blank_lines_upper_bound = 1
blank_lines_lower_bound = 0
remove_nested_parens = true
```

### deny.toml

```toml
# deny.toml
[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/RustSec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"
notice = "warn"
ignore = []

[licenses]
unlicensed = "deny"
allow-osi-fsf-free = "both"
copyleft = "warn"
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
]
exceptions = [
    { allow = ["OpenSSL"], name = "ring" },
]

[bans]
multiple-versions = "warn"
wildcards = "allow"
highlight = "all"
workspace-duplicates = "warn"
deny = [
    { name = "openssl-sys", use-instead = "rustls" },
    { name = "native-tls", use-instead = "rustls" },
]

[sources]
unknown-registry = "warn"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

### .cargo/config.toml

```toml
# .cargo/config.toml
[build]
target-dir = "target"

[target.x86_64-unknown-linux-gnu]
rustflags = [
    "-C", "link-arg=-fuse-ld=lld",
    "-C", "link-arg=-Wl,--gc-sections",
    "-C", "codegen-units=1",
]

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"

[profile.release-debug]
inherits = "release"
strip = false
debug = true

[profile.dev]
opt-level = 0

[profile.bench]
inherits = "release"
debug = true

[env]
RUSTFLAGS = "-C target-cpu=native"
RUST_LOG = "info"
```

### .dockerignore

```
# .dockerignore
target/
**/*.rs.bk
.DS_Store
.git/
.gitignore
.github/
.vscode/
*.md
!README.md
tests/
benches/
examples/
*.svg
 diagrams/
coverage/
*.log
.env.*
!.env.example
```

## Common Commands Quick Reference

```bash
# Development
make dev-setup              # Initial setup
make dev                    # Start dev environment
make watch                  # Watch and rebuild
make test-watch             # Watch and test

# Building
make build                  # Debug build
make build-release          # Release build
make docker-build           # Build containers

# Testing
make test                   # All tests
make test-unit              # Unit tests
make test-integration       # Integration tests
make test-coverage          # Coverage report

# Quality
make lint                   # All linters
make fmt                    # Format code
make audit                  # Security audit

# Documentation
make docs                   # Build docs
make docs-open              # Open docs

# CI/CD
make ci                     # Run CI locally
make release                # Prepare release
```
