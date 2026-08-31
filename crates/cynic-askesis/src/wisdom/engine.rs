//! Wisdom Context Formatting — inject curated patterns into Dog prompts

use super::DomainCurations;

/// Format wisdom signals as prompt context for evaluator Dogs.
/// Pattern: "## Domain Signals (D{domain})" followed by numbered patterns + falsifiable claims
pub fn format_wisdom_context(
    curations: &DomainCurations,
    domain: &str,
    content: &str,
    budget_chars: usize,
) -> Option<String> {
    let all_signals = curations.find_matching_signals(domain, content);

    // Filter low-confidence signals (strength < 0.4) to avoid injecting noisy wisdom
    const STRENGTH_THRESHOLD: f64 = 0.4;
    let signals: Vec<_> = all_signals
        .into_iter()
        .filter(|s| s.strength >= STRENGTH_THRESHOLD)
        .collect();

    if signals.is_empty() {
        return None;
    }

    let mut output = format!("## Domain Signals ({}) — {}\n", domain, signals.len());
    let mut chars_used = output.len();

    for (idx, signal) in signals.iter().enumerate() {
        let entry = format!(
            "{}. [{}] {} (strength: {:.2})\n   Falsifiable: {}\n",
            idx + 1,
            signal.signal_id,
            signal.pattern,
            signal.strength,
            signal.falsifiable_claim
        );

        if chars_used + entry.len() > budget_chars {
            break;
        }

        output.push_str(&entry);
        chars_used += entry.len();
    }

    if output.len() > "## Domain Signals".len() {
        Some(output)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wisdom::DomainSignal;

    #[test]
    fn test_format_wisdom_context() {
        let mut curations = DomainCurations::new();
        curations.signals.insert(
            "D1".to_string(),
            vec![DomainSignal {
                signal_id: "D1_001".to_string(),
                domain: "D1".to_string(),
                pattern: "Rust compiler ICE".to_string(),
                strength: 0.9,
                sources: vec![],
                falsifiable_claim: "If LOC > 50k then SIGSEGV".to_string(),
            }],
        );

        let context = format_wisdom_context(&curations, "D1", "Rust compiler error", 1000);
        assert!(context.is_some());
        let c = context.unwrap();
        assert!(c.contains("## Domain Signals (D1)"));
        assert!(c.contains("D1_001"));
        assert!(c.contains("Rust compiler ICE"));
    }

    #[test]
    fn test_empty_signals_returns_none() {
        let curations = DomainCurations::new();
        assert!(format_wisdom_context(&curations, "D1", "content", 1000).is_none());
    }

    #[test]
    fn test_respects_budget() {
        let mut curations = DomainCurations::new();
        curations.signals.insert(
            "D1".to_string(),
            vec![
                DomainSignal {
                    signal_id: "D1_001".to_string(),
                    domain: "D1".to_string(),
                    pattern:
                        "Very long pattern description that takes lots of characters to write out"
                            .to_string(),
                    strength: 0.9,
                    sources: vec![],
                    falsifiable_claim: "Long claim string".to_string(),
                },
                DomainSignal {
                    signal_id: "D1_002".to_string(),
                    domain: "D1".to_string(),
                    pattern: "Second pattern".to_string(),
                    strength: 0.8,
                    sources: vec![],
                    falsifiable_claim: "Claim 2".to_string(),
                },
            ],
        );

        let context = format_wisdom_context(&curations, "D1", "pattern", 120);
        assert!(context.is_some());
        let c = context.unwrap();
        assert!(!c.contains("D1_002"));
    }
}
