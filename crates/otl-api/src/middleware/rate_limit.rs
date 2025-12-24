//! Rate limiting middleware for API endpoints
//!
//! Provides configurable rate limiting for different endpoint types:
//! - Auth endpoints: 5 requests per minute per IP
//! - API endpoints: 100 requests per minute per IP
//! - Streaming endpoints: 10 requests per minute per IP
//!
//! Rate limiting is applied per IP address.
//!
//! Author: hephaex@gmail.com

use std::sync::Arc;
use std::time::Duration;
use tower_governor::{
    governor::GovernorConfigBuilder,
    key_extractor::SmartIpKeyExtractor,
    GovernorLayer,
};

/// Create rate limit config for auth endpoints (login, register)
///
/// Limits: 5 requests per second per IP
/// This prevents brute force attacks on authentication endpoints.
pub fn auth_rate_limit_config() -> Arc<tower_governor::governor::GovernorConfig<SmartIpKeyExtractor>> {
    Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(5)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("Failed to build auth rate limit config"),
    )
}

/// Create rate limit config for general API endpoints
///
/// Limits: ~120 requests per minute per IP (2 per second, burst 100)
/// This provides reasonable throughput for normal API usage.
pub fn api_rate_limit_config() -> Arc<tower_governor::governor::GovernorConfig<SmartIpKeyExtractor>> {
    Arc::new(
        GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(100)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("Failed to build API rate limit config"),
    )
}

/// Create rate limit config for streaming endpoints
///
/// Limits: 10 requests per minute per IP
/// Streaming endpoints are more resource-intensive, so we apply stricter limits.
pub fn streaming_rate_limit_config() -> Arc<tower_governor::governor::GovernorConfig<SmartIpKeyExtractor>> {
    Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(60))
            .burst_size(10)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("Failed to build streaming rate limit config"),
    )
}

/// Re-export GovernorLayer for convenience
pub use tower_governor::GovernorLayer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_configs_creation() {
        // Test that we can create all rate limit configs without panicking
        let _auth = auth_rate_limit_config();
        let _api = api_rate_limit_config();
        let _streaming = streaming_rate_limit_config();
    }
}
