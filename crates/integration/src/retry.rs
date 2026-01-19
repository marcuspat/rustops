// Retry logic with exponential backoff
//
// Implements retry with exponential backoff for transient failures

use std::time::Duration;
use tracing::{warn};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial backoff interval
    pub initial_interval: Duration,

    /// Maximum backoff interval
    pub max_interval: Duration,

    /// Multiplier for backoff
    pub multiplier: f64,

    /// Randomization factor
    pub randomization_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_interval: Duration::from_secs(1),
            max_interval: Duration::from_secs(60),
            multiplier: 2.0,
            randomization_factor: 0.2,
        }
    }
}

/// Execute operation with retry logic
pub async fn retry_with_backoff<F, Fut, T, E>(
    config: RetryConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    let mut current_delay = config.initial_interval;

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) if attempt >= config.max_attempts => {
                warn!(
                    error = %err,
                    attempt = attempt,
                    max_attempts = config.max_attempts,
                    "Operation failed after all retry attempts"
                );
                return Err(err);
            }
            Err(err) => {
                warn!(
                    error = %err,
                    retry_after = ?current_delay,
                    attempt = attempt,
                    max_attempts = config.max_attempts,
                    "Operation failed, retrying..."
                );

                tokio::time::sleep(current_delay).await;

                // Calculate next delay with exponential backoff
                current_delay = std::cmp::min(
                    Duration::from_secs_f64(
                        current_delay.as_secs_f64() * config.multiplier
                    ),
                    config.max_interval,
                );

                // Add simple jitter (50-100% of delay)
                let jitter_ms = (current_delay.as_millis() as f64 * config.randomization_factor) as u64;
                if jitter_ms > 0 {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let nanos = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .subsec_nanos() as u64;
                    let jitter = Duration::from_millis(nanos % jitter_ms);
                    current_delay = current_delay.saturating_add(jitter);
                }
            }
        }
    }
}

/// Execute operation with retry logic (simplified)
pub async fn retry_simple<F, Fut, T, E>(
    max_attempts: u32,
    operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    retry_with_backoff(
        RetryConfig {
            max_attempts,
            ..Default::default()
        },
        operation,
    )
    .await
}

/// Check if error is retryable
pub fn is_retryable_error<E: std::fmt::Display>(err: &E) -> bool {
    let err_str = err.to_string().to_lowercase();

    // Retry on network errors, timeouts, rate limits
    err_str.contains("timeout")
        || err_str.contains("connection")
        || err_str.contains("rate limit")
        || err_str.contains("unavailable")
        || err_str.contains("503")
        || err_str.contains("502")
        || err_str.contains("429")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_success_after_failure() {
        let mut attempts = 0;

        let result = retry_simple(3, || {
            async move {
                attempts += 1;
                if attempts < 3 {
                    Err("temporary failure")
                } else {
                    Ok("success")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_retry_exhausts_attempts() {
        let result = retry_simple(3, || {
            async { Err::<(), _>("persistent failure") }
        })
        .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_is_retryable_error() {
        assert!(is_retryable_error(&"connection timeout"));
        assert!(is_retryable_error(&"HTTP 503 Service Unavailable"));
        assert!(is_retryable_error(&"rate limit exceeded"));

        assert!(!is_retryable_error(&"authentication failed"));
        assert!(!is_retryable_error(&"HTTP 404 Not Found"));
    }
}
