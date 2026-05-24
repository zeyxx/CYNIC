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

### Score Computation

```
active_signals = [s for s in all_signals if s.condition(token_data)]
raw_score = sum(s.weight for s in active_signals)
score = clamp(raw_score, 0.0, 1.0)
```

Simple sum of active signal weights, clamped. No geometric mean, no phi-bound — this is a risk indicator, not a judgment. Dogs apply the judgment layer.

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

### Location in stimulus

New section `[RUG RISK]` injected in `build_token_stimulus()` (`domain/stimulus.rs`), after `[BEHAVIORAL]`, before `[TRAJECTORY]`.

### Format

```
[RUG RISK]
score: 0.73
signals:
  - young_concentrated (0.25): age=3h, holders=12, top1=67%
  - no_commitment (0.18): lp=unsecured, supply_burned=0%
  - distribution_pattern (0.15): buy_sell_ratio=0.3, divergence=DISTRIBUTION
  - pump_fun_origin (0.10): origin=pump.fun (98.6% baseline rug rate)
```

### When absent

If `score == 0.0` and `signals.is_empty()` → no `[RUG RISK]` section in stimulus. No noise for clean tokens.

### Deterministic Dog integration

`dogs/deterministic/token.rs` parses the `[RUG RISK]` section and integrates:
- `score > 0.5` → FIDELITY penalty (-0.10), SOVEREIGNTY penalty (-0.05)
- `score > 0.3` → FIDELITY penalty (-0.05)
- `score <= 0.3` → no adjustment (Dogs judge on other signals)

These adjustments are **additive** to existing deterministic Dog scoring, not replacements.

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
    ├─ Stage 5b: assess_rug_risk(&token_data) → RugRiskAssessment    ← NEW
    │    └─ Inject [RUG RISK] section into stimulus string
    │
    ├─ Stage 9: Dogs evaluate stimulus
    │    └─ Deterministic Dog: parse [RUG RISK], adjust FIDELITY/SOVEREIGNTY
    │    └─ LLM Dogs: read [RUG RISK] signals in context
    │
    └─ Verdict → outcome_collector (T+7d) → calibration loop
```

### Integration points

| File | Change |
|---|---|
| `domain/rug_risk.rs` | NEW — pure function + tests |
| `domain/stimulus.rs` | Inject `[RUG RISK]` section in `build_token_stimulus()` |
| `pipeline/mod.rs` | Call `assess_rug_risk()` between Stage 5a and Stage 9 |
| `dogs/deterministic/token.rs` | Parse `[RUG RISK]` section, adjust FIDELITY/SOVEREIGNTY |
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

## Out of Scope

- Bundle detection (Phase 2, contingent on rho < 0.3)
- Unique buyers in first hour (Phase 2)
- Creator wallet history / serial rug deployer detection (Phase 2)
- LP add/remove event parsing (Phase 2)
- Calibration script (ships after 2 weeks of outcome data)
- Auto-weight adjustment (future consumer of calibration data)
- Alerts / notifications when risk score crosses threshold (future)
