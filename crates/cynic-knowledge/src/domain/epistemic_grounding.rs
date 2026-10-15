//! Epistemic Grounding & Source Synthesis (Chapter 6, L5)
//!
//! Enforces empirical coverage thresholds, topological altitude ordering,
//! lost-in-the-middle context placement, and verifiable source citations.

use std::collections::HashSet;

use crate::domain::chunk::AuthorityTier;
use crate::domain::scorer::SearchHit;
use serde::{Deserialize, Serialize};

pub const EPISTEMIC_PROMPT_SCHEMA_V1: &str = "cynic.knowledge.epistemic_prompt.v1";
pub const UNCOVERED_REJECTION_TOKEN: &str = "NON COUVERT PAR LE CORPUS ACTUEL";

/// Default minimum confidence score floor for epistemic coverage admission (on continuous fusion score scale).
/// Relevant query hits range 50.0..250.0; weak or spurious matches fall below 40.0.
pub const DEFAULT_EPISTEMIC_COVERAGE_THRESHOLD: f64 = 40.0;

/// Minimum query term coverage ratio required to avoid spurious single-word matches.
/// Legitimate technical queries achieve 60%..100% term coverage; adversarial traps fall below 50%.
pub const DEFAULT_MIN_TERM_COVERAGE_RATIO: f64 = 0.50;

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
        dropped_sources_count: usize,
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

/// Extracts non-stopword alphanumeric tokens from text.
pub fn extract_query_terms(text: &str) -> Vec<String> {
    let stop_words: HashSet<&'static str> = [
        "the", "a", "an", "is", "are", "in", "on", "of", "to", "for", "and", "or", "by", "with",
        "how", "what", "which", "de", "le", "la", "les", "du", "des", "en", "et", "ou", "pour",
        "par", "sur", "dans", "un", "une", "est", "sont", // Syntax noise keywords
        "pub", "struct", "impl", "fn", "let", "mut", "enum", "trait", "type", "mod",
        // Pervasive domain baseline words (filter to isolate salient informative terms)
        "cynic", "cortex", "system", "code", "file", "json",
    ]
    .into_iter()
    .collect();

    text.split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .map(|t| {
            t.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|t| t.len() >= 3 && !stop_words.contains(t.as_str()))
        .collect()
}

/// Computes the ratio of non-trivial query terms found in the chunk content or path.
/// Strict exact token boundary matching (no loose substring slicing).
pub fn compute_term_coverage(query: &str, content: &str, source_path: &str) -> f64 {
    let terms = extract_query_terms(query);
    if terms.is_empty() {
        return 0.0;
    }

    let mut doc_words: HashSet<String> = HashSet::new();
    for token in content
        .split_whitespace()
        .chain(source_path.split(|c: char| !c.is_alphanumeric()))
    {
        let clean = token
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if clean.len() >= 3 {
            doc_words.insert(clean);
        }
    }

    let matched_terms = terms
        .iter()
        .filter(|t| doc_words.contains(t.as_str()))
        .count();
    matched_terms as f64 / terms.len() as f64
}

/// Reorders hits according to the "Lost in the Middle" attention optimization:
/// Places the highest priority hits at the outer boundaries (beginning and end)
/// of the context window, alternating inwards.
pub fn reorder_lost_in_the_middle<'a>(hits: &[SearchHit<'a>]) -> Vec<SearchHit<'a>> {
    if hits.len() <= 2 {
        return hits.to_vec();
    }

    let mut reordered = Vec::with_capacity(hits.len());
    let mut left = true;

    // Distribute into double-ended queue: [0, 2, ..., 3, 1]
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

/// Sorts search hits respecting strict topological authority supremacy.
/// C4 resolution: Topological tiers ($L_0 \succ L_1 \succ L_2 \succ L_3$) form strict outer partition;
/// within each authority tier, hits are ordered by descending relevance score, then optionally
/// arranged with Lost-in-the-Middle.
pub fn arrange_by_topological_authority_and_litm<'a>(
    hits: &[SearchHit<'a>],
    enable_litm: bool,
) -> Vec<SearchHit<'a>> {
    let tiers = [
        AuthorityTier::L0Constitution,
        AuthorityTier::L1Specification,
        AuthorityTier::L2Design,
        AuthorityTier::L3Operation,
    ];

    let mut arranged = Vec::with_capacity(hits.len());

    for tier in tiers {
        let mut tier_hits: Vec<SearchHit<'a>> = hits
            .iter()
            .filter(|h| h.chunk.authority == tier)
            .copied()
            .collect();

        if tier_hits.is_empty() {
            continue;
        }

        // Sort descending by score within the tier
        tier_hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.chunk.chunk_id.cmp(&b.chunk.chunk_id))
        });

        // Apply Lost-in-the-Middle permutation strictly WITHIN this tier
        if enable_litm && tier_hits.len() > 2 {
            arranged.extend(reorder_lost_in_the_middle(&tier_hits));
        } else {
            arranged.extend(tier_hits);
        }
    }

    arranged
}

/// Compatibility wrapper for sorting by topological authority.
pub fn sort_by_topological_authority<'a>(hits: &mut [SearchHit<'a>]) {
    let sorted = arrange_by_topological_authority_and_litm(hits, false);
    hits.copy_from_slice(&sorted);
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

    // Epistemic Guard 1: Term coverage across candidate hits
    // Evaluates whether ANY candidate chunk in the top pool provides sufficient lexical grounding
    let mut best_term_coverage = 0.0;
    for hit in hits.iter().take(config.max_sources) {
        let cov = compute_term_coverage(query, &hit.chunk.content, &hit.chunk.source_path);
        if cov > best_term_coverage {
            best_term_coverage = cov;
        }
    }

    if best_term_coverage < config.min_term_coverage {
        return EpistemicCoverageVerdict::Uncovered {
            rejection_token: UNCOVERED_REJECTION_TOKEN.to_string(),
            confidence: top_confidence,
            term_coverage: best_term_coverage,
            reason: format!(
                "insufficient query term coverage across pool: {:.1}% < {:.1}% threshold",
                best_term_coverage * 100.0,
                config.min_term_coverage * 100.0
            ),
        };
    }

    // Epistemic Guard 2: Minimum confidence threshold check (calibrated on continuous fusion score scale)
    if top_confidence < config.min_confidence {
        return EpistemicCoverageVerdict::Uncovered {
            rejection_token: UNCOVERED_REJECTION_TOKEN.to_string(),
            confidence: top_confidence,
            term_coverage: best_term_coverage,
            reason: format!(
                "confidence score {:.2} below epistemic admission floor {:.2}",
                top_confidence, config.min_confidence
            ),
        };
    }

    // Filter and take up to max_sources
    let candidate_hits: Vec<SearchHit<'a>> =
        hits.iter().take(config.max_sources).copied().collect();

    // Order by topological authority supremacy with within-tier LITM (C4 resolution)
    let ordered_hits = arrange_by_topological_authority_and_litm(
        &candidate_hits,
        config.lost_in_the_middle_reordering,
    );

    let top_authority = ordered_hits
        .first()
        .map(|h| h.chunk.authority)
        .unwrap_or(AuthorityTier::L3Operation);

    let mut context_buffer = String::new();
    let mut sources = Vec::new();
    let mut dropped_sources_count = 0;

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
            dropped_sources_count += ordered_hits.len() - idx;
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
        dropped_sources_count,
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
        let chunk_content = "pouvoirs séparation constituante judiciaire exécutif";
        let path = "docs/specs/cynic-government-architecture.md";

        let coverage = compute_term_coverage(query, chunk_content, path);
        assert!(
            coverage > 0.60,
            "Legitimate query must have high term coverage, got {:.2}",
            coverage
        );
    }

    #[test]
    fn arrange_by_topological_authority_maintains_l0_supremacy_over_l1() {
        let c_l0 = make_test_chunk("AGENTS.md", "constitution", AuthorityTier::L0Constitution);
        let c_l1 = make_test_chunk(
            "specs/doc.md",
            "specification",
            AuthorityTier::L1Specification,
        );

        let hits = vec![
            SearchHit {
                chunk: &c_l1,
                score: 95.0,
            },
            SearchHit {
                chunk: &c_l0,
                score: 80.0,
            },
        ];

        let arranged = arrange_by_topological_authority_and_litm(&hits, true);
        assert_eq!(arranged.len(), 2);
        // L0 Constitution MUST come first regardless of L1 score
        assert_eq!(arranged[0].chunk.authority, AuthorityTier::L0Constitution);
        assert_eq!(arranged[1].chunk.authority, AuthorityTier::L1Specification);
    }

    #[test]
    fn epistemic_verdict_rejects_hallucination_trap_via_guard_1() {
        let trap_chunk = make_test_chunk(
            "crates/cynic-node/src/main.rs",
            "fn main() { println!(\"CYNIC node\"); }",
            AuthorityTier::L3Operation,
        );
        let hits = vec![SearchHit {
            chunk: &trap_chunk,
            score: 80.0,
        }];

        let config = EpistemicAssemblyConfig::default();
        let verdict = assemble_epistemic_context(
            "deploy kubernetes helm chart cynic cluster pods",
            &hits,
            &config,
        );

        match verdict {
            EpistemicCoverageVerdict::Uncovered {
                rejection_token,
                reason,
                ..
            } => {
                assert_eq!(rejection_token, UNCOVERED_REJECTION_TOKEN);
                assert!(reason.contains("insufficient query term coverage"));
            }
            EpistemicCoverageVerdict::Covered { .. } => {
                panic!("Trap query must be rejected as uncovered");
            }
        }
    }

    #[test]
    fn epistemic_verdict_rejects_weak_score_via_guard_2() {
        // C3 test proof: Query terms match 100%, but score is below admission floor
        let chunk = make_test_chunk(
            "crates/cynic-node/src/main.rs",
            "cynic node temporal reconciliation protocol",
            AuthorityTier::L3Operation,
        );
        let hits = vec![SearchHit {
            chunk: &chunk,
            score: 15.0, // Well below 40.0 floor
        }];

        let config = EpistemicAssemblyConfig {
            min_confidence: 40.0,
            min_term_coverage: 0.30,
            ..Default::default()
        };

        let verdict =
            assemble_epistemic_context("cynic node temporal reconciliation", &hits, &config);

        match verdict {
            EpistemicCoverageVerdict::Uncovered {
                rejection_token,
                reason,
                confidence,
                term_coverage,
            } => {
                assert_eq!(rejection_token, UNCOVERED_REJECTION_TOKEN);
                assert!(term_coverage >= 0.75, "Term coverage was high");
                assert!(confidence < 40.0, "Score was low");
                assert!(
                    reason.contains("below epistemic admission floor"),
                    "Guard 2 must fire explicitly: {}",
                    reason
                );
            }
            EpistemicCoverageVerdict::Covered { .. } => {
                panic!("Low-confidence query must be rejected via Guard 2");
            }
        }
    }
}
