// Retry logic with exponential backoff
//
// Implements retry with exponential backoff for transient failures

use backoff::future::retry_notify;
use backoff::ExponentialBackoff;
use std::time::Duration;
use tracing::{warn, Instrument};

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
    operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let backoff = ExponentialBackoff {
        initial_interval: config.initial_interval,
        max_interval: config.max_interval,
        multiplier: config.multiplier,
        randomization_factor: config.randomization_factor,
        ..Default::default()
    };

    let mut attempt = 0;
    let max_attempts = config.max_attempts;

    retry_notify(backoff, operation, |err, dur: Duration| {
        attempt += 1;
        if attempt < max_attempts {
            warn!(
                error = %err,
                retry_after = ?dur,
                attempt = attempt,
                max_attempts = max_attempts,
                "Operation failed, retrying..."
            );
        }
    })
    .await
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
