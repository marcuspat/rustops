// Rate limiter for integrations
//
// Implements token bucket rate limiting per integration

use crate::resilience::{IntegrationError, IntegrationResult};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Requests per second
    pub requests_per_second: u32,

    /// Burst size
    pub burst: u32,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            burst: 20,
        }
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(config: RateLimiterConfig) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(config.requests_per_second).unwrap())
            .allow_burst(NonZeroU32::new(config.burst).unwrap());

        Self {
            limiter: Arc::new(GovernorRateLimiter::direct(quota)),
        }
    }

    /// Acquire permission to proceed
    pub async fn acquire(&self) -> IntegrationResult<()> {
        match self.limiter.check() {
            Ok(_) => Ok(()),
            Err(_) => Err(IntegrationError::RateLimitExceeded),
        }
    }

    /// Acquire with timeout
    pub async fn acquire_with_timeout(&self, timeout: Duration) -> IntegrationResult<()> {
        match tokio::time::timeout(timeout, self.acquire()).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(IntegrationError::RateLimitExceeded),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_enforces_limit() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            requests_per_second: 2,
            burst: 2,
        });

        // First burst should succeed
        limiter.acquire().await.unwrap();
        limiter.acquire().await.unwrap();

        // Third call should be rate limited
        let result = limiter.acquire().await;
        assert!(matches!(result, Err(IntegrationError::RateLimitExceeded)));
    }

    #[tokio::test]
    async fn test_rate_limiter_refills() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            requests_per_second: 10,
            burst: 2,
        });

        // Use burst
        limiter.acquire().await.unwrap();
        limiter.acquire().await.unwrap();

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should have more tokens
        limiter.acquire().await.unwrap();
    }
}
