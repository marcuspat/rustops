# RustOps Makefile

.PHONY: help build test clean lint fmt docs dev-setup test-integration test-e2e test-property test-bench test-coverage docker-test-up docker-test-down docker-build-all docker-compose-up docker-compose-down docker-compose-logs

# Default target
.DEFAULT_GOAL := help

# Variables
CARGO := cargo
PROJECT_NAME := rustops

# Colors
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[0;33m
RED := \033[0;31m
NC := \033[0m

## help: Show this help message
help:
	@echo '$(PROJECT_NAME) - Build Commands'
	@echo ''
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(GREEN)%-25s$(NC) %s\n", $$1, $$2}' $(MAKEFILE_LIST)

## dev-setup: Set up development environment
dev-setup:
	@echo "$(BLUE)Setting up development environment...$(NC)"
	@echo "$(GREEN)Installing required tools...$(NC)"
	cargo install cargo-watch cargo-nextest cargo-tarpaulin
	@echo "$(GREEN)Done! Run 'make test' to verify setup.$(NC)"

## build: Build all workspace crates
build:
	@echo "$(BLUE)Building workspace...$(NC)"
	$(CARGO) build --workspace

## build-release: Build optimized release binaries
build-release:
	@echo "$(BLUE)Building release binaries...$(NC)"
	$(CARGO) build --workspace --release

# ============================================================================
# Testing Commands
# ============================================================================

## test: Run all tests (unit + integration + property)
test: test-unit test-integration test-property
	@echo "$(GREEN)All tests passed!$(NC)"

## test-unit: Run unit tests only
test-unit:
	@echo "$(BLUE)Running unit tests...$(NC)"
	$(CARGO) test --workspace --lib

## test-integration: Run integration tests
test-integration:
	@echo "$(BLUE)Running integration tests...$(NC)"
	$(CARGO) test --workspace --test '*'

## test-e2e: Run end-to-end tests (requires Docker)
test-e2e: docker-test-up
	@echo "$(BLUE)Running E2E tests...$(NC)"
	$(CARGO) test --workspace --test e2e
	@$(MAKE) docker-test-down

## test-property: Run property-based tests
test-property:
	@echo "$(BLUE)Running property-based tests...$(NC)"
	$(CARGO) test --workspace --features test-utils

## test-bench: Run benchmarks
test-bench:
	@echo "$(BLUE)Running benchmarks...$(NC)"
	$(CARGO) bench --workspace

## test-coverage: Generate coverage report with tarpaulin
test-coverage:
	@echo "$(BLUE)Generating coverage report...$(NC)"
	cargo tarpaulin --workspace \
		--exclude-files "*/tests/*" \
		--exclude-files "*/benches/*" \
		--exclude-files "*/examples/*" \
		--timeout 300 \
		--out Lcov \
		--output-dir ./coverage \
		-- --test-threads=1
	@echo "$(GREEN)Coverage report generated in ./coverage$(NC)"

## test-watch: Watch mode for tests (requires cargo-watch)
test-watch:
	@echo "$(BLUE)Watching for changes...$(NC)"
	cargo watch -x 'test --workspace'

## test-nextest: Run tests with nextest (faster test runner)
test-nextest:
	@echo "$(BLUE)Running tests with nextest...$(NC)"
	cargo nextest run --workspace

## test-verbose: Run tests with verbose output
test-verbose:
	@echo "$(BLUE)Running tests with verbose output...$(NC)"
	$(CARGO) test --workspace -- --nocapture --test-threads=1

# ============================================================================
# Docker Test Environment
# ============================================================================

## docker-test-up: Start test environment with docker-compose
docker-test-up:
	@echo "$(BLUE)Starting test environment...$(NC)"
	docker-compose -f tests/docker-compose.yml up -d
	@echo "$(YELLOW)Waiting for services to be healthy...$(NC)"
	@timeout 120 bash -c 'until docker-compose -f tests/docker-compose.yml ps | grep -q "healthy"; do sleep 2; done'
	@echo "$(GREEN)Test environment ready!$(NC)"

## docker-test-down: Stop test environment
docker-test-down:
	@echo "$(BLUE)Stopping test environment...$(NC)"
	docker-compose -f tests/docker-compose.yml down -v

## docker-test-logs: Show test environment logs
docker-test-logs:
	docker-compose -f tests/docker-compose.yml logs -f

# ============================================================================
# Docker Deployment
# ============================================================================

## docker-build-all: Build all Docker images (api, agent, pipeline)
docker-build-all:
	@echo "$(BLUE)Building all Docker images...$(NC)"
	@echo "$(GREEN)Building rustops-api...$(NC)"
	docker build -t rustops-api:latest -f docker/Dockerfile.api .
	@echo "$(GREEN)Building rustops-agent...$(NC)"
	docker build -t rustops-agent:latest -f docker/Dockerfile.agent .
	@echo "$(GREEN)Building rustops-pipeline...$(NC)"
	docker build -t rustops-pipeline:latest -f docker/Dockerfile.pipeline .
	@echo "$(GREEN)All Docker images built successfully!$(NC)"

## docker-compose-up: Start all services with docker-compose
docker-compose-up:
	@echo "$(BLUE)Starting all services...$(NC)"
	docker-compose up -d
	@echo "$(YELLOW)Waiting for services to be healthy...$(NC)"
	@timeout 180 bash -c 'until docker-compose ps | grep -q "healthy\|Up"; do sleep 2; done'
	@echo "$(GREEN)All services started!$(NC)"
	@echo "$(YELLOW)Run 'make docker-compose-logs' to view logs$(NC)"
	@echo "$(YELLOW)Run 'make docker-compose-down' to stop services$(NC)"

## docker-compose-down: Stop all services
docker-compose-down:
	@echo "$(BLUE)Stopping all services...$(NC)"
	docker-compose down -v
	@echo "$(GREEN)All services stopped!$(NC)"

## docker-compose-logs: Show logs from all services
docker-compose-logs:
	docker-compose logs -f

## docker-compose-ps: Show status of all services
docker-compose-ps:
	docker-compose ps

# ============================================================================
# Quality Gates
# ============================================================================

## lint: Run all linters
lint: lint-clippy lint-fmt lint-docs
	@echo "$(GREEN)Linting passed!$(NC)"

## lint-clippy: Run clippy linter
lint-clippy:
	@echo "$(BLUE)Running clippy...$(NC)"
	$(CARGO) clippy --workspace --all-targets -- -D warnings -W clippy::pedantic

## lint-fmt: Check code formatting
lint-fmt:
	@echo "$(BLUE)Checking code formatting...$(NC)"
	$(CARGO) fmt --all -- --check

## lint-docs: Check documentation
lint-docs:
	@echo "$(BLUE)Checking documentation...$(NC)"
	$(CARGO) doc --workspace --no-deps
	@echo "$(GREEN)Documentation check passed!$(NC)"

## security-audit: Run security audit on dependencies
security-audit:
	@echo "$(BLUE)Running security audit...$(NC)"
	cargo install cargo-audit
	cargo audit

## security-deny: Check licenses and banned dependencies
security-deny:
	@echo "$(BLUE)Checking licenses and dependencies...$(NC)"
	cargo install cargo-deny
	cargo deny check

## fmt: Format code
fmt:
	@echo "$(BLUE)Formatting code...$(NC)"
	$(CARGO) fmt --all

## docs: Build and open documentation
docs:
	@echo "$(BLUE)Building documentation...$(NC)"
	$(CARGO) doc --workspace --no-deps --open

## docs-internal: Build documentation with private items
docs-internal:
	@echo "$(BLUE)Building internal documentation...$(NC)"
	$(CARGO) doc --workspace --document-private-items

# ============================================================================
# Build Cleanup
# ============================================================================

## clean: Clean build artifacts
clean:
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	$(CARGO) clean

## clean-coverage: Clean coverage reports
clean-coverage:
	@echo "$(BLUE)Cleaning coverage reports...$(NC)"
	rm -rf ./coverage

## clean-all: Clean everything including docker volumes
clean-all: clean clean-coverage docker-test-down
	@echo "$(BLUE)Cleaning all artifacts...$(NC)"

# ============================================================================
# CI Commands
# ============================================================================

## ci: Run full CI checks locally
ci: lint test-unit test-integration test-property
	@echo "$(GREEN)CI checks passed!$(NC)"

## ci-full: Run full CI with coverage
ci-full: lint test test-coverage
	@echo "$(GREEN)Full CI checks passed!$(NC)"

## ci-quick: Run quick CI checks (unit tests + lint)
ci-quick: lint-clippy lint-fmt test-unit
	@echo "$(GREEN)Quick CI checks passed!$(NC)"

# ============================================================================
# Development Utilities
# ============================================================================

## update-deps: Update dependencies
update-deps:
	@echo "$(BLUE)Updating dependencies...$(NC)"
	$(CARGO) update

## check-deps: Check for dependency updates
check-deps:
	@echo "$(BLUE)Checking for dependency updates...$(NC)"
	cargo install cargo-outdated
	cargo outdated

## tree: Visualize dependency tree
tree:
	@echo "$(BLUE)Dependency tree:$(NC)"
	cargo install cargo-tree
	cargo tree

## size: Analyze binary size
size:
	@echo "$(BLUE)Analyzing binary size...$(NC)"
	cargo install cargo-size
	cargo size --workspace

## flamethrower: Generate flamegraph (requires flamegraph)
flamethrower:
	@echo "$(BLUE)Generating flamegraph...$(NC)"
	cargo install flamegraph
	cargo flamegraph --bench metric_bench

## expand-macros: Show macro expansion (for debugging)
expand-macros:
	@echo "$(BLUE)Expanding macros...$(NC)"
	$(CARGO) rustc --workspace -- -Z unstable-options --pretty=expanded
