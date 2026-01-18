# Build Requirements

## Rust Toolchain

The project requires **Rust 1.74 or newer**.

### Install/Update Rust

```bash
# Using rustup
rustup update stable
rustup default stable

# Or install specific version
rustup install 1.74
rustup default 1.74
```

## Verify Installation

```bash
rustc --version
cargo --version
```

Expected output:
- rustc 1.74.0 or newer
- cargo 1.74.0 or newer

## Build the Project

```bash
# Check all crates
cargo check --workspace

# Build release version
cargo build --release --workspace

# Run tests
cargo test --workspace

# Run tests with coverage
cargo tarpaulin --workspace --out Html
```

## Feature Flags

### Remediation Crate
- `default` - Enables Kubernetes support
- `kubernetes` - Kubernetes activity executors
- `aws` - AWS cloud provider support
- `full` - All features

### Topology Crate
- `default` - Enables Kubernetes and validation
- `kubernetes` - Kubernetes service discovery
- `validation` - Input validation
- `full` - All features

## Dependencies

The project has the following major dependencies:

- **tokio** 1.43+ - Async runtime
- **kube** 0.97+ - Kubernetes client
- **neo4rs** 0.7+ - Neo4j graph database client
- **serde/serde_json** - Serialization
- **tracing** - Structured logging
- **thiserror/anyhow** - Error handling

## Known Issues

### Rust 1.70 Compatibility

Rust 1.70 is **not supported** due to dependencies requiring newer versions. Please upgrade to Rust 1.74+.

### Neo4j Connection

The topology crate requires a running Neo4j instance for graph operations. For local development:

```bash
# Using Docker
docker run -p 7474:7474 -p 7687:7687 \
  -e NEO4J_AUTH=neo4j/password \
  neo4j:5.15
```

### Kubernetes Client

The remediation crate requires a Kubernetes cluster. For local development:

```bash
# Using kind
kind create cluster --name rustops-dev

# Using minikube
minikube start
```
