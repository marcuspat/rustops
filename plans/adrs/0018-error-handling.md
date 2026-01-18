# ADR 0018: Error Handling with Result and anyhow

## Metadata

| Field | Value |
|-------|-------|
| **ADR ID** | 0018 |
| **Title** | Error Handling Strategy: Result, anyhow, thiserror |
| **Status** | Proposed |
| **Date** | 2026-01-18 |
| **Authors** | Rust Engineering Team |
| **Related ADRs** | 0017 (Tokio), 0019 (Concurrency) |

---

## 1. Status

**Proposed** - Under review

---

## 2. Context

### Problem Statement

RustOps must handle errors across many layers:

| Layer | Error Types | Handling Strategy |
|-------|-------------|-------------------|
| **Network** | Connection timeout, DNS failure, TLS | Retry, backoff |
| **Serialization** | Invalid JSON, missing fields | Validation, defaults |
| **Database** | Connection lost, query failed | Retry, circuit breaker |
| **Kubernetes** | API error, resource not found | Retry, graceful degradation |
| **Business logic** | Invalid input, state violation | Validation, user error |

**Error handling requirements**:
- **Explicit**: Must handle errors (no panics in production)
- **Contextual**: Errors should include what, where, why
- **Actionable**: Should indicate recovery strategy
- **Observable**: Errors should be logged and tracked
- **Performant**: Minimal overhead for error paths

### Requirements

| Requirement | Target |
|-------------|--------|
| **No panics** | Zero panics in production code |
| **Error context** | All errors include context |
| **Stack traces** | Available for debugging |
| **Performance** | <1% overhead for error handling |
| **Compatibility** | Works with async, logging, metrics |

---

## 3. Decision

### Strategy: Layered Error Handling with anyhow and thiserror

```mermaid
graph TB
    subgraph "Application Layer"
        APP[Application Code] -->|Result| ANYHOW anyhow::Error
    end

    subgraph "Library Layer"
        LIB1[Telemetry Library] -->|Result| THIS1 thiserror::Error
        LIB2[ML Library] -->|Result| THIS2 thiserror::Error
        LIB3[Kubernetes Library] -->|Result| THIS3 thiserror::Error
    end

    subgraph "Error Handling"
        ANYHOW --> CONTEXT .context .with_context
        THIS1 --> DERIVE derive Display Error
        THIS2 --> DERIVE
        THIS3 --> DERIVE
    end

    subgraph "Observability"
        CONTEXT --> LOG[Logging]
        CONTEXT --> METRICS[Metrics]
        CONTEXT --> TRACE[Tracing]
    end
```

### Error Type Hierarchy

```rust
// 1. Application errors (use anyhow)
use anyhow::{Result, Context, anyhow, bail};
use thiserror::Error;

// Application-level Result (simpler error handling)
pub type AppResult<T> = Result<T, anyhow::Error>;

// 2. Library errors (use thiserror)
#[derive(Error, Debug)]
pub enum TelemetryError {
    #[error("Connection failed to {host}:{port}")]
    ConnectionFailed {
        host: String,
        port: u16,
        #[source]
        source: std::io::Error,
    },

    #[error("Serialization failed: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    #[error("Invalid metric: {name}")]
    InvalidMetric {
        name: String,
        reason: String,
    },

    #[error("Rate limited: try again in {wait_secs}s")]
    RateLimited {
        wait_secs: u64,
    },
}

#[derive(Error, Debug)]
pub enum MLError {
    #[error("Model not found: {model_name}")]
    ModelNotFound {
        model_name: String,
        available: Vec<String>,
    },

    #[error("Inference failed: {reason}")]
    InferenceFailed {
        reason: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Invalid input shape: {got}, expected {expected}")]
    InvalidShape {
        got: Vec<usize>,
        expected: Vec<usize>,
    },
}
```

### Error Handling Patterns

```rust
use anyhow::{Result, Context, anyhow, bail};

// 1. Conversion with context (.context())
pub async fn fetch_metrics(source: &str) -> Result<Vec<Metric>> {
    let client = reqwest::Client::new();
    let response = client
        .get(source)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .context("Failed to fetch metrics from source")?;

    if !response.status().is_success() {
        bail!("Source returned error status: {}", response.status());
    }

    let metrics = response
        .json()
        .await
        .context("Failed to deserialize metrics response")?;

    Ok(metrics)
}

// 2. Conditional context (.with_context())
pub async fn process_metric(metric: Metric) -> Result<Anomaly> {
    let validated = metric
        .validate()
        .with_context(|| format!("Validation failed for metric: {}", metric.name))?;

    detect_anomaly(validated).await
}

// 3. Custom errors with thiserror
#[derive(Error, Debug)]
pub enum IngestionError {
    #[error("Source {source} is unavailable")]
    SourceUnavailable { source: String },

    #[error("Rate limit exceeded for {source}")]
    RateLimit { source: String, retry_after: u64 },
}

impl IngestionError {
    pub fn is_retriable(&self) -> bool {
        match self {
            IngestionError::SourceUnavailable { .. } => true,
            IngestionError::RateLimit { .. } => true,
        }
    }
}

// 4. Error conversion with ? operator
pub async fn ingest_metrics(metrics: Vec<Metric>) -> Result<()> {
    for metric in metrics {
        let validated = metric.validate()?;  // Converts validation error

        store_metric(validated).await
            .context("Failed to store metric")?;
    }

    Ok(())
}

// 5. Error aggregation
#[derive(Error, Debug)]
pub enum BatchError {
    #[error("{0} errors occurred")]
    PartialFailure {
        total: usize,
        failed: usize,
        errors: Vec<anyhow::Error>,
    },
}

pub async fn process_batch(items: Vec<Item>) -> Result<(), BatchError> {
    let mut errors = Vec::new();

    for item in items {
        if let Err(e) = process_item(item).await {
            errors.push(e);
        }
    }

    if !errors.is_empty() {
        return Err(BatchError::PartialFailure {
            total: items.len(),
            failed: errors.len(),
            errors,
        }.into());
    }

    Ok(())
}
```

### Retry Logic with Error Inspection

```rust
use tokio::time::{sleep, Duration};
use anyhow::Result;

pub async fn retry_with_backoff<T, E, F, Fut>(
    mut operation: F,
    max_retries: u32,
    initial_delay: Duration,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut delay = initial_delay;
    let mut attempt = 0;

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let error: anyhow::Error = e.into();

                // Check if error is retriable
                if !is_retriable(&error) || attempt >= max_retries {
                    return Err(error.context(format!(
                        "Failed after {} attempts",
                        attempt
                    )));
                }

                warn!(
                    error = %error,
                    attempt,
                    "Operation failed, retrying in {:?}",
                    delay
                );

                sleep(delay).await;
                delay = delay * 2;  // Exponential backoff
            }
        }
    }
}

fn is_retriable(error: &anyhow::Error) -> bool {
    // Check error chain for retriable errors
    let mut err = error.source();

    while let Some(e) = err {
        if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
            return matches!(
                io_err.kind(),
                std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::Interrupted
            );
        }

        if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
            return reqwest_err.is_timeout() || reqwest_err.is_connect();
        }

        err = e.source();
    }

    false
}

// Usage
pub async fn fetch_with_retry(source: &str) -> Result<Vec<Metric>> {
    retry_with_backoff(
        || fetch_metrics(source),
        3,
        Duration::from_millis(100),
    ).await
}
```

### Error Observability

```rust
use tracing::{error, warn, instrument, Level};

#[instrument(skip(data))]
pub async fn process_telemetry(data: Vec<u8>) -> Result<()> {
    let result = try_process_telemetry(data).await;

    match &result {
        Ok(_) => {
            info!("Telemetry processed successfully");
        }
        Err(e) => {
            // Log error with context
            error!(
                error = %e,
                error_chain = %e.chain()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(" -> "),
                "Failed to process telemetry"
            );

            // Track error metrics
            metrics::error_counter
                .with_label_values(&[
                    error_kind(e),
                    error_source(e)
                ])
                .inc();
        }
    }

    result
}

fn error_kind(error: &anyhow::Error) -> &'static str {
    if let Some(io_err) = error.downcast_ref::<std::io::Error>() {
        return match io_err.kind() {
            std::io::ErrorKind::ConnectionRefused => "connection_refused",
            std::io::ErrorKind::TimedOut => "timeout",
            _ => "io_error",
        };
    }

    if error.downcast_ref::<serde_json::Error>().is_some() {
        return "serialization";
    }

    "unknown"
}

fn error_source(error: &anyhow::Error) -> String {
    error.chain()
        .last()
        .map(|e| e.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
```

### Panic Prevention

```rust
// 1. Catch unwinds in async tasks
pub async fn spawn_supervised<F>(future: F) -> JoinHandle<Result<F::Output>>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(async move {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(future)).await {
            Ok(result) => Ok(result),
            Err(panic) => {
                let panic_msg = if let Some(msg) = panic.downcast_ref::<String>() {
                    msg.clone()
                } else if let Some(msg) = panic.downcast_ref::<&str>() {
                    msg.to_string()
                } else {
                    "Unknown panic".to_string()
                };

                error!("Task panicked: {}", panic_msg);

                Err(anyhow::anyhow!("Task panicked: {}", panic_msg))
            }
        }
    })
}

// 2. Never use unwrap() in production
pub async fn safe_config() -> Result<Config> {
    // Bad: config.unwrap()
    // Good:
    config.ok_or_else(|| anyhow!("Configuration not loaded"))
}

// 3. Use expect() only for truly impossible cases
pub fn divide(a: f64, b: f64) -> Result<f64> {
    if b == 0.0 {
        bail!("Division by zero: {} / {}", a, b);
    }
    Ok(a / b)
}
```

---

## 4. Alternatives Considered

### Alternative 1: Custom Error Types Everywhere

**Description**: Define custom error enum for every module

**Pros**:
- Type-safe error handling
- Exhaustive matching
- Clear error categories

**Cons**:
- **Lots of boilerplate**
- **Hard to compose** errors across layers
- **Versioning** issues with error enums
- **Verbose** error propagation

**Rejected**: Too much boilerplate for application code

### Alternative 2: Box<dyn Error> (No Crates)

**Description**: Use standard library only

**Pros**:
- No dependencies
- Standard approach

**Cons**:
- **No context** (no .context() method)
- **Verbose** error construction
- **Downcasting** required for error inspection
- **No stack traces**

**Rejected**: Missing critical features

### Alternative 3: Failure Crate

**Description**: Use failure crate

**Pros**:
- Precedes thiserror/anyhow
- Good feature set

**Cons**:
- **Deprecated** by authors
- **Replaced by** thiserror + anyhow
- **No longer maintained**

**Rejected:** Deprecated crate

---

## 5. Consequences

### Positive

| Benefit | Impact |
|---------|--------|
| **Ergonomic** | .context() for easy error enrichment |
| **Type-safe** | thiserror for library errors |
| **Observable** | Errors logged with full context |
| **Maintainable** | Clear error handling strategy |
| **Performant** | Minimal overhead |

### Negative

| Challenge | Mitigation |
|-----------|------------|
| **Dependency** | Two additional crates | Widely used, stable |
| **Learning curve** | Team learns patterns | Documentation, examples |

### Neutral

- **Error size**: anyhow::Error is larger than custom enums
- **Type information**: Lost when using anyhow (by design)

---

## 6. Implementation

### Phase 1: Setup (Week 1)

- Add anyhow, thiserror to dependencies
- Define library error types
- Create error handling guidelines

### Phase 2: Migration (Weeks 2-3)

- Migrate application code to anyhow
- Migrate library code to thiserror
- Add context to all errors

### Phase 3: Observability (Weeks 4-5)

- Error logging
- Error metrics
- Error tracing

### Phase 4: Validation (Weeks 6-7)

- Code review
- Testing
- Documentation

---

## 7. References

### Documentation
- [anyhow Documentation](https://docs.rs/anyhow/)
- [thiserror Documentation](https://docs.rs/thiserror/)
- [Rust Error Handling Book](https://doc.rust-lang.org/book/ch09-00-error-handling.html)

### Articles
- [Error Handling in Rust](https://blog.burntsushi.net/rust-error-handling/)
- [The Error Handling Problem](https://without.boats/blog/the-error-handling-problem/)

### Research
- "Error Handling Best Practices" - RustConf 2024
    - "A Critique of the Rust Error System" - ACM 2023
