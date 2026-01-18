# RustOps Makefile

.PHONY: help build test clean lint fmt docs dev-setup

# Default target
.DEFAULT_GOAL := help

# Variables
CARGO := cargo
PROJECT_NAME := rustops

# Colors
BLUE := \033[0;34m
GREEN := \033[0;32m
NC := \033[0m

## help: Show this help message
help:
	@echo '$(PROJECT_NAME) - Build Commands'
	@echo ''
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}' $(MAKEFILE_LIST)

## dev-setup: Set up development environment
dev-setup:
	@echo "$(BLUE)Setting up development environment...$(NC)"
	@echo "$(GREEN)Run: cargo install cargo-watch cargo-nextest cargo-tarpaulin$(NC)"

## build: Build all workspace crates
build:
	@echo "$(BLUE)Building workspace...$(NC)"
	$(CARGO) build --workspace

## build-release: Build optimized release binaries
build-release:
	@echo "$(BLUE)Building release binaries...$(NC)"
	$(CARGO) build --workspace --release

## test: Run all tests
test:
	@echo "$(BLUE)Running tests...$(NC)"
	$(CARGO) test --workspace

## test-unit: Run unit tests only
test-unit:
	@echo "$(BLUE)Running unit tests...$(NC)"
	$(CARGO) test --workspace --lib

## test-coverage: Generate coverage report
test-coverage:
	@echo "$(BLUE)Generating coverage report...$(NC)"
	@echo "Install: cargo install cargo-tarpaulin"

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

## clean: Clean build artifacts
clean:
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	$(CARGO) clean

## ci: Run CI checks locally
ci: lint test-unit
	@echo "$(GREEN)CI checks passed!$(NC)"
