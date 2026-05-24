# Rug Risk Scoring — Design Spec

> **Date:** 2026-05-24
> **Scope:** Predictive rug detection on existing TokenData features
> **Depends on:** PR #257 (Solana rework — prefilter, kscore, outcome collector)

---

## Summary

A pure rug risk scoring function that predicts rug probability on live tokens using the 14 features already available in `TokenData`. No new API calls. No new infrastructure. The slow rug detector is the outcome collector + trajectory cron already deployed — it needs data accumulation, not code.

Epistemic status: the 14 features are **observed** (available in `TokenData`). Their predictive power for rug detection is **conjecture** — validated empirically via outcome collector at T+14d. The compound loop (score → outcome → calibrate → better score) is the product.

---

## Phase 1: `domain/rug_risk.rs` — Pure Risk Scoring

### Interface

```rust
pub struct RugRiskAssessment {
    pub score: f64,               // 0.0 (safe) → 1.0 (imminent rug)
    pub signals: Vec<RugSignal>,  // active signals with evidence
}

pub struct RugSignal {
    pub name: &'static str,       // "young_concentrated", "distribution_pattern", etc.
    pub weight: f64,              // contribution to total score
    pub evidence: String,         // human-readable: "age=3h, top1=67%"
}

pub fn assess_rug_risk(token: &TokenData) -> RugRiskAssessment
```

Pure function. No I/O, no async. Takes `&TokenData`, returns assessment. Same pattern as `rug_prefilter.rs` and `kscore.rs`.

### Signal Categories

#### Temporal — is the token too young to be credible?

| Signal | Condition | Weight (conjecture) | Rationale |
|---|---|---|---|
| `very_young` | `age_hours < 6` | 0.25 | Peak pump phase — most rugs happen here |
| `young_low_holders` | `age_hours < 24 AND holder_count < 50` | 0.20 | Young + thin distribution |
| `dying_trajectory` | `trajectory_class == "DYING"` | 0.15 | Daily cron measured active decay |
| `declining_trajectory` | `trajectory_class == "DECLINING"` | 0.10 | Slower decay, still a signal |

#### Structural — does the token have rug properties?

| Signal | Condition | Weight (conjecture) | Rationale |
|---|---|---|---|
| `mint_authority_active` | `mint_authority_active == true` | 0.20 | Can inflate supply at will |
| `freeze_authority_active` | `freeze_authority_active == true` | 0.15 | Can freeze holder wallets |
| `no_lp_commitment` | `lp_status != "burned" AND age_hours > 24` | 0.15 | No permanent liquidity commitment after 24h |
| `no_supply_burn` | `supply_burned_pct == None/0 AND age_hours > 24` | 0.10 | No conviction signal after 24h |
| `pump_fun_origin` | `origin == Some("pump.fun")` | 0.10 | 98.6% rug rate baseline (Solidus Labs) |

#### Behavioral — are holders acting like a rug is coming?

| Signal | Condition | Weight (conjecture) | Rationale |
|---|---|---|---|
| `extreme_concentration` | `top1_pct > 50` | 0.20 | Single entity controls majority |
| `unhealthy_distribution` | `herfindahl > 0.3` | 0.15 | High concentration index |
| `active_distribution` | `buy_sell_ratio < 0.5` | 0.20 | More sellers than buyers |
| `distribution_class` | `divergence_class == "DISTRIBUTION"` | 0.15 | Enrichment already classified this |
| `no_diamond_hands` | `kscore.diamond_hands < 0.2` | 0.10 | Nobody is holding |
| `thin_liquidity` | `volume_24h_usd > 0 AND liquidity_usd < 1000` | 0.15 | Volume without depth = manipulation |

### Data Quality Guard

When `holder_data_available == false` (RPC degraded), holder-dependent signals are suppressed:
- Skip: `extreme_concentration`, `unhealthy_distribution`, `young_low_holders` (holder_count is 0 = default, not real)
- Keep: temporal, authority, trajectory, K-Score, volume/liquidity signals

### Notes on Signal Interactions

- `dying_trajectory` and `declining_trajectory` are **mutually exclusive** (same `trajectory_class` field — only one value at a time)
- `very_young` (age < 6h) and `young_low_holders` (age < 24h + holders < 50) **can co-fire** for very young tokens with thin distribution — intentional double-penalizing
- `no_lp_commitment` and `no_supply_burn` require `age_hours > 24` — silent for young tokens. Structural signals only meaningful after the commitment window
- `no_diamond_hands` is absent (not zero) when `kscore == None` — fail-open, no false positive

### Score Computation

```
active_signals = [s for s in all_signals if s.condition(token_data)]
raw_score = sum(s.weight for s in active_signals)
score = clamp(raw_score, 0.0, 1.0)
```

Simple sum of active signal weights, clamped. No geometric mean, no phi-bound — this is a risk indicator, not a judgment. Dogs apply the judgment layer.

**Resolution loss above threshold:** Maximum raw score with all signals active is ~2.35 (2.10 with mutual exclusions). Clamping to 1.0 means a "very bad" and "catastrophically bad" token both score 1.0. This is acceptable for v1 — the Dogs add the nuanced judgment layer. If calibration shows resolution matters at the top of the scale, switch to `tanh(raw_score)` or normalize by max possible.

### What the score is NOT

- Not a verdict. Dogs see it in the stimulus and factor it into their 6-axiom scoring.
- Not a gate. The prefilter is the gate (dead tokens). The risk score is information for live tokens.
- Not ground truth. All weights are conjectures. Calibrated empirically via outcome correlation.

### Relation to existing modules

| Module | Purpose | When | Output |
|---|---|---|---|
| `rug_prefilter.rs` | Gate: skip Dogs on dead tokens | BEFORE Dogs | Pass/Rug/Inconclusive |
| `rug_risk.rs` | Signal: predict rug on live tokens | BEFORE Dogs (in stimulus) | score + signals |
| `kscore.rs` | Signal: holder behavior quality | During enrichment | K-Score composite |

`rug_prefilter` fires first. If `Pass` or `Inconclusive` → `rug_risk` runs. If `Rug` → neither runs (token is already dead).

---

## Phase 2: Stimulus Injection

### Where the injection happens

`assess_rug_risk()` is called in `pipeline/mod.rs` (Stage 5c, after prefilter). The resulting `[RUG RISK]` section is **appended** to the enriched content string in `pipeline/mod.rs` — NOT inside `build_token_stimulus()`. This keeps `stimulus.rs` free of `rug_risk.rs` dependency and follows the same pattern as social convergence injection (appended in pipeline, not in stimulus builder).

### Location in stimulus string

Appended after the output of `build_token_stimulus()`, before `[BASELINES]` if possible, or at the end. The actual section order in the stimulus is: `[METRICS]` → `[BEHAVIORAL]` → `[BUY/SELL DIVERGENCE]` → `[HOLDER CONTEXT]` → `[HOLDER IDENTITIES]` → `[TRAJECTORY]` → `[BASELINES]`. The `[RUG RISK]` section is appended after `[BASELINES]`.

### Exact format (K20: parser contract)

```
[RUG RISK]
rug_risk_score: 0.73
  - very_young (0.25): age=3h
  - active_distribution (0.20): buy_sell_ratio=0.3
  - no_lp_commitment (0.15): lp=unsecured, age=26h
  - pump_fun_origin (0.10): origin=pump.fun
```

**Emit rules:**
- First line: `rug_risk_score: {score:.2}` — parseable by `v.parse::<f64>()`
- Signal lines: `  - {name} ({weight:.2}): {evidence}` — top 5 signals by weight (K16: cap display)
- Signal `name` field matches EXACTLY the names in the Phase 1 signal table (e.g., `very_young`, `no_lp_commitment`, NOT `young_concentrated` or `no_commitment`)

### When absent

If `score == 0.0` and `signals.is_empty()` → no `[RUG RISK]` section appended. No noise for clean tokens.

### Deterministic Dog integration (deferred to v2)

For v1, the `[RUG RISK]` section is consumed by **LLM Dogs only** (they read it in context naturally — zero code cost). The deterministic Dog does NOT parse `[RUG RISK]` in v1 — it already scores FIDELITY/SOVEREIGNTY from the same underlying fields (`mint_authority_active`, `top1_pct`, etc.). Adding deterministic Dog adjustments risks double-counting.

**Promotion condition for deterministic Dog integration:** if calibration shows LLM Dogs consistently under-weight the risk score compared to outcomes, add deterministic Dog parsing in v2. This avoids K20 parser complexity before we know the signal has value.

---

## Phase 3: Slow Rug — Organic Detection (No New Code)

The slow rug detector is NOT a new module. It is the convergence of three existing systems:

```
daily_snapshot.py (cron 06:00, deployed)
    └─ trajectory: STABLE → DECLINING → DYING → DEAD

outcome_collector_kernel.py (deployed, Tier 1)
    └─ T+7d: price_delta, holder_delta, liquidity_delta

Verdict history (kernel /verdicts)
    └─ Original q_score + verdict kind at judgment time
```

A token judged WAG/HOWL that later shows `trajectory_class == DECLINING` + `outcome_label == DECLINE/RUG` is a slow rug. No new code detects this — the data surfaces it.

### Calibration script (future, not this spec)

At T+14d (after outcome collector has accumulated data):
1. Query all token-analysis verdicts with outcomes
2. Correlate `rug_risk.score` at judgment time vs `outcome_label` at T+7d
3. Compute rho per signal → which signals actually predicted rugs?
4. Adjust weights in `rug_risk.rs` based on empirical rho

**Decision gate:** if `rho(risk_score, outcome_label) < 0.3` after 2 weeks → the 14 features are insufficient. Invest in the 4 missing features (bundle detection, unique buyers, creator history, LP events).

---

## Pipeline Integration

```
POST /judge {content: "<mint>", domain: "token-analysis"}
    │
    ├─ Stage 5: enrich_token() → TokenData
    │
    ├─ Stage 5a: rug_prefilter(&token_data)
    │    └─ Rug → BARK, skip everything
    │
    ├─ Stage 5c: assess_rug_risk(&token_data) → RugRiskAssessment    ← NEW
    │    └─ Append [RUG RISK] section to enriched content string
    │
    ├─ Stage 9: Dogs evaluate stimulus
    │    └─ LLM Dogs: read [RUG RISK] signals in context (v1)
    │    └─ Deterministic Dog: no change (v1 — deferred to v2 pending calibration)
    │
    └─ Verdict → outcome_collector (T+7d) → calibration loop
```

### Integration points

| File | Change |
|---|---|
| `domain/rug_risk.rs` | NEW — pure function + tests |
| `pipeline/mod.rs` | Call `assess_rug_risk()` at Stage 5c, append `[RUG RISK]` to content |
| `domain/mod.rs` | Add `pub mod rug_risk;` |

---

## Falsification Conditions

| Claim | Falsified if |
|---|---|
| 14 features are sufficient for fast rug detection | `rho(risk_score, outcome_label) < 0.3` at T+14d |
| pump.fun origin is a discriminant signal | Removing it from the model doesn't change rho |
| Temporal signals (age) dominate | Removing temporal category doesn't decrease rho |
| Risk score helps Dogs judge better | Verdicts WITH risk score have higher rho with outcomes than without |
| Slow rug emerges from existing data | No WAG/HOWL token transitions to DECLINE/RUG in outcome data at T+30d |

---

## Test Contract

Unit tests for `rug_risk.rs` (shipped in same commit per K20):

| Test | Input | Expected |
|---|---|---|
| `healthy_token_score_zero` | JUP-like token (500K holders, burned LP, old) | score == 0.0, signals empty |
| `young_pump_fun_rug` | age=2h, 5 holders, top1=80%, pump.fun origin, mint active | score >= 0.5 |
| `moderate_risk` | age=30h, 200 holders, LP unsecured, no burn | 0.0 < score < 0.5 |
| `distribution_pattern` | buy_sell_ratio=0.3, divergence=DISTRIBUTION | `active_distribution` signal present |
| `degraded_rpc_skips_holder_signals` | holder_data_available=false, holder_count=0 | `young_low_holders` NOT fired, `extreme_concentration` NOT fired |
| `trajectory_dying` | trajectory_class="DYING" | `dying_trajectory` signal present |
| `trajectory_none_is_silent` | trajectory_class=None | no trajectory signal |
| `mutual_exclusion_trajectory` | trajectory_class="DYING" | only `dying_trajectory`, NOT `declining_trajectory` |
| `deterministic_output` | same input twice | same score + signals |

---

## Out of Scope

- Bundle detection (Phase 2, contingent on rho < 0.3)
- Unique buyers in first hour (Phase 2)
- Creator wallet history / serial rug deployer detection (Phase 2)
- LP add/remove event parsing (Phase 2)
- Calibration script (ships after 2 weeks of outcome data)
- Auto-weight adjustment (future consumer of calibration data)
- Alerts / notifications when risk score crosses threshold (future)
