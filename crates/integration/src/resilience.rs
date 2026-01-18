// Resilience patterns for integrations
//
// Implements circuit breakers, rate limiting, and retry logic

use crate::{IntegrationError, IntegrationResult};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Integration error types
#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Circuit breaker is open")]
    CircuitBreakerOpen,

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Integration result type
pub type IntegrationResult<T> = Result<T, IntegrationError>;

/// Health status for integrations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Call outcome for monitoring
#[derive(Debug, Clone)]
pub struct CallOutcome {
    pub status: CallStatus,
    pub latency: std::time::Duration,
    pub circuit_breaker_open: bool,
    pub rate_limit_hit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallStatus {
    Success,
    Failure,
    Timeout,
    RateLimited,
}

/// Integration health metrics
#[derive(Debug, Clone)]
pub struct IntegrationHealth {
    pub integration_id: String,
    pub status: HealthStatus,
    pub last_successful_call: Option<DateTime<Utc>>,
    pub error_rate: f64,
    pub avg_latency: std::time::Duration,
    pub circuit_breaker_open: bool,
    pub rate_limit_hits: u64,
}

impl IntegrationHealth {
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
