//! Helius credit tracking — observability for RPC API budget management.
//!
//! Tracks per-session and aggregate Helius API calls:
//! - API calls: method, status, latency
//! - Estimated credits consumed (10 per DAS API call)
//! - Budget alerts when credit burn is high

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeliusCallMetric {
    /// RPC method (getAsset, getTokenLargestAccounts, etc.)
    pub method: String,
    /// HTTP status code (200, 429, 402, etc.)
    pub status: u16,
    /// Latency in milliseconds
    pub latency_ms: u128,
    /// Estimated credits consumed
    pub credits_cost: u32,
    /// Timestamp
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeliumsCreditBudget {
    /// Total API calls made this session
    pub total_calls: u64,
    /// Failed calls (non-200 status)
    pub failed_calls: u64,
    /// Estimated total credits consumed
    pub total_credits_consumed: u64,
    /// Average latency per call (ms)
    pub avg_latency_ms: u128,
    /// Sum of all latencies (for averaging)
    pub sum_latencies_ms: u128,
    /// Session start timestamp
    pub session_start: String,
}

impl HeliumsCreditBudget {
    pub fn record_call(&mut self, latency_ms: u128, success: bool, credits: u32) {
        self.total_calls += 1;
        if !success {
            self.failed_calls += 1;
        }
        self.sum_latencies_ms += latency_ms;
        self.total_credits_consumed += credits as u64;

        if self.total_calls > 0 {
            self.avg_latency_ms = self.sum_latencies_ms / (self.total_calls as u128);
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_calls == 0 {
            return 1.0;
        }
        ((self.total_calls - self.failed_calls) as f64) / (self.total_calls as f64)
    }
}

#[derive(Debug)]
pub struct HeliumsCreditTracker {
    budget: Arc<Mutex<HeliumsCreditBudget>>,
}

impl HeliumsCreditTracker {
    pub fn new(session_start: String) -> Self {
        Self {
            budget: Arc::new(Mutex::new(HeliumsCreditBudget {
                session_start,
                ..Default::default()
            })),
        }
    }

    pub fn record_call(&self, latency_ms: u128, success: bool, credits: u32) {
        if let Ok(mut budget) = self.budget.lock() {
            budget.record_call(latency_ms, success, credits);
        }
    }

    pub fn snapshot(&self) -> Option<HeliumsCreditBudget> {
        self.budget.lock().ok().map(|b| b.clone())
    }
}
