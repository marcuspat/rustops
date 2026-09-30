// Resilience patterns for integrations
//
// Implements circuit breakers, rate limiting, and retry logic

use chrono::{DateTime, Utc};

/// Integration error types
#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    /// Network-level communication failure
    #[error("Network error: {0}")]
    Network(String),

    /// Authentication or authorization failure
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Rate limit was exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Circuit breaker prevented the call
    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,

    /// Operation timed out
    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// Request serialization failure
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Response deserialization failure
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Response was invalid or unexpected
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// External service is unavailable
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Unclassified error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

// From implementations for common error types
impl From<serde_json::Error> for IntegrationError {
    fn from(err: serde_json::Error) -> Self {
        IntegrationError::Serialization(err.to_string())
    }
}

impl From<hyper::Error> for IntegrationError {
    fn from(err: hyper::Error) -> Self {
        IntegrationError::Network(err.to_string())
    }
}

impl From<hyper::http::Error> for IntegrationError {
    fn from(err: hyper::http::Error) -> Self {
        IntegrationError::Network(err.to_string())
    }
}

impl From<std::string::FromUtf8Error> for IntegrationError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        IntegrationError::Deserialization(err.to_string())
    }
}

/// Integration result type
pub type IntegrationResult<T> = Result<T, IntegrationError>;

/// Health status for integrations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Operating normally
    Healthy,
    /// Operating with degraded capability
    Degraded,
    /// Not operating correctly
    Unhealthy,
    /// Status not yet determined
    Unknown,
}

/// Call outcome for monitoring
#[derive(Debug, Clone)]
pub struct CallOutcome {
    /// Outcome status of the call
    pub status: CallStatus,
    /// Call latency
    pub latency: std::time::Duration,
    /// Whether the circuit breaker was open
    pub circuit_breaker_open: bool,
    /// Whether a rate limit was hit
    pub rate_limit_hit: bool,
}

/// Status of a monitored call
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallStatus {
    /// Call succeeded
    Success,
    /// Call failed
    Failure,
    /// Call timed out
    Timeout,
    /// Call was rejected by the rate limiter
    RateLimited,
}

/// Integration health metrics
#[derive(Debug, Clone)]
pub struct IntegrationHealth {
    /// Integration identifier
    pub integration_id: String,
    /// Current health status
    pub status: HealthStatus,
    /// Timestamp of the last successful call
    pub last_successful_call: Option<DateTime<Utc>>,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
    /// Average call latency
    pub avg_latency: std::time::Duration,
    /// Whether the circuit breaker is open
    pub circuit_breaker_open: bool,
    /// Number of rate limit hits
    pub rate_limit_hits: u64,
}

impl IntegrationHealth {
    /// Create new health metrics for the given integration
    pub fn new(integration_id: impl Into<String>) -> Self {
        Self {
            integration_id: integration_id.into(),
            status: HealthStatus::Unknown,
            last_successful_call: None,
            error_rate: 0.0,
            avg_latency: std::time::Duration::ZERO,
            circuit_breaker_open: false,
            rate_limit_hits: 0,
        }
    }

    /// Update health metrics from a call outcome
    pub fn update(&mut self, outcome: &CallOutcome) {
        match outcome.status {
            CallStatus::Success => {
                self.status = HealthStatus::Healthy;
                self.last_successful_call = Some(Utc::now());
            }
            CallStatus::Failure | CallStatus::Timeout => {
                self.status = HealthStatus::Degraded;
            }
            CallStatus::RateLimited => {
                self.status = HealthStatus::Unhealthy;
                self.rate_limit_hits += 1;
            }
        }

        self.circuit_breaker_open = outcome.circuit_breaker_open;
        if outcome.rate_limit_hit {
            self.rate_limit_hits += 1;
        }

        // Update average latency with exponential smoothing
        let alpha = 0.2;
        let new_latency_ms = outcome.latency.as_millis() as f64;
        let current_avg_ms = self.avg_latency.as_millis() as f64;
        let smoothed = alpha * new_latency_ms + (1.0 - alpha) * current_avg_ms;
        self.avg_latency = std::time::Duration::from_millis(smoothed as u64);
    }
}
