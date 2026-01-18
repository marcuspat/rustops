# Testing Strategy

Comprehensive testing approach for the RustOps AIOps platform.

## Testing Pyramid

```
                    /\
                   /  \
                  / E2E \
                 /______\
                /        \
               /Integration\
              /__________  \
             /             \ \
            /  Unit Tests    \\
           /___________________\
```

| Test Type | Count | Execution Time | Coverage Goal |
|-----------|-------|----------------|---------------|
| Unit Tests | 1000+ | < 5 min | 80%+ |
| Integration Tests | 200+ | < 20 min | Key flows |
| E2E Tests | 50+ | < 30 min | Critical paths |
| Property Tests | 100+ | < 10 min | Core logic |
| Fuzz Tests | 20+ | < 1 hr | Input parsing |

## 1. Unit Tests

### Structure

```rust
// crates/agent/src/collectors/metrics.rs
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Basic unit test
    #[test]
    fn test_metric_collector_creation() {
        let collector = MetricCollector::new(Config::default());
        assert!(collector.is_ok());
    }

    // Parameterized test
    #[rstest]
    #[case(100, true)]
    #[case(0, false)]
    #[case(-1, false)]
    fn test_metric_validation(#[case] value: i64, #[case] expected: bool) {
        let result = validate_metric(value);
        assert_eq!(result.is_ok(), expected);
    }
}
```

### Organization

```
crates/
└── <crate>/
    └── src/
        ├── main.rs
        ├── lib.rs
        └── <module>.rs
            └── <module>_test.rs  # Or inline #[cfg(test)] mod tests
```

### Best Practices

1. **Test Public API**: Test behavior, not implementation
2. **Descriptive Names**: `test_<function>_<scenario>_<expected>`
3. **One Assertion Per Test**: When possible
4. **Test Error Cases**: Not just happy paths
5. **Use Test Builders**: For complex setup

### Example Test Builder

```rust
// crates/common/src/testing/mod.rs
pub struct MetricBuilder {
    name: String,
    value: f64,
    labels: HashMap<String, String>,
    timestamp: Option<i64>,
}

impl MetricBuilder {
    pub fn new() -> Self {
        Self {
            name: "test_metric".to_string(),
            value: 0.0,
            labels: HashMap::new(),
            timestamp: None,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    pub fn label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }

    pub fn timestamp(mut self, ts: i64) -> Self {
        self.timestamp = Some(ts);
        self
    }

    pub fn build(self) -> Metric {
        Metric {
            name: self.name,
            value: self.value,
            labels: self.labels,
            timestamp: self.timestamp.unwrap_or_else(|| {
                chrono::Utc::now().timestamp()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_builder() {
        let metric = MetricBuilder::new()
            .name("cpu_usage")
            .value(75.5)
            .label("host", "server1")
            .build();

        assert_eq!(metric.name, "cpu_usage");
        assert_eq!(metric.value, 75.5);
        assert_eq!(metric.labels.get("host"), Some(&"server1".to_string()));
    }
}
```

## 2. Integration Tests

### Structure

```
tests/
├── integration/
│   ├── mod.rs
│   ├── agent_tests.rs
│   ├── pipeline_tests.rs
│   └── api_tests.rs
├── e2e/
│   ├── mod.rs
│   ├── incident_workflow.rs
│   └── remediation_flow.rs
├── fixtures/
│   ├── metrics.json
│   ├── logs.json
│   └── topologies/
└── mocks/
    ├── prometheus.rs
    └── kafka.rs
```

### Docker Compose Test Environment

```yaml
# tests/docker-compose.yml
version: '3.8'

services:
  kafka:
    image: bitnami/kafka:latest
    environment:
      KAFKA_CFG_ZOOKEEPER_CONNECT: zookeeper:2181
    ports:
      - "9092:9092"

  clickhouse:
    image: clickhouse/clickhouse-server:latest
    ports:
      - "8123:8123"

  redis:
    image: redis:alpine
    ports:
      - "6379:6379"

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./fixtures/prometheus.yml:/etc/prometheus/prometheus.yml
```

### Integration Test Example

```rust
// tests/integration/pipeline_tests.rs
use rustops_pipeline::{Pipeline, PipelineConfig};
use rustops_common::telemetry::Metric;
use std::time::Duration;

#[tokio::test]
async fn test_pipeline_ingestion() {
    // Setup: Start docker-compose services
    let _compose = docker_compose::up("tests/docker-compose.yml");

    // Given: A configured pipeline
    let config = PipelineConfig::from_file("tests/fixtures/pipeline.yaml").unwrap();
    let pipeline = Pipeline::new(config).await.unwrap();

    // When: Ingesting metrics
    let metric = Metric::builder()
        .name("test_metric")
        .value(42.0)
        .build();

    pipeline.ingest(metric).await.unwrap();

    // Then: Verify processing
    tokio::time::sleep(Duration::from_secs(2)).await;
    let stored = pipeline.query("test_metric").await.unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].value, 42.0);

    // Cleanup
    docker_compose::down("tests/docker-compose.yml");
}
```

## 3. Property-Based Testing

### Proptest Examples

```rust
// crates/correlation/src/dedup.rs
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_dedup_preserves_unique_ids(
            alerts in prop::collection::vec(
                any::<Alert>(),
                1..100
            )
        ) {
            let unique_count = alerts.iter()
                .map(|a| &a.id)
                .collect::<HashSet<_>>()
                .len();

            let deduped = deduplicate_alerts(alerts);
            assert_eq!(deduped.len(), unique_count);
        }

        #[test]
        fn test_time_window_groups(
            timestamps in prop::collection::vec(
                (0i64..1000),
                10..100
            )
        ) {
            let alerts: Vec<_> = timestamps
                .into_iter()
                .map(|ts| Alert::new("test".to_string(), ts))
                .collect();

            let grouped = group_by_time_window(&alerts, 300);
            // All groups should have alerts within 300 seconds
            for group in grouped {
                if group.len() > 1 {
                    let range = group.iter().map(|a| a.timestamp).max().unwrap()
                        - group.iter().map(|a| a.timestamp).min().unwrap();
                    assert!(range <= 300);
                }
            }
        }
    }
}
```

### Quickcheck Examples

```rust
#[cfg(test)]
mod tests {
    use quickcheck::QuickCheck;

    #[test]
    fn test_metric_aggregation() {
        fn prop(values: Vec<f64>) -> bool {
            let result = aggregate_metrics(&values);
            if values.is_empty() {
                result.is_nan()
            } else {
                result >= values.iter().cloned().fold(f64::INFINITY, f64::min)
                    && result <= values.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            }
        }

        QuickCheck::new()
            .tests(100)
            .quickcheck(prop as fn(Vec<f64>) -> bool);
    }
}
```

## 4. Fuzzing

### LibFuzzer Setup

```rust
// crates/agent/src/parsers.rs

#[cfg(fuzzing)]
pub fn parse_log_line(input: &[u8]) -> Result<LogEntry, ParseError> {
    // Parsing logic
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(fuzzing)]
    fuzz_target!(|data: &[u8]| {
        if let Ok(entry) = parse_log_line(data) {
            // Invariants that must always hold
            assert!(!entry.message.is_empty());
            assert!(entry.timestamp > 0);
        }
    });
}
```

### Fuzzing Configuration

```bash
# Fuzzing script: scripts/fuzz.sh
#!/bin/bash
cargo install cargo-fuzz

# Fuzz input parsing
cargo fuzz run parse_log_input -- -max_total_time=3600

# Fuzz config parsing
cargo fuzz run parse_config -- -max_total_time=3600

# Fuzz protocol parsing
cargo fuzz run parse_protocol -- -max_total_time=3600
```

## 5. Benchmark Tests

### Criterion Benchmarks

```rust
// crates/anomaly/benches/detection.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rustops_anomaly::Detector;

fn bench_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("detection");

    for size in [100, 1000, 10000, 100000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let data = generate_test_data(size);
            let detector = Detector::new();

            b.iter(|| {
                detector.detect(black_box(&data))
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_detection);
criterion_main!(benches);
```

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench detection

# Compare baselines
cargo bench --bench detection -- --save-baseline main
cargo bench --bench detection -- --baseline main

# Generate flamegraph
cargo install flamegraph
cargo flamegraph --bench detection
```

## 6. Coverage Reporting

### Tarpaulin Configuration

```bash
# .github/workflows/test.yml
- name: Run tests with coverage
  run: |
    cargo install cargo-tarpaulin
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
```

### Coverage Goals

| Crate Type | Minimum Coverage | Target Coverage |
|------------|------------------|-----------------|
| Core Logic | 90% | 95% |
| API Handlers | 80% | 90% |
| Integrations | 70% | 85% |
| Common Utils | 85% | 90% |
| Overall | 80% | 85% |

## 7. Mock and Fixture Management

### Mock Service

```rust
// tests/mocks/prometheus.rs
use mockito::{Server, Mock};
use reqwest::Client;

pub struct MockPrometheus {
    server: Server,
}

impl MockPrometheus {
    pub fn new() -> Self {
        Self {
            server: Server::new(),
        }
    }

    pub fn mock_query_range(&self, response: &str) -> Mock {
        self.server
            .mock("GET", "/api/v1/query_range")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(response)
            .create()
    }

    pub fn url(&self) -> String {
        self.server.url()
    }
}
```

### Test Fixtures

```json
// tests/fixtures/metrics/sample_batch.json
{
  "metrics": [
    {
      "name": "cpu_usage_percent",
      "value": 75.5,
      "labels": {
        "host": "server1",
        "datacenter": "us-west"
      },
      "timestamp": 1705536000
    },
    {
      "name": "memory_usage_bytes",
      "value": 8589934592,
      "labels": {
        "host": "server1"
      },
      "timestamp": 1705536000
    }
  ]
}
```

## 8. Test Automation

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Run quick tests
cargo test --workspace --lib --quiet

# Check formatting
cargo fmt -- --check

# Run clippy
cargo clippy --workspace --quiet -- -D warnings

# Check documentation
cargo doc --workspace --no-deps --quiet
```

### CI Test Matrix

```yaml
# .github/workflows/test.yml
strategy:
  matrix:
    rust: [stable, beta, nightly]
    os: [ubuntu-latest, windows-latest, macos-latest]
    test_type: [unit, integration, property]

steps:
  - name: Run tests
    run: |
      if [ "${{ matrix.test_type }}" == "unit" ]; then
        cargo test --workspace --lib
      elif [ "${{ matrix.test_type }}" == "integration" ]; then
        cargo test --workspace --test '*'
      elif [ "${{ matrix.test_type }}" == "property" ]; then
        cargo test --workspace --features test-utils
      fi
```

## 9. Continuous Testing

### Local Development

```bash
# Watch mode
cargo install cargo-watch
cargo watch -x 'test --workspace'

# Nextest watch mode
cargo nextest run --workspace --rerun-ignore-file .git/nextest-rerun.txt
```

### Performance Regression Detection

```yaml
# .github/workflows/bench.yml
- name: Run benchmarks
  run: cargo bench --bench detection -- --save-baseline main

- name: Compare baselines
  run: |
    cargo bench --bench detection -- --baseline main
    # Fail if regression > 5%

- name: Store results
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: 'cargo'
    output-file-path: target/criterion/report/index.html
```

## 10. Test Data Management

### Generating Test Data

```rust
// crates/common/src/testing/data.rs
use rand::Rng;

pub struct TestDataGenerator {
    rng: StdRng,
}

impl TestDataGenerator {
    pub fn new() -> Self {
        Self {
            rng: StdRng::from_entropy(),
        }
    }

    pub fn metric(&mut self) -> Metric {
        Metric {
            name: format!("metric_{}", self.rng.gen::<u32>()),
            value: self.rng.gen(),
            labels: self.labels(),
            timestamp: self.timestamp(),
        }
    }

    pub fn log_entry(&mut self) -> LogEntry {
        LogEntry {
            level: *["INFO", "WARN", "ERROR"].choose(&mut self.rng).unwrap(),
            message: self.random_string(50..200),
            timestamp: self.timestamp(),
            labels: self.labels(),
        }
    }

    fn labels(&mut self) -> HashMap<String, String> {
        let mut labels = HashMap::new();
        labels.insert("host".to_string(), format!("host-{}", self.rng.gen::<u32>()));
        labels.insert("region".to_string(), format!("region-{}", self.rng.gen_range(0..3)));
        labels
    }

    fn timestamp(&mut self) -> i64 {
        chrono::Utc::now().timestamp() - self.rng.gen_range(0..86400)
    }

    fn random_string(&mut self, range: Range<usize>) -> String {
        (0..self.rng.gen_range(range))
            .map(|_| self.rng.gen_range(b'a'..b'z') as char)
            .collect()
    }
}
```

## Testing Checklist

Before committing code, ensure:

- [ ] All unit tests pass (`cargo test --workspace --lib`)
- [ ] Integration tests pass (`cargo test --workspace --test '*'`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy --workspace -- -D warnings`)
- [ ] Documentation builds (`cargo doc --workspace --no-deps`)
- [ ] Coverage meets threshold (80%+)
- [ ] Benchmarks run without regression
- [ ] Property tests pass (100+ iterations)
- [ ] New tests added for new functionality
- [ ] Edge cases covered
- [ ] Error paths tested
