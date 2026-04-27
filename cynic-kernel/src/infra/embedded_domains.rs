//! Embedded domain prompts — compiled into the binary.
//! Eliminates runtime path discovery fragility: domains work from any cwd, no CYNIC_PROJECT_ROOT needed.

use std::collections::HashMap;

/// Load all domain prompts from embedded strings.
/// Returns a map of domain name → axiom prompt text.
pub fn load_embedded_domain_prompts() -> HashMap<String, String> {
    let mut prompts = HashMap::new();

    // Each domain is embedded as a static string. Strip H1 heading, preserve H2+.
    let domains = [
        ("chess", include_str!("../../domains/chess.md")),
        ("trading", include_str!("../../domains/trading.md")),
        (
            "token-analysis",
            include_str!("../../domains/token-analysis.md"),
        ),
        ("dev", include_str!("../../domains/dev.md")),
        ("general", include_str!("../../domains/general.md")),
        (
            "wallet-judgment",
            include_str!("../../domains/wallet-judgment.md"),
        ),
        ("twitter", include_str!("../../domains/twitter.md")),
    ];

    for (domain_name, content) in &domains {
        let prompt = strip_domain_heading(content);
        if !prompt.is_empty() {
            crate::klog!(
                "[config] Domain prompt loaded (embedded): '{}' ({} chars)",
                domain_name,
                prompt.len()
            );
            prompts.insert(domain_name.to_string(), prompt);
        }
    }

    prompts
}

/// Strip the H1 heading (first `# ...` line) but preserve H2+ sections.
/// pub(crate): also used by config.rs for filesystem-loaded domain prompts.
pub(crate) fn strip_domain_heading(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

    // Find first line that starts with `#` (H1 heading)
    let mut skip_until = 0;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        // Match `# ` followed by non-`#` (H1, not H2+)
        if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            skip_until = i + 1;
            break;
        }
    }

    // Return everything after the H1 heading
    if skip_until < lines.len() {
        lines[skip_until..].join("\n").trim().to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_embedded_domains() {
        let prompts = load_embedded_domain_prompts();
        assert!(!prompts.is_empty(), "should load at least one domain");
        assert!(prompts.contains_key("chess"), "should load chess domain");
        assert!(
            prompts.contains_key("trading"),
            "should load trading domain"
        );
        assert!(
            prompts.contains_key("general"),
            "should load general domain"
        );
        assert!(
            prompts.contains_key("wallet-judgment"),
            "should load wallet-judgment domain"
        );
    }

    #[test]
    fn test_strip_domain_heading() {
        let content = "# Chess Domain\n\n## FIDELITY\nTest\n## PHI\nTest2";
        let stripped = strip_domain_heading(content);
        assert!(
            stripped.starts_with("## FIDELITY"),
            "should preserve H2 sections"
        );
        assert!(
            !stripped.contains("# Chess Domain"),
            "should remove H1 heading"
        );
    }
}
