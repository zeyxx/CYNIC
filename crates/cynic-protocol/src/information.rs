//! Shannon information-theoretic primitives for CYNIC.
//!
//! Modality-agnostic: callers reduce any modality to a discrete probability
//! distribution; this module only computes entropy, mutual information, and
//! divergences in bits. Pure, no I/O.

use std::vec::Vec;
use core::fmt;

pub const INFORMATION_SCHEMA: &str = "cynic.information.v1";

#[derive(Debug, Clone, PartialEq)]
pub enum EntropyError {
    EmptyDistribution,
    NegativeProbability { index: usize },
    NonFiniteProbability { index: usize },
    ProbabilitySumOutOfRange { sum: f64 },
    LengthMismatch { lhs: usize, rhs: usize },
    ShapeMismatch { expected: usize, actual: usize },
    UnsupportedEntropyOrder { alpha: f64 },
    KlDivergenceDivergent { index: usize },
    NoObservations,
    InsufficientBuckets { buckets: usize },
}

impl fmt::Display for EntropyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDistribution => f.write_str("empty_distribution"),
            Self::NegativeProbability { index } => write!(f, "negative_probability_at_{index}"),
            Self::NonFiniteProbability { index } => write!(f, "non_finite_probability_at_{index}"),
            Self::ProbabilitySumOutOfRange { sum } => write!(f, "probability_sum_out_of_range:{sum}"),
            Self::LengthMismatch { lhs, rhs } => write!(f, "length_mismatch:{lhs}_vs_{rhs}"),
            Self::ShapeMismatch { expected, actual } => write!(f, "shape_mismatch:expected_{expected}_got_{actual}"),
            Self::UnsupportedEntropyOrder { alpha } => write!(f, "unsupported_entropy_order:{alpha}"),
            Self::KlDivergenceDivergent { index } => write!(f, "kl_divergence_divergent_at_{index}"),
            Self::NoObservations => f.write_str("no_observations"),
            Self::InsufficientBuckets { buckets } => write!(f, "insufficient_buckets:{buckets}"),
        }
    }
}

const LN_2: f64 = 0.693_147_180_559_945_3;
const SUM_TOLERANCE: f64 = 1e-9;

pub fn validate_distribution(probabilities: &[f64]) -> Result<(), EntropyError> {
    if probabilities.is_empty() {
        return Err(EntropyError::EmptyDistribution);
    }
    let mut sum = 0.0_f64;
    for (index, value) in probabilities.iter().enumerate() {
        if !value.is_finite() {
            return Err(EntropyError::NonFiniteProbability { index });
        }
        if *value < 0.0 {
            return Err(EntropyError::NegativeProbability { index });
        }
        sum += value;
    }
    if (sum - 1.0).abs() > SUM_TOLERANCE {
        return Err(EntropyError::ProbabilitySumOutOfRange { sum });
    }
    Ok(())
}

pub fn shannon_entropy_bits(probabilities: &[f64]) -> Result<f64, EntropyError> {
    validate_distribution(probabilities)?;
    let mut entropy = 0.0_f64;
    for value in probabilities {
        if *value > 0.0 {
            entropy -= value * (value.ln() / LN_2);
        }
    }
    if entropy < 0.0 && entropy > -1e-12 {
        entropy = 0.0;
    }
    Ok(entropy)
}

pub fn shannon_entropy_nats(probabilities: &[f64]) -> Result<f64, EntropyError> {
    validate_distribution(probabilities)?;
    let mut entropy = 0.0_f64;
    for value in probabilities {
        if *value > 0.0 {
            entropy -= value * value.ln();
        }
    }
    Ok(entropy)
}

pub fn joint_entropy_bits(joint: &[f64], rows: usize, cols: usize) -> Result<f64, EntropyError> {
    if rows == 0 || cols == 0 || rows.saturating_mul(cols) != joint.len() {
        return Err(EntropyError::ShapeMismatch {
            expected: rows.saturating_mul(cols),
            actual: joint.len(),
        });
    }
    shannon_entropy_bits(joint)
}

pub fn marginals_from_joint(joint: &[f64], rows: usize, cols: usize) -> Result<(Vec<f64>, Vec<f64>), EntropyError> {
    if rows == 0 || cols == 0 || rows.saturating_mul(cols) != joint.len() {
        return Err(EntropyError::ShapeMismatch {
            expected: rows.saturating_mul(cols),
            actual: joint.len(),
        });
    }
    validate_distribution(joint)?;
    let mut row_marginal = vec![0.0_f64; rows];
    let mut col_marginal = vec![0.0_f64; cols];
    for row in 0..rows {
        for col in 0..cols {
            let value = joint[row * cols + col];
            row_marginal[row] += value;
            col_marginal[col] += value;
        }
    }
    Ok((row_marginal, col_marginal))
}

pub fn conditional_entropy_bits(joint: &[f64], rows: usize, cols: usize) -> Result<f64, EntropyError> {
    let (_, col_marginal) = marginals_from_joint(joint, rows, cols)?;
    let joint_h = shannon_entropy_bits(joint)?;
    let y_h = shannon_entropy_bits(&col_marginal)?;
    Ok(joint_h - y_h)
}

pub fn mutual_information_bits(joint: &[f64], rows: usize, cols: usize) -> Result<f64, EntropyError> {
    let (row_marginal, col_marginal) = marginals_from_joint(joint, rows, cols)?;
    let h_x = shannon_entropy_bits(&row_marginal)?;
    let h_y = shannon_entropy_bits(&col_marginal)?;
    let h_xy = shannon_entropy_bits(joint)?;
    let value = h_x + h_y - h_xy;
    if value < 0.0 && value > -1e-12 {
        return Ok(0.0);
    }
    Ok(value)
}

pub fn kl_divergence_bits(p: &[f64], q: &[f64]) -> Result<f64, EntropyError> {
    if p.len() != q.len() {
        return Err(EntropyError::LengthMismatch { lhs: p.len(), rhs: q.len() });
    }
    validate_distribution(p)?;
    validate_distribution(q)?;
    let mut divergence = 0.0_f64;
    for (index, (pi, qi)) in p.iter().zip(q.iter()).enumerate() {
        if *pi > 0.0 {
            if *qi <= 0.0 {
                return Err(EntropyError::KlDivergenceDivergent { index });
            }
            divergence += pi * (pi / qi).ln() / LN_2;
        }
    }
    if divergence < 0.0 && divergence > -1e-12 {
        divergence = 0.0;
    }
    Ok(divergence)
}

pub fn renyi_entropy_bits(probabilities: &[f64], alpha: f64) -> Result<f64, EntropyError> {
    validate_distribution(probabilities)?;
    if !alpha.is_finite() || alpha < 0.0 || (alpha - 1.0).abs() < 1e-12 {
        return Err(EntropyError::UnsupportedEntropyOrder { alpha });
    }
    if alpha == 0.0 {
        let support = probabilities.iter().filter(|v| **v > 0.0).count();
        if support == 0 {
            return Err(EntropyError::EmptyDistribution);
        }
        return Ok((support as f64).log2());
    }
    if alpha > 1e6 {
        let max_p = probabilities.iter().copied().fold(0.0_f64, f64::max);
        if max_p <= 0.0 {
            return Err(EntropyError::EmptyDistribution);
        }
        return Ok(-(max_p.log2()));
    }
    let inv = 1.0 / (1.0 - alpha);
    let mut sum = 0.0_f64;
    for value in probabilities {
        if *value > 0.0 {
            sum += value.powf(alpha);
        }
    }
    Ok(inv * (sum.ln() / LN_2))
}

pub fn min_entropy_bits(probabilities: &[f64]) -> Result<f64, EntropyError> {
    validate_distribution(probabilities)?;
    let max_p = probabilities.iter().copied().fold(0.0_f64, f64::max);
    if max_p <= 0.0 {
        return Err(EntropyError::EmptyDistribution);
    }
    Ok(-(max_p.log2()))
}

pub fn max_entropy_bits(support_size: usize) -> Result<f64, EntropyError> {
    if support_size == 0 {
        return Err(EntropyError::EmptyDistribution);
    }
    Ok((support_size as f64).log2())
}

pub fn consensus_entropy_bits(counts: &[usize]) -> Result<f64, EntropyError> {
    let total: usize = counts.iter().sum();
    if total == 0 {
        return Err(EntropyError::NoObservations);
    }
    let probabilities: Vec<f64> = counts.iter().map(|c| *c as f64 / total as f64).collect();
    shannon_entropy_bits(&probabilities)
}

pub fn consensus_confidence_modifier(counts: &[usize], buckets: usize) -> Result<f64, EntropyError> {
    if buckets < 2 {
        return Err(EntropyError::InsufficientBuckets { buckets });
    }
    let total: usize = counts.iter().sum();
    if total == 0 {
        return Err(EntropyError::NoObservations);
    }
    let h = consensus_entropy_bits(counts)?;
    let max_h = (buckets as f64).log2();
    let gamma = 1.0 - (h / max_h);
    if gamma < 0.0 { return Ok(0.0); }
    if gamma > 1.0 { return Ok(1.0); }
    Ok(gamma)
}

pub fn entropy_rate_bits(windows: &[&[f64]]) -> Result<f64, EntropyError> {
    if windows.is_empty() {
        return Err(EntropyError::EmptyDistribution);
    }
    let mut sum = 0.0_f64;
    for window in windows {
        sum += shannon_entropy_bits(window)?;
    }
    Ok(sum / windows.len() as f64)
}

pub fn typicality_bits(probability: f64, n: usize) -> Result<f64, EntropyError> {
    if n == 0 {
        return Err(EntropyError::EmptyDistribution);
    }
    if !probability.is_finite() || probability <= 0.0 {
        return Err(EntropyError::NonFiniteProbability { index: 0 });
    }
    Ok(-(probability.log2()) / n as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn approx(a: f64, b: f64) -> bool { (a - b).abs() < 1e-9 }

    #[test]
    fn uniform_binary_entropy_is_one_bit() {
        assert!(approx(shannon_entropy_bits(&[0.5, 0.5]).unwrap(), 1.0));
    }

    #[test]
    fn deterministic_distribution_has_zero_entropy() {
        assert!(approx(shannon_entropy_bits(&[1.0, 0.0]).unwrap(), 0.0));
    }

    #[test]
    fn validation_rejects_bad_inputs() {
        assert!(shannon_entropy_bits(&[0.5, 0.4]).is_err());
        assert!(shannon_entropy_bits(&[1.1, -0.1]).is_err());
        assert!(shannon_entropy_bits(&[]).is_err());
    }

    #[test]
    fn mutual_information_independent_is_zero() {
        let joint = [0.25, 0.25, 0.25, 0.25];
        assert!(approx(mutual_information_bits(&joint, 2, 2).unwrap(), 0.0));
    }

    #[test]
    fn mutual_information_perfect_correlation() {
        let joint = [0.5, 0.0, 0.0, 0.5];
        assert!(approx(mutual_information_bits(&joint, 2, 2).unwrap(), 1.0));
        assert!(approx(conditional_entropy_bits(&joint, 2, 2).unwrap(), 0.0));
    }

    #[test]
    fn kl_identical_is_zero_and_asymmetric() {
        let p = [0.9, 0.1];
        let q = [0.5, 0.5];
        assert!(approx(kl_divergence_bits(&p, &p).unwrap(), 0.0));
        assert!(kl_divergence_bits(&p, &q).unwrap() != kl_divergence_bits(&q, &p).unwrap());
    }

    #[test]
    fn kl_reports_divergent_support() {
        let p = [0.5, 0.5];
        let q = [1.0, 0.0];
        assert!(matches!(kl_divergence_bits(&p, &q).unwrap_err(), EntropyError::KlDivergenceDivergent { index: 1 }));
    }

    #[test]
    fn renyi_orders_monotone() {
        let p = [0.5, 0.25, 0.25];
        let h0 = renyi_entropy_bits(&p, 0.0).unwrap();
        let h1 = shannon_entropy_bits(&p).unwrap();
        let h2 = renyi_entropy_bits(&p, 2.0).unwrap();
        let hinf = min_entropy_bits(&p).unwrap();
        assert!(h0 >= h1 - 1e-9);
        assert!(h1 >= h2 - 1e-9);
        assert!(h2 >= hinf - 1e-9);
    }

    #[test]
    fn consensus_entropy_and_modifier() {
        let counts = [4usize, 0, 0];
        assert!(approx(consensus_entropy_bits(&counts).unwrap(), 0.0));
        assert!(approx(consensus_confidence_modifier(&counts, 3).unwrap(), 1.0));

        let counts2 = [1usize, 1, 1];
        assert!(approx(consensus_confidence_modifier(&counts2, 3).unwrap(), 0.0));
    }

    #[test]
    fn entropy_rate_and_typicality() {
        let w1 = [0.5, 0.5];
        let w2 = [1.0, 0.0];
        let windows = [w1.as_slice(), w2.as_slice()];
        assert!(approx(entropy_rate_bits(&windows).unwrap(), 0.5));
        assert!(approx(typicality_bits(0.25, 2).unwrap(), 1.0));
    }

    #[test]
    fn max_entropy_support() {
        assert!(approx(max_entropy_bits(8).unwrap(), 3.0));
        assert!(max_entropy_bits(0).is_err());
    }
}
