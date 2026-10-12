//! Pure agent-trajectory entropy-rate classification.
//!
//! A degenerate agent loop is a low-entropy repetition: the same token
//! distribution keeps collapsing onto a single outcome. This module consumes
//! already-observed trajectory windows as probability distributions and uses
//! the canonical `cynic_protocol::information` entropy-rate primitive.
//!
//! Unknown is data: no window, an invalid window, or too few windows is
//! `Unobservable`, never `Normal`.

use serde::{Deserialize, Serialize};

use cynic_protocol::information;

pub const TRAJECTORY_ENTROPY_REPORT_SCHEMA: &str = "cynic.host_trajectory_entropy.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrajectoryEntropyLevel {
    /// Enough valid windows and entropy at or above the configured threshold.
    Normal,
    /// Enough valid windows, but entropy rate is below the degenerate threshold.
    DegenerateLowEntropy,
    /// Data missing, invalid, or insufficient; never fabricated as healthy.
    Unobservable,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryEntropyParams {
    /// Entropy rate below this level is classified as degenerate.
    pub min_entropy_bits: f64,
    /// Minimum number of valid windows required for a classification.
    pub min_windows: usize,
}

impl Default for TrajectoryEntropyParams {
    fn default() -> Self {
        Self {
            min_entropy_bits: 0.25,
            min_windows: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryEntropyReport {
    pub schema: String,
    pub windows_observed: usize,
    pub entropy_rate_bits: Option<f64>,
    pub level: TrajectoryEntropyLevel,
    pub params: TrajectoryEntropyParams,
    pub residual_unknowns: Vec<String>,
}

/// Classify a sequence of trajectory windows.
///
/// Each window is a probability distribution over tokens or symbolic outcomes.
/// The entropy rate is the arithmetic mean of the per-window Shannon entropies.
pub fn classify_trajectory_entropy(
    windows: &[&[f64]],
    params: TrajectoryEntropyParams,
) -> TrajectoryEntropyReport {
    let windows_observed = windows.len();
    let mut residual_unknowns = Vec::new();
    let entropy_rate_bits = if windows.is_empty() {
        residual_unknowns.push("no_trajectory_windows_observed".to_string());
        None
    } else {
        match information::entropy_rate_bits(windows) {
            Ok(rate) => Some(rate),
            Err(error) => {
                residual_unknowns.push(format!("invalid_trajectory_distribution:{error}"));
                None
            }
        }
    };

    let level = match entropy_rate_bits {
        Some(rate) if windows_observed >= params.min_windows => {
            if rate < params.min_entropy_bits {
                TrajectoryEntropyLevel::DegenerateLowEntropy
            } else {
                TrajectoryEntropyLevel::Normal
            }
        }
        _ => TrajectoryEntropyLevel::Unobservable,
    };

    if entropy_rate_bits.is_some() && windows_observed < params.min_windows {
        residual_unknowns.push("insufficient_windows".to_string());
    }

    TrajectoryEntropyReport {
        schema: TRAJECTORY_ENTROPY_REPORT_SCHEMA.to_string(),
        windows_observed,
        entropy_rate_bits,
        level,
        params,
        residual_unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_high_entropy_is_normal() {
        let window = [0.5, 0.5];
        let windows = [window.as_slice(); 3];
        let report = classify_trajectory_entropy(&windows, TrajectoryEntropyParams::default());
        assert_eq!(report.level, TrajectoryEntropyLevel::Normal);
        assert_eq!(report.entropy_rate_bits, Some(1.0));
        assert!(report.residual_unknowns.is_empty());
    }

    #[test]
    fn low_entropy_repetition_is_degenerate() {
        let window = [1.0, 0.0];
        let windows = [window.as_slice(); 3];
        let report = classify_trajectory_entropy(&windows, TrajectoryEntropyParams::default());
        assert_eq!(report.level, TrajectoryEntropyLevel::DegenerateLowEntropy);
        assert_eq!(report.entropy_rate_bits, Some(0.0));
    }

    #[test]
    fn no_windows_is_unobservable() {
        let report = classify_trajectory_entropy(&[], TrajectoryEntropyParams::default());
        assert_eq!(report.level, TrajectoryEntropyLevel::Unobservable);
        assert!(report.entropy_rate_bits.is_none());
        assert!(report
            .residual_unknowns
            .contains(&"no_trajectory_windows_observed".to_string()));
    }

    #[test]
    fn invalid_distribution_is_unobservable() {
        let window = [0.5, 0.4];
        let windows = [window.as_slice(); 3];
        let report = classify_trajectory_entropy(&windows, TrajectoryEntropyParams::default());
        assert_eq!(report.level, TrajectoryEntropyLevel::Unobservable);
        assert!(report.entropy_rate_bits.is_none());
        assert!(report
            .residual_unknowns
            .iter()
            .any(|u| u.starts_with("invalid_trajectory_distribution:")));
    }

    #[test]
    fn too_few_windows_is_unobservable() {
        let window = [0.5, 0.5];
        let windows = [window.as_slice()];
        let report = classify_trajectory_entropy(&windows, TrajectoryEntropyParams::default());
        assert_eq!(report.level, TrajectoryEntropyLevel::Unobservable);
        assert_eq!(report.windows_observed, 1);
        assert!(report
            .residual_unknowns
            .contains(&"insufficient_windows".to_string()));
    }
}
