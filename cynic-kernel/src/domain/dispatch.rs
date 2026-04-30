//! Domain Routing Dispatcher
//!
//! Maps request domain hints to Dogs and stimulus builders.
//! Canonical taxonomy: D1-D6 from cynic-python/domains/__init__.py
//!
//! D1: Solana/Token
//! D2: Inference/LLM
//! D3: Sovereignty
//! D4: Security/Scams
//! D5: Macro/Politics
//! D6: Epistemology/Philosophy

use std::collections::HashMap;

/// Domain identifier (canonical per cynic-python/domains/__init__.py)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Domain {
    Token,       // D1: Solana/Tokens (rug detection, launch patterns)
    Inference,   // D2: Inference/LLM (model selection, VRAM math)
    Sovereignty, // D3: Sovereignty (infrastructure independence)
    Security,    // D4: Security/Scams (exploit patterns, social engineering)
    Macro,       // D5: Macro/Politics (regulatory, geopolitical)
    Philosophy,  // D6: Epistemology/Philosophy (calibration, confidence bounds)
    Chess,       // Special: Chess personality & archetype validation (Blitz & Chill)
    Wallet,      // Special: Wallet anti-Sybil judgment (game authenticity)
    Unknown,     // Fallback: no domain specified
}

/// Dogs assigned to each domain
#[derive(Debug, Clone)]
pub struct DogSet {
    pub dogs: Vec<String>,
    pub description: String,
}

impl Domain {
    /// Parse domain string to canonical Domain enum
    pub fn parse(domain_str: Option<&str>) -> Self {
        match domain_str {
            None => Domain::Unknown,
            Some(d) => {
                let lower = d.to_lowercase();
                match lower.as_str() {
                    // Canonical D-names
                    "d1" | "token" | "solana" | "tokens" => Domain::Token,
                    "d2" | "inference" | "llm" => Domain::Inference,
                    "d3" | "sovereignty" | "sovereign" => Domain::Sovereignty,
                    "d4" | "security" | "scams" | "scam" => Domain::Security,
                    "d5" | "macro" | "politics" | "geopolitical" => Domain::Macro,
                    "d6" | "philosophy" | "epistemology" | "philosophical" => Domain::Philosophy,
                    // Special domains (Hermes X origins)
                    "chess" | "chess-personality" | "game" => Domain::Chess,
                    "wallet" | "wallet-judgment" | "sybil" | "anti-sybil" => Domain::Wallet,
                    _ => Domain::Unknown,
                }
            }
        }
    }

    /// Get Dogs assigned to this domain
    /// **Production truth:** Matches cynic-python/domains/__init__.py consumers
    pub fn dog_set(&self) -> DogSet {
        match self {
            Domain::Token => DogSet {
                dogs: vec![
                    "deterministic-dog".into(),
                    "qwen-7b-hf".into(),
                    "qwen-9b-core".into(),
                    "qwen-9b-gpu".into(),
                ],
                description: "Token domain: rug detection, launch patterns, token authenticity".into(),
            },
            Domain::Inference => DogSet {
                dogs: vec![
                    // Inference is kernel-embedded; no external LLM Dogs
                    // Model selection and VRAM math happen in inference_router.rs
                ],
                description: "Inference/LLM: embedded in kernel (no external Dogs)".into(),
            },
            Domain::Sovereignty => DogSet {
                dogs: vec![
                    // NOT YET IMPLEMENTED: needs sovereign-focused Dog
                    // Placeholder: deterministic-dog can give partial answer
                    "deterministic-dog".into(),
                ],
                description: "Sovereignty: infrastructure independence, epistemic authority (STUB)".into(),
            },
            Domain::Security => DogSet {
                dogs: vec![
                    "qwen-7b-hf".into(),
                    "qwen-9b-core".into(),
                    "qwen-9b-gpu".into(),
                ],
                description: "Security/Scams: exploit patterns, social engineering, scam taxonomy".into(),
            },
            Domain::Macro => DogSet {
                dogs: vec![
                    "qwen-7b-hf".into(),
                    "qwen-9b-core".into(),
                    "qwen-9b-gpu".into(),
                ],
                description: "Macro/Politics: regulatory, geopolitical, election impact, macro cycles".into(),
            },
            Domain::Philosophy => DogSet {
                dogs: vec![
                    // NOT YET IMPLEMENTED: needs axiom-calibration Dog
                    "deterministic-dog".into(),
                ],
                description: "Philosophy/Epistemology: calibration, confidence bounds, falsification (STUB)".into(),
            },
            Domain::Chess => DogSet {
                dogs: vec![
                    "deterministic-dog".into(),
                    // qwen-9b for personality card depth (optional)
                ],
                description: "Chess: personality card validation, archetype consistency, move pattern analysis".into(),
            },
            Domain::Wallet => DogSet {
                dogs: vec![
                    "deterministic-dog".into(),
                ],
                description: "Wallet: anti-Sybil judgment, game authenticity, archetype consistency".into(),
            },
            Domain::Unknown => DogSet {
                dogs: vec![
                    "deterministic-dog".into(),
                    "qwen-7b-hf".into(),
                ],
                description: "Unknown domain: default fallback (deterministic + one LLM)".into(),
            },
        }
    }

    /// Get stimulus context string for this domain
    /// Used to guide Dogs toward domain-specific judgment
    pub fn stimulus_context(&self) -> String {
        match self {
            Domain::Token => {
                "[DOMAIN: token-judgment]\nEvaluate for: launch legitimacy, liquidity authenticity, rugpull indicators, team credibility.".into()
            }
            Domain::Inference => {
                "[DOMAIN: inference-judgment]\nEvaluate for: model accuracy, latency, VRAM efficiency, framework maturity.".into()
            }
            Domain::Sovereignty => {
                "[DOMAIN: sovereignty-judgment]\nEvaluate for: infrastructure independence, epistemic authority, cultural preservation, agency preservation.".into()
            }
            Domain::Security => {
                "[DOMAIN: security-judgment]\nEvaluate for: exploit patterns, social engineering risk, honeypot indicators, scam sophistication.".into()
            }
            Domain::Macro => {
                "[DOMAIN: macro-judgment]\nEvaluate for: regulatory impact, geopolitical risk, macro cycle alignment, election sensitivity.".into()
            }
            Domain::Philosophy => {
                "[DOMAIN: philosophy-judgment]\nEvaluate for: epistemic honesty, confidence bounds, falsifiability, axiom alignment.".into()
            }
            Domain::Chess => {
                "[DOMAIN: chess-personality]\nEvaluate for: archetype authenticity, move pattern consistency, game knowledge, anti-sybil signals.".into()
            }
            Domain::Wallet => {
                "[DOMAIN: wallet-judgment]\nEvaluate for: game history consistency, archetype alignment, sybil resistance, authentic human play.".into()
            }
            Domain::Unknown => {
                "[DOMAIN: unknown]\nEvaluate for: general fidelity, verifiability, cultural fit, efficiency.".into()
            }
        }
    }
}

/// Load DogSet configuration from environment/config
/// For now, returns hardcoded; future: load from backends.toml per K11 extraction
pub fn load_dog_sets() -> HashMap<String, DogSet> {
    let mut sets = HashMap::new();

    for domain in [
        Domain::Token,
        Domain::Inference,
        Domain::Sovereignty,
        Domain::Security,
        Domain::Macro,
        Domain::Philosophy,
        Domain::Chess,
        Domain::Wallet,
        Domain::Unknown,
    ] {
        sets.insert(format!("{domain:?}").to_lowercase(), domain.dog_set());
    }

    sets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_parse_canonical() {
        assert_eq!(Domain::parse(Some("d1")), Domain::Token);
        assert_eq!(Domain::parse(Some("token")), Domain::Token);
        assert_eq!(Domain::parse(Some("D1")), Domain::Token);
    }

    #[test]
    fn test_domain_parse_special() {
        assert_eq!(Domain::parse(Some("chess")), Domain::Chess);
        assert_eq!(Domain::parse(Some("wallet")), Domain::Wallet);
    }

    #[test]
    fn test_domain_parse_unknown() {
        assert_eq!(Domain::parse(None), Domain::Unknown);
        assert_eq!(Domain::parse(Some("unknown")), Domain::Unknown);
        assert_eq!(Domain::parse(Some("foobar")), Domain::Unknown);
    }

    #[test]
    fn test_token_dog_set() {
        let dogs = Domain::Token.dog_set();
        assert!(dogs.dogs.contains(&"deterministic-dog".into()));
        assert!(!dogs.dogs.is_empty());
    }

    #[test]
    fn test_all_domains_have_dogs() {
        for domain in [
            Domain::Token,
            Domain::Inference,
            Domain::Security,
            Domain::Chess,
            Domain::Wallet,
        ] {
            let dogs = domain.dog_set();
            // At minimum, fallback or deterministic-dog should be present
            assert!(!dogs.dogs.is_empty(), "{:?} has no dogs", domain);
        }
    }

    #[test]
    fn test_stimulus_context() {
        let ctx = Domain::Token.stimulus_context();
        assert!(ctx.contains("[DOMAIN: token-judgment]"));
        assert!(ctx.contains("rugpull"));
    }
}
