//! Wisdom domain contract types — pure data shapes for domain signal enrichment.
//!
//! Canonical domain owner: `cynic-askesis` (SoC foundation §481).
//! These types live in `cynic-protocol` because they are pure serde contracts
//! consumed by both `cynic-kernel` (runtime) and `cynic-askesis` (domain logic).

use serde::{Deserialize, Serialize};

/// Curated domain signal: falsifiable knowledge about a domain pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSignal {
    pub signal_id: String,
    pub domain: String,
    pub pattern: String,
    /// 0.0-1.0 confidence in pattern validity
    pub strength: f64,
    /// tweet IDs, references, evidence sources
    pub sources: Vec<String>,
    /// "If X then Y < threshold" — how to invalidate this
    pub falsifiable_claim: String,
}

/// Domain curations — loaded at startup from jsonl files.
#[derive(Debug, Clone, Default)]
pub struct DomainCurations {
    /// Map: domain → signals
    pub signals: std::collections::HashMap<String, Vec<DomainSignal>>,
    pub loaded_domains: Vec<String>,
}

impl DomainCurations {
    pub fn new() -> Self {
        Self::default()
    }

    /// Find signals matching a domain and containing keywords from content.
    pub fn find_matching_signals(&self, domain: &str, content: &str) -> Vec<DomainSignal> {
        let signals = self
            .signals
            .get(domain)
            .map(|s| s.as_slice())
            .unwrap_or(&[]);

        // Extract keywords: lowercase, split by non-alphanumeric
        let content_lower = content.to_lowercase();
        let keywords: Vec<&str> = content_lower
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() > 2) // ignore very short words
            .collect();

        signals
            .iter()
            .filter(|signal| {
                // Check if pattern contains any keywords (fuzzy match)
                let pattern_lower = signal.pattern.to_lowercase();
                keywords.iter().any(|kw| pattern_lower.contains(kw))
                    || signal
                        .falsifiable_claim
                        .to_lowercase()
                        .split(|c: char| !c.is_alphanumeric())
                        .any(|w| keywords.contains(&w))
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_signal_serde() {
        let signal = DomainSignal {
            signal_id: "D1_001".to_string(),
            domain: "D1".to_string(),
            pattern: "Liquidity locked forever".to_string(),
            strength: 0.85,
            sources: vec!["tweet_123".to_string()],
            falsifiable_claim: "If liquidity unlock event count > 0 then pattern invalid"
                .to_string(),
        };

        let json = serde_json::to_string(&signal).expect("serialize");
        let back: DomainSignal = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.signal_id, signal.signal_id);
        assert!((back.strength - signal.strength).abs() < f64::EPSILON);
    }

    #[test]
    fn test_find_matching_signals() {
        let mut curations = DomainCurations::new();
        curations.signals.insert(
            "D1".to_string(),
            vec![DomainSignal {
                signal_id: "D1_001".to_string(),
                domain: "D1".to_string(),
                pattern: "Liquidity locked forever pattern".to_string(),
                strength: 0.85,
                sources: vec![],
                falsifiable_claim: "test".to_string(),
            }],
        );

        let matches = curations.find_matching_signals("D1", "check liquidity status");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].signal_id, "D1_001");

        let no_matches = curations.find_matching_signals("D2", "anything");
        assert!(no_matches.is_empty());
    }
}
