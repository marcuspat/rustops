//! RustOps Integration Bounded Context
//!
//! Implements the Integration layer following DDD principles with:
//! - Adapter pattern for unified interface
//! - Circuit breakers for external systems
//! - Rate limiting per integration
//! - Retry logic with exponential backoff
//! - Phase 1 integrations: Prometheus, Kubernetes, ServiceNow, PagerDuty, Slack

#![warn(missing_docs)]
#![warn(clippy::all)]

/// Adapter.
pub mod adapter;
/// Circuit breaker.
pub mod circuit_breaker;
/// Infrastructure.
pub mod infrastructure;
/// Itsm.
pub mod itsm;
/// Rate limiter.
pub mod rate_limiter;
/// Resilience.
pub mod resilience;
/// Retry.
pub mod retry;
/// Telemetry.
pub mod telemetry;

// Re-exports
pub use adapter::{
    ITSMNotifier, InfrastructureMonitor, IntegrationAdapter, TelemetryCollector, TelemetryEvent,
};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use rate_limiter::{RateLimiter, RateLimiterConfig};
pub use resilience::{HealthStatus, IntegrationError, IntegrationResult};
pub use retry::{retry_with_backoff, RetryConfig};

/// Integration kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegrationKind {
    /// Telemetry collector (Prometheus, Datadog, etc.)
    TelemetryCollector,

    /// ITSM/Incident management (ServiceNow, Jira, etc.)
    ITSMNotifier,

    /// Infrastructure monitoring (Kubernetes, AWS, etc.)
    InfrastructureMonitor,

    /// Collaboration platform (Slack, Teams, etc.)
    CollaborationChannel,
}

/// Integration configuration
#[derive(Debug, Clone)]
pub struct IntegrationConfig {
    /// Integration identifier
    pub id: String,

    /// Integration kind
    pub kind: IntegrationKind,

    /// Enable/disable integration
    pub enabled: bool,

    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,

    /// Rate limiter configuration
    pub rate_limiter: RateLimiterConfig,

    /// Retry configuration
    pub retry: RetryConfig,

    /// Connection timeout
    pub timeout: std::time::Duration,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            id: ulid::Ulid::new().to_string(),
            kind: IntegrationKind::TelemetryCollector,
            enabled: true,
            circuit_breaker: CircuitBreakerConfig::default(),
            rate_limiter: RateLimiterConfig::default(),
            retry: RetryConfig::default(),
            timeout: std::time::Duration::from_secs(30),
        }
    }
}
