use serde::{Deserialize, Serialize};

use crate::domain::chunk::DocumentChunk;

pub const CHUNK_QUALITY_REPORT_SCHEMA_V1: &str = "cynic.knowledge.chunk_quality_report.v1";

/// Maximum recommended character length before a chunk suffers severe truncation in dense models.
pub const DEFAULT_MAX_CHUNK_CHARS: usize = 3000;

/// Minimum recommended character length before a chunk is considered semantically starved.
pub const DEFAULT_MIN_CHUNK_CHARS: usize = 60;

/// Distribution percentiles of chunk length.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkLengthPercentiles {
    pub min: usize,
    pub p10: usize,
    pub p50: usize,
    pub p90: usize,
    pub p99: usize,
    pub max: usize,
    pub mean: f64,
}

/// Outlier reference for oversized or underfed chunks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkAnomaly {
    pub chunk_id: String,
    pub source_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub char_length: usize,
    pub reason: String,
}

/// Comprehensive structural audit of corpus chunk quality.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkQualityReport {
    pub schema: String,
    pub total_chunks: usize,
    pub char_percentiles: ChunkLengthPercentiles,
    pub line_percentiles: ChunkLengthPercentiles,
    pub oversized_chunks: Vec<ChunkAnomaly>,
    pub starved_chunks: Vec<ChunkAnomaly>,
    pub healthy_ratio: f64,
}

impl ChunkQualityReport {
    /// Audits a collection of chunks against organic size constraints.
    pub fn audit(chunks: &[DocumentChunk]) -> Self {
        Self::audit_with_thresholds(chunks, DEFAULT_MIN_CHUNK_CHARS, DEFAULT_MAX_CHUNK_CHARS)
    }

    /// Audits chunks with customizable minimum and maximum character thresholds.
    pub fn audit_with_thresholds(
        chunks: &[DocumentChunk],
        min_chars: usize,
        max_chars: usize,
    ) -> Self {
        if chunks.is_empty() {
            return Self {
                schema: CHUNK_QUALITY_REPORT_SCHEMA_V1.to_string(),
                total_chunks: 0,
                char_percentiles: ChunkLengthPercentiles::empty(),
                line_percentiles: ChunkLengthPercentiles::empty(),
                oversized_chunks: Vec::new(),
                starved_chunks: Vec::new(),
                healthy_ratio: 1.0,
            };
        }

        let mut char_lengths: Vec<usize> = chunks.iter().map(|c| c.content.len()).collect();
        let mut line_lengths: Vec<usize> = chunks
            .iter()
            .map(|c| c.line_end.saturating_sub(c.line_start) + 1)
            .collect();

        char_lengths.sort_unstable();
        line_lengths.sort_unstable();

        let char_percentiles = compute_percentiles(&char_lengths);
        let line_percentiles = compute_percentiles(&line_lengths);

        let mut oversized_chunks = Vec::new();
        let mut starved_chunks = Vec::new();

        for c in chunks {
            let len = c.content.len();
            if len > max_chars {
                oversized_chunks.push(ChunkAnomaly {
                    chunk_id: c.chunk_id.clone(),
                    source_path: c.source_path.clone(),
                    line_start: c.line_start,
                    line_end: c.line_end,
                    char_length: len,
                    reason: format!(
                        "Exceeds max threshold of {max_chars} chars ({} lines)",
                        c.line_end.saturating_sub(c.line_start) + 1
                    ),
                });
            } else if len < min_chars {
                starved_chunks.push(ChunkAnomaly {
                    chunk_id: c.chunk_id.clone(),
                    source_path: c.source_path.clone(),
                    line_start: c.line_start,
                    line_end: c.line_end,
                    char_length: len,
                    reason: format!(
                        "Below min threshold of {min_chars} chars (insufficient semantic context)"
                    ),
                });
            }
        }

        let healthy_count = chunks
            .len()
            .saturating_sub(oversized_chunks.len() + starved_chunks.len());
        let healthy_ratio = healthy_count as f64 / chunks.len() as f64;

        Self {
            schema: CHUNK_QUALITY_REPORT_SCHEMA_V1.to_string(),
            total_chunks: chunks.len(),
            char_percentiles,
            line_percentiles,
            oversized_chunks,
            starved_chunks,
            healthy_ratio,
        }
    }
}

impl ChunkLengthPercentiles {
    fn empty() -> Self {
        Self {
            min: 0,
            p10: 0,
            p50: 0,
            p90: 0,
            p99: 0,
            max: 0,
            mean: 0.0,
        }
    }
}

fn compute_percentiles(sorted: &[usize]) -> ChunkLengthPercentiles {
    let n = sorted.len();
    if n == 0 {
        return ChunkLengthPercentiles::empty();
    }
    let sum: usize = sorted.iter().sum();
    let mean = sum as f64 / n as f64;

    ChunkLengthPercentiles {
        min: sorted[0],
        p10: sorted[(n as f64 * 0.10) as usize],
        p50: sorted[(n as f64 * 0.50) as usize],
        p90: sorted[((n as f64 * 0.90) as usize).min(n - 1)],
        p99: sorted[((n as f64 * 0.99) as usize).min(n - 1)],
        max: sorted[n - 1],
        mean,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chunk::AuthorityTier;

    #[test]
    fn test_chunk_quality_empty() {
        let report = ChunkQualityReport::audit(&[]);
        assert_eq!(report.total_chunks, 0);
        assert_eq!(report.healthy_ratio, 1.0);
    }

    #[test]
    fn test_chunk_quality_detection() {
        let chunks = vec![
            DocumentChunk::new(
                "src/tiny.rs".to_string(),
                1,
                1,
                vec!["tiny".to_string()],
                "pub struct X;".to_string(), // 13 chars (< 60)
                AuthorityTier::L1Specification,
            ),
            DocumentChunk::new(
                "src/normal.rs".to_string(),
                1,
                10,
                vec!["normal".to_string()],
                "a".repeat(500), // 500 chars (healthy)
                AuthorityTier::L1Specification,
            ),
            DocumentChunk::new(
                "src/giant.rs".to_string(),
                1,
                200,
                vec!["giant".to_string()],
                "b".repeat(5000), // 5000 chars (> 3000)
                AuthorityTier::L1Specification,
            ),
        ];

        let report = ChunkQualityReport::audit(&chunks);
        assert_eq!(report.total_chunks, 3);
        assert_eq!(report.starved_chunks.len(), 1);
        assert_eq!(report.oversized_chunks.len(), 1);
        assert!((report.healthy_ratio - (1.0 / 3.0)).abs() < 1e-4);
        assert_eq!(report.char_percentiles.min, 13);
        assert_eq!(report.char_percentiles.max, 5000);
    }
}
