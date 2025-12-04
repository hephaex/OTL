//! Application state management
//!
//! Author: hephaex@gmail.com

use otl_core::config::AppConfig;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

/// Application state shared across handlers
pub struct AppState {
    /// Application configuration
    pub config: AppConfig,
    /// Server start time
    pub start_time: Instant,
    /// Request counter
    pub request_count: AtomicU64,
    /// Ready status
    pub is_ready: AtomicBool,
}

impl AppState {
    /// Create new application state with config
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            start_time: Instant::now(),
            request_count: AtomicU64::new(0),
            is_ready: AtomicBool::new(true),
        }
    }

    /// Increment request counter
    pub fn increment_requests(&self) -> u64 {
        self.request_count.fetch_add(1, Ordering::SeqCst)
    }

    /// Get total request count
    pub fn get_request_count(&self) -> u64 {
        self.request_count.load(Ordering::SeqCst)
    }

    /// Get uptime in seconds
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Check if service is ready
    pub fn is_ready(&self) -> bool {
        self.is_ready.load(Ordering::SeqCst)
    }

    /// Set ready status
    pub fn set_ready(&self, ready: bool) {
        self.is_ready.store(ready, Ordering::SeqCst);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(AppConfig::default())
    }
}
