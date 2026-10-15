//! Epistemic Grounding & Source Synthesis (Chapter 6, L5)
//!
//! Enforces empirical coverage thresholds, topological altitude ordering,
//! lost-in-the-middle context placement, and verifiable source citations.

use crate::domain::chunk::AuthorityTier;
use crate::domain::scorer::SearchHit;
use serde::{Deserialize, Serialize};

pub const EPISTEMIC_PROMPT_SCHEMA_V1: &str = "cynic.knowledge.epistemic_prompt.v1";
pub const UNCOVERED_REJECTION_TOKEN: &str = "NON COUVERT PAR LE CORPUS ACTUEL";

/// Default minimum confidence threshold for epistemic coverage admission.
pub const DEFAULT_EPISTEMIC_COVERAGE_THRESHOLD: f64 = 0.25;

/// Minimum query term coverage ratio required to avoid spurious single-word matches.
pub const DEFAULT_MIN_TERM_COVERAGE_RATIO: f64 = 0.30;

/// Structured cited source with exact line coordinates and authority tier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpistemicSource {
    pub source_id: usize,
    pub source_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub authority: AuthorityTier,
    pub score: f64,
}

impl EpistemicSource {
    /// Formats the citation tag in strict canonical format: `[SOURCE N: path:line_start-line_end]`
    pub fn citation_tag(&self) -> String {
        format!(
            "[SOURCE {}: {}:{}-{}]",
            self.source_id, self.source_path, self.line_start, self.line_end
        )
    }
}

/// Epistemic coverage verdict on whether a query has sufficient ground truth support.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EpistemicCoverageVerdict {
    Covered {
        assembled_context: String,
        sources: Vec<EpistemicSource>,
        confidence: f64,
        top_authority: AuthorityTier,
    },
    Uncovered {
        rejection_token: String,
        confidence: f64,
        term_coverage: f64,
        reason: String,
    },
}

impl EpistemicCoverageVerdict {
    pub fn is_covered(&self) -> bool {
        matches!(self, Self::Covered { .. })
    }
}

/// Configuration for epistemic context assembly.
#[derive(Debug, Clone)]
pub struct EpistemicAssemblyConfig {
    pub min_confidence: f64,
    pub min_term_coverage: f64,
    pub max_sources: usize,
    pub max_context_chars: usize,
    pub lost_in_the_middle_reordering: bool,
}

impl Default for EpistemicAssemblyConfig {
    fn default() -> Self {
        Self {
            min_confidence: DEFAULT_EPISTEMIC_COVERAGE_THRESHOLD,
            min_term_coverage: DEFAULT_MIN_TERM_COVERAGE_RATIO,
            max_sources: 5,
            max_context_chars: 8_000,
            lost_in_the_middle_reordering: true,
        }
    }
}

/// Computes the ratio of non-trivial query terms found in the chunk content or path.
pub fn compute_term_coverage(query: &str, content: &str, source_path: &str) -> f64 {
    let stop_words = [
        "the", "a", "an", "is", "are", "in", "on", "of", "to", "for", "and", "or", "by", "with",
        "how", "what", "which", "de", "le", "la", "les", "du", "des", "en", "et", "ou", "pour",
        "par", "sur", "dans", "un", "une", "est", "sont",
    ];

    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| {
            t.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|t| t.len() >= 2 && !stop_words.contains(&t.as_str()))
        .collect();

    if terms.is_empty() {
        return 1.0;
    }

    let lower_content = content.to_lowercase();
    let lower_path = source_path.to_lowercase();

    let matched_terms = terms
        .iter()
        .filter(|&t| {
            lower_content.contains(t)
                || lower_path.contains(t)
                || (t.len() > 4 && lower_content.contains(&t[..t.len() - 1]))
                || (t.len() > 5 && lower_content.contains(&t[..t.len() - 2]))
        })
        .count();

    matched_terms as f64 / terms.len() as f64
}

/// Reorders hits according to the "Lost in the Middle" attention optimization:
/// Places the highest priority hits at the boundaries (beginning and end)
/// of the context window, alternating inwards.
pub fn reorder_lost_in_the_middle<'a>(hits: &[SearchHit<'a>]) -> Vec<SearchHit<'a>> {
    if hits.len() <= 2 {
        return hits.to_vec();
    }

    let mut reordered = Vec::with_capacity(hits.len());
    let mut left = true;

    // We distribute into a double-ended queue: [0, 2, ..., 3, 1]
    let mut front = Vec::new();
    let mut back = Vec::new();

    for hit in hits {
        if left {
            front.push(*hit);
        } else {
            back.push(*hit);
        }
        left = !left;
    }

    reordered.extend(front);
    back.reverse();
    reordered.extend(back);
    reordered
}

/// Sorts search hits by topological authority tier and score.
/// Higher authority tiers ($L_0 \succ L_1 \succ L_2 \succ L_3$) prevail when scores are within
/// competitive margins.
pub fn sort_by_topological_authority<'a>(hits: &mut [SearchHit<'a>]) {
    hits.sort_by(|a, b| {
        // Authority weight: L0 = 4, L1 = 3, L2 = 2, L3 = 1
        let auth_rank = |tier: AuthorityTier| match tier {
            AuthorityTier::L0Constitution => 4,
            AuthorityTier::L1Specification => 3,
            AuthorityTier::L2Design => 2,
            AuthorityTier::L3Operation => 1,
        };

        let rank_a = auth_rank(a.chunk.authority);
        let rank_b = auth_rank(b.chunk.authority);

        // If authority tiers differ, apply authority preference unless score difference is huge (>2.0x)
        if rank_a != rank_b
            && (a.score / (b.score + 1e-6) < 2.0)
            && (b.score / (a.score + 1e-6) < 2.0)
        {
            rank_b.cmp(&rank_a)
        } else {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });
}

/// Assembles epistemic context for a query from candidate search hits.
pub fn assemble_epistemic_context<'a>(
    query: &str,
    hits: &[SearchHit<'a>],
    config: &EpistemicAssemblyConfig,
) -> EpistemicCoverageVerdict {
    if hits.is_empty() {
        return EpistemicCoverageVerdict::Uncovered {
            rejection_token: UNCOVERED_REJECTION_TOKEN.to_string(),
            confidence: 0.0,
            term_coverage: 0.0,
            reason: "no matching candidate chunks retrieved from corpus".to_string(),
        };
    }

    let top_hit = hits[0];
    let top_confidence = top_hit.score;
    let term_coverage =
        compute_term_coverage(query, &top_hit.chunk.content, &top_hit.chunk.source_path);

    // Epistemic Guard 1: Term coverage ratio check (filters spurious single-keyword matches)
    if term_coverage < config.min_term_coverage {
        return EpistemicCoverageVerdict::Uncovered {
            rejection_token: UNCOVERED_REJECTION_TOKEN.to_string(),
            confidence: top_confidence,
            term_coverage,
            reason: format!(
                "insufficient query term coverage: {:.1}% < {:.1}% threshold",
                term_coverage * 100.0,
                config.min_term_coverage * 100.0
            ),
        };
    }

    // Epistemic Guard 2: Minimum confidence threshold check
    if top_confidence < config.min_confidence {
        return EpistemicCoverageVerdict::Uncovered {
            rejection_token: UNCOVERED_REJECTION_TOKEN.to_string(),
            confidence: top_confidence,
            term_coverage,
            reason: format!(
                "confidence score {:.4} below epistemic admission floor {:.4}",
                top_confidence, config.min_confidence
            ),
        };
    }

    // Filter and take up to max_sources
    let mut selected_hits: Vec<SearchHit<'a>> =
        hits.iter().take(config.max_sources).copied().collect();

    // Sort by topological authority
    sort_by_topological_authority(&mut selected_hits);

    let top_authority = selected_hits
        .first()
        .map(|h| h.chunk.authority)
        .unwrap_or(AuthorityTier::L3Operation);

    // Apply Lost in the Middle placement if configured
    let ordered_hits = if config.lost_in_the_middle_reordering {
        reorder_lost_in_the_middle(&selected_hits)
    } else {
        selected_hits
    };

    let mut context_buffer = String::new();
    let mut sources = Vec::new();

    for (idx, hit) in ordered_hits.iter().enumerate() {
        let source_id = idx + 1;
        let source = EpistemicSource {
            source_id,
            source_path: hit.chunk.source_path.clone(),
            line_start: hit.chunk.line_start,
            line_end: hit.chunk.line_end,
            authority: hit.chunk.authority,
            score: hit.score,
        };

        let chunk_header = format!(
            "=== {} (Autorité: {:?}) ===\n{}\n\n",
            source.citation_tag(),
            source.authority,
            hit.chunk.content.trim()
        );

        if context_buffer.len() + chunk_header.len() > config.max_context_chars {
            break;
        }

        context_buffer.push_str(&chunk_header);
        sources.push(source);
    }

    EpistemicCoverageVerdict::Covered {
        assembled_context: context_buffer,
        sources,
        confidence: top_confidence,
        top_authority,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chunk::DocumentChunk;

    fn make_test_chunk(path: &str, content: &str, authority: AuthorityTier) -> DocumentChunk {
        DocumentChunk::new(
            path.to_string(),
            1,
            20,
            vec!["Section".to_string()],
            content.to_string(),
            authority,
        )
    }

    #[test]
    fn compute_term_coverage_detects_spurious_single_token_match() {
        let query = "deploy kubernetes helm chart cynic cluster pods";
        let chunk_content = "CYNIC is a sovereign operating system running on Solana.";
        let path = "AGENTS.md";

        let coverage = compute_term_coverage(query, chunk_content, path);
        // Only "cynic" matches out of [deploy, kubernetes, helm, chart, cynic, cluster, pods] (1/7 = 0.14)
        assert!(
            coverage < 0.20,
            "Trap query must have low term coverage, got {:.2}",
            coverage
        );
    }

    #[test]
    fn compute_term_coverage_high_on_legitimate_query() {
        let query = "5 pouvoirs séparation constituante judiciaire";
        let chunk_content = "Les 5 pouvoirs sont séparés : pouvoir constituant, judiciaire, exécutif, législatif, contrôle.";
        let path = "docs/specs/cynic-government-architecture.md";

        let coverage = compute_term_coverage(query, chunk_content, path);
        assert!(
            coverage > 0.60,
            "Legitimate query must have high term coverage, got {:.2}",
            coverage
        );
    }

    #[test]
    fn reorder_lost_in_the_middle_places_top_hits_at_boundaries() {
        let c1 = make_test_chunk("c1", "content 1", AuthorityTier::L0Constitution);
        let c2 = make_test_chunk("c2", "content 2", AuthorityTier::L0Constitution);
        let c3 = make_test_chunk("c3", "content 3", AuthorityTier::L0Constitution);
        let c4 = make_test_chunk("c4", "content 4", AuthorityTier::L0Constitution);

        let hits = vec![
            SearchHit {
                chunk: &c1,
                score: 10.0,
            },
            SearchHit {
                chunk: &c2,
                score: 8.0,
            },
            SearchHit {
                chunk: &c3,
                score: 6.0,
            },
            SearchHit {
                chunk: &c4,
                score: 4.0,
            },
        ];

        let reordered = reorder_lost_in_the_middle(&hits);
        assert_eq!(reordered.len(), 4);
        assert_eq!(reordered[0].chunk.source_path, "c1"); // Highest at start
        assert_eq!(reordered[3].chunk.source_path, "c2"); // Second highest at end
    }

    #[test]
    fn epistemic_verdict_rejects_hallucination_trap() {
        let trap_chunk = make_test_chunk(
            "crates/cynic-node/src/main.rs",
            "fn main() { println!(\"CYNIC node\"); }",
            AuthorityTier::L3Operation,
        );
        let hits = vec![SearchHit {
            chunk: &trap_chunk,
            score: 0.15,
        }];

        let config = EpistemicAssemblyConfig::default();
        let verdict = assemble_epistemic_context(
            "deploy kubernetes helm chart cynic cluster pods",
            &hits,
            &config,
        );

        match verdict {
            EpistemicCoverageVerdict::Uncovered {
                rejection_token, ..
            } => {
                assert_eq!(rejection_token, UNCOVERED_REJECTION_TOKEN);
            }
            EpistemicCoverageVerdict::Covered { .. } => {
                panic!("Trap query must be rejected as uncovered");
            }
        }
    }
}
