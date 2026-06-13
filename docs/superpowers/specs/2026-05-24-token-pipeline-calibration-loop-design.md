# Token Pipeline Calibration Loop

**Date:** 2026-05-24
**Status:** APPROVED
**Scope:** Daily benchmark cron + domain prompt tuning + deterministic dog weight calibration

---

## Problem

The token judgment pipeline has three calibration gaps:

1. **No daily discrimination tracking.** The `token-snapshot.timer` captures holder data and trajectory at 06:00 UTC, but never calls `/judge`. We don't know if discrimination (LEGIT Q - DEAD Q) is stable, improving, or degrading over time. Without measurement, calibration drift is invisible.

2. **Domain prompt misaligned with empirical data.** `domains/token-analysis.md` was written before PR#260 discovered inverted signals (diamond_hands rho=-0.396, k_score rho=-0.327). LLM dogs may still reward these signals positively while the deterministic dog has already corrected. The prompt and the deterministic scorer disagree on signal direction.

3. **Deterministic dog weight assignments aren't proportional to signal strength.** `dogs/deterministic/token.rs` uses `ADJUST_SMALL=0.05` and `ADJUST_MEDIUM=0.10` with some differentiation, and has already corrected inverted signals (diamond_hands, k_score removed, liquidity removed). But the assignment of SMALL vs MEDIUM doesn't correlate with measured rho values -- a signal with rho=0.632 (longevity) may get the same tier as one with rho=0.2. `ADJUST_LARGE=0.15` is defined in `mod.rs` but unused in `token.rs`. Weight tiers should track measured predictive power.

## Falsification Criteria

- If daily `/judge` on 45 tokens takes >30 min total: sovereign Dog slot starvation makes the cron impractical. Mitigation: reduce set or increase sleep interval.
- If discrimination is already positive and stable across 7+ days: prompt/weight tuning has lower ROI. Proceed cautiously.
- If fresh correlation analysis shows no field with |rho| > 0.3: the enrichment data itself is insufficient. The problem is upstream (data quality), not downstream (scoring).

---

## Phase 1: Daily Benchmark Cron

### Purpose

Automated daily judgment on the 45 tracked tokens. Produces a growing dataset of verdicts + enrichment snapshots. Exposes discrimination metric for drift detection.

### Design

**Script:** `scripts/daily_benchmark.sh` (bash, not Python -- see Architecture Exception below)

**Input:** Union of `cynic-python/heuristics/collection/watchlist.json` (12 tokens) + `cynic-python/heuristics/data/calibration_results_real.json` (33 tokens). Uses `jq` to merge and deduplicate by mint.

**Category mapping:** The two input sources use different labeling schemes. The benchmark maps them to a 4-tier ordinal for discrimination:

| Source | Field | Value | Maps to |
|--------|-------|-------|---------|
| watchlist.json | `source` | `blue_chip` | LEGIT |
| watchlist.json | `source` | `defi`, `infra` | LEGIT |
| watchlist.json | `source` | `mid_meme`, `ai_agent` | SKETCHY |
| watchlist.json | `source` | `low_cap` | SURVIVOR |
| calibration_results_real.json | `conviction_tier` | `strong` | LEGIT |
| calibration_results_real.json | `conviction_tier` | `mixed` | SURVIVOR |
| calibration_results_real.json | `conviction_tier` | `weak` | DEAD |

This mapping is approximate (epistemic status: inferred from source semantics). The daily benchmark will accumulate data that can refine these labels empirically -- tokens whose verdicts consistently contradict their category will be flagged for relabeling.

**Architecture Exception (python.md):** The daily benchmark calls `/judge` via REST from a shell script, not Python. This follows the pattern of the existing `e2e-token-benchmark.sh` and complies with python.md's rule ("Production Python never calls kernel via REST"). Bash scripts calling kernel REST endpoints is the established pattern (R19: "Scripts are thin. Bash = curl + status code. Logic lives in the kernel.").

**Process:**
1. Load token list, apply category mapping, deduplicate by mint
2. For each token:
   - `curl POST /judge` with `{"content": "<mint>", "domain": "token-analysis"}`
   - Parse response with `jq`: Q-score, per-axiom scores, per-dog scores, verdict tier
   - Sleep 2s between calls (K25: don't starve interactive `/judge`)
   - On HTTP 429/503: retry once after 10s backoff (P18: transient errors are retryable)
   - On HTTP 400/404/422: log error, skip token, continue (P18: permanent errors)
3. Compute discrimination: `min(Q where category=LEGIT) - max(Q where category=DEAD)`
4. Write results to `cynic-python/data/benchmark_daily/YYYY-MM-DD.jsonl`
5. If discrimination < 0: post alert observation to kernel with `domain=benchmark`, `tags=["drift-alert", "no-rejudge"]` (K21: explicit tag prevents compound-loop re-judgment by nightshift)

**JSONL row schema:**
```json
{
  "date": "2026-05-24",
  "mint": "JUPyiwr...",
  "symbol": "JUP",
  "category": "LEGIT",
  "q_score": 0.485,
  "verdict": "WAG",
  "axiom_scores": {"fidelity": 0.45, "phi": 0.50},
  "dog_scores": {"deterministic-dog": {"q": 0.42}, "qwen25-7b": {"q": 0.51}},
  "enrichment_snapshot": {"holders": 15234, "top_1_pct": 12.3},
  "discrimination": 0.15,
  "schema_version": 1
}
```

**Schema version** (P17): every row carries `schema_version: 1`. When the schema changes, bump to 2. Consumers filter by version.

**Systemd integration:** Create `token-benchmark.timer` with `OnCalendar=*-*-* 06:20:00` (20 min after snapshot start, allowing snapshot to complete its ~15 min run). The timer fires independently -- no `After=` dependency on `token-snapshot.service`. If the snapshot hasn't completed yet, the benchmark still runs (it uses the kernel's live enrichment, not snapshot files). If the snapshot failed, the benchmark runs unaffected.

```ini
# infra/systemd/token-benchmark.timer
[Unit]
Description=Daily token benchmark timer

[Timer]
OnCalendar=*-*-* 06:20:00
RandomizedDelaySec=120
Persistent=true

[Install]
WantedBy=timers.target
```

```ini
# infra/systemd/token-benchmark.service
[Unit]
Description=Daily token benchmark — judge 45 tracked tokens

[Service]
Type=oneshot
WorkingDirectory=/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC
ExecStart=/usr/bin/bash scripts/daily_benchmark.sh
TimeoutStartSec=1800
Environment=CYNIC_REST_ADDR=%h/.cynic-env
EnvironmentFile=%h/.cynic-env
```

**Consumer (K15):**
- **Acting consumer (primary):** Drift alert observation posted when discrimination < 0 -- this triggers nightshift review and is routed to the human via Slack `#cynic` (domain=benchmark, tags include `no-rejudge` to avoid K21 compound-loop)
- **Data consumer:** Correlation analysis (Phase 3) reads the JSONL files to compute fresh rho values, which then change `token.rs` weights (acting: gate behavior changes)
- **Debug utility (non-K15):** `scripts/daily_benchmark.sh --report` prints 7d trend to stdout. This is NOT a K15 consumer -- it's a human debugging tool.

**Rate limiting:**
- 45 tokens x 2s sleep = ~90s of sleep + inference time
- At ~15s per judgment (2 sovereign dogs + deterministic): ~675s inference + 90s sleep = ~13 min total
- Well under 30 min. Sovereign Dog slots recover between calls.

**Helius credit cost:** Each `/judge` call triggers full enrichment: ~170-200 credits/token (getAsset + getTokenAccounts + batch identity + enhanced transactions). 45 tokens/day = ~7,650-9,000 credits/day = ~230K-270K credits/month. Developer plan has 30M credits/month -- this is <1% of budget. No plan upgrade needed.

### Error Handling

- If `/judge` returns HTTP 429/503: retry once after 10s backoff (P18: transient errors).
- If `/judge` returns HTTP 400/404/422: log error, skip token, continue (P18: permanent errors).
- If kernel unreachable (connection refused): abort with exit 1 (systemd will log the failure).
- If <50% of tokens succeed: don't compute discrimination (sample too small), post warning observation.

---

## Phase 2: Domain Prompt Tuning

### Purpose

Align `domains/token-analysis.md` with empirical correlation data so LLM dogs weight signals in the same direction as measured reality.

### Measurement Protocol

**Before (baseline):**
1. Run `scripts/e2e-token-benchmark.sh` -- 12 tokens, record per-dog Q-scores
2. Record discrimination: min(LEGIT Q) - max(DEAD Q)
3. Record tier match rate: % of tokens whose verdict matches expected category

**Changes to `domains/token-analysis.md`:**

1. **Invert diamond_hands guidance.** Currently the prompt implies high diamond_hands = positive conviction signal. Empirical rho=-0.396 shows the opposite (high diamond_hands correlates with worse outcomes -- possibly bag-holding, not conviction). Change axiom evidence to treat high diamond_hands as a risk signal.

2. **Invert k_score composite guidance.** rho=-0.327. The composite mixes signals with opposing directions, destroying information. Guide LLM dogs to weight component signals (longevity, organic_growth) individually rather than the composite.

3. **Amplify longevity signal.** rho=+0.632 -- strongest positive predictor. The prompt should explicitly state this is the primary survival indicator.

4. **Amplify accumulator dominance as risk.** rho=-0.622 -- strong negative predictor. Many accumulators != healthy distribution; it means concentration by active buyers (often insiders or bots).

5. **Remove or downweight liquidity.** rho=+0.038 -- noise. The prompt currently lists it as an axiom evidence factor. Demote to "available but not predictive."

6. **Add empirical correlation summary.** Brief section listing the top 5 signals by |rho| so LLM dogs have the same calibration data the deterministic dog uses.

**After:**
1. Re-run `scripts/e2e-token-benchmark.sh` -- same 12 tokens
2. Compare discrimination delta
3. Compare tier match rate delta
4. **Decision rule (single criterion):** Compute `delta = after_discrimination - before_discrimination`. If `delta > 0` AND no LEGIT token drops below WAG: keep changes. If `delta <= 0` OR any LEGIT token drops below WAG: rollback. When the two conditions conflict (discrimination improved but a LEGIT token dropped), rollback wins -- safety over optimization.

**Rollback:** If after-benchmark is worse by the decision rule above, revert the prompt change via `git checkout -- domains/token-analysis.md`. The before/after data itself becomes part of the benchmark dataset.

### What NOT to change

- Axiom structure (6 axioms, score range 0.05-0.618)
- Age-based confidence caps (these are structural, not signal-dependent)
- The 98.6% pump.fun prior (empirically validated)
- The baselines section (reference data, not scoring guidance)

---

## Phase 3: Deterministic Dog Weight Calibration

### Purpose

Replace flat `ADJUST_SMALL/MEDIUM` constants in `dogs/deterministic/token.rs` with weights derived from measured correlation with ground truth outcomes.

### Correlation Analysis

**Script:** `scripts/correlation_analysis.py` (Tier 1 EXPERIMENTAL -- analysis tool, not production infra. No systemd service. Death date: if not promoted to Tier 2 within 30 days of creation, delete. Type annotations required (P1), no coverage gate for Tier 1.)

**Input:** All JSONL files from `cynic-python/data/benchmark_daily/`. If insufficient data (<7 days), fall back to `cynic-python/heuristics/data/calibration_results_real.json` + any existing validation datasets.

**Process:**
1. For each enrichment field (holders, top_1_pct, herfindahl, age_hours, k_score, diamond_hands, organic_growth, longevity, supply_burned_pct, lp_secured, volume_24h, liquidity, accumulator_ratio, extractor_ratio, trajectory_class):
   - Compute Spearman rho vs ground truth outcome (category label mapped to ordinal: DEAD=0, SKETCHY=1, SURVIVOR=2, LEGIT=3)
   - Compute p-value
   - Flag if |rho| < 0.1 (noise -- candidate for removal from scoring)
   - Flag if rho sign differs from current code assumption

2. Output: `cynic-python/data/correlation_matrix.json` (human-gated decision artifact -- the acting consumer is a developer who reads the matrix, decides which weights to update in `token.rs`, and commits the change. This is intentionally NOT automated: weight changes affect all future verdicts and require human judgment on whether the sample size is sufficient. The drift-alert from Phase 1 is the trigger that tells the human "run correlation_analysis.py now.")
```json
{
  "computed_at": "2026-05-24",
  "n_tokens": 45,
  "n_days": 7,
  "correlations": {
    "longevity": {"rho": 0.632, "p": 0.001, "direction": "positive", "current_code_direction": "positive", "aligned": true},
    "diamond_hands": {"rho": -0.396, "p": 0.01, "direction": "negative", "current_code_direction": "negative", "aligned": true}
  }
}
```

### Weight Derivation

Replace flat constants with correlation-derived weights:

```
SIGNAL_WEIGHT[field] = |rho| * SCALE_FACTOR
```

Where `SCALE_FACTOR` normalizes so the strongest signal gets `ADJUST_LARGE = 0.15` and signals below |rho| < 0.1 get 0 (removed).

Concrete mapping (using existing rho values as starting point, to be validated by fresh analysis):

| Signal | |rho| | Weight | Direction |
|--------|-------|--------|-----------|
| longevity | 0.632 | 0.15 | positive |
| accumulator_ratio | 0.622 | 0.15 | negative |
| diamond_hands | 0.396 | 0.09 | negative |
| k_score | 0.327 | 0.08 | negative |
| organic_growth | ~0.3* | 0.07 | positive* |
| supply_burned | ~0.2* | 0.05 | positive* |
| herfindahl | ~0.15* | 0.04 | negative* |
| liquidity | 0.038 | 0.00 | REMOVED |

*Starred values are conjectured -- to be measured in Phase 3 analysis.

### Implementation

The existing code already uses differentiated `ADJUST_SMALL` (0.05) / `ADJUST_MEDIUM` (0.10) / `ADJUST_LARGE` (0.15, defined in `mod.rs` but not yet used in `token.rs`). The implementation refines these assignments based on measured rho values rather than introducing a new weight-lookup architecture:

1. For each signal in each axiom's scoring block, reassign the constant based on the correlation matrix:
   - |rho| >= 0.5: use `ADJUST_LARGE` (0.15)
   - |rho| 0.2-0.5: use `ADJUST_MEDIUM` (0.10)
   - |rho| 0.1-0.2: use `ADJUST_SMALL` (0.05)
   - |rho| < 0.1: remove from scoring (noise)
2. Document the rho source as a comment next to each constant usage: `// rho=0.632 (longevity, 2026-05-25)`
3. Each axiom still selects WHICH signals matter to it (FIDELITY cares about authorities, PHI about distribution, etc.) -- the rho-based tier determines how much

### Before/After

1. Run 12-token e2e benchmark with current flat weights
2. Apply new weights
3. Re-run benchmark
4. Success: discrimination improves, no LEGIT token drops tier

---

## Execution Order

```
Phase 1: daily_benchmark.sh
  |-- Write script
  |-- Wire systemd (after token-snapshot)
  |-- Manual test run (validate JSONL output)
  |-- Let accumulate >=3 days

Phase 2: Domain prompt tuning (can start immediately with 12-token set)
  |-- Baseline: 12-token e2e benchmark
  |-- Edit domains/token-analysis.md
  |-- After: 12-token e2e benchmark
  |-- Compare before/after discrimination

Phase 3: Deterministic dog weights (after Phase 1 produces data)
  |-- correlation_analysis.py
  |-- Compute fresh rho values
  |-- Derive weights
  |-- Apply to token.rs
  |-- 12-token before/after benchmark
  |-- Validate with daily cron data (ongoing)
```

**Phases 1 and 2 are independent** -- execute in parallel.
**Phase 3 depends on Phase 1 data** (or existing calibration dataset as fallback).

---

## Dependencies

- Kernel running and reachable at `${CYNIC_REST_ADDR}`
- `token-snapshot.timer` active and firing (Phase 1 runs independently but benefits from fresh snapshot data)
- At least 1 sovereign Dog available for `/judge`
- `cynic-python/heuristics/collection/watchlist.json` exists (12 tokens)
- `cynic-python/heuristics/data/calibration_results_real.json` exists (33 tokens with `conviction_tier` + `expected_verdict` fields)
- `jq` available on PATH (for JSON parsing in bash script)
- Helius Developer plan active with sufficient credits (~230K-270K/month = <1% budget)

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Sovereign Dog slot starvation during 45-token batch | Interactive `/judge` blocked for ~13 min | 2s sleep between calls; run at 06:20 UTC (before user peak hours 19-22h) |
| Correlation analysis on 45 tokens has low statistical power | rho confidence intervals wide | Acknowledge in output; accumulate more days before hardcoding weights |
| Prompt change makes LLM dogs worse on unknown tokens | False confidence in calibrated set | Monitor daily benchmark for tokens NOT in calibration set |
| calibration_results_real.json missing on disk | Phase 1 runs on 12 tokens only (watchlist) | Degrade gracefully; warn in output |
| Category mapping is approximate (source/conviction_tier to LEGIT/DEAD) | Discrimination metric may not reflect true token quality | Accumulate data to validate mapping; flag tokens whose verdicts consistently contradict category for relabeling |
| Helius credit cost (~9K/day) during API degradation | Wasted credits on failed enrichments | Script already skips on non-200; enrichment failures produce partial stimuli (deterministic dog still scores) |

## Files Created/Modified

| File | Action | Phase |
|------|--------|-------|
| `scripts/daily_benchmark.sh` | CREATE | 1 |
| `infra/systemd/token-benchmark.service` | CREATE | 1 |
| `infra/systemd/token-benchmark.timer` | CREATE | 1 |
| `domains/token-analysis.md` | MODIFY | 2 |
| `scripts/correlation_analysis.py` | CREATE (Tier 1 EXPERIMENTAL) | 3 |
| `cynic-kernel/src/dogs/deterministic/token.rs` | MODIFY | 3 |
