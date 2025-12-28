//! Rate limiting middleware for API endpoints
//!
//! Provides configurable rate limiting for different endpoint types:
//! - Auth endpoints: 5 requests per minute per IP (prevent brute force)
//! - API endpoints: 100 requests per minute per IP (normal operations)
//! - Streaming endpoints: 10 requests per minute per IP (resource intensive)
//!
//! Rate limiting is applied per IP address with headers showing limit status.
//!
//! Author: hephaex@gmail.com

use otl_core::config::RateLimitConfig;
use std::sync::Arc;
use tower_governor::{
    governor::GovernorConfigBuilder,
    key_extractor::SmartIpKeyExtractor,
    GovernorLayer,
};

// Use StateInformationMiddleware to get rate limit headers
type RateLimitLayer = GovernorLayer<
    SmartIpKeyExtractor,
    governor::middleware::StateInformationMiddleware,
>;

/// Create rate limiting layer for auth endpoints (login, register)
///
/// This prevents brute force attacks on authentication endpoints.
/// Returns rate limit headers (X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset).
pub fn auth_rate_limit_layer(config: &RateLimitConfig) -> RateLimitLayer {
    // Ensure at least 1 request per second
    let per_second = std::cmp::max(1, (config.auth_requests_per_minute / 60) as u64);

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(config.auth_burst_size)
            .key_extractor(SmartIpKeyExtractor)
            .use_headers()  // Enable rate limit headers
            .finish()
            .expect("Failed to build auth rate limit config"),
    );

    GovernorLayer {
        config: governor_conf,
    }
}

/// Create rate limiting layer for general API endpoints
///
/// This provides reasonable throughput for normal API usage.
/// Returns rate limit headers (X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset).
pub fn api_rate_limit_layer(config: &RateLimitConfig) -> RateLimitLayer {
    // Ensure at least 1 request per second
    let per_second = std::cmp::max(1, (config.api_requests_per_minute / 60) as u64);

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(config.api_burst_size)
            .key_extractor(SmartIpKeyExtractor)
            .use_headers()  // Enable rate limit headers
            .finish()
            .expect("Failed to build API rate limit config"),
    );

    GovernorLayer {
        config: governor_conf,
    }
}

/// Create rate limiting layer for streaming endpoints
///
/// Streaming endpoints are more resource-intensive, so we apply stricter limits.
/// Returns rate limit headers (X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset).
pub fn streaming_rate_limit_layer(config: &RateLimitConfig) -> RateLimitLayer {
    // Ensure at least 1 request per second
    let per_second = std::cmp::max(1, (config.streaming_requests_per_minute / 60) as u64);

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(per_second)
            .burst_size(config.streaming_burst_size)
            .key_extractor(SmartIpKeyExtractor)
            .use_headers()  // Enable rate limit headers
            .finish()
            .expect("Failed to build streaming rate limit config"),
    );

    GovernorLayer {
        config: governor_conf,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_layers_creation() {
        // Test that we can create all rate limit layers without panicking
        let config = RateLimitConfig::default();
        let _auth = auth_rate_limit_layer(&config);
        let _api = api_rate_limit_layer(&config);
        let _streaming = streaming_rate_limit_layer(&config);
    }

    #[test]
    fn test_custom_rate_limits() {
        // Test custom rate limit configuration
        let custom_config = RateLimitConfig {
            enabled: true,
            auth_requests_per_minute: 60,
            auth_burst_size: 10,
            streaming_requests_per_minute: 60,
            streaming_burst_size: 20,
            api_requests_per_minute: 600,
            api_burst_size: 200,
        };

        let _auth = auth_rate_limit_layer(&custom_config);
        let _api = api_rate_limit_layer(&custom_config);
        let _streaming = streaming_rate_limit_layer(&custom_config);
    }
}
