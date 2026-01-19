// RustOps Integration Bounded Context
//
// Implements the Integration layer following DDD principles with:
// - Adapter pattern for unified interface
// - Circuit breakers for external systems
// - Rate limiting per integration
// - Retry logic with exponential backoff
// - Phase 1 integrations: Prometheus, Kubernetes, ServiceNow, PagerDuty, Slack

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod adapter;
pub mod resilience;
pub mod circuit_breaker;
pub mod rate_limiter;
pub mod retry;
pub mod telemetry;
pub mod itsm;
pub mod infrastructure;
pub mod prometheus;

// Re-exports
pub use adapter::{IntegrationAdapter, TelemetryCollector, ITSMNotifier, InfrastructureMonitor, TelemetryEvent};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use rate_limiter::{RateLimiter, RateLimiterConfig};
pub use retry::{RetryConfig, retry_with_backoff};
pub use resilience::{IntegrationError, IntegrationResult, HealthStatus};
pub use prometheus::{
    PrometheusAdapter, PrometheusQuery, AlertRule, ServiceDiscoveryConfig, AlertEvaluation,
    AlertStatus, ServiceTarget, KubernetesSDConfig, StaticTarget, RelabelConfig,
    RelabelAction,
};

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
