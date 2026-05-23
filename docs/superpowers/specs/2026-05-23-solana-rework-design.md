# Solana Surface Rework — Design Spec

> **Date:** 2026-05-23
> **Scope:** D (structural extraction) + A (intelligence deepening)
> **Branch:** TBD at implementation time

---

## Summary

Rework the Solana judgment surface in four phases:

1. **Structural extraction** — decompose `helius.rs` (1.2K lines), pull domain logic out of backend
2. **Convergence auto-trigger** — wire X organ convergence observations to auto-judge tokens above threshold
3. **Rug pre-filter** — lightweight gate before Dog dispatch, skip obvious dead/rug tokens
4. **Outcome feedback loop** — collect T+24h/T+7d outcomes, enable empirical calibration

Epistemic status: structure is **deduced** from observed code. All threshold values are **conjectures** — ship conservative, measure, calibrate.

---

## Phase 1: `helius.rs` Decomposition

### Problem

`cynic-kernel/src/backends/helius.rs` is 1.2K lines mixing HTTP transport, Solana domain logic (AMM IDs, burn addresses, holder classification), K-Score computation, and credit tracking. Violates K2 (adapter through port trait), K16 (context is metabolic), and makes testing K-Score impossible without HTTP mocks.

### Observed Separability

K-Score computation has two layers:
- **Pure math** (lines 1122-1193): `fn compute_kscore(behaviors, holder_count, top10_pct, age_hours, config) → KScore`. Zero Helius dependency.
- **Data acquisition** (lines 1059-1120): `resolve_owner()` + `get_wallet_total_bought()` — Helius RPC + REST calls. Genuinely Helius-coupled (Enhanced Transactions API response shape).

### Target Structure

```
cynic-kernel/src/
├── domain/
│   ├── kscore.rs              # NEW — pure K-Score computation
│   │   fn compute_kscore(behaviors: &[WalletBehavior], holder_count: u64,
│   │                     top10_pct: f64, age_hours: u64, config: &KScoreConfig) → KScore
│   │   fn classify_wallet(retention: f64, config: &KScoreConfig) → HolderClass
│   │   // Moved from helius.rs:1099-1193
│   │
│   ├── solana_constants.rs    # NEW — AMM program IDs, burn/locker addresses, known exchanges
│   │   // Moved from helius.rs:654-666, 721-729, 775-784
│   │   // SSOT: deterministic Dog + enricher both import from here (K11)
│   │
│   ├── enrichment.rs          # UNCHANGED — TokenData, TokenEnricherPort, HolderType, etc.
│   └── helius_credit.rs       # UNCHANGED — stays in domain/ (adapter-agnostic)
│
├── backends/
│   ├── helius/
│   │   ├── mod.rs             # HeliusEnricher struct, enrich() orchestration, TokenEnricherPort impl
│   │   ├── rpc.rs             # RPC calls: getAsset, getTokenLargestAccounts, getSignaturesForAddress,
│   │   │                      #   getAccountInfo, getMultipleAccounts, getTokenAccounts
│   │   ├── rest.rs            # REST calls: token-metadata, enhanced transactions (SWAP), batch-identity
│   │   └── holders.rs         # Holder resolution, classification, HolderContext computation
│   │                          #   Calls domain::kscore::compute_kscore() for pure math
│   │
│   └── dexscreener.rs         # UNCHANGED
```

### Key Constraints

- `helius_credit.rs` stays in `domain/` — it's already there and is adapter-agnostic
- `solana_constants.rs` becomes the single source of truth for AMM IDs — deterministic Dog (`dogs/deterministic/token.rs`) currently has no hardcoded AMM IDs but consumes holder classification that depends on them
- Python can import constants via a generated JSON artifact if needed (P14: single source of truth per domain)

### Test Impact

- `domain::kscore` gets unit tests with synthetic `WalletBehavior` data — no HTTP mocks needed
- `domain::solana_constants` gets compile-time validation (known program IDs are valid base58)
- `backends::helius::*` remain integration-test territory (require live Helius API or recorded fixtures)

### What Moves Where

| Current location (helius.rs) | Target | Lines |
|---|---|---|
| AMM program IDs, burn/locker addresses | `domain/solana_constants.rs` | 654-666, 721-729, 775-784 |
| HolderClass assignment from retention thresholds (inline in `analyze_behaviors`) | `domain/kscore.rs` (extracted to `fn classify_wallet`) | 1099-1113 |
| `compute_kscore()` pure math | `domain/kscore.rs` | 1122-1193 |
| `resolve_owner()`, `resolve_owners()` | `backends/helius/holders.rs` | 1195-1265 |
| `get_wallet_total_bought()` | `backends/helius/rest.rs` | 985-1047 |
| `analyze_behaviors()` orchestration | `backends/helius/holders.rs` | 1059-1120 (calls domain kscore) |
| `batch_identity()` | `backends/helius/rest.rs` | 1266+ |
| `get_token_age_hours()` | `backends/helius/rpc.rs` | 513+ |
| `estimate_holder_count()` | `backends/helius/rpc.rs` | 420-462 |
| `enrich()` orchestration | `backends/helius/mod.rs` | top-level method |
| Credit tracking | `domain/helius_credit.rs` | already there |

---

## Phase 2: Convergence Auto-Trigger

### Problem

X organ detects `$TICKER` convergence → resolves mint → stores observation. The existing `convergence_consumer.rs` (`infra/tasks/convergence_consumer.rs`) picks up these signals and judges them, but has two gaps:
1. **No threshold gate** — any single convergence observation triggers a judgment (no minimum count)
2. **Session-scoped dedup only** — `judged_targets: HashSet<String>` clears on restart, no persistent cooldown
3. **No token-analysis routing** — judges as generic D1 social signal, doesn't trigger full Helius enrichment via `domain="token-analysis"`

### Design

Extend the existing `convergence_consumer.rs` (poll-based, 60s interval) — not a new consumer. The poll model is already correctly designed for BURN (batch query of 20 per tick, lower DB pressure than event-per-observation).

Changes to `run_cycle()`:

```
Existing: poll → filter *-convergence → dedup → judge immediately
                                                    │
Extended:                                           │
    ├─ NEW: extract mint from tag "mint:{mint}"
    │
    ├─ NEW: count observations for this mint in window (storage query)
    │    Requires: StoragePort::count_observations_by_target_in_window(domain, target, hours) → u64
    │    OR: client-side count from list_observations_by_tag (less efficient, works today)
    │
    ├─ NEW: threshold gate (count >= CONVERGENCE_THRESHOLD)
    │
    ├─ NEW: persistent cooldown check (query latest verdict for this mint)
    │    Requires: storage.list_verdicts_by_target(mint, limit=1) or similar
    │    Replaces in-memory HashSet for cross-restart persistence
    │
    ├─ CHANGED: when threshold met, judge with domain="token-analysis"
    │    (triggers full Helius enrichment via enrich_token())
    │    Add tag "auto-convergence" to verdict for audit trail
    │
    └─ KEPT: social stimulus building (format_social_section) for non-mint signals
         (D1 signals without mint: tag continue to be judged as before)
```

### Storage Method Needed

`StoragePort` currently has `list_observations_by_tag(domain, tag, limit)` which returns a flat list. Two options for the threshold count:

- **Option A (simple, works today):** Client-side aggregation — call `list_observations_by_tag("D1", "mint:{mint}", 20)`, filter by timestamp > now - window, count. O(20) per mint per tick. Good enough for current volume.
- **Option B (efficient, new method):** Add `count_observations_by_target_in_window(domain: &str, target: &str, window_hours: u64) → Result<u64>` to `StoragePort`. Single DB query. Better at scale.

Start with Option A. Promote to Option B if convergence volume exceeds 100 observations/hour.

### Configuration

| Parameter | Default | Rationale |
|---|---|---|
| `convergence_threshold` | 3 | Minimum distinct observations for the same mint in the time window |
| `convergence_window_hours` | 1 | Time window for counting observations |
| `convergence_cooldown_hours` | 6 | Minimum time between re-judgments of the same mint |

All values are **initial conjectures**. Calibrate after 2 weeks of production data.

### Sovereignty Gates

- **Threshold** — 3 mentions from distinct sources in 1h. One random tweet doesn't trigger enrichment.
- **Cooldown** — 6h between re-judgments (persistent across restarts). Prevents buzz-driven credit drain.
- **Tag** — Auto-triggered judgments carry `"auto-convergence"` tag for audit trail distinction.
- **Config** — All parameters in `KernelConfig`, tunable at boot.

### Observability

The consumer logs every decision with outcome:
- `convergence_triggered{mint, count, window}` — threshold crossed, judgment dispatched
- `convergence_below_threshold{mint, count, threshold}` — not enough signal yet
- `convergence_cooldown{mint, last_judged, cooldown}` — recent judgment, skipped

This log data is the calibration input for threshold tuning.

### K15 Compliance

Convergence observations (producer: x_ingest_daemon) already have a consumer (`convergence_consumer.rs`). This phase adds the threshold gate and token-analysis routing — strengthening the existing K15 chain, not creating a new one.

---

## Phase 3: Rug Pre-Filter

### Problem

Dogs waste inference on obvious rugs. 98.6% of pump.fun tokens are fraudulent (Solidus Labs). Judging a dead rug with 3 LLM Dogs costs ~200 Helius credits + inference slots.

### Design

Synchronous gate in the enrichment pipeline, AFTER Helius enrichment (needs data), BEFORE Dog dispatch.

```
enrich_token() returns TokenData
    │
    └─ rug_prefilter(&token_data) → PreFilterResult
         │
         ├─ Pass → proceed to Dogs (normal path)
         │
         ├─ Rug(reason: String) → skip Dogs, emit deterministic BARK verdict
         │    Q-score = 0.0, contributing_verdicts = ["prefilter"]
         │    reason stored in verdict context for audit trail
         │
         └─ Inconclusive → proceed to Dogs (fail-open)
```

### Location

`domain/rug_prefilter.rs` — pure function, no I/O, no async. Takes `&TokenData`, returns `PreFilterResult` enum.

### Pipeline Integration Point

Called in `pipeline/enrichment.rs::enrich_token()`, after DexScreener market data merge (~line 209), before returning `TokenEnrichmentResult`. The caller in `pipeline/mod.rs` checks the `PreFilterResult`:
- `Pass` / `Inconclusive` → proceed to Dogs (existing path)
- `Rug(reason)` → emit deterministic BARK, skip Dog dispatch

### Heuristics

**Hard-fail signals** (any one = `Rug`):

| Signal | Condition | Rationale |
|---|---|---|
| Dead token | `holder_count == 0 OR liquidity_usd < 100` | No market, nothing to judge |
| Honeypot | `mint_authority_active AND age_hours > 168` | Week-old with active mint = can inflate supply |
| Single holder | `top1_holder_pct > 95 AND holder_count < 10` | No distribution whatsoever |

**Soft signals** (scored, combined threshold TBD):

| Signal | Condition | Weight (conjecture) |
|---|---|---|
| No commitment | `supply_burned_pct == 0 AND lp_status != "burned" AND age_hours > 48` | +1 |
| Dead trajectory | `trajectory_class == Some("DEAD")` | +1 |
| No trading | `volume_24h_usd == 0 AND age_hours > 72` | +1 |

Soft signals: if combined score >= 3 → `Rug`. Otherwise `Inconclusive`.

Note: `trajectory_class` is `Option<String>` — `None` (token not tracked by cron) is treated as `Inconclusive`, not as `DEAD`. Only an explicit `"DEAD"` classification counts.

All thresholds are **conjectures**. Validated retroactively against existing verdict corpus before going live.

### Fail-Open Principle

Pre-filter is BURN optimization, not a safety gate. If it can't decide, Dogs judge normally. It must never suppress a legitimate token.

**Calibration protocol:**
1. Run pre-filter retroactively on all historical verdicts
2. Compare: pre-filter result vs Dog consensus (>=2 Dogs agree on WAG/HOWL)
3. If pre-filter would have filtered a consensus WAG/HOWL → false positive → loosen heuristic
4. Target: **0 false positives**, accept false negatives (good tokens that look rugged)

### K15 Compliance

Pre-filter consumes `TokenData` (produced by enrichment) and changes system behavior (gates Dog dispatch). BARK verdicts it produces are consumed by CCM.

---

## Phase 4: Outcome Feedback Loop

### Problem

The organism judges tokens but never learns if it was right. No ground truth → no empirical calibration → Dog weights are tuned on vibes.

### Design

```
Verdict emitted (existing path, domain="token-analysis")
    │
    └─ verdict has mint → store outcome_task {mint, verdict_id, judged_at, q_score}
         │
         T+24h ──→ outcome_collector (cron)
         │           ├─ DexScreener: price_delta_24h, volume_delta_24h (free)
         │           └─ Store observation (tool="outcome_24h")
         │
         T+7d ───→ outcome_collector (cron)
                     ├─ DexScreener: price_delta_7d, volume_delta_7d (free)
                     ├─ daily_snapshot.py: trajectory_class (already collected, free)
                     ├─ Helius: holder_delta, liquidity_delta (~30 credits, structural only)
                     └─ Store observation (tool="outcome_7d")
                          │
                          └─ Calibration analysis: compare verdict vs outcome
                               HOWL + token died → Dog over-scored
                               BARK + token thrived → Dog under-scored
```

### Where It Lives

`cynic-python/heuristics/collection/outcome_collector.py` — **Tier 2 INFRASTRUCTURE**

- Cron: `0 8 * * *` (runs after daily_snapshot at `0 6 * * *`)
- Queries kernel for verdicts with `domain="token-analysis"` and age 24h or 7d
- DexScreener for price/volume (free, no credits)
- Reuses `daily_snapshot.py` trajectory data — no redundant Helius holder re-query
- Helius re-query only at T+7d for structural delta (holder count, liquidity) — ~30 credits/token

### Outcome Schema

```json
{
  "tool": "outcome_7d",
  "target": "<mint>",
  "domain": "token-analysis",
  "context": {
    "verdict_id": "...",
    "original_qscore": 0.485,
    "original_verdict": "WAG",
    "price_delta_pct": -87.3,
    "volume_delta_pct": -95.1,
    "holder_delta_pct": -62.0,
    "trajectory_class": "DYING",
    "outcome_label": "RUG"
  },
  "tags": ["outcome", "7d"]
}
```

### Outcome Labels

Derived heuristically (not ground truth — epistemic status: heuristic):

| Label | Condition |
|---|---|
| `RUG` | price < -80% AND (holders < -50% OR liquidity < -90%) |
| `DECLINE` | price < -30% OR holders < -20% |
| `STABLE` | price within +/- 30% AND holders within +/- 20% |
| `GROWTH` | price > +30% AND holders > +10% |

### Data Flow — Reuse Over Redundancy

| Signal | Source | Cost |
|---|---|---|
| Price delta (24h, 7d) | DexScreener | Free |
| Volume delta (24h, 7d) | DexScreener | Free |
| Trajectory class | `daily_snapshot.py` (already runs at 06:00) | Free (already collected) |
| Holder count delta | Helius `getAsset` at T+7d | 10 credits |
| Liquidity delta | Helius `getAsset` at T+7d | included above |

Total incremental Helius cost: ~10 credits/token at T+7d only.

### What This Does NOT Do

- **Auto-tune Dog weights.** This ships the data pipeline. Calibration is a future layer that reads outcome observations and adjusts deterministic Dog thresholds.
- **Replace manual calibration.** The outcome data enriches the calibration corpus. A human (or calibration agent) still reviews rho correlations and decides weight changes.

### K15 Exception (Explicit)

Outcome observations (producer: outcome_collector) do NOT have an acting consumer at launch. This is a **conscious K15 exception** under `python-lifecycle.md` Tier 1 rules.

- **Tier:** 1 EXPERIMENTAL
- **Death date:** 2026-06-22 (30 days). If no acting consumer is wired by then, delete.
- **Promotion condition:** A calibration script or agent reads outcome observations, computes rho between Dog scores and outcomes, and proposes Dog weight changes. That script becomes the acting K15 consumer, and outcome_collector promotes to Tier 2.
- **Why ship without consumer:** The data pipeline must exist before calibration can begin. The 30-day death date prevents it from rotting.

---

## Phase Ordering & Dependencies

```
Phase 1 (structural extraction)
    │
    ├─ Phase 2 (convergence auto-trigger) — independent of Phase 1
    │    extends existing convergence_consumer.rs
    │
    ├─ Phase 3 (rug pre-filter) — depends on Phase 1
    │    (rug_prefilter.rs in domain/ uses same TokenData)
    │    (pre-filter integrates into pipeline after enrichment)
    │
    └─ Phase 4 (outcome feedback) — independent of Phase 1-3
         (Python cron, reads verdicts from kernel API)
         but benefits from Phase 3 (pre-filtered BARKs are tagged, excluded from calibration noise)
```

**Recommended execution order:**
1. Phase 1 — structural extraction (unblocks clean integration of 2+3)
2. Phase 3 — rug pre-filter (quick win, pure domain function, easy to test)
3. Phase 2 — convergence auto-trigger (event consumer, needs integration testing)
4. Phase 4 — outcome feedback (Python cron, independent, can start after Phase 1)

Phases 2 and 4 can be parallelized (different languages, different layers).

---

## Falsification Conditions

| Claim | Falsified if |
|---|---|
| K-Score is separable from Helius | `compute_kscore()` can't compile without `reqwest` imports |
| Convergence threshold 3 is reasonable | >50% of auto-triggered judgments are noise (token not worth judging) |
| Rug pre-filter has 0 false positives | Any historical consensus WAG/HOWL (>=2 Dogs agree) would have been filtered |
| Outcome labels are meaningful | Outcome label has rho < 0.3 with verdict Q-score |
| DexScreener is sufficient for price | DexScreener coverage < 80% of judged tokens |
| T+7d is the right outcome window | Tokens that rug do so in <24h (T+7d is too late) OR >30d (T+7d is too early) |

---

## Credit Budget Impact

| Component | Credits/token | Frequency | Notes |
|---|---|---|---|
| Current enrichment | ~200 | per judgment | Unchanged |
| Rug pre-filter savings | -200 | per filtered token | Dogs not dispatched |
| Outcome T+7d | ~10 | per judged token, once | DexScreener free, Helius minimal |
| Convergence auto-trigger | +200 | per auto-triggered token | New spend, gated by threshold |

Net (conjecture): if pre-filter catches >5% of tokens AND auto-triggered tokens are net-new (not already manually judged), the savings offset auto-trigger costs. Both assumptions need measurement.

---

## Out of Scope

- Wallet profiling as first-class domain (future layer)
- NFT/collection judgment (different data surface)
- Watchlist auto-rejudge (layered on after outcome loop proves useful)
- Auto-tuning Dog weights from outcomes (future consumer of Phase 4 data)
- Rug ML model (SolRPDS AdaBoost) — heuristic pre-filter first, ML if heuristics are insufficient
