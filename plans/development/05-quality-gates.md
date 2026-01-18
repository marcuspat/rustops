# Quality Gates

Standards and automated checks to ensure code quality, security, and performance.

## Linting Standards

### Clippy Configuration

```toml
# clippy.toml
array-local-expr-threshold = 512000
array-size-threshold = 512000
vec-box-size-threshold = 512000
stack-size-threshold = 512000
enum-variant-name-threshold = 3
enum-variant-size-threshold = 200
literal-representation-threshold = 10
trivial-copy-size-limit = 256

# Disable specific lints
blacklisted-names = ["foo", "bar", "baz", "quux"]
doc-valid-idents = ["RustOps", "AIOps", "SLO", "MTTR", "MTTD"]

# Enforce pedantic lints in CI
too-many-arguments-threshold = 7
type-complexity-threshold = 250
```

### Required Clippy Lints

```bash
# CI Command
cargo clippy --workspace --all-targets -- -D warnings \
  -W clippy::all \
  -W clippy::pedantic \
  -W clippy::cargo \
  -A clippy::multiple-crate-versions
```

### Lint Categories

| Category | Status | Description |
|----------|--------|-------------|
| `clippy::all` | Error | All lints that aren't necessarily noisy |
| `clippy::pedantic` | Error | Pedantic lints focusing on precision |
| `clippy::cargo` | Warn | Cargo-specific lints |
| `clippy::nursery` | Warn | Experimental lints |
| `clippy::complexity` | Error | Code complexity issues |
| `clippy::correctness` | Error | Code correctness issues |
| `clippy::perf` | Error | Performance issues |
| `clippy::suspicious` | Error | Suspicious code patterns |
| `clippy::style` | Warn | Style inconsistencies |

### Common Issues

```rust
// ❌ BAD: Complex boolean expression
if x == true || y == false {
}

// ✅ GOOD: Simplified
if x || !y {
}

// ❌ BAD: Redundant clone
let s = data.to_string().clone();

// ✅ GOOD: Direct
let s = data.to_string();

// ❌ BAD: Unnecessary allocation
let mut v = Vec::new();
v.push(1);
v.push(2);

// ✅ GOOD: Macro
let v = vec![1, 2];

// ❌ BAD: Indexing without bounds check
let value = arr[10];

// ✅ GOOD: Checked access
let value = arr.get(10)?;
```

## Code Formatting

### rustfmt Configuration

```toml
# rustfmt.toml
edition = "2024"
max_width = 100
hard_tabs = false
tab_spaces = 4
indent_style = "Block"
fn_single_line = false
where_single_line = false

# Imports
imports_indent = "Block"
imports_layout = "Mixed"
group_imports = "StdExternalCrate"
reorder_imports = true
reorder_modules = true

# Comments
comment_width = 100
wrap_comments = true
normalize_comments = true
normalize_doc_attributes = true

# Formatting
format_code_in_doc_comments = true
format_strings = true
format_macro_matchers = true
format_macro_bodies = true

# Structs
struct_field_align_threshold = 0
struct_lit_single_line = true
struct_variant_width = 35

# Other
use_field_init_shorthand = true
use_try_shorthand = true
use_small_heuristics = "Default"
merge_derives = true
overflow_delimited_expr = true
blank_lines_upper_bound = 1
blank_lines_lower_bound = 0
remove_nested_parens = true
```

### Formatting Rules

1. **Max line width**: 100 characters
2. **Indentation**: 4 spaces
3. **Trailing commas**: Required in multi-line structures
4. **Imports**: Group std, external, and internal
5. **Doc comments**: Use `///` for item docs
6. **Module docs**: Use `//!` for module-level docs

```rust
// ✅ Correct formatting
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info};

use rustops_common::config::Config;
use rustops_common::error::Result;

/// Collects metrics from various sources.
///
/// This component is responsible for gathering metrics from
/// configured collectors and forwarding them to the pipeline.
pub struct MetricCollector {
    config: Config,
    sender: mpsc::Sender<Metric>,
}
```

## Security Standards

### Dependency Scanning

```bash
# Security audit
cargo audit

# Check for vulnerabilities
cargo audit --db https://github.com/RustSec/advisory-db

# CI Integration
cargo audit --json | jq -r '.vulnerabilities | length'
```

### cargo-deny Configuration

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
    "Unicode-DFS-2016",
]
exceptions = [
    { allow = ["OpenSSL"], name = "ring" },
]

[[licenses.clarify]]
name = "ring"
expression = "MIT AND ISC AND OpenSSL"
license-files = []

[bans]
multiple-versions = "warn"
wildcards = "allow"
highlight = "all"
workspace-duplicates = "warn"
deny = [
    { name = "openssl-sys", use-instead = "rustls" },
    { name = "native-tls", use-instead = "rustls" },
]
skip = []
skip-tree = []

[sources]
unknown-registry = "warn"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

### Code Security Checklist

- [ ] No hardcoded secrets or credentials
- [ ] Input validation on all external inputs
- [ ] Proper error handling without leaking sensitive info
- [ ] Use of rustls instead of native-tls
- [ ] No unsafe code without safety justification
- [ ] SQL injection prevention (use parameterized queries)
- [ ] XSS prevention in web UI
- [ ] CSRF tokens for state-changing operations
- [ ] Rate limiting on public APIs
- [ ] Audit logging for sensitive operations

## Performance Standards

### Benchmark Baselines

```rust
// benches/baseline.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn metric_ingestion(c: &mut Criterion) {
    c.bench_function("metric_ingestion", |b| {
        let collector = MetricCollector::new();
        let metric = Metric::test_data();

        b.iter(|| {
            collector.ingest(black_box(metric.clone()))
        });
    });
}

criterion_group!(benches, metric_ingestion);
criterion_main!(benches);
```

### Performance Thresholds

| Operation | P50 | P95 | P99 | Max |
|-----------|-----|-----|-----|-----|
| Metric ingestion | 1ms | 5ms | 10ms | 100ms |
| Alert correlation | 50ms | 200ms | 500ms | 1s |
| Anomaly detection | 100ms | 500ms | 1s | 2s |
| API response | 10ms | 50ms | 100ms | 500ms |
| Query execution | 50ms | 200ms | 500ms | 2s |

### Profiling Commands

```bash
# CPU profiling
cargo flamegraph --bin rustops-agent -- --profile

# Memory profiling
valgrind --tool=massif ./target/release/rustops-agent

# Heap profiling
heaptrack ./target/release/rustops-agent

# Compare performance
cargo bench --bench detection -- --baseline main
```

## Code Coverage

### Coverage Configuration

```bash
# Generate coverage
cargo tarpaulin --workspace \
  --exclude-files "*/tests/*" \
  --exclude-files "*/benches/*" \
  --exclude-files "*/examples/*" \
  --timeout 300 \
  --out Lcov \
  --output-dir ./coverage \
  -- --test-threads=1
```

### Coverage Requirements

| Component Type | Minimum | Target |
|----------------|---------|--------|
| Core logic | 90% | 95% |
| API handlers | 80% | 90% |
| Integrations | 70% | 85% |
| Common utilities | 85% | 90% |
| **Overall** | **80%** | **85%** |

### Coverage Exclusions

```rust
// Allow lower coverage for:
- Debug/development only code
- Platform-specific code (untestable in CI)
- Error handling paths (when truly exceptional)
- Generated code (protos, derives)
- Tests and benchmarks
```

## Documentation Standards

### Rustdoc Requirements

```rust
/// Summary sentence (one line, present tense).
///
/// More detailed explanation goes here. This can span multiple
/// paragraphs and include examples.
///
/// # Examples
///
/// ```
/// use rustops_agent::MetricCollector;
///
/// let collector = MetricCollector::new();
/// collector.start().await?;
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - Configuration is invalid
/// - Network connection fails
///
/// # Panics
///
/// This function will panic if the internal state is corrupted.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer.
pub async fn start(&self) -> Result<()> {
    // ...
}
```

### Documentation Checklist

- [ ] All public items documented
- [ ] Doc examples compile and pass
- [ ] Error conditions documented
- [ ] Panics documented
- [ ] Safety considerations documented (for unsafe code)
- [ ] Performance characteristics mentioned
- [ ] Thread safety documented
- [ ] Panics in examples explained
- [ ] Cross-references to related items
- [ ] External links to relevant specs

### Doc Test Standards

```rust
/// Adds two numbers together.
///
/// # Examples
///
/// ```
/// use rustops_common::math::add;
///
/// let result = add(2, 3);
/// assert_eq!(result, 5);
/// ```
///
/// ```should_panic
/// use rustops_common::math::add;
///
/// // This will panic
/// add(1, 2);
/// ```
///
/// ```no_run
/// use rustops_agent::Agent;
///
/// # async fn example() {
/// let agent = Agent::new();
/// agent.run().await.unwrap();
/// # }
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

## Code Review Checklist

### Before Opening PR

- [ ] All tests pass (`cargo test --workspace`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Documentation builds (`cargo doc`)
- [ ] Coverage threshold met (80%+)
- [ ] No new security vulnerabilities
- [ ] Benchmarks run without regression
- [ ] Commits follow conventional format
- [ ] PR description filled out completely

### Review Criteria

- [ ] Code is readable and maintainable
- [ ] Follows Rust best practices
- [ ] Error handling is comprehensive
- [ ] Tests cover edge cases
- [ ] Documentation is clear and accurate
- [ ] Performance implications considered
- [ ] Security implications reviewed
- [ ] Thread safety verified
- [ ] Resource cleanup handled
- [ ] Logging/metrics appropriate

## Automated Quality Checks

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

set -e

echo "Running pre-commit checks..."

# Format check
echo "Checking formatting..."
cargo fmt --all -- --check

# Clippy
echo "Running linter..."
cargo clippy --workspace --all-targets -- -D warnings

# Unit tests
echo "Running unit tests..."
cargo test --workspace --lib --quiet

# Documentation
echo "Checking documentation..."
cargo doc --workspace --no-deps --quiet 2>&1 | grep -q "warning" && exit 1 || true

echo "All checks passed!"
```

### CI Quality Gate

```yaml
# .github/workflows/quality.yml
name: Quality Gate

on:
  pull_request:
    branches: [main]

jobs:
  quality:
    runs-on: ubuntu-latest
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

      - name: Security audit
        run: |
          cargo install cargo-audit
          cargo audit

      - name: License check
        run: |
          cargo install cargo-deny
          cargo deny check licenses

      - name: Documentation
        run: cargo doc --workspace --no-deps

      - name: Coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --workspace -- --test-threads=1
        env:
          COVERAGE_MIN: 80
```

## Quality Metrics Dashboard

Track these metrics over time:

| Metric | Target | Current | Trend |
|--------|--------|---------|-------|
| Test Coverage | 80% | - | 📈 |
| Clippy Warnings | 0 | - | 📉 |
| Security Vulnerabilities | 0 | - | 📉 |
| Mean Time to Review | < 24h | - | 📉 |
| PR Merge Time | < 48h | - | 📉 |
| Failed Builds | < 5% | - | 📉 |
| Performance Regression | 0% | - | ➡️ |
| Documentation Coverage | 100% | - | ➡️ |
