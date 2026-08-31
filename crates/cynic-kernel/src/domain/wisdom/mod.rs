//! Wisdom Enrichment — Domain Intelligence Integration (K15 Consumer)
//!
//! Closed-loop feedback: judge → crystal → curate → enrich Dogs → better judges
//!
//! D1-D6 curated patterns inject domain-specific knowledge into Dog prompts.
//! Falsifiable: measure Dogs discrimination before/after enrichment.
//!
//! Contract types (`DomainSignal`, `DomainCurations`) live in `cynic-protocol`.
//! This module adds I/O (loading curations from disk) and re-exports the types.

pub mod engine;

// Re-export canonical types from protocol — single source of truth.
pub use cynic_protocol::wisdom::{DomainCurations, DomainSignal};

/// Extension trait for loading curations from disk (I/O, not in protocol).
pub trait DomainCurationsLoader {
    /// Load curations from jsonl files in the given directory.
    /// Format: each line is a DomainSignal JSON.
    fn load_from_path(path: &std::path::Path) -> Result<DomainCurations, String>;
}

impl DomainCurationsLoader for DomainCurations {
    fn load_from_path(path: &std::path::Path) -> Result<DomainCurations, String> {
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
                && name_str.starts_with('D')
                && name_str.contains("_curated")
            {
                let domain_id = name_str.split('_').next().unwrap_or("unknown");
                if let Err(e) = load_file(&mut curations, &file_path, domain_id) {
                    tracing::warn!(domain = domain_id, error = %e, "Failed to load curation file");
                } else {
                    curations.loaded_domains.push(domain_id.to_string());
                    tracing::info!(domain = domain_id, "Curation file loaded");
                }
            }
        }

        Ok(curations)
    }
}

fn load_file(
    curations: &mut DomainCurations,
    path: &std::path::Path,
    domain: &str,
) -> Result<(), String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;

    let mut signals = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let signal: DomainSignal = serde_json::from_str(line)
            .map_err(|e| format!("JSON parse error on line {}: {e}", line_num + 1))?;
        signals.push(signal);
    }

    curations.signals.insert(domain.to_string(), signals);
    Ok(())
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
            falsifiable_claim: "If liquidity unlock event count > 0 then pattern invalid"
                .to_string(),
        };

        let json = serde_json::to_string(&signal).expect("serialize");
        let back: DomainSignal = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.signal_id, signal.signal_id);
        assert!((back.strength - signal.strength).abs() < f64::EPSILON);
    }

    #[test]
    fn test_find_matching_signals_via_protocol() {
        let mut curations = DomainCurations::new();
        curations.signals.insert(
            "D1".to_string(),
            vec![DomainSignal {
                signal_id: "D1_001".to_string(),
                domain: "D1".to_string(),
                pattern: "Liquidity locked forever pattern".to_string(),
                strength: 0.85,
                sources: vec![],
                falsifiable_claim: "test claim".to_string(),
            }],
        );

        let matches = curations.find_matching_signals("D1", "check liquidity status");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].signal_id, "D1_001");

        let no_matches = curations.find_matching_signals("D2", "anything");
        assert!(no_matches.is_empty());
    }
}
