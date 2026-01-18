# Cargo Workspace Configuration

Complete Cargo workspace setup for the RustOps monorepo.

## Root Cargo.toml

```toml
[workspace]
members = [
    "crates/agent",
    "crates/pipeline",
    "crates/anomaly",
    "crates/correlation",
    "crates/remediation",
    "crates/topology",
    "crates/api",
    "crates/common",
    "integrations/prometheus",
    "integrations/datadog",
    "integrations/cloudwatch",
    "integrations/kubernetes",
    "integrations/servicenow",
    "integrations/pagerduty",
    "integrations/slack",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
authors = ["RustOps Team <platform@rustops.dev>"]
license = "Apache-2.0"
repository = "https://github.com/rustops/rustops"
homepage = "https://rustops.dev"
keywords = ["aiops", "monitoring", "observability", "devops"]
categories = ["development-tools", "api-bindings"]

[workspace.dependencies]
# Async Runtime
tokio = { version = "1.43", features = ["full"] }
async-trait = "0.1.86"
futures = "0.3.31"

# Telemetry
tracing = "0.1.41"
tracing-subscriber = { version = "0.3.19", features = ["env-filter", "json"] }
tracing-opentelemetry = "0.28"
opentelemetry = "0.27"
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
opentelemetry-otlp = "0.27"

# Metrics & Monitoring
prometheus = { version = "0.13.4", features = ["process"] }
metrics = "0.24"
metrics-exporter-prometheus = "0.16"

# Serialization
serde = { version = "1.0.217", features = ["derive"] }
serde_json = "1.0.134"
bincode = "1.3.3"

# Configuration
config = "0.15.4"
toml = "0.8.19"

# Error Handling
anyhow = "1.0.96"
thiserror = "2.0.11"
color-eyre = "0.6.3"

# Networking
axum = { version = "0.8.1", features = ["macros", "ws"] }
tower = { version = "0.5.2", features = ["full"] }
tower-http = { version = "0.6.2", features = ["trace", "cors", "compression"] }
hyper = { version = "1.6", features = ["full"] }
reqwest = { version = "0.12.12", features = ["json", "rustls-tls"] }
tonic = "0.12.3"

# Data Processing
rdkafka = { version = "0.37.0", features = ["ssl", "sasl", "dynamic-linking"] }
redis = { version = "0.27.6", features = ["tokio-comp", "connection-manager"] }
clickhouse = { version = "0.12.3", features = ["tokio", "rustls-tls"] }

# ML & Analytics
ort = { version = "2.0.0-rc9", features = ["cuda", "load-dynamic"] }
ndarray = "0.16.1"
ndarray-stats = "0.6"
statrs = "0.17.1"
smartcore = "0.4.0"

# Kubernetes
kube = { version = "0.98.0", features = ["runtime", "client", "ws"] }
k8s-openapi = { version = "0.23.0", features = ["v1_30"] }

# Utilities
uuid = { version = "1.12", features = ["v4", "serde"] }
chrono = { version = "0.4.39", features = ["serde"] }
regex = "1.11.1"
once_cell = "1.20.2"
parking_lot = "0.12.3"
itertools = "0.14.0"

# Testing
proptest = "1.6.0"
criterion = { version = "0.5.1", features = ["html_reports"] }
quickcheck = "1.0.3"
mockall = "0.13.1"
wiremock = "0.6.2"

# Dev Tools
cargo-audit = "0.21.0"
cargo-deny = "0.16.0"
cargo-nextest = "0.9.78"

[workspace.metadata.dylint]
libraries = [{ path = "crates/linting" }]

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

[profile.dev-max-opt]
inherits = "dev"
opt-level = 3

[profile.bench]
inherits = "release"
debug = true

# Optimizations for specific crates
[profile.release.package.agent]
opt-level = "z"  # Optimize for size
lto = "thin"

[profile.release.package.anomaly]
opt-level = 3
codegen-units = 16  # Better compile time for ML
```

## Feature Flags

### Common Features (defined in `crates/common/Cargo.toml`)

```toml
[features]
default = ["kubernetes", "prometheus"]

# Cloud providers
aws = ["dep:aws-config", "dep:aws-sdk-ec2", "dep:aws-sdk-cloudwatch"]
azure = ["dep:azure_identity", "dep:azure_mgmt_compute"]
gcp = ["dep:google-cloud-auth", "dep:google-cloud-pubsub"]

# Monitoring integrations
prometheus = ["dep:prometheus"]
datadog = ["dep:datadog-api"]
cloudwatch = ["aws"]

# Container orchestration
kubernetes = ["dep:kube", "dep:k8s-openapi"]
ecs = ["aws"]
aks = ["azure"]
gke = ["gcp"]

# ML features
ml-onnx = ["dep:ort"]
ml-cuda = ["ort?/cuda"]
ml-tensorrt = ["ort?/tensorrt"]

# Storage
redis = ["dep:redis"]
clickhouse = ["dep:clickhouse"]
postgres = ["dep:tokio-postgres", "dep:deadpool-postgres"]

# API
rest = ["dep:axum"]
graphql = ["dep:async-graphql", "dep:async-graphql-axum"]
grpc = ["dep:tonic"]

# Testing
test-utils = ["dep:proptest", "dep:mockall", "dep:wiremock"]
integration-test = []

# Development
dev = ["test-utils"]
```

## Crate Dependencies Examples

### Agent Crate (`crates/agent/Cargo.toml`)

```toml
[package]
name = "rustops-agent"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
description = "Telemetry collection agent for RustOps"

[dependencies]
rustops-common = { path = "../common", features = ["kubernetes", "prometheus"] }
tokio = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
serde = { workspace = true }
prometheus = { workspace = true }
kube = { workspace = true, optional = true }

[dev-dependencies]
proptest = { workspace = true }
mockall = { workspace = true }

[features]
default = ["kubernetes"]
kubernetes = ["rustops-common/kubernetes", "kube"]
cloudwatch = ["rustops-common/aws"]
```

### Anomaly Detection Crate (`crates/anomaly/Cargo.toml`)

```toml
[package]
name = "rustops-anomaly"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
description = "ML-based anomaly detection for RustOps"

[dependencies]
rustops-common = { path = "../common" }
tokio = { workspace = true }
tracing = { workspace = true }
anyhow = { workspace = true }
ort = { workspace = true }
ndarray = { workspace = true }
statrs = { workspace = true }
serde = { workspace = true }

[dev-dependencies]
criterion = { workspace = true }
proptest = { workspace = true }

[[bench]]
name = "detection"
harness = false

[features]
default = ["onnx"]
onnx = ["ort"]
cuda = ["ort/cuda"]
```

## Build Script Optimization

### Common `build.rs` Pattern

```rust
// crates/common/build.rs
use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTOPS_VERSION");
    println!("cargo:rerun-if-changed=.git/HEAD");

    // Set version from git
    let version = if let Ok(output) = std::process::Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
    {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        env!("CARGO_PKG_VERSION").to_string()
    };

    println!("cargo:rustc-env=RUSTOPS_VERSION={version}");

    // Enable conditional compilation
    #[cfg(feature = "cuda")]
    {
        println!("cargo:rustc-cfg=cuda_enabled");
        println!("cargo:rustc-link-lib=cudart");
    }
}
```

## Conditional Compilation Examples

### Cloud Provider Selection

```rust
#[cfg(feature = "aws")]
pub mod aws;

#[cfg(feature = "azure")]
pub mod azure;

#[cfg(feature = "gcp")]
pub mod gcp;

pub struct CloudProvider {
    #[cfg(feature = "aws")]
    aws: Option<aws::AwsClient>,

    #[cfg(feature = "azure")]
    azure: Option<azure::AzureClient>,

    #[cfg(feature = "gcp")]
    gcp: Option<gcp::GcpClient>,
}
```

### Platform-Specific Code

```rust
#[cfg(target_os = "linux")]
pub fn collect_system_metrics() -> Result<Metrics, Error> {
    // Linux-specific implementation
}

#[cfg(target_os = "macos")]
pub fn collect_system_metrics() -> Result<Metrics, Error> {
    // macOS-specific implementation
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("Unsupported platform");
```

## Workspace Dependency Management

### Updating Dependencies

```bash
# Update all dependencies to latest compatible
cargo update

# Update specific dependency
cargo update -p tokio

# Check for outdated dependencies
cargo outdated
```

### Dependency Audit

```bash
# Security audit
cargo audit

# License compliance
cargo deny check licenses

# Advisory check
cargo deny check advisories

# Full check
cargo deny check all
```

## Binary Size Optimization

### `.cargo/config.toml`

```toml
[build]
target-dir = "target"

[target.x86_64-unknown-linux-gnu]
rustflags = [
    "-C", "link-arg=-fuse-ld=lld",          # Use lld linker
    "-C", "link-arg=-Wl,--gc-sections",     # Remove unused sections
    "-C", "codegen-units=1",                # Better optimization
]

[env]
RUSTFLAGS = "-C target-cpu=native"          # CPU-specific optimizations
RUST_LOG = "info"
```

## Cross-Compilation

```toml
# .cargo/config.toml
[build]
target = "x86_64-unknown-linux-gnu"

[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"

[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

## Performance Profiling

### Flamegraph Support

```toml
[profile.flamegraph]
inherits = "release"
debug = true
strip = false
```

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin rustops-agent
```

## Testing Configuration

### `.cargo/nextest.toml`

```toml
[store]
dir = "target/nextest"

[profile.default]
retries = 2
status-level = "all"
failure-output = "immediate-final"

[profile.ci]
retries = 0
status-level = "passing"
failure-output = "immediate"
fail-fast = false
```

## Workspace Tools

### Useful Aliases

```bash
# .cargo/config.toml
[alias]
# Build all workspace members
b-all = "build --workspace --all-targets"

# Test with nextest
t-next = "nextest run --workspace"

# Check all features
check-all = "check --all-features --workspace"

# Document all
doc-all = "doc --workspace --no-deps --open"

# Clean thoroughly
clean-all = ["clean", "clean --doc"]

# Run clippy on all
lint-all = "clippy --workspace --all-features -- -D warnings"
```
