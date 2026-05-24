# Rug Risk Scoring — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a pure rug risk scoring function that predicts rug probability from 14 existing TokenData features, injected into the stimulus for LLM Dogs.

**Architecture:** Pure domain function (`rug_risk.rs`) computes a 0.0-1.0 score with named signals. Pipeline appends `[RUG RISK]` section to stimulus string after prefilter. No deterministic Dog changes in v1 — LLM Dogs consume it naturally.

**Tech Stack:** Rust (kernel domain + pipeline), no new dependencies.

**Spec:** `docs/superpowers/specs/2026-05-24-rug-risk-scoring-design.md`

---

## Pre-flight

| Question | Answer |
|---|---|
| Where does prefilter return? | `pipeline/mod.rs:299` — after early return for `PreFilterResult::Rug`, control falls through to Stage 6 |
| What is `content` at line 299? | The enriched stimulus string from `enrich_token()` (includes `[METRICS]`, `[BEHAVIORAL]`, etc.) |
| Does `content` need to be mutable? | Yes — currently `let content = token_result.content;` at line 198. Must be `let mut content` to append `[RUG RISK]` |
| Is `TokenData` available at line 299? | Yes — `captured_token_data: Option<TokenData>` from line 199 |
| Does `RugRiskAssessment` need Serialize? | No — only used to build a string section. But add it anyway for future outcome correlation storage |

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `cynic-kernel/src/domain/rug_risk.rs` | CREATE | Pure risk scoring function + 15 signal definitions + tests |
| `cynic-kernel/src/domain/mod.rs` | MODIFY | Add `pub mod rug_risk;` |
| `cynic-kernel/src/pipeline/mod.rs` | MODIFY | Call `assess_rug_risk()` at Stage 5c, append `[RUG RISK]` to content |

3 files total. No stimulus.rs change. No deterministic Dog change (v1).

---

## Task 1 — Create `domain/rug_risk.rs` with tests

**Files:**
- Create: `cynic-kernel/src/domain/rug_risk.rs`
- Modify: `cynic-kernel/src/domain/mod.rs`

- [ ] **Step 1: Write the full module with tests (RED then GREEN)**

Create `cynic-kernel/src/domain/rug_risk.rs`:

```rust
//! Rug risk scoring — predictive signal for live tokens.
//! Pure function on TokenData. No I/O, no async.
//! NOT a gate (that's rug_prefilter). This is information for Dogs.
//! K20: emits [RUG RISK] section; v1 = LLM Dogs only (no deterministic parser).

use crate::domain::enrichment::TokenData;

/// Assessment result: score + active signals with evidence.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RugRiskAssessment {
    /// 0.0 (safe) → 1.0 (imminent rug). Clamped sum of active signal weights.
    pub score: f64,
    /// Active signals sorted by weight descending.
    pub signals: Vec<RugSignal>,
}

/// A single risk signal with its contribution to the total score.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RugSignal {
    /// Signal identifier — matches spec signal table exactly.
    pub name: &'static str,
    /// Weight contribution to total score.
    pub weight: f64,
    /// Human-readable evidence string for stimulus display.
    pub evidence: String,
}

/// Assess rug risk from enriched token data.
/// Returns score 0.0 for clean tokens, up to 1.0 (clamped) for likely rugs.
pub fn assess_rug_risk(token: &TokenData) -> RugRiskAssessment {
    let mut signals = Vec::new();
    let holder_ok = token.holder_data_available;

    // ── Temporal signals ──

    if token.age_hours < 6 {
        signals.push(RugSignal {
            name: "very_young",
            weight: 0.25,
            evidence: format!("age={}h", token.age_hours),
        });
    }

    if holder_ok && token.age_hours < 24 && token.holder_count < 50 {
        signals.push(RugSignal {
            name: "young_low_holders",
            weight: 0.20,
            evidence: format!("age={}h, holders={}", token.age_hours, token.holder_count),
        });
    }

    match token.trajectory_class.as_deref() {
        Some("DYING") => signals.push(RugSignal {
            name: "dying_trajectory",
            weight: 0.15,
            evidence: format!(
                "trajectory=DYING, decay={:.2}",
                token.trajectory_decay.unwrap_or(0.0)
            ),
        }),
        Some("DECLINING") => signals.push(RugSignal {
            name: "declining_trajectory",
            weight: 0.10,
            evidence: format!(
                "trajectory=DECLINING, decay={:.2}",
                token.trajectory_decay.unwrap_or(0.0)
            ),
        }),
        _ => {}
    }

    // ── Structural signals ──

    if token.mint_authority_active {
        signals.push(RugSignal {
            name: "mint_authority_active",
            weight: 0.20,
            evidence: "mint authority can inflate supply".into(),
        });
    }

    if token.freeze_authority_active {
        signals.push(RugSignal {
            name: "freeze_authority_active",
            weight: 0.15,
            evidence: "freeze authority can lock wallets".into(),
        });
    }

    if token.lp_status != "burned" && token.age_hours > 24 {
        signals.push(RugSignal {
            name: "no_lp_commitment",
            weight: 0.15,
            evidence: format!("lp={}, age={}h", token.lp_status, token.age_hours),
        });
    }

    if token.supply_burned_pct.is_none_or(|p| p == 0.0) && token.age_hours > 24 {
        signals.push(RugSignal {
            name: "no_supply_burn",
            weight: 0.10,
            evidence: format!(
                "supply_burned={}%, age={}h",
                token.supply_burned_pct.unwrap_or(0.0),
                token.age_hours
            ),
        });
    }

    if token.origin.as_deref() == Some("pump.fun") {
        signals.push(RugSignal {
            name: "pump_fun_origin",
            weight: 0.10,
            evidence: "origin=pump.fun (98.6% baseline rug rate)".into(),
        });
    }

    // ── Behavioral signals ──

    if holder_ok && token.top1_pct > 50.0 {
        signals.push(RugSignal {
            name: "extreme_concentration",
            weight: 0.20,
            evidence: format!("top1={:.1}%", token.top1_pct),
        });
    }

    if holder_ok {
        if let Some(h) = token.herfindahl {
            if h > 0.3 {
                signals.push(RugSignal {
                    name: "unhealthy_distribution",
                    weight: 0.15,
                    evidence: format!("herfindahl={:.3}", h),
                });
            }
        }
    }

    if let Some(bsr) = token.buy_sell_ratio {
        if bsr < 0.5 {
            signals.push(RugSignal {
                name: "active_distribution",
                weight: 0.20,
                evidence: format!("buy_sell_ratio={:.2}", bsr),
            });
        }
    }

    if token.divergence_class.as_deref() == Some("DISTRIBUTION") {
        signals.push(RugSignal {
            name: "distribution_class",
            weight: 0.15,
            evidence: "divergence=DISTRIBUTION".into(),
        });
    }

    if let Some(ref ks) = token.kscore {
        if ks.diamond_hands < 0.2 {
            signals.push(RugSignal {
                name: "no_diamond_hands",
                weight: 0.10,
                evidence: format!("diamond_hands={:.3}", ks.diamond_hands),
            });
        }
    }

    if let (Some(vol), Some(liq)) = (token.volume_24h_usd, token.liquidity_usd) {
        if vol > 0.0 && liq < 1000.0 {
            signals.push(RugSignal {
                name: "thin_liquidity",
                weight: 0.15,
                evidence: format!("volume=${:.0}, liquidity=${:.0}", vol, liq),
            });
        }
    }

    // Sort by weight descending for stimulus display
    signals.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));

    let raw_score: f64 = signals.iter().map(|s| s.weight).sum();
    let score = raw_score.clamp(0.0, 1.0);

    RugRiskAssessment { score, signals }
}

/// Format risk assessment as a stimulus section string.
/// Returns empty string if score == 0.0 (no noise for clean tokens).
/// Caps display at top 5 signals by weight (K16: context is metabolic).
pub fn format_rug_risk_section(assessment: &RugRiskAssessment) -> String {
    if assessment.score == 0.0 && assessment.signals.is_empty() {
        return String::new();
    }

    let mut section = format!("\n[RUG RISK]\nrug_risk_score: {:.2}\n", assessment.score);
    for signal in assessment.signals.iter().take(5) {
        section.push_str(&format!(
            "  - {} ({:.2}): {}\n",
            signal.name, signal.weight, signal.evidence
        ));
    }
    section
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::enrichment::{KScore, TokenData};

    fn healthy() -> TokenData {
        TokenData {
            mint: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".into(),
            holder_count: 250_000,
            holder_data_available: true,
            top1_pct: 12.5,
            top10_pct: 45.2,
            age_hours: 1200,
            mint_authority_active: false,
            freeze_authority_active: false,
            lp_status: "burned".into(),
            supply_burned_pct: Some(0.0),
            liquidity_usd: Some(5_000_000.0),
            volume_24h_usd: Some(500_000.0),
            origin: None,
            trajectory_class: Some("STABLE".into()),
            buy_sell_ratio: Some(1.2),
            divergence_class: Some("STRONG_HOLD".into()),
            herfindahl: Some(0.05),
            kscore: Some(KScore {
                diamond_hands: 0.7,
                ..KScore::default()
            }),
            ..TokenData::default()
        }
    }

    fn has_signal(assessment: &RugRiskAssessment, name: &str) -> bool {
        assessment.signals.iter().any(|s| s.name == name)
    }

    #[test]
    fn healthy_token_score_zero() {
        let a = assess_rug_risk(&healthy());
        assert_eq!(a.score, 0.0);
        assert!(a.signals.is_empty());
    }

    #[test]
    fn young_pump_fun_rug() {
        let t = TokenData {
            mint: "RUGxxx1111111111111111111111111111111111111".into(),
            holder_count: 5,
            holder_data_available: true,
            top1_pct: 80.0,
            top10_pct: 99.0,
            age_hours: 2,
            mint_authority_active: true,
            freeze_authority_active: false,
            lp_status: "unsecured".into(),
            supply_burned_pct: None,
            liquidity_usd: Some(500.0),
            volume_24h_usd: Some(5000.0),
            origin: Some("pump.fun".into()),
            trajectory_class: None,
            buy_sell_ratio: Some(0.3),
            divergence_class: Some("DISTRIBUTION".into()),
            herfindahl: Some(0.6),
            kscore: Some(KScore {
                diamond_hands: 0.05,
                ..KScore::default()
            }),
            ..TokenData::default()
        };
        let a = assess_rug_risk(&t);
        assert!(a.score >= 0.5, "young pump.fun rug should score >= 0.5, got {}", a.score);
        assert!(has_signal(&a, "very_young"));
        assert!(has_signal(&a, "mint_authority_active"));
        assert!(has_signal(&a, "pump_fun_origin"));
    }

    #[test]
    fn moderate_risk() {
        let t = TokenData {
            mint: "MODxxx1111111111111111111111111111111111111".into(),
            holder_count: 200,
            holder_data_available: true,
            top1_pct: 25.0,
            top10_pct: 60.0,
            age_hours: 30,
            mint_authority_active: false,
            freeze_authority_active: false,
            lp_status: "unsecured".into(),
            supply_burned_pct: Some(0.0),
            liquidity_usd: Some(10_000.0),
            volume_24h_usd: Some(5000.0),
            ..TokenData::default()
        };
        let a = assess_rug_risk(&t);
        assert!(a.score > 0.0, "moderate risk should be > 0.0");
        assert!(a.score < 0.5, "moderate risk should be < 0.5, got {}", a.score);
        assert!(has_signal(&a, "no_lp_commitment"));
        assert!(has_signal(&a, "no_supply_burn"));
    }

    #[test]
    fn distribution_pattern() {
        let mut t = healthy();
        t.buy_sell_ratio = Some(0.3);
        t.divergence_class = Some("DISTRIBUTION".into());
        let a = assess_rug_risk(&t);
        assert!(has_signal(&a, "active_distribution"));
        assert!(has_signal(&a, "distribution_class"));
    }

    #[test]
    fn degraded_rpc_skips_holder_signals() {
        let t = TokenData {
            holder_data_available: false,
            holder_count: 0,
            top1_pct: 0.0,
            herfindahl: Some(0.0),
            age_hours: 2,
            ..TokenData::default()
        };
        let a = assess_rug_risk(&t);
        assert!(!has_signal(&a, "young_low_holders"), "should skip when holder_data_available=false");
        assert!(!has_signal(&a, "extreme_concentration"), "should skip when holder_data_available=false");
        assert!(!has_signal(&a, "unhealthy_distribution"), "should skip when holder_data_available=false");
        // very_young should still fire (not holder-dependent)
        assert!(has_signal(&a, "very_young"));
    }

    #[test]
    fn trajectory_dying() {
        let mut t = healthy();
        t.trajectory_class = Some("DYING".into());
        t.trajectory_decay = Some(0.85);
        let a = assess_rug_risk(&t);
        assert!(has_signal(&a, "dying_trajectory"));
    }

    #[test]
    fn trajectory_none_is_silent() {
        let mut t = healthy();
        t.trajectory_class = None;
        let a = assess_rug_risk(&t);
        assert!(!has_signal(&a, "dying_trajectory"));
        assert!(!has_signal(&a, "declining_trajectory"));
    }

    #[test]
    fn mutual_exclusion_trajectory() {
        let mut t = healthy();
        t.trajectory_class = Some("DYING".into());
        let a = assess_rug_risk(&t);
        assert!(has_signal(&a, "dying_trajectory"));
        assert!(!has_signal(&a, "declining_trajectory"), "DYING and DECLINING are mutually exclusive");
    }

    #[test]
    fn deterministic_output() {
        let t = healthy();
        let a1 = assess_rug_risk(&t);
        let a2 = assess_rug_risk(&t);
        assert_eq!(a1.score, a2.score);
        assert_eq!(a1.signals.len(), a2.signals.len());
        for (s1, s2) in a1.signals.iter().zip(a2.signals.iter()) {
            assert_eq!(s1.name, s2.name);
            assert_eq!(s1.weight, s2.weight);
        }
    }

    #[test]
    fn format_empty_for_clean_token() {
        let a = assess_rug_risk(&healthy());
        assert_eq!(format_rug_risk_section(&a), "");
    }

    #[test]
    fn format_contains_section_header() {
        let mut t = healthy();
        t.trajectory_class = Some("DYING".into());
        let a = assess_rug_risk(&t);
        let section = format_rug_risk_section(&a);
        assert!(section.contains("[RUG RISK]"));
        assert!(section.contains("rug_risk_score:"));
        assert!(section.contains("dying_trajectory"));
    }

    #[test]
    fn format_caps_at_five_signals() {
        // Build a token that fires many signals
        let t = TokenData {
            mint: "MANYsignals111111111111111111111111111111111".into(),
            holder_count: 5,
            holder_data_available: true,
            top1_pct: 80.0,
            age_hours: 2,
            mint_authority_active: true,
            freeze_authority_active: true,
            lp_status: "unsecured".into(),
            supply_burned_pct: None,
            liquidity_usd: Some(500.0),
            volume_24h_usd: Some(5000.0),
            origin: Some("pump.fun".into()),
            trajectory_class: Some("DYING".into()),
            buy_sell_ratio: Some(0.2),
            divergence_class: Some("DISTRIBUTION".into()),
            herfindahl: Some(0.8),
            kscore: Some(KScore { diamond_hands: 0.05, ..KScore::default() }),
            ..TokenData::default()
        };
        let a = assess_rug_risk(&t);
        assert!(a.signals.len() > 5, "should have >5 signals active");
        let section = format_rug_risk_section(&a);
        let signal_lines = section.lines().filter(|l| l.starts_with("  - ")).count();
        assert_eq!(signal_lines, 5, "format should cap at 5 signals");
    }
}
```

- [ ] **Step 2: Register module in `domain/mod.rs`**

Add `pub mod rug_risk;` alphabetically (after `rug_prefilter`).

- [ ] **Step 3: Run tests to verify they pass**

```bash
export RUST_MIN_STACK=67108864
cargo test -p cynic-kernel --lib -- rug_risk
```
Expected: 12 tests pass.

- [ ] **Step 4: Verify clippy**

```bash
cargo clippy -p cynic-kernel --all-targets -- -D warnings
```
Expected: clean. `is_none_or` may need adjustment depending on Rust edition — check if it compiles; if not, use `map_or(true, |p| p == 0.0)` instead.

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/domain/rug_risk.rs cynic-kernel/src/domain/mod.rs
git commit -m "feat(domain): rug risk scoring — 15 signals, 3 categories, pure function

Temporal: very_young, young_low_holders, dying/declining_trajectory
Structural: mint/freeze_authority, no_lp_commitment, no_supply_burn, pump_fun
Behavioral: concentration, distribution, diamond_hands, thin_liquidity
Data quality guard: holder_data_available=false suppresses holder signals
12 tests including degraded RPC, mutual exclusion, format cap"
```

---

## Task 2 — Wire into pipeline

**Files:**
- Modify: `cynic-kernel/src/pipeline/mod.rs`

- [ ] **Step 1: Read pipeline/mod.rs lines 195-310**

Understand the flow: Stage 5 (enrich) → Stage 5b (prefilter) → Stage 6 (wallet). The insertion point is at line 299 (after prefilter early-return block closes, before Stage 6).

- [ ] **Step 2: Make `content` mutable**

At line 198, change:
```rust
let content = token_result.content;
```
to:
```rust
let mut content = token_result.content;
```

- [ ] **Step 3: Insert Stage 5c after the prefilter block**

After line 299 (the closing `}` of the prefilter `if let` block), before `// ── Stage 6: Wallet enrichment ──`:

```rust
    // ── Stage 5c: Rug risk scoring (signal for Dogs — not a gate) ──
    if let Some(ref td) = captured_token_data {
        let risk = crate::domain::rug_risk::assess_rug_risk(td);
        let section = crate::domain::rug_risk::format_rug_risk_section(&risk);
        if !section.is_empty() {
            content.push_str(&section);
            tracing::info!(
                phase = "rug_risk",
                mint = %td.mint,
                score = format!("{:.2}", risk.score),
                signals = risk.signals.len(),
                "rug risk assessed"
            );
        }
    }
```

- [ ] **Step 4: Verify full build + tests**

```bash
export RUST_MIN_STACK=67108864
cargo check --workspace --all-targets
cargo test -p cynic-kernel --lib
```
Expected: all tests pass (existing + 12 new rug_risk tests).

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/pipeline/mod.rs
git commit -m "feat(pipeline): wire rug risk scoring at Stage 5c

Appends [RUG RISK] section to stimulus for LLM Dogs.
No deterministic Dog change (v1 — deferred to v2).
Fail-open: no section appended for clean tokens (score == 0.0)."
```

---

## Task 3 — Smoke test with live kernel

- [ ] **Step 1: Build and deploy**

```bash
cargo build --release
mv ~/bin/cynic-kernel ~/bin/cynic-kernel.bak
cp target/release/cynic-kernel ~/bin/cynic-kernel
systemctl --user restart cynic-kernel
sleep 2
systemctl --user status cynic-kernel
```

- [ ] **Step 2: Judge a known risky token**

```bash
source ~/.cynic-env
curl -s -X POST "http://${CYNIC_REST_ADDR}/judge" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"content":"<KNOWN_PUMP_FUN_MINT>","domain":"token-analysis"}' | \
  python3 -c "import json,sys; v=json.load(sys.stdin); print(f'kind={v[\"kind\"]} q={v[\"q_score\"][\"total\"]:.3f}')"
```

Check kernel logs for `rug risk assessed` with score and signal count:
```bash
journalctl --user -u cynic-kernel --since "1 min ago" | grep rug_risk
```

- [ ] **Step 3: Judge a known healthy token (JUP)**

```bash
curl -s -X POST "http://${CYNIC_REST_ADDR}/judge" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" \
  -H "Content-Type: application/json" \
  -d '{"content":"JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN","domain":"token-analysis"}' | \
  python3 -c "import json,sys; v=json.load(sys.stdin); print(f'kind={v[\"kind\"]} q={v[\"q_score\"][\"total\"]:.3f}')"
```

Check: NO `rug_risk` log line (clean token → no section appended).

- [ ] **Step 4: Clean up**

```bash
rm -f ~/bin/cynic-kernel.bak
```

---

## Build Gate

```bash
# After Task 1:
cargo test -p cynic-kernel --lib -- rug_risk     # 12 tests

# After Task 2:
cargo check --workspace --all-targets
cargo test -p cynic-kernel --lib                   # full suite

# Before PR:
make gate
```
