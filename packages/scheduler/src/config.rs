//! Configuration for CYNIC Scheduler

use crate::{PHI, PHI_INV};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// CYNIC MCP server URL
    pub cynic_url: String,

    /// CYNIC API key
    pub cynic_api_key: Option<String>,

    /// Maximum transactions in priority queue
    pub max_queue_size: usize,

    /// Batch size for worker messages
    pub batch_size: usize,

    /// Number of worker threads
    pub num_workers: usize,

    /// Timeout for CYNIC API calls
    pub api_timeout: Duration,

    /// Cache duration for reputation scores
    pub reputation_cache_ttl: Duration,

    /// Enable GROWL filtering (drop malicious transactions)
    pub enable_growl_filter: bool,

    /// Enable priority boost for WAG verdicts
    pub enable_wag_boost: bool,

    /// Minimum E-Score to allow transaction (0 = no filter)
    pub min_e_score: f64,

    /// φ multiplier for WAG transactions
    pub wag_multiplier: f64,

    /// φ⁻¹ multiplier for BARK transactions
    pub bark_multiplier: f64,

    /// Shared memory region name for TPU->Pack queue
    pub tpu_to_pack_shm: String,

    /// Shared memory region name for Pack->Worker queues
    pub pack_to_worker_shm_prefix: String,

    /// Shared memory region name for Worker->Pack queues
    pub worker_to_pack_shm_prefix: String,

    /// Shared memory region name for progress tracker
    pub progress_shm: String,

    /// Log level
    pub log_level: String,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            cynic_url: "https://cynic-mcp.onrender.com".to_string(),
            cynic_api_key: None,
            max_queue_size: 100_000,
            batch_size: 64,
            num_workers: 4,
            api_timeout: Duration::from_millis(100),
            reputation_cache_ttl: Duration::from_secs(60),
            enable_growl_filter: true,
            enable_wag_boost: true,
            min_e_score: 0.0,
            wag_multiplier: PHI,
            bark_multiplier: PHI_INV,
            tpu_to_pack_shm: "/cynic_tpu_to_pack".to_string(),
            pack_to_worker_shm_prefix: "/cynic_pack_to_worker_".to_string(),
            worker_to_pack_shm_prefix: "/cynic_worker_to_pack_".to_string(),
            progress_shm: "/cynic_progress".to_string(),
            log_level: "info".to_string(),
        }
    }
}

impl SchedulerConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(url) = std::env::var("CYNIC_URL") {
            config.cynic_url = url;
        }

        if let Ok(key) = std::env::var("CYNIC_API_KEY") {
            config.cynic_api_key = Some(key);
        }

        if let Ok(size) = std::env::var("CYNIC_MAX_QUEUE_SIZE") {
            if let Ok(n) = size.parse() {
                config.max_queue_size = n;
            }
        }

        if let Ok(size) = std::env::var("CYNIC_BATCH_SIZE") {
            if let Ok(n) = size.parse() {
                config.batch_size = n;
            }
        }

        if let Ok(n) = std::env::var("CYNIC_NUM_WORKERS") {
            if let Ok(workers) = n.parse() {
                config.num_workers = workers;
            }
        }

        if let Ok(ms) = std::env::var("CYNIC_API_TIMEOUT_MS") {
            if let Ok(timeout) = ms.parse() {
                config.api_timeout = Duration::from_millis(timeout);
            }
        }

        if let Ok(val) = std::env::var("CYNIC_ENABLE_GROWL_FILTER") {
            config.enable_growl_filter = val == "true" || val == "1";
        }

        if let Ok(val) = std::env::var("CYNIC_ENABLE_WAG_BOOST") {
            config.enable_wag_boost = val == "true" || val == "1";
        }

        if let Ok(score) = std::env::var("CYNIC_MIN_E_SCORE") {
            if let Ok(s) = score.parse() {
                config.min_e_score = s;
            }
        }

        if let Ok(level) = std::env::var("CYNIC_LOG_LEVEL") {
            config.log_level = level;
        }

        config
    }

    /// Validate configuration
    pub fn validate(&self) -> crate::Result<()> {
        if self.max_queue_size == 0 {
            return Err(crate::SchedulerError::config("max_queue_size must be > 0"));
        }

        if self.batch_size == 0 {
            return Err(crate::SchedulerError::config("batch_size must be > 0"));
        }

        if self.num_workers == 0 {
            return Err(crate::SchedulerError::config("num_workers must be > 0"));
        }

        if self.wag_multiplier <= 0.0 {
            return Err(crate::SchedulerError::config("wag_multiplier must be > 0"));
        }

        if self.bark_multiplier <= 0.0 || self.bark_multiplier >= 1.0 {
            return Err(crate::SchedulerError::config(
                "bark_multiplier must be in (0, 1)",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SchedulerConfig::default();
        assert!(config.validate().is_ok());
        assert!((config.wag_multiplier - PHI).abs() < 1e-10);
        assert!((config.bark_multiplier - PHI_INV).abs() < 1e-10);
    }

    #[test]
    fn test_invalid_config() {
        let mut config = SchedulerConfig::default();
        config.max_queue_size = 0;
        assert!(config.validate().is_err());
    }
}
