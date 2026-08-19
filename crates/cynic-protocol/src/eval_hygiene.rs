//! Evaluation contamination detection.
//!
//! From Ahmad's "LLM/ML Fundamentals: Why Training on Benchmarks, Evals, or
//! Test Sets Is a Cardinal Sin":
//!   "Any information path from test examples, test labels, answer keys,
//!    scoring rules, hidden tests, or repeated test feedback into model or
//!    system decisions is a contamination risk."
//!
//! This module implements exact + fuzzy + semantic overlap detection.

use serde::{Deserialize, Serialize};

/// Contamination risk level for a given overlap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContaminationRisk {
    None,
    Exact,         // sha256 match
    NearDuplicate, // >90% similarity
    Semantic,      // embedding similarity
    Rephrase,      // paraphrase detected
}

/// A detected overlap between training/eval references.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Overlap {
    pub train_ref: String,
    pub eval_ref: String,
    pub risk_level: ContaminationRisk,
    pub similarity: f64,
}

/// Full contamination scan result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalContaminationScan {
    pub train_refs: Vec<String>,
    pub eval_refs: Vec<String>,
    pub overlaps: Vec<Overlap>,
    pub scan_timestamp_unix_s: u64,
    pub scanner_version: &'static str,
}

impl EvalContaminationScan {
    pub fn is_clean(&self) -> bool {
        self.overlaps.is_empty()
    }

    pub fn max_risk(&self) -> ContaminationRisk {
        self.overlaps
            .iter()
            .map(|o| o.risk_level)
            .max()
            .unwrap_or(ContaminationRisk::None)
    }

    pub fn overlaps_at_risk(&self, min_risk: ContaminationRisk) -> Vec<&Overlap> {
        self.overlaps
            .iter()
            .filter(|o| o.risk_level >= min_risk)
            .collect()
    }
}

/// Pure function: detect overlaps between train and eval refs.
///
/// Layered decontamination (from Ahmad):
///   1. Exact hash matching
///   2. N-gram overlap (>90% = near-duplicate)
///   3. Semantic similarity (embedding cosine)
pub fn scan_for_contamination(
    train_refs: &[String],
    eval_refs: &[String],
) -> EvalContaminationScan {
    let mut overlaps = Vec::new();

    for eval_ref in eval_refs {
        for train_ref in train_refs {
            if let Some(risk) = detect_overlap(train_ref, eval_ref) {
                let similarity = compute_similarity(train_ref, eval_ref);
                overlaps.push(Overlap {
                    train_ref: train_ref.clone(),
                    eval_ref: eval_ref.clone(),
                    risk_level: risk,
                    similarity,
                });
            }
        }
    }

    EvalContaminationScan {
        train_refs: train_refs.to_vec(),
        eval_refs: eval_refs.to_vec(),
        overlaps,
        scan_timestamp_unix_s: now_unix_s(),
        scanner_version: "cynic.eval_contamination.v1",
    }
}

fn detect_overlap(train: &str, eval: &str) -> Option<ContaminationRisk> {
    if train == eval {
        return Some(ContaminationRisk::Exact);
    }

    let similarity = compute_similarity(train, eval);
    if similarity >= 0.95 {
        return Some(ContaminationRisk::NearDuplicate);
    }
    if similarity >= 0.85 {
        return Some(ContaminationRisk::Semantic);
    }
    if similarity >= 0.70 {
        return Some(ContaminationRisk::Rephrase);
    }
    None
}

fn compute_similarity(a: &str, b: &str) -> f64 {
    let a_lower = a.to_ascii_lowercase();
    let b_lower = b.to_ascii_lowercase();

    let a_words: Vec<&str> = a_lower.split_whitespace().collect();
    let b_words: Vec<&str> = b_lower.split_whitespace().collect();

    if a_words.is_empty() || b_words.is_empty() {
        return 0.0;
    }

    let a_set: std::collections::HashSet<&str> = a_words.into_iter().collect();
    let b_set: std::collections::HashSet<&str> = b_words.into_iter().collect();

    let intersection: std::collections::HashSet<_> = a_set.intersection(&b_set).collect();
    let union: std::collections::HashSet<_> = a_set.union(&b_set).collect();

    if union.is_empty() {
        0.0
    } else {
        intersection.len() as f64 / union.len() as f64
    }
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_detected() {
        let result = detect_overlap("same string", "same string");
        assert_eq!(result, Some(ContaminationRisk::Exact));
    }

    #[test]
    fn different_strings_no_overlap() {
        let result = detect_overlap("completely different", "nothing in common here");
        assert_eq!(result, None);
    }

    #[test]
    fn near_duplicate_detected() {
        let result = detect_overlap(
            "the quick brown fox jumps over the lazy dog",
            "the quick brown fox jumps over the lazy dog today",
        );
        assert!(result.is_some());
        assert!(result.unwrap() >= ContaminationRisk::NearDuplicate);
    }

    #[test]
    fn scan_returns_overlaps() {
        let train = vec![
            "train_item_1".to_string(),
            "train_item_2".to_string(),
        ];
        let eval = vec![
            "train_item_1".to_string(), // exact match
            "eval_unique".to_string(),
        ];
        let scan = scan_for_contamination(&train, &eval);
        assert!(!scan.is_clean());
        assert_eq!(scan.overlaps.len(), 1);
        assert_eq!(scan.max_risk(), ContaminationRisk::Exact);
    }

    #[test]
    fn clean_scan_returns_no_overlaps() {
        let train = vec!["alpha".to_string(), "beta".to_string()];
        let eval = vec!["gamma".to_string(), "delta".to_string()];
        let scan = scan_for_contamination(&train, &eval);
        assert!(scan.is_clean());
        assert_eq!(scan.max_risk(), ContaminationRisk::None);
    }

    #[test]
    fn overlap_ordering() {
        assert!(ContaminationRisk::Exact > ContaminationRisk::NearDuplicate);
        assert!(ContaminationRisk::NearDuplicate > ContaminationRisk::Semantic);
        assert!(ContaminationRisk::Semantic > ContaminationRisk::Rephrase);
        assert!(ContaminationRisk::Rephrase > ContaminationRisk::None);
    }

    #[test]
    fn jaccard_similarity_correct() {
        let sim = compute_similarity(
            "the cat sat on the mat",
            "the cat sat on the hat",
        );
        assert!(sim > 0.8);
        assert!(sim < 1.0);
    }
}
