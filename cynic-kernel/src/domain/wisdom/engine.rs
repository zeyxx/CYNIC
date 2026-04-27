//! Wisdom Context Formatting — inject curated patterns into Dog prompts

use super::DomainCurations;

/// Format wisdom signals as prompt context for Dogs.
/// Pattern: "## Domain Signals (D{domain})" followed by numbered patterns + falsifiable claims
pub fn format_wisdom_context(
    curations: &DomainCurations,
    domain: &str,
    content: &str,
    budget_chars: usize,
) -> Option<String> {
    let signals = curations.find_matching_signals(domain, content);

    if signals.is_empty() {
        return None;
    }

    let mut output = format!("## Domain Signals ({}) — {}\n", domain, signals.len());

    let mut chars_used = output.len();

    for (idx, signal) in signals.iter().enumerate() {
        if chars_used >= budget_chars {
            break;
        }

        let signal_text = format!(
            "{}. {} (strength: {:.2})\n   Falsifiable: {}\n",
            idx + 1,
            signal.pattern,
            signal.strength,
            signal.falsifiable_claim
        );

        if chars_used + signal_text.len() <= budget_chars {
            output.push_str(&signal_text);
            chars_used += signal_text.len();
        } else {
            // Truncate to fit
            let remaining = budget_chars - chars_used;
            if remaining > 20 {
                output.push_str(&signal_text[..remaining.saturating_sub(5)]);
                output.push_str("...");
            }
            break;
        }
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
    use crate::domain::wisdom::DomainSignal;

    #[test]
    fn test_format_wisdom_context() {
        let mut curations = DomainCurations::new();

        let signal = DomainSignal {
            signal_id: "D1_test".to_string(),
            domain: "D1".to_string(),
            pattern: "Liquidity is permanently locked".to_string(),
            strength: 0.85,
            sources: vec![],
            falsifiable_claim: "If locked: rug probability < 0.1".to_string(),
        };

        curations.signals.insert("D1".to_string(), vec![signal]);

        let content = "Token with liquidity locked forever";
        let context = format_wisdom_context(&curations, "D1", content, 500);

        assert!(context.is_some());
        let formatted = context.unwrap();
        assert!(formatted.contains("Domain Signals"));
        assert!(formatted.contains("Liquidity is permanently locked"));
        assert!(formatted.contains("0.85"));
    }

    #[test]
    fn test_empty_signals_returns_none() {
        let curations = DomainCurations::new();
        let context = format_wisdom_context(&curations, "D1", "random content", 500);
        assert!(context.is_none());
    }

    #[test]
    fn test_respects_budget() {
        let mut curations = DomainCurations::new();

        let signals = (1..=5)
            .map(|i| DomainSignal {
                signal_id: format!("D1_{i}"),
                domain: "D1".to_string(),
                pattern: format!("Pattern number {i} with some text"),
                strength: 0.8,
                sources: vec![],
                falsifiable_claim: "Test claim".to_string(),
            })
            .collect();

        curations.signals.insert("D1".to_string(), signals);

        let context = format_wisdom_context(&curations, "D1", "pattern", 100);
        assert!(context.is_some());
        let formatted = context.unwrap();
        assert!(formatted.len() <= 120); // Budget + small margin
    }
}
