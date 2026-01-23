//! CYNIC Scheduler - φ-weighted transaction scheduling for Solana
//!
//! Implements an external scheduler for Agave validators that integrates
//! CYNIC reputation scores (K-Score, E-Score) into transaction prioritization.
//!
//! # Architecture
//!
//! ```text
//! ┌───────────────┐       ┌─────────────────┐
//! │  tpu_to_pack  │       │ progress_tracker│
//! └───────┬───────┘       └───────┬─────────┘
//!         │                       │
//!         ▼                       ▼
//!     ┌───────────────────────────────┐
//!     │      CYNIC SCHEDULER          │
//!     │  - φ-weighted prioritization  │
//!     │  - Reputation lookup          │
//!     │  - GROWL filtering            │
//!     └─▲─────────▲───────────────▲───┘
//!       │         │               │
//!    ┌──▼──┐   ┌──▼──┐        ┌───▼──┐
//!    │wrkr1│   │wrkr2│  ...   │wrkrN │
//!    └─────┘   └─────┘        └──────┘
//! ```
//!
//! # φ Constants
//!
//! - PHI (φ) = 1.618033988749895
//! - PHI_INV (φ⁻¹) = 0.618033988749895
//! - Max confidence = 61.8%

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod config;
pub mod cynic_client;
pub mod error;
pub mod priority;
pub mod scheduler;

// Re-exports
pub use config::SchedulerConfig;
pub use cynic_client::CynicClient;
pub use error::{SchedulerError, Result};
pub use priority::{PriorityQueue, TransactionPriority};
pub use scheduler::{CynicScheduler, SchedulerState, SchedulerStats};

/// φ (Golden Ratio)
pub const PHI: f64 = 1.618033988749895;

/// φ⁻¹ (Inverse Golden Ratio) - CYNIC max confidence
pub const PHI_INV: f64 = 0.618033988749895;

/// φ² for higher boosts
pub const PHI_SQUARED: f64 = 2.618033988749895;

/// CYNIC Verdict types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    /// Excellent - boost priority by φ
    Wag,
    /// Good - standard priority
    Howl,
    /// Suspicious - reduce priority by φ⁻¹
    Bark,
    /// Dangerous - drop transaction
    Growl,
}

impl Verdict {
    /// Get priority multiplier for this verdict
    pub fn multiplier(&self) -> f64 {
        match self {
            Verdict::Wag => PHI,
            Verdict::Howl => 1.0,
            Verdict::Bark => PHI_INV,
            Verdict::Growl => 0.0,
        }
    }

    /// Check if transaction should be dropped
    pub fn should_drop(&self) -> bool {
        matches!(self, Verdict::Growl)
    }
}

/// Reputation score from CYNIC
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ReputationScore {
    /// Q-Score (0-100)
    pub q_score: f64,
    /// Verdict
    pub verdict: Verdict,
    /// K-Score for tokens (optional)
    pub k_score: Option<f64>,
    /// E-Score for wallets (optional)
    pub e_score: Option<f64>,
    /// Confidence (max 61.8%)
    pub confidence: f64,
}

impl Default for ReputationScore {
    fn default() -> Self {
        Self {
            q_score: 50.0,
            verdict: Verdict::Howl,
            k_score: None,
            e_score: None,
            confidence: PHI_INV * 100.0, // 61.8%
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phi_constants() {
        assert!((PHI * PHI_INV - 1.0).abs() < 1e-10);
        assert!((PHI - 1.0 - PHI_INV).abs() < 1e-10);
    }

    #[test]
    fn test_verdict_multipliers() {
        assert!((Verdict::Wag.multiplier() - PHI).abs() < 1e-10);
        assert!((Verdict::Howl.multiplier() - 1.0).abs() < 1e-10);
        assert!((Verdict::Bark.multiplier() - PHI_INV).abs() < 1e-10);
        assert!((Verdict::Growl.multiplier() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_verdict_drop() {
        assert!(!Verdict::Wag.should_drop());
        assert!(!Verdict::Howl.should_drop());
        assert!(!Verdict::Bark.should_drop());
        assert!(Verdict::Growl.should_drop());
    }
}
