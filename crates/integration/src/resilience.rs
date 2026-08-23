// Resilience patterns for integrations
//
// Implements circuit breakers, rate limiting, and retry logic

use chrono::{DateTime, Utc};

/// Integration error types
#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    #[error("Network error: {0}")]
    /// Network.
    Network(String),

    #[error("Authentication failed: {0}")]
    /// Authentication.
    Authentication(String),

    #[error("Rate limit exceeded")]
    /// RateLimitExceeded.
    RateLimitExceeded,

    #[error("Circuit breaker is open")]
    /// CircuitBreakerOpen.
    CircuitBreakerOpen,

    #[error("Timeout after {0:?}")]
    /// Timeout.
    Timeout(std::time::Duration),

    #[error("Serialization error: {0}")]
    /// Serialization.
    Serialization(String),

    #[error("Deserialization error: {0}")]
    /// Deserialization.
    Deserialization(String),

    #[error("Invalid response: {0}")]
    /// InvalidResponse.
    InvalidResponse(String),

    #[error("Service unavailable: {0}")]
    /// ServiceUnavailable.
    ServiceUnavailable(String),

    #[error("Unknown error: {0}")]
    /// Unknown.
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
    /// Healthy.
    Healthy,
    /// Degraded.
    Degraded,
    /// Unhealthy.
    Unhealthy,
    /// Unknown.
    Unknown,
}

/// Call outcome for monitoring
#[derive(Debug, Clone)]
pub struct CallOutcome {
    /// Pub.
    pub status: CallStatus,
    /// Pub.
    pub latency: std::time::Duration,
    /// Pub.
    pub circuit_breaker_open: bool,
    /// Pub.
    pub rate_limit_hit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// CallStatus.
pub enum CallStatus {
    /// Success.
    Success,
    /// Failure.
    Failure,
    /// Timeout.
    Timeout,
    /// RateLimited.
    RateLimited,
}

/// Integration health metrics
#[derive(Debug, Clone)]
pub struct IntegrationHealth {
    /// Pub.
    pub integration_id: String,
    /// Pub.
    pub status: HealthStatus,
    /// Pub.
    pub last_successful_call: Option<DateTime<Utc>>,
    /// Pub.
    pub error_rate: f64,
    /// Pub.
    pub avg_latency: std::time::Duration,
    /// Pub.
    pub circuit_breaker_open: bool,
    /// Pub.
    pub rate_limit_hits: u64,
}

impl IntegrationHealth {
    /// New.
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

    /// Update.
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
