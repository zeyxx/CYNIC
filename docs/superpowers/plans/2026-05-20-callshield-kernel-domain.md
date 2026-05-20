# CallShield — Kernel phone_number Domain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `phone-number` domain to the CYNIC kernel so that phone numbers can be judged via `/judge domain="phone-number"`, with a deterministic Dog for heuristic scoring and a stimulus builder for structured phone metadata.

**Architecture:** The CYNIC kernel is already domain-agnostic. Adding `phone-number` requires: a domain prompt file, a one-line registration, a `PhoneData` struct + stimulus builder, a deterministic Dog with heuristic scoring (call frequency, report consensus, decay), and an enrichment stage. No refactoring — this follows the exact pattern of `token-analysis` and `wallet-judgment`.

**Tech Stack:** Rust (existing kernel), SurrealDB (existing storage), TFLite model export deferred to Plan 2 (mobile apps).

**Spec:** `docs/superpowers/specs/2026-05-19-callshield-anti-spam-app-design.md`

**Scope:** Kernel domain only (Plan 1 of 4). Does NOT include mobile apps, voice proxy, or federation.

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `cynic-kernel/domains/phone-number.md` | CREATE | Axiom evaluation prompt for phone number judgment |
| `cynic-kernel/src/infra/embedded_domains.rs` | MODIFY | Register `phone-number` domain in embedded array |
| `cynic-kernel/src/domain/mod.rs` | MODIFY | Add `pub mod phone_number;` |
| `cynic-kernel/src/domain/phone_number.rs` | CREATE | `PhoneData` struct, `PhoneReport`, domain types |
| `cynic-kernel/src/domain/stimulus.rs` | MODIFY | Add `build_phone_stimulus()` |
| `cynic-kernel/src/dogs/deterministic/mod.rs` | MODIFY | Add `mod phone_number;` + dispatch |
| `cynic-kernel/src/dogs/deterministic/phone_number.rs` | CREATE | `parse()` + `score()` heuristic scorer |
| `cynic-kernel/src/pipeline/mod.rs` | MODIFY | Add `phone-number` to SOVEREIGN_DOMAINS + enrichment gate |
| `cynic-kernel/src/pipeline/enrichment.rs` | MODIFY | Add `enrich_phone()` stub (real enrichment in Plan 2) |

---

## Task 1: Domain Prompt File

**Files:**
- Create: `cynic-kernel/domains/phone-number.md`

- [ ] **Step 1: Study the token-analysis.md template**

Run: `head -80 cynic-kernel/domains/token-analysis.md`

Note the structure: H1 title, then `## AXIOM_NAME` sections with HIGH/MEDIUM/LOW bands and numeric ranges.

- [ ] **Step 2: Create phone-number.md**

Create `cynic-kernel/domains/phone-number.md`:

```markdown
# Phone Number Domain — Axiom Evaluation Criteria

You are evaluating a phone number based on community reports and call metadata. Your task is to assess whether this number is legitimate, nuisance (telemarketing), or a scam.

## FIDELITY
How faithful are the signals about this number?

- HIGH (0.7-1.0): 10+ independent reporters with agreement_rate > 0.75, consistent labels over time
- MEDIUM (0.4-0.7): 3-10 reporters, mixed labels, some temporal consistency
- LOW (0.0-0.4): <3 reporters, contradictory labels, or single-source data

## PHI
Is the evidence proportional and structurally sound?

- HIGH (0.7-1.0): Report count proportional to call volume, no suspicious spikes, temporal distribution is natural
- MEDIUM (0.4-0.7): Some clustering in reports but within normal bounds
- LOW (0.0-0.4): Report spike in single minute (possible coordinated attack), or single reporter dominates

## VERIFY
Is the judgment falsifiable and testable?

- HIGH (0.7-1.0): Score is derived from measurable signals (frequency, consensus, decay), contestation mechanism exists
- MEDIUM (0.4-0.7): Score is derived but some inputs are uncertain (spoofed CLI, low confidence)
- LOW (0.0-0.4): Score based on single unverified report, no corroboration possible

## CULTURE
Does this respect community patterns and norms?

- HIGH (0.7-1.0): Number behavior matches known patterns (business hours for telemarketing, rotating numbers for scam)
- MEDIUM (0.4-0.7): Behavior partially matches known patterns
- LOW (0.0-0.4): Number behavior is anomalous, doesn't fit known categories

## BURN
Is the judgment efficient and minimal?

- HIGH (0.7-1.0): Decision is clear from heuristic signals alone, no inference needed
- MEDIUM (0.4-0.7): Ambiguous zone, additional signals (challenge result) would help
- LOW (0.0-0.4): Insufficient data to score meaningfully, wasting resources on judgment

## SOVEREIGNTY
Does this preserve caller and callee agency?

- HIGH (0.7-1.0): Presumption of innocence applied, contestation available, no single entity controls the score
- MEDIUM (0.4-0.7): Score influenced by federated consensus but single-node dominance possible
- LOW (0.0-0.4): Score from single source with no contestation path
```

- [ ] **Step 3: Verify file is well-formed**

Run: `wc -l cynic-kernel/domains/phone-number.md && head -3 cynic-kernel/domains/phone-number.md`

Expected: ~40-50 lines, first line starts with `# Phone Number Domain`

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/domains/phone-number.md
git commit -m "feat(domain): add phone-number axiom evaluation prompt"
```

---

## Task 2: Register Domain in Kernel

**Files:**
- Modify: `cynic-kernel/src/infra/embedded_domains.rs`

- [ ] **Step 1: Read the embedded domains file**

Run: Read `cynic-kernel/src/infra/embedded_domains.rs` to find the domains array and test function.

- [ ] **Step 2: Add test assertion first (TDD)**

In the test function (around line 87), add:

```rust
assert!(prompts.contains_key("phone-number"), "phone-number domain prompt missing");
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p cynic-kernel test_load_embedded_domains -- --nocapture`

Expected: FAIL — `phone-number domain prompt missing`

- [ ] **Step 4: Add domain registration**

In the domains array (around line 26), add the tuple:

```rust
("phone-number", include_str!("../../domains/phone-number.md")),
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p cynic-kernel test_load_embedded_domains -- --nocapture`

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/infra/embedded_domains.rs
git commit -m "feat(domain): register phone-number in embedded domains"
```

---

## Task 3: PhoneData Domain Types

**Files:**
- Modify: `cynic-kernel/src/domain/mod.rs`
- Create: `cynic-kernel/src/domain/phone_number.rs`

- [ ] **Step 1: Read domain/mod.rs to see module structure**

Run: Read `cynic-kernel/src/domain/mod.rs`

- [ ] **Step 2: Read wallet_judgment module as template**

Run: Read `cynic-kernel/src/domain/wallet_judgment/mod.rs` (or the flat file) to see the struct pattern.

- [ ] **Step 3: Create phone_number.rs with PhoneData struct**

Create `cynic-kernel/src/domain/phone_number.rs`:

```rust
//! Phone number domain types for CallShield.
//!
//! A phone number is judged by community reports (human Dog verdicts),
//! call metadata, and temporal patterns. The score represents spam
//! likelihood: 0.0 = safe, 1.0 = confirmed scam.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Core data structure for a phone number under judgment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneData {
    /// E.164 format phone number (e.g., "+33612345678")
    pub number: String,
    /// ISO 3166 country code
    pub country_code: String,
    /// Total call events recorded for this number
    pub total_events: u64,
    /// Label distribution from reporters
    pub label_distribution: LabelDistribution,
    /// Number of independent reporters
    pub reporter_count: u32,
    /// Weighted mean trust tier of reporters (0.2 = all NEW, 1.0 = all TRUSTED)
    pub mean_reporter_trust: f32,
    /// Days since first seen
    pub age_days: u32,
    /// Days since last report
    pub days_since_last_report: u32,
    /// Challenge pass rate if voice proxy was used (None if never challenged)
    pub challenge_pass_rate: Option<f32>,
    /// Number of verified contestations
    pub contestation_count: u32,
    /// Whether this number has been OTP-verified by its owner
    pub owner_verified: bool,
}

/// Distribution of labels from reporters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LabelDistribution {
    pub legitimate: u32,
    pub nuisance: u32,
    pub scam: u32,
    pub unknown: u32,
}

impl LabelDistribution {
    pub fn total(&self) -> u32 {
        self.legitimate + self.nuisance + self.scam + self.unknown
    }

    /// Weighted spam score: 0.0 (all legitimate) to 1.0 (all scam)
    pub fn spam_score(&self) -> f32 {
        let total = self.total() as f32;
        if total == 0.0 {
            return 0.5; // no data = ambiguous, not safe
        }
        let weighted = (self.nuisance as f32 * 0.75) + (self.scam as f32 * 1.0)
            + (self.unknown as f32 * 0.5);
        weighted / total
    }
}

/// A single report from a human reporter (a "human Dog verdict").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneReport {
    pub reporter_id: String,
    pub label: PhoneLabel,
    pub trust_weight: f32,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PhoneLabel {
    Legitimate,
    Nuisance,
    Scam,
    Unknown,
}

impl PhoneLabel {
    pub fn numeric_value(&self) -> f32 {
        match self {
            Self::Legitimate => 0.0,
            Self::Unknown => 0.5,
            Self::Nuisance => 0.75,
            Self::Scam => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_distribution_spam_score_all_legit() {
        let dist = LabelDistribution { legitimate: 10, nuisance: 0, scam: 0, unknown: 0 };
        assert!((dist.spam_score() - 0.0).abs() < 0.001);
    }

    #[test]
    fn label_distribution_spam_score_all_scam() {
        let dist = LabelDistribution { legitimate: 0, nuisance: 0, scam: 10, unknown: 0 };
        assert!((dist.spam_score() - 1.0).abs() < 0.001);
    }

    #[test]
    fn label_distribution_spam_score_mixed() {
        let dist = LabelDistribution { legitimate: 5, nuisance: 3, scam: 2, unknown: 0 };
        // (3*0.75 + 2*1.0) / 10 = 4.25/10 = 0.425
        assert!((dist.spam_score() - 0.425).abs() < 0.001);
    }

    #[test]
    fn label_distribution_spam_score_empty() {
        let dist = LabelDistribution::default();
        assert!((dist.spam_score() - 0.5).abs() < 0.001);
    }

    #[test]
    fn phone_label_numeric_values() {
        assert!((PhoneLabel::Legitimate.numeric_value() - 0.0).abs() < 0.001);
        assert!((PhoneLabel::Nuisance.numeric_value() - 0.75).abs() < 0.001);
        assert!((PhoneLabel::Scam.numeric_value() - 1.0).abs() < 0.001);
    }
}
```

- [ ] **Step 4: Add module to domain/mod.rs**

Add `pub mod phone_number;` to the module list in `cynic-kernel/src/domain/mod.rs`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p cynic-kernel phone_number -- --nocapture`

Expected: 5 tests PASS (label_distribution_spam_score_*, phone_label_numeric_values)

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/domain/phone_number.rs cynic-kernel/src/domain/mod.rs
git commit -m "feat(domain): add PhoneData and LabelDistribution types"
```

---

## Task 4: Stimulus Builder

**Files:**
- Modify: `cynic-kernel/src/domain/stimulus.rs`

- [ ] **Step 1: Read existing stimulus builders**

Run: Read `cynic-kernel/src/domain/stimulus.rs` lines 30-60 (build_token_stimulus) and lines 356-397 (build_wallet_stimulus) to understand the pattern.

- [ ] **Step 2: Write test first**

Add at the bottom of the test module in `stimulus.rs`:

```rust
#[test]
fn test_build_phone_stimulus_structure() {
    use crate::domain::phone_number::{PhoneData, LabelDistribution};
    let data = PhoneData {
        number: "+33612345678".to_string(),
        country_code: "FR".to_string(),
        total_events: 47,
        label_distribution: LabelDistribution {
            legitimate: 3, nuisance: 30, scam: 12, unknown: 2,
        },
        reporter_count: 35,
        mean_reporter_trust: 0.65,
        age_days: 14,
        days_since_last_report: 1,
        challenge_pass_rate: Some(0.15),
        contestation_count: 0,
        owner_verified: false,
    };
    let stimulus = build_phone_stimulus(&data);
    assert!(stimulus.contains("[DOMAIN: phone-number]"));
    assert!(stimulus.contains("+33612345678"));
    assert!(stimulus.contains("total_events: 47"));
    assert!(stimulus.contains("spam_score:"));
    assert!(stimulus.contains("challenge_pass_rate: 15.0%"));
    assert!(stimulus.contains("[QUESTION]"));
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p cynic-kernel test_build_phone_stimulus_structure -- --nocapture`

Expected: FAIL — `build_phone_stimulus` not found

- [ ] **Step 4: Implement build_phone_stimulus**

Add to `stimulus.rs` (after `build_wallet_stimulus`):

```rust
pub fn build_phone_stimulus(data: &crate::domain::phone_number::PhoneData) -> String {
    let mut s = String::with_capacity(512);
    s.push_str("[DOMAIN: phone-number]\n\n");

    // Metrics
    s.push_str("[METRICS]\n");
    s.push_str(&format!("number: {}\n", data.number));
    s.push_str(&format!("country: {}\n", data.country_code));
    s.push_str(&format!("total_events: {}\n", data.total_events));
    s.push_str(&format!("reporter_count: {}\n", data.reporter_count));
    s.push_str(&format!("mean_reporter_trust: {:.2}\n", data.mean_reporter_trust));
    s.push_str(&format!("age_days: {}\n", data.age_days));
    s.push_str(&format!("days_since_last_report: {}\n", data.days_since_last_report));
    s.push_str(&format!("contestation_count: {}\n", data.contestation_count));
    s.push_str(&format!("owner_verified: {}\n", data.owner_verified));

    // Label distribution
    s.push_str(&format!("labels: legitimate={} nuisance={} scam={} unknown={}\n",
        data.label_distribution.legitimate,
        data.label_distribution.nuisance,
        data.label_distribution.scam,
        data.label_distribution.unknown));
    s.push_str(&format!("spam_score: {:.3}\n", data.label_distribution.spam_score()));

    // Challenge data
    match data.challenge_pass_rate {
        Some(rate) => s.push_str(&format!("challenge_pass_rate: {:.1}%\n", rate * 100.0)),
        None => s.push_str("challenge_pass_rate: N/A (never challenged)\n"),
    }

    // Axiom evidence
    s.push_str("\n[AXIOM EVIDENCE]\n");

    // FIDELITY: based on reporter count and trust
    if data.reporter_count >= 10 && data.mean_reporter_trust > 0.7 {
        s.push_str("FIDELITY: HIGH — 10+ independent trusted reporters\n");
    } else if data.reporter_count >= 3 {
        s.push_str("FIDELITY: MEDIUM — multiple reporters, moderate trust\n");
    } else {
        s.push_str("FIDELITY: LOW — insufficient independent reporters\n");
    }

    // VERIFY: contestation and owner verification
    if data.owner_verified {
        s.push_str("VERIFY: owner has OTP-verified this number\n");
    }
    if data.contestation_count > 0 {
        s.push_str(&format!("VERIFY: {} verified contestation(s)\n", data.contestation_count));
    }

    // Question
    s.push_str("\n[QUESTION]\n");
    s.push_str("Based on the above evidence, evaluate this phone number across all six axioms.\n");
    s.push_str("Is this number legitimate, nuisance (telemarketing), or a scam?\n");

    s
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p cynic-kernel test_build_phone_stimulus_structure -- --nocapture`

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/domain/stimulus.rs
git commit -m "feat(domain): add build_phone_stimulus for phone-number domain"
```

---

## Task 5: Deterministic Dog — Phone Number Scorer

**Files:**
- Modify: `cynic-kernel/src/dogs/deterministic/mod.rs`
- Create: `cynic-kernel/src/dogs/deterministic/phone_number.rs`

- [ ] **Step 1: Read the token deterministic dog as template**

Run: Read `cynic-kernel/src/dogs/deterministic/token.rs` lines 1-80 to see struct + parse + score pattern.

- [ ] **Step 2: Read the dispatch chain in mod.rs**

Run: Read `cynic-kernel/src/dogs/deterministic/mod.rs` lines 190-210 to see how token/twitter are dispatched.

- [ ] **Step 3: Create phone_number.rs with parse + score**

Create `cynic-kernel/src/dogs/deterministic/phone_number.rs`:

```rust
//! Deterministic Dog for phone number scoring.
//!
//! Parses the structured phone stimulus (from build_phone_stimulus)
//! and scores heuristically based on call frequency, reporter consensus,
//! temporal decay, and challenge results.

use crate::domain::dog::AxiomScores;

#[derive(Debug)]
pub(super) struct PhoneMetrics {
    pub total_events: u64,
    pub reporter_count: u32,
    pub mean_reporter_trust: f32,
    pub spam_score: f32,
    pub age_days: u32,
    pub days_since_last_report: u32,
    pub challenge_pass_rate: Option<f32>,
    pub contestation_count: u32,
    pub owner_verified: bool,
    pub legitimate_count: u32,
    pub scam_count: u32,
}

pub(super) fn parse(content: &str) -> Option<PhoneMetrics> {
    if !content.contains("[DOMAIN: phone-number]") {
        return None;
    }

    Some(PhoneMetrics {
        total_events: extract_u64(content, "total_events:").unwrap_or(0),
        reporter_count: extract_u32(content, "reporter_count:").unwrap_or(0),
        mean_reporter_trust: extract_f32(content, "mean_reporter_trust:").unwrap_or(0.2),
        spam_score: extract_f32(content, "spam_score:").unwrap_or(0.5),
        age_days: extract_u32(content, "age_days:").unwrap_or(0),
        days_since_last_report: extract_u32(content, "days_since_last_report:").unwrap_or(0),
        challenge_pass_rate: extract_challenge_rate(content),
        contestation_count: extract_u32(content, "contestation_count:").unwrap_or(0),
        owner_verified: content.contains("owner_verified: true"),
        legitimate_count: extract_label_count(content, "legitimate="),
        scam_count: extract_label_count(content, "scam="),
    })
}

pub(super) fn score(m: &PhoneMetrics) -> AxiomScores {
    let mut scores = AxiomScores::default();

    // All AxiomScores fields are f64. Cast f32 inputs as needed.
    let spam = m.spam_score as f64;
    let trust = m.mean_reporter_trust as f64;
    let reporters = m.reporter_count as f64;
    let events = m.total_events.max(1) as f64;

    // FIDELITY: based on reporter count and trust quality
    scores.fidelity = if m.reporter_count >= 10 && trust > 0.7 {
        0.85
    } else if m.reporter_count >= 5 {
        0.6
    } else if m.reporter_count >= 2 {
        0.4
    } else {
        0.2
    };

    // PHI: structural proportionality — is the evidence balanced?
    let reporter_ratio = reporters / events;
    scores.phi = if reporter_ratio > 0.3 {
        0.8
    } else if reporter_ratio > 0.1 {
        0.5
    } else {
        0.3
    };

    // VERIFY: falsifiability — can we contest/verify?
    scores.verify = 0.5;
    if m.owner_verified {
        scores.verify += 0.2;
    }
    if m.contestation_count > 0 {
        scores.verify += 0.1;
    }
    if m.challenge_pass_rate.is_some() {
        scores.verify += 0.1;
    }
    scores.verify = scores.verify.min(1.0);

    // CULTURE: does behavior match known patterns?
    scores.culture = if spam < 0.3 && m.age_days > 30 {
        0.8
    } else if spam > 0.7 && m.age_days < 7 {
        0.7
    } else if spam > 0.7 {
        0.6
    } else {
        0.4
    };

    // BURN: efficiency — is the judgment clear?
    scores.burn = if spam > 0.8 || spam < 0.2 {
        0.9
    } else if m.reporter_count >= 5 {
        0.6
    } else {
        0.3
    };

    // SOVEREIGNTY: does scoring preserve agency?
    scores.sovereignty = 0.7;
    if m.owner_verified && m.contestation_count > 0 {
        scores.sovereignty = 0.9;
    }

    scores
}

// --- Parsing helpers ---

fn extract_u64(content: &str, key: &str) -> Option<u64> {
    content.lines()
        .find(|l| l.trim().starts_with(key))
        .and_then(|l| l.trim().strip_prefix(key))
        .and_then(|v| v.trim().parse().ok())
}

fn extract_u32(content: &str, key: &str) -> Option<u32> {
    extract_u64(content, key).map(|v| v as u32)
}

fn extract_f32(content: &str, key: &str) -> Option<f32> {
    content.lines()
        .find(|l| l.trim().starts_with(key))
        .and_then(|l| l.trim().strip_prefix(key))
        .and_then(|v| v.trim().parse().ok())
}

fn extract_challenge_rate(content: &str) -> Option<f32> {
    content.lines()
        .find(|l| l.trim().starts_with("challenge_pass_rate:"))
        .and_then(|l| {
            let val = l.trim().strip_prefix("challenge_pass_rate:")?.trim();
            if val.starts_with("N/A") {
                None
            } else {
                // "15.0%" -> 0.15
                val.strip_suffix('%')?.trim().parse::<f32>().ok().map(|v| v / 100.0)
            }
        })
}

fn extract_label_count(content: &str, key: &str) -> u32 {
    content.lines()
        .find(|l| l.trim().starts_with("labels:"))
        .and_then(|l| {
            l.split_whitespace()
                .find(|seg| seg.starts_with(key))
                .and_then(|seg| seg.strip_prefix(key))
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::phone_number::{PhoneData, LabelDistribution};
    use crate::domain::stimulus::build_phone_stimulus;

    fn sample_phone_data() -> PhoneData {
        PhoneData {
            number: "+33612345678".to_string(),
            country_code: "FR".to_string(),
            total_events: 47,
            label_distribution: LabelDistribution {
                legitimate: 3, nuisance: 30, scam: 12, unknown: 2,
            },
            reporter_count: 35,
            mean_reporter_trust: 0.75,
            age_days: 14,
            days_since_last_report: 1,
            challenge_pass_rate: Some(0.15),
            contestation_count: 0,
            owner_verified: false,
        }
    }

    #[test]
    fn parse_phone_stimulus() {
        let stimulus = build_phone_stimulus(&sample_phone_data());
        let metrics = parse(&stimulus).expect("should parse phone stimulus");
        assert_eq!(metrics.total_events, 47);
        assert_eq!(metrics.reporter_count, 35);
        assert!((metrics.mean_reporter_trust - 0.75).abs() < 0.01);
        assert!((metrics.spam_score - 0.425).abs() < 0.01); // from LabelDistribution
        assert_eq!(metrics.age_days, 14);
        assert!((metrics.challenge_pass_rate.unwrap() - 0.15).abs() < 0.01);
        assert_eq!(metrics.scam_count, 12);
        assert_eq!(metrics.legitimate_count, 3);
    }

    #[test]
    fn parse_non_phone_returns_none() {
        let stimulus = "[DOMAIN: token-analysis]\nsome token stuff";
        assert!(parse(stimulus).is_none());
    }

    #[test]
    fn score_confirmed_spam() {
        let stimulus = build_phone_stimulus(&sample_phone_data());
        let metrics = parse(&stimulus).unwrap();
        let scores = score(&metrics);
        // 35 reporters with trust 0.75 -> FIDELITY should be HIGH
        assert!(scores.fidelity > 0.7, "fidelity should be high: {}", scores.fidelity);
        // Sovereignty base = 0.7 (no contestation, no owner verification)
        assert!((scores.sovereignty - 0.7).abs() < 0.01);
    }

    #[test]
    fn score_unknown_number() {
        let data = PhoneData {
            number: "+33700000000".to_string(),
            country_code: "FR".to_string(),
            total_events: 1,
            label_distribution: LabelDistribution::default(),
            reporter_count: 0,
            mean_reporter_trust: 0.2,
            age_days: 0,
            days_since_last_report: 0,
            challenge_pass_rate: None,
            contestation_count: 0,
            owner_verified: false,
        };
        let stimulus = build_phone_stimulus(&data);
        let metrics = parse(&stimulus).unwrap();
        let scores = score(&metrics);
        // 0 reporters -> FIDELITY should be LOW
        assert!(scores.fidelity < 0.3, "fidelity should be low: {}", scores.fidelity);
        // BURN should be low (not enough data)
        assert!(scores.burn < 0.5, "burn should be low: {}", scores.burn);
    }
}
```

- [ ] **Step 4: Wire into deterministic dog dispatch**

In `cynic-kernel/src/dogs/deterministic/mod.rs`:

Add at top with other module declarations:
```rust
mod phone_number;
```

Add in the `evaluate()` dispatch chain (after twitter parse):
```rust
if let Some(ref p) = phone_number::parse(content) {
    return Ok(phone_number::score(p));
}
```

- [ ] **Step 5: Run all phone_number tests**

Run: `cargo test -p cynic-kernel phone_number -- --nocapture`

Expected: All tests pass (domain types + deterministic dog)

- [ ] **Step 6: Run full check to ensure nothing broken**

Run: `cargo check --workspace --all-targets`

Expected: No errors

- [ ] **Step 7: Commit**

```bash
git add cynic-kernel/src/dogs/deterministic/phone_number.rs cynic-kernel/src/dogs/deterministic/mod.rs
git commit -m "feat(dogs): add deterministic phone number scorer"
```

---

## Task 6: Pipeline Wiring — Sovereign Domain + Enrichment Stub

**Files:**
- Modify: `cynic-kernel/src/pipeline/mod.rs`
- Modify: `cynic-kernel/src/pipeline/enrichment.rs`

- [ ] **Step 1: Read pipeline/mod.rs to find SOVEREIGN_DOMAINS and enrichment gates**

Run: Read `cynic-kernel/src/pipeline/mod.rs` lines 35-50 (SOVEREIGN_DOMAINS) and lines 155-200 (enrichment stages).

- [ ] **Step 2: Add phone-number to SOVEREIGN_DOMAINS**

In `pipeline/mod.rs`, find:
```rust
const SOVEREIGN_DOMAINS: &[&str] = &["social-dm", "private", "wallet-judgment"];
```

Change to:
```rust
const SOVEREIGN_DOMAINS: &[&str] = &["social-dm", "private", "wallet-judgment", "phone-number"];
```

- [ ] **Step 3: Add phone-number to dynamic enrichment check**

Find the `has_dynamic_enrichment` check (around line 159) and add `"phone-number"` to the list of domains that skip cache.

- [ ] **Step 4: Read enrichment.rs to see the pattern**

Run: Read `cynic-kernel/src/pipeline/enrichment.rs` lines 285-310 (enrich_wallet) to see the guard pattern.

- [ ] **Step 5: Add enrich_phone stub**

In `enrichment.rs`, add after `enrich_wallet`:

```rust
/// Phone number enrichment — stub for Plan 1.
/// Real enrichment (community consensus aggregation, federation lookup)
/// will be added when mobile apps feed /observe with CallEvents.
pub(super) fn enrich_phone(
    _stimulus: &mut String,
    domain_hint: &str,
) {
    if domain_hint != "phone-number" {
        return;
    }
    // Phase 1: no enrichment beyond what build_phone_stimulus provides.
    // Phase 2 (mobile apps): aggregate CallEvents from /observe,
    // compute live NumberReputation, inject into stimulus.
}
```

- [ ] **Step 6: Wire enrich_phone in pipeline/mod.rs**

After the `enrich_wallet` call (around line 199), add:

```rust
// Stage 6.5: Phone enrichment
enrichment::enrich_phone(&mut content, domain_hint);
```

- [ ] **Step 7: Run cargo check**

Run: `cargo check --workspace --all-targets`

Expected: No errors

- [ ] **Step 8: Commit**

```bash
git add cynic-kernel/src/pipeline/mod.rs cynic-kernel/src/pipeline/enrichment.rs
git commit -m "feat(pipeline): wire phone-number as sovereign domain + enrichment stub"
```

---

## Task 7: Integration Test — End-to-End /judge Round-Trip

**Files:**
- Modify: `cynic-kernel/src/pipeline/tests.rs` (or appropriate test file)

- [ ] **Step 1: Read existing pipeline tests**

Run: Read `cynic-kernel/src/pipeline/tests.rs` lines 30-60 to see the test pattern with `PipelineDeps` and `NullStorage`.

- [ ] **Step 2: Write integration test**

Add a test that sends a phone number stimulus through the full pipeline. Follow the exact pattern from the existing `pipeline_runs_with_null_storage_and_null_embedding` test:

```rust
#[tokio::test]
async fn test_phone_number_domain_produces_verdict() {
    let dogs: Vec<Arc<dyn Dog>> = vec![Arc::new(crate::dogs::deterministic::DeterministicDog)];
    let judge = test_judge(dogs);
    let storage = NullStorage;
    let embedding = NullEmbedding;
    let usage = Mutex::new(DogUsageTracker::new());
    let verdict_cache = VerdictCache::new();
    let metrics = Metrics::new();
    let domain_curations = crate::domain::wisdom::DomainCurations::new();

    let deps = PipelineDeps {
        judge: &judge,
        storage: &storage,
        embedding: &embedding,
        usage: &usage,
        verdict_cache: &verdict_cache,
        metrics: &metrics,
        event_tx: None,
        request_id: None,
        on_dog: None,
        expected_dog_count: judge.dog_ids().len(),
        enricher: None,
        domain_curations: &domain_curations,
        domain_router: None,
        priority: SlotPriority::User,
    };

    let phone_stimulus = "[DOMAIN: phone-number]\n\n\
        [METRICS]\n\
        number: +33612345678\n\
        country: FR\n\
        total_events: 50\n\
        reporter_count: 40\n\
        mean_reporter_trust: 0.80\n\
        age_days: 7\n\
        days_since_last_report: 0\n\
        contestation_count: 0\n\
        owner_verified: false\n\
        labels: legitimate=2 nuisance=30 scam=16 unknown=2\n\
        spam_score: 0.770\n\
        challenge_pass_rate: 10.0%\n\n\
        [AXIOM EVIDENCE]\n\
        FIDELITY: HIGH — 10+ independent trusted reporters\n\n\
        [QUESTION]\n\
        Based on the above evidence, evaluate this phone number.\n";

    let result = run(
        phone_stimulus.into(),
        None,
        Some("phone-number".into()),
        None,
        true,
        &deps,
    )
    .await;

    match result {
        Ok(PipelineResult::Evaluated { verdict, .. }) => {
            assert!(verdict.q_score.total > 0.0, "Q-Score should be > 0");
            assert!(!verdict.dog_scores.is_empty(), "should have dog scores");
        }
        Ok(PipelineResult::CacheHit { .. }) => panic!("expected evaluation, got cache hit"),
        Ok(other) => panic!("unexpected result variant: {other:?}"),
        Err(e) => panic!("pipeline error: {e}"),
    }
}
```

- [ ] **Step 3: Run integration test**

Run: `cargo test -p cynic-kernel test_phone_number_domain_produces_verdict -- --nocapture`

Expected: PASS — the pipeline processes the phone stimulus, deterministic dog scores it, verdict is produced.

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/pipeline/tests.rs
git commit -m "test(pipeline): add phone-number domain integration test"
```

---

## Task 8: Final Validation — make check

- [ ] **Step 1: Run the full gate**

Run: `make check`

Expected: build + test + clippy + lint-rules + lint-drift all pass.

- [ ] **Step 2: Fix any issues**

If clippy or lint-rules flag anything, fix before committing.

- [ ] **Step 3: Verify all phone_number tests pass**

Run: `cargo test -p cynic-kernel phone_number -- --nocapture`

Expected: All tests pass.

- [ ] **Step 4: Final commit if any fixes needed**

```bash
git commit -m "fix: address clippy/lint issues from phone-number domain"
```

---

## What This Plan Does NOT Cover (Plans 2-4)

| Plan | Scope | Depends on |
|------|-------|-----------|
| **Plan 2: Mobile Apps** | Android (Kotlin) + iOS (Swift) apps, CallScreeningService/CallKit, Embedded Dog (TFLite/CoreML), post-call labeling UX, local SQLite cache | Plan 1 (kernel domain must exist) |
| **Plan 3: Voice Proxy** | SIP integration (Twilio adapter), VoiceProxyPort trait, challenge flow, premium tier gating | Plan 1 + Plan 2 (apps must trigger proxy) |
| **Plan 4: Federation** | libp2p gossip layer in kernel, Ed25519 node identity, cross-node NumberReputation sync, Solana integrity anchoring | Plan 1 (domain types must exist) |

Plan 2 is the next priority — it produces the user-facing product.
