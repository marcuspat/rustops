// Circuit breaker implementation for integration resilience
//
// Prevents cascading failures by stopping calls to failing services

use crate::resilience::IntegrationError;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing, reject calls
    HalfOpen, // Testing if service recovered
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening
    pub error_threshold: usize,

    /// Number of consecutive successes before closing
    pub success_threshold: usize,

    /// How long to wait before attempting recovery
    pub timeout: Duration,

    /// Maximum number of calls to track
    pub max_calls: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            error_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
            max_calls: 100,
        }
    }
}

/// Circuit breaker for preventing cascading failures
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerState>>,
    failure_count: Arc<RwLock<usize>>,
    success_count: Arc<RwLock<usize>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    call_count: Arc<RwLock<usize>>,
}

#[derive(Debug, Clone, Copy)]
struct CircuitBreakerState {
    state: CircuitState,
    last_state_change: Instant,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState {
                state: CircuitState::Closed,
                last_state_change: Instant::now(),
            })),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            call_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Execute operation with circuit breaker protection
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T, IntegrationError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        // Check circuit state
        {
            let state_guard = self.state.read().await;
            if state_guard.state == CircuitState::Open {
                // Check if timeout has elapsed
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() < self.config.timeout {
                        return Err(IntegrationError::CircuitBreakerOpen);
                    }
                }
            }
        }

        // Increment call count
        {
            let mut call_count = self.call_count.write().await;
            *call_count += 1;
            if *call_count > self.config.max_calls {
                *call_count = 1;
            }
        }

        // Execute operation
        let result = operation.await;

        match result {
            Ok(value) => {
                self.report_success().await;
                Ok(value)
            }
            Err(err) => {
                self.report_failure().await;
                Err(IntegrationError::Network(err.to_string()))
            }
        }
    }

    /// Report successful call
    pub async fn report_success(&self) {
        {
            let mut state = self.state.write().await;
            if state.state == CircuitState::HalfOpen {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                if *success_count >= self.config.success_threshold {
                    state.state = CircuitState::Closed;
                    state.last_state_change = Instant::now();
                    *success_count = 0;
                    *self.failure_count.write().await = 0;
                }
            }
        }
    }

    /// Report failed call
    pub async fn report_failure(&self) {
        let mut state = self.state.write().await;
        let mut failure_count = self.failure_count.write().await;

        *failure_count += 1;
        *self.last_failure_time.write().await = Some(Instant::now());

        if *failure_count >= self.config.error_threshold {
            state.state = CircuitState::Open;
            state.last_state_change = Instant::now();
            *failure_count = 0;
            *self.success_count.write().await = 0;
        }
    }

    /// Get current circuit state
    pub async fn state(&self) -> CircuitState {
        let state = self.state.read().await;
        state.state
    }

    /// Manually reset circuit breaker
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        state.state = CircuitState::Closed;
        state.last_state_change = Instant::now();
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;
    }

    /// Check if circuit is open
    pub async fn is_open(&self) -> bool {
        self.state().await == CircuitState::Open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_threshold() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            error_threshold: 3,
            ..Default::default()
        });

        assert!(!cb.is_open().await);

        // Report failures up to threshold
        for _ in 0..3 {
            cb.report_failure().await;
        }

        assert!(cb.is_open().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_resets_on_success() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            error_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            ..Default::default()
        });

        // Open circuit
        cb.report_failure().await;
        cb.report_failure().await;
        assert!(cb.is_open().await);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Report successes
        cb.report_success().await;
        cb.report_success().await;

        assert!(!cb.is_open().await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_blocks_calls_when_open() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            error_threshold: 2,
            ..Default::default()
        });

        // Open circuit
        cb.report_failure().await;
        cb.report_failure().await;

        // Try to execute operation
        let result = cb.call(async { Ok::<(), String>(()) }).await;
        assert!(matches!(result, Err(IntegrationError::CircuitBreakerOpen)));
    }
}
