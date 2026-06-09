//! Wisdom Enrichment — Domain Intelligence Integration (K15 Consumer)
//!
//! Closed-loop feedback: judge → crystal → curate → enrich Dogs → better judges
//!
//! D1-D6 curated patterns inject domain-specific knowledge into Dog prompts.
//! Falsifiable: measure Dogs discrimination before/after enrichment.

pub mod engine;

use serde::{Deserialize, Serialize};

/// Curated domain signal: falsifiable knowledge about a domain pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSignal {
    pub signal_id: String,
    pub domain: String,
    pub pattern: String,
    pub strength: f64,             // 0.0-1.0 confidence in pattern validity
    pub sources: Vec<String>,      // tweet IDs, references
    pub falsifiable_claim: String, // "If X then Y < threshold" — how to invalidate this
}

/// Domain curations — loaded at kernel startup from cynic-python/curation/*.jsonl
#[derive(Debug, Clone, Default)]
pub struct DomainCurations {
    // Map: domain → signals
    pub signals: std::collections::HashMap<String, Vec<DomainSignal>>,
    pub loaded_domains: Vec<String>,
}

impl DomainCurations {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load curations from jsonl files in the given directory.
    /// Format: each line is a DomainSignal JSON.
    pub fn load_from_path(path: &std::path::Path) -> Result<Self, String> {
        let mut curations = DomainCurations::new();

        for entry in std::fs::read_dir(path)
            .map_err(|e| format!("Failed to read curation directory: {e}"))?
        {
            let entry = entry.map_err(|e| format!("Directory read error: {e}"))?;
            let file_path = entry.path();

            if file_path.extension().is_none_or(|ext| ext != "jsonl") {
                continue;
            }

            if let Some(file_name) = file_path.file_name()
                && let Some(name_str) = file_name.to_str()
                && name_str.starts_with("D")
                && name_str.contains("_curated")
            {
                let domain_id = name_str.split('_').next().unwrap_or("unknown");
                if let Err(e) = curations.load_file(&file_path, domain_id) {
                    tracing::warn!(domain = domain_id, error = %e, "Failed to load curation file");
                } else {
                    curations.loaded_domains.push(domain_id.to_string());
                    tracing::info!(domain = domain_id, "Curation file loaded");
                }
            }
        }

        Ok(curations)
    }

    fn load_file(&mut self, path: &std::path::Path, domain: &str) -> Result<(), String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;

        let mut signals = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let signal: DomainSignal = serde_json::from_str(line)
                .map_err(|e| format!("JSON parse error on line {}: {e}", line_num + 1))?;
            signals.push(signal);
        }

        self.signals.insert(domain.to_string(), signals);
        Ok(())
    }

    /// Find signals matching a domain and containing keywords from content
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
    fn test_domain_signal_roundtrip() {
        let signal = DomainSignal {
            signal_id: "D1_test_001".to_string(),
            domain: "D1".to_string(),
            pattern: "Liquidity locked forever".to_string(),
            strength: 0.85,
            sources: vec!["tweet_123".to_string()],
            falsifiable_claim: "If liquidity is locked: rug probability < 0.1".to_string(),
        };

        let json = serde_json::to_string(&signal).unwrap();
        let parsed: DomainSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.signal_id, "D1_test_001");
        assert_eq!(parsed.strength, 0.85);
    }

    #[test]
    fn test_find_matching_signals() {
        let mut curations = DomainCurations::new();

        let signal1 = DomainSignal {
            signal_id: "D1_rug_001".to_string(),
            domain: "D1".to_string(),
            pattern: "Liquidity burned and supply capped".to_string(),
            strength: 0.9,
            sources: vec![],
            falsifiable_claim: "If both: rug probability < 0.1".to_string(),
        };

        let signal2 = DomainSignal {
            signal_id: "D1_safe_001".to_string(),
            domain: "D1".to_string(),
            pattern: "Multi-sig governance with timelock".to_string(),
            strength: 0.8,
            sources: vec![],
            falsifiable_claim: "If present: authenticity > 0.6".to_string(),
        };

        curations
            .signals
            .insert("D1".to_string(), vec![signal1, signal2]);

        let content = "Token has liquidity burned and supply capped";
        let matches = curations.find_matching_signals("D1", content);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].signal_id, "D1_rug_001");
    }
}
