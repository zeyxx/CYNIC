//! Token-level uncertainty entropy for inference judge calibration.
//!
//! This module turns raw logits or already-normalized probabilities into a
//! Shannon entropy measurement. The primitive entropy calculation lives in
//! `cynic_protocol::information`; this module adds the inference-domain
//! vocabulary: one token snapshot, a sequence assessment, and explicit
//! uncertainty levels.
//!
//! Unknown is data: empty, non-finite, or invalid input yields an
//! `Unobservable` assessment rather than a fabricated confidence.

use cynic_protocol::information::{self, EntropyError};
use serde::{Deserialize, Serialize};
use std::vec::Vec;

pub const TOKEN_ENTROPY_SNAPSHOT_SCHEMA: &str = "cynic.inference_token_entropy.v1";
pub const TOKEN_UNCERTAINTY_ASSESSMENT_SCHEMA: &str = "cynic.inference_token_uncertainty.v1";

/// One token-position uncertainty measurement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenEntropySnapshot {
    pub token_index: usize,
    /// Shannon entropy of the token distribution, in bits.
    pub entropy_bits: f64,
    /// Probability of the most likely token.
    pub top_probability: f64,
    /// Index of the most likely token.
    pub top_token: u32,
}

/// Sequence-level uncertainty classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenUncertaintyLevel {
    /// Model is frequently uncertain: low mean top-token probability.
    Exploration,
    /// Uncertainty is between exploration and exploitation thresholds.
    Balanced,
    /// Model is frequently confident: high mean top-token probability.
    Exploitation,
    /// No usable snapshots; uncertainty is unknown.
    Unobservable,
}

/// Thresholds for sequence-level uncertainty classification.
///
/// Thresholds operate on mean top-token probability so the classification does
/// not assume a vocabulary size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TokenUncertaintyParams {
    pub exploration_top_probability: f64,
    pub exploitation_top_probability: f64,
}

impl Default for TokenUncertaintyParams {
    fn default() -> Self {
        Self {
            exploration_top_probability: 0.6,
            exploration_top_probability: 0.5,
            exploitation_top_probability: 0.75,
        }
    }
}

/// Sequence-level uncertainty assessment for judge calibration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenUncertaintyAssessment {
    pub schema: String,
    pub snapshots_observed: usize,
    pub mean_entropy_bits: Option<f64>,
    pub mean_top_probability: Option<f64>,
    pub level: TokenUncertaintyLevel,
    pub params: TokenUncertaintyParams,
    pub residual_unknowns: Vec<String>,
}

/// Shannon entropy and top-token measurement for one token position.
pub fn token_entropy_from_probabilities(
    probabilities: &[f64],
    token_index: usize,
) -> Result<TokenEntropySnapshot, EntropyError> {
    let entropy_bits = information::shannon_entropy_bits(probabilities)?;
    let mut max_probability = f64::NEG_INFINITY;
    let mut top_token = u32::MAX;
    for (index, value) in probabilities.iter().enumerate() {
        if *value > max_probability {
            max_probability = *value;
            top_token = index as u32;
        }
    }

    Ok(TokenEntropySnapshot {
        token_index,
        entropy_bits,
        top_probability: max_probability,
        top_token,
    })
}

/// Shannon entropy and top-token measurement from raw logits.
///
/// Logits are converted to a softmax distribution with the max-logit shift for
/// numerical stability. The entropy itself is delegated to the canonical
/// `cynic_protocol::information` primitive.
pub fn token_entropy_from_logits(
    logits: &[f64],
    token_index: usize,
) -> Result<TokenEntropySnapshot, EntropyError> {
    if logits.is_empty() {
        return Err(EntropyError::EmptyDistribution);
    }
    for (index, value) in logits.iter().enumerate() {
        if !value.is_finite() {
            return Err(EntropyError::NonFiniteProbability { index });
        }
    }
    let max_logit = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !max_logit.is_finite() {
        for (index, value) in logits.iter().enumerate() {
            if !value.is_finite() {
                return Err(EntropyError::NonFiniteProbability { index });
            }
        }
        return Err(EntropyError::EmptyDistribution);
    }

    let mut exps = Vec::with_capacity(logits.len());
    let mut exp_sum = 0.0_f64;
    for value in logits {
        let exp = (value - max_logit).exp();
        exps.push(exp);
        exp_sum += exp;
    }
    if exp_sum <= 0.0 || !exp_sum.is_finite() {
        return Err(EntropyError::ProbabilitySumOutOfRange { sum: exp_sum });
    }
    let probabilities: Vec<f64> = exps.iter().map(|value| value / exp_sum).collect();
    token_entropy_from_probabilities(&probabilities, token_index)
}

/// Assess a sequence of token snapshots for judge calibration.
pub fn assess_token_uncertainty(
    snapshots: &[TokenEntropySnapshot],
    params: TokenUncertaintyParams,
) -> TokenUncertaintyAssessment {
    let mut residual_unknowns = Vec::new();
    if snapshots.is_empty() {
        residual_unknowns.push("no_token_snapshots_observed".to_string());
        return TokenUncertaintyAssessment {
            schema: TOKEN_UNCERTAINTY_ASSESSMENT_SCHEMA.to_string(),
            snapshots_observed: 0,
            mean_entropy_bits: None,
            mean_top_probability: None,
            level: TokenUncertaintyLevel::Unobservable,
            params,
            residual_unknowns,
        };
    }

    for snapshot in snapshots {
        if !snapshot.entropy_bits.is_finite()
            || snapshot.entropy_bits < 0.0
            || !snapshot.top_probability.is_finite()
            || snapshot.top_probability < 0.0
            || snapshot.top_probability > 1.0
        {
            residual_unknowns.push("invalid_token_snapshot".to_string());
            return TokenUncertaintyAssessment {
                schema: TOKEN_UNCERTAINTY_ASSESSMENT_SCHEMA.to_string(),
                snapshots_observed: snapshots.len(),
                mean_entropy_bits: None,
                mean_top_probability: None,
                level: TokenUncertaintyLevel::Unobservable,
                params,
                residual_unknowns,
            };
        }
    }

    let mean_entropy_bits = snapshots
        .iter()
        .map(|snapshot| snapshot.entropy_bits)
        .sum::<f64>()
        / snapshots.len() as f64;
    let mean_top_probability = snapshots
        .iter()
        .map(|snapshot| snapshot.top_probability)
        .sum::<f64>()
        / snapshots.len() as f64;

    let level = if mean_top_probability < params.exploration_top_probability {
        TokenUncertaintyLevel::Exploration
    } else if mean_top_probability > params.exploitation_top_probability {
    let level = if mean_top_probability <= params.exploration_top_probability {
        TokenUncertaintyLevel::Exploration
    } else if mean_top_probability >= params.exploitation_top_probability {
        TokenUncertaintyLevel::Exploitation
    } else {
        TokenUncertaintyLevel::Balanced
    };

    TokenUncertaintyAssessment {
        schema: TOKEN_UNCERTAINTY_ASSESSMENT_SCHEMA.to_string(),
        snapshots_observed: snapshots.len(),
        mean_entropy_bits: Some(mean_entropy_bits),
        mean_top_probability: Some(mean_top_probability),
        level,
        params,
        residual_unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_two_token_entropy_is_one_bit() {
        let snapshot = token_entropy_from_probabilities(&[0.5, 0.5], 0).unwrap();
        assert_eq!(snapshot.entropy_bits, 1.0);
        assert_eq!(snapshot.top_probability, 0.5);
        assert_eq!(snapshot.top_token, 0);
    }

    #[test]
    fn one_hot_logits_have_zero_entropy() {
        let snapshot = token_entropy_from_logits(&[10.0, 0.0, -5.0], 0).unwrap();
        assert!(snapshot.entropy_bits < 1e-3);
        assert!(snapshot.top_probability > 0.999);
        assert_eq!(snapshot.top_token, 0);
    }

    #[test]
    fn uniform_logits_produce_uniform_probabilities() {
        let snapshot = token_entropy_from_logits(&[0.0, 0.0], 1).unwrap();
        assert_eq!(snapshot.entropy_bits, 1.0);
        assert_eq!(snapshot.top_probability, 0.5);
        assert_eq!(snapshot.token_index, 1);
    }

    #[test]
    fn non_finite_logits_are_rejected() {
        let error = token_entropy_from_logits(&[0.0, f64::NAN], 0).unwrap_err();
        assert!(matches!(
            error,
            EntropyError::NonFiniteProbability { index: 1 }
        ));
    }

    #[test]
    fn empty_snapshots_are_unobservable() {
        let assessment = assess_token_uncertainty(&[], TokenUncertaintyParams::default());
        assert_eq!(assessment.level, TokenUncertaintyLevel::Unobservable);
        assert_eq!(assessment.snapshots_observed, 0);
        assert!(assessment
            .residual_unknowns
            .contains(&"no_token_snapshots_observed".to_string()));
        assert!(
            assessment
                .residual_unknowns
                .contains(&"no_token_snapshots_observed".to_string())
        );
    }

    #[test]
    fn high_uncertainty_is_exploration() {
        let snapshot = token_entropy_from_probabilities(&[0.5, 0.5], 0).unwrap();
        let assessment = assess_token_uncertainty(&[snapshot], TokenUncertaintyParams::default());
        assert_eq!(assessment.level, TokenUncertaintyLevel::Exploration);
        assert_eq!(assessment.mean_entropy_bits, Some(1.0));
    }

    #[test]
    fn high_confidence_is_exploitation() {
        let snapshot = token_entropy_from_probabilities(&[1.0, 0.0], 0).unwrap();
        let assessment = assess_token_uncertainty(&[snapshot], TokenUncertaintyParams::default());
        assert_eq!(assessment.level, TokenUncertaintyLevel::Exploitation);
        assert_eq!(assessment.mean_top_probability, Some(1.0));
    }
}
