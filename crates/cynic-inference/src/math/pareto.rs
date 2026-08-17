//! Multi-objective inference selection through Pareto dominance.
//!
//! The frontier contains every configuration that is not strictly worse than
//! another observed configuration. No absent metric is imputed. A single
//! "balanced" choice uses a documented, equal-regret projection only after the
//! frontier is known.

use serde::{Deserialize, Serialize};

/// Measured objective vector. Throughput and context are maximized; TTFT and
/// jitter variance are minimized. Quality remains unknown until benchmarked.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveVector {
    pub throughput_tps: f32,
    pub ttft_ms: f32,
    pub jitter_variance_ms: f32,
    pub context_tokens: u32,
    pub quality_score: Option<f32>,
}

impl ObjectiveVector {
    fn has_valid_core_metrics(&self) -> bool {
        self.throughput_tps.is_finite()
            && self.ttft_ms.is_finite()
            && self.jitter_variance_ms.is_finite()
            && self.throughput_tps >= 0.0
            && self.ttft_ms >= 0.0
            && self.jitter_variance_ms >= 0.0
    }

    /// Returns true when every comparison metric is observed and `self` is at
    /// least as good in every dimension, strictly better in one.
    pub fn dominates(&self, other: &Self) -> bool {
        if !self.has_valid_core_metrics() || !other.has_valid_core_metrics() {
            return false;
        }

        let core_not_worse = self.throughput_tps >= other.throughput_tps
            && self.ttft_ms <= other.ttft_ms
            && self.jitter_variance_ms <= other.jitter_variance_ms
            && self.context_tokens >= other.context_tokens;
        let core_strictly_better = self.throughput_tps > other.throughput_tps
            || self.ttft_ms < other.ttft_ms
            || self.jitter_variance_ms < other.jitter_variance_ms
            || self.context_tokens > other.context_tokens;

        match (self.quality_score, other.quality_score) {
            (Some(left), Some(right)) if left.is_finite() && right.is_finite() => {
                core_not_worse && left >= right && (core_strictly_better || left > right)
            }
            (None, None) => core_not_worse && core_strictly_better,
            // Quality is unknown for one point. It must remain non-dominated.
            _ => false,
        }
    }
}

/// Data-derived normalization ranges for an already observed candidate set.
#[derive(Debug, Clone, Copy)]
pub struct ObjectiveBounds {
    min_tps: f32,
    max_tps: f32,
    min_ttft: f32,
    max_ttft: f32,
    min_jitter: f32,
    max_jitter: f32,
    min_ctx: u32,
    max_ctx: u32,
    min_quality: Option<f32>,
    max_quality: Option<f32>,
}

impl ObjectiveBounds {
    pub fn from_vectors(vectors: &[ObjectiveVector]) -> Option<Self> {
        let first = vectors.first()?;
        if vectors.iter().any(|vector| !vector.has_valid_core_metrics()) {
            return None;
        }

        let min_tps = vectors
            .iter()
            .map(|v| v.throughput_tps)
            .fold(first.throughput_tps, f32::min);
        let max_tps = vectors
            .iter()
            .map(|v| v.throughput_tps)
            .fold(first.throughput_tps, f32::max);
        let min_ttft = vectors.iter().map(|v| v.ttft_ms).fold(first.ttft_ms, f32::min);
        let max_ttft = vectors.iter().map(|v| v.ttft_ms).fold(first.ttft_ms, f32::max);
        let min_jitter = vectors
            .iter()
            .map(|v| v.jitter_variance_ms)
            .fold(first.jitter_variance_ms, f32::min);
        let max_jitter = vectors
            .iter()
            .map(|v| v.jitter_variance_ms)
            .fold(first.jitter_variance_ms, f32::max);
        let min_ctx = vectors.iter().map(|v| v.context_tokens).min()?;
        let max_ctx = vectors.iter().map(|v| v.context_tokens).max()?;

        let all_quality_observed = vectors
            .iter()
            .all(|vector| vector.quality_score.is_some_and(f32::is_finite));
        let (min_quality, max_quality) = if all_quality_observed {
            let quality: Vec<f32> = vectors
                .iter()
                .map(|vector| vector.quality_score.expect("checked above"))
                .collect();
            (
                quality.iter().copied().reduce(f32::min),
                quality.iter().copied().reduce(f32::max),
            )
        } else {
            (None, None)
        };

        Some(Self {
            min_tps,
            max_tps,
            min_ttft,
            max_ttft,
            min_jitter,
            max_jitter,
            min_ctx,
            max_ctx,
            min_quality,
            max_quality,
        })
    }

    /// Equal-regret distance after data-derived normalization. This is a
    /// documented default projection, not a replacement for the frontier.
    fn maximum_normalized_regret(&self, vector: ObjectiveVector) -> Option<f32> {
        if !vector.has_valid_core_metrics() {
            return None;
        }
        let mut regret = [
            normalized_regret(self.max_tps - vector.throughput_tps, self.max_tps - self.min_tps),
            normalized_regret(vector.ttft_ms - self.min_ttft, self.max_ttft - self.min_ttft),
            normalized_regret(
                vector.jitter_variance_ms - self.min_jitter,
                self.max_jitter - self.min_jitter,
            ),
            normalized_regret(
                (self.max_ctx - vector.context_tokens) as f32,
                (self.max_ctx - self.min_ctx) as f32,
            ),
        ]
        .into_iter()
        .fold(0.0_f32, f32::max);

        if let (Some(min), Some(max), Some(quality)) =
            (self.min_quality, self.max_quality, vector.quality_score)
        {
            regret = regret.max(normalized_regret(max - quality, max - min));
        }
        Some(regret)
    }
}

fn normalized_regret(numerator: f32, range: f32) -> f32 {
    if range <= f32::EPSILON {
        0.0
    } else {
        (numerator / range).clamp(0.0, 1.0)
    }
}

/// Extracts the non-dominated Pareto frontier. Invalid measurements are
/// preserved as non-dominated rather than silently discarded.
pub fn extract_pareto_frontier<T>(
    candidates: &[T],
    vector_fn: impl Fn(&T) -> ObjectiveVector,
) -> Vec<&T> {
    let vectors: Vec<ObjectiveVector> = candidates.iter().map(vector_fn).collect();
    candidates
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            !vectors
                .iter()
                .enumerate()
                .any(|(other, vector)| other != *index && vector.dominates(&vectors[*index]))
        })
        .map(|(_, candidate)| candidate)
        .collect()
}

/// Chooses the Pareto point with the smallest maximum data-normalized regret.
/// It returns `None` for an empty or invalid frontier.
pub fn select_utopian_optimal<'a, T>(
    pareto_frontier: &'a [&'a T],
    vector_fn: impl Fn(&T) -> ObjectiveVector,
) -> Option<&'a T> {
    let vectors: Vec<ObjectiveVector> = pareto_frontier.iter().map(|item| vector_fn(item)).collect();
    let bounds = ObjectiveBounds::from_vectors(&vectors)?;

    pareto_frontier
        .iter()
        .zip(vectors)
        .filter_map(|(candidate, vector)| {
            bounds
                .maximum_normalized_regret(vector)
                .map(|regret| (*candidate, regret))
        })
        .min_by(|(_, left), (_, right)| {
            left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(candidate, _)| candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector(
        throughput_tps: f32,
        ttft_ms: f32,
        jitter_variance_ms: f32,
        context_tokens: u32,
        quality_score: Option<f32>,
    ) -> ObjectiveVector {
        ObjectiveVector {
            throughput_tps,
            ttft_ms,
            jitter_variance_ms,
            context_tokens,
            quality_score,
        }
    }

    #[test]
    fn detects_strict_pareto_dominance() {
        assert!(vector(25.0, 100.0, 1.0, 65_536, Some(0.98))
            .dominates(&vector(20.0, 200.0, 2.0, 32_768, Some(0.90))));
    }

    #[test]
    fn preserves_missing_quality_as_explicit_unknown() {
        let measured = vector(25.0, 100.0, 1.0, 4_096, Some(0.98));
        let unknown = vector(20.0, 200.0, 2.0, 4_096, None);
        assert!(!measured.dominates(&unknown));
        assert!(!unknown.dominates(&measured));
    }

    #[test]
    fn extracts_only_non_dominated_points() {
        let points = vec![
            vector(30.0, 100.0, 1.0, 4_096, Some(0.95)),
            vector(15.0, 300.0, 1.5, 65_536, Some(0.95)),
            vector(10.0, 400.0, 5.0, 4_096, Some(0.80)),
        ];
        let frontier = extract_pareto_frontier(&points, |point| *point);
        assert_eq!(frontier.len(), 2);
        assert!(frontier.contains(&&points[0]));
        assert!(frontier.contains(&&points[1]));
    }
}
