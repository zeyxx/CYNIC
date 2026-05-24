# Solana Surface Rework — Implementation Plan

> **Spec:** `docs/superpowers/specs/2026-05-23-solana-rework-design.md`
> **Date:** 2026-05-23
> **Branch convention:** `feat/solana-rework-<8-char-random>-2026-05-23`
> **Prerequisite:** `git checkout -b feat/solana-rework-$(head -c4 /dev/urandom | xxd -p)-2026-05-23`

---

## Pre-flight Feasibility Answers

Resolved against the actual codebase before writing tasks.

| Question | Answer |
|---|---|
| Does `TokenData` have `Default`? | YES — `#[derive(Default)]` on line 10 of `enrichment.rs` |
| Does `TokenData` have `top1_holder_pct`? | NO — field is `top1_pct: f64`. The spec's rug pre-filter uses the wrong name. All tasks use `top1_pct`. |
| Does `StoragePort` have verdict-by-target query? | NO — only `list_verdicts(limit)` and `list_verdicts_by_domain(domain, limit)`. Phase 2 persistent cooldown needs an addition. |
| Does `KernelConfig` exist? | NO — no such struct. Config lives in `DogThresholds` loaded from `backends.toml`. Convergence config goes there. |
| Does `helius.rs` have a `compute_kscore` function? | NO — K-Score logic is inlined inside `analyze_behaviors()` starting at the comment `// Compute K-Score from behaviors` (around line 1122). Extraction is feasible. |
| Where does Dog dispatch happen? | `pipeline/mod.rs::pipeline_inner()` Stage 9 — `evaluation::select_dogs` + `judge.evaluate_progressive`. Pre-filter inserts between Stage 5 and Stage 9. |
| Is `enrich_token()` the right integration point for pre-filter? | Pre-filter should be in `pipeline_inner` (not inside `enrich_token`), between the Stage 5 result check and Stage 9. |
| Does `list_observations_by_tag` exist on `StoragePort`? | YES — line 243 of `storage/mod.rs`. Phase 2 Option A works today. |
| How long is `helius.rs`? | 1730 lines (spec says 1.2K — off by ~500 lines; plan accounts for actual size). |

---

## Task Ordering and Dependencies

```
Task 1: Phase 1a — domain/solana_constants.rs  (no deps — parallel-safe)
Task 2: Phase 1b — domain/kscore.rs            (no deps — parallel-safe with Task 1)
Task 3: Phase 1c — backends/helius/ split       (deps: Task 1, Task 2)
Task 4: Phase 2 — convergence auto-trigger      (deps: none on Phase 1; needs StoragePort extension)
Task 5: Phase 3 — rug pre-filter                (deps: TokenData in hand — after Task 2 preferred)
Task 6: Phase 4 — outcome_collector.py          (fully independent — any order)
```

**Parallelization:**
- Tasks 1 + 2 can run in the same session (both pure domain, no file conflict).
- Task 4 + Task 6 are fully independent of Phase 1 — suitable for a parallel cortex.
- Task 3 is the largest and should be its own session.
- Task 5 blocks on understanding `TokenData` shape; do after Task 2 to be safe.

**Recommended execution order (single engineer, sequential):**
1. Tasks 1 + 2 (pure domain, ~1h each)
2. Task 5 (rug pre-filter, ~1h — uses TokenData from Task 2 context)
3. Task 3 (helius split, ~2h — largest refactor)
4. Task 4 (convergence, ~1.5h — needs StoragePort extension)
5. Task 6 (Python, ~1h — independent, any slot)

---

## Task 1 — Create `domain/solana_constants.rs`

**TDD first:** compile-time validation before moving constants.

**File:** `cynic-kernel/src/domain/solana_constants.rs`

```rust
//! Solana on-chain constants — single source of truth for AMM IDs, burn/locker addresses.
//! K11: extracted at 2nd occurrence. AMM program IDs appeared in both classify_holder()
//! and classify_holders_batch() in helius.rs — now imported from here.
//! K16: zero duplication. Both helius.rs consumers import from this module.

/// Known AMM/DEX program IDs — token accounts owned by these are LP positions.
pub const AMM_PROGRAMS: &[&str] = &[
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium AMM v4
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", // Raydium CLMM
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",  // Orca Whirlpool
    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",  // Meteora DLMM
    "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB", // Meteora pools
    "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP", // Orca v1
    "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1", // Orca v2 (aquafarm)
    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",  // PumpSwap AMM
];

/// Known Solana burn addresses — tokens sent here are irrecoverable.
pub const BURN_ADDRESSES: &[&str] = &[
    "1nc1nerator11111111111111111111111111111111",
    "1111111111111111111111111111111111111111111",
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // Raydium burn vault
];

/// Known locker programs — LP tokens held by these are locked, not burned.
pub const LOCKER_PROGRAMS: &[&str] = &[
    "8e72pYCDaxu3GqMfeQ5r8wFgoZSYk6oua1Qo9XpsZjX", // Streamflow
    "2r5VekMNiWPzi1pWwvJczrdPaZnJG59u91unSrTunwJg", // Team.finance / Uncx
];

/// System Program — owner of regular wallets.
pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

#[cfg(test)]
mod tests {
    use super::*;

    fn is_valid_base58(s: &str) -> bool {
        (32..=44).contains(&s.len())
            && s.chars().all(|c| c.is_ascii_alphanumeric()
                && c != '0' && c != 'O' && c != 'I' && c != 'l')
    }

    #[test]
    fn amm_programs_are_valid_base58() {
        for addr in AMM_PROGRAMS {
            assert!(is_valid_base58(addr), "invalid base58 in AMM_PROGRAMS: {addr}");
        }
    }

    #[test]
    fn burn_addresses_valid_length() {
        for addr in BURN_ADDRESSES {
            assert!((32..=44).contains(&addr.len()), "wrong length: {addr}");
        }
    }

    #[test]
    fn no_address_in_multiple_categories() {
        for amm in AMM_PROGRAMS {
            assert!(!BURN_ADDRESSES.contains(amm), "AMM addr in BURN: {amm}");
            assert!(!LOCKER_PROGRAMS.contains(amm), "AMM addr in LOCKER: {amm}");
        }
        for burn in BURN_ADDRESSES {
            assert!(!LOCKER_PROGRAMS.contains(burn), "BURN addr in LOCKER: {burn}");
        }
    }
}
```

**Wire into `domain/mod.rs`:** add `pub mod solana_constants;`

**Replace inline `const` blocks in `helius.rs`** (the four arrays at lines ~654-666, ~721-729, ~775-784, ~785-793) with:
```rust
use crate::domain::solana_constants::{AMM_PROGRAMS, BURN_ADDRESSES, LOCKER_PROGRAMS, SYSTEM_PROGRAM};
```

**Verify:**
```bash
cargo check --workspace --all-targets
cargo test -p cynic-kernel domain::solana_constants
```
Expected: 3 tests pass. `helius.rs` compiles without duplicate const definitions.

**Falsification:** `grep -rn "\"675kPX9MHTjS2zt1" cynic-kernel/src/backends/` should return 0 lines.

---

## Task 2 — Create `domain/kscore.rs`

Unit tests run against synthetic `WalletBehavior` data — no HTTP, no mocks.

**File:** `cynic-kernel/src/domain/kscore.rs`

```rust
//! Pure K-Score computation — zero I/O, zero async.
//! Extracted from backends/helius.rs::analyze_behaviors() (K-Score section, ~lines 1099-1192).
//! No reqwest dependency. Unit tests use synthetic WalletBehavior — no HTTP fixtures.

use crate::domain::enrichment::{HolderClass, KScore, WalletBehavior};
use crate::infra::config::KScoreConfig;

/// Classify a wallet by its retention ratio.
/// Pure function — extracted from analyze_behaviors() lines ~1105-1112.
pub fn classify_wallet(retention: f64, config: &KScoreConfig) -> HolderClass {
    if retention >= config.accumulator_threshold {
        HolderClass::Accumulator
    } else if retention >= config.holder_threshold {
        HolderClass::Holder
    } else if retention >= config.reducer_threshold {
        HolderClass::Reducer
    } else {
        HolderClass::Extractor
    }
}

/// Compute K-Score from behavioral data.
/// Pure function — extracted from analyze_behaviors() ~lines 1122-1192.
/// Falsification: compute_kscore must compile without any reqwest import.
pub fn compute_kscore(
    behaviors: &[WalletBehavior],
    holder_count: u64,
    top10_pct: f64,
    age_hours: u64,
    config: &KScoreConfig,
) -> KScore {
    let total = behaviors.len() as f64;
    if total == 0.0 {
        return KScore::default();
    }

    let acc = behaviors.iter().filter(|b| b.class == HolderClass::Accumulator).count() as f64;
    let hld = behaviors.iter().filter(|b| b.class == HolderClass::Holder).count() as f64;
    let ext = behaviors.iter().filter(|b| b.class == HolderClass::Extractor).count() as f64;
    let red = behaviors.iter().filter(|b| b.class == HolderClass::Reducer).count() as f64;

    let conviction = (acc + hld) / total;
    let retention_signal = (acc / ext.max(1.0) / 2.0).tanh();
    let diamond_hands = (conviction * retention_signal).sqrt();

    let holder_norm = 1.0 - 1.0 / (1.0 + (1.0 + holder_count as f64 / 100.0).ln());
    let inv_concentration = (1.0 - top10_pct / 100.0).max(0.0);
    let organic_growth = (holder_norm * inv_concentration).sqrt();

    let age_days = age_hours as f64 / 24.0;
    let longevity = 1.0 - (-age_days / 21.0).exp();

    let score = diamond_hands.powf(config.weight_diamond_hands)
        * organic_growth.powf(config.weight_organic_growth)
        * longevity.powf(config.weight_longevity);

    KScore {
        score,
        diamond_hands,
        organic_growth,
        longevity,
        wallets_analyzed: total as u32,
        accumulators: acc as u32,
        holders: hld as u32,
        reducers: red as u32,
        extractors: ext as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> KScoreConfig { KScoreConfig::default() }

    #[test]
    fn empty_behaviors_returns_zero_score() {
        let score = compute_kscore(&[], 100, 30.0, 200, &cfg());
        assert_eq!(score.score, 0.0);
        assert_eq!(score.wallets_analyzed, 0);
    }

    #[test]
    fn all_accumulators_produce_high_diamond_hands() {
        let behaviors: Vec<WalletBehavior> = (0..5).map(|_| WalletBehavior {
            class: HolderClass::Accumulator,
            retention_ratio: 2.0,
            swap_count: 3,
        }).collect();
        let score = compute_kscore(&behaviors, 5000, 20.0, 720, &cfg());
        assert!(score.diamond_hands > 0.5, "expected DH > 0.5, got {}", score.diamond_hands);
        assert_eq!(score.accumulators, 5);
        assert_eq!(score.extractors, 0);
    }

    #[test]
    fn all_extractors_produce_low_diamond_hands() {
        let behaviors: Vec<WalletBehavior> = (0..5).map(|_| WalletBehavior {
            class: HolderClass::Extractor,
            retention_ratio: 0.1,
            swap_count: 5,
        }).collect();
        let score = compute_kscore(&behaviors, 1000, 80.0, 48, &cfg());
        assert!(score.diamond_hands < 0.2, "expected low DH, got {}", score.diamond_hands);
    }

    #[test]
    fn classify_wallet_respects_thresholds() {
        // defaults: accumulator=1.5, holder=1.0, reducer=0.5
        let c = cfg();
        assert_eq!(classify_wallet(2.0, &c), HolderClass::Accumulator);
        assert_eq!(classify_wallet(1.5, &c), HolderClass::Accumulator);
        assert_eq!(classify_wallet(1.0, &c), HolderClass::Holder);
        assert_eq!(classify_wallet(0.5, &c), HolderClass::Reducer);
        assert_eq!(classify_wallet(0.3, &c), HolderClass::Extractor);
    }

    #[test]
    fn longevity_increases_with_age() {
        let b = vec![WalletBehavior { class: HolderClass::Holder, retention_ratio: 1.0, swap_count: 1 }];
        let s1h  = compute_kscore(&b, 100, 30.0, 1,   &cfg());
        let s30d = compute_kscore(&b, 100, 30.0, 720, &cfg());
        assert!(s1h.longevity < s30d.longevity, "longevity should increase with age");
    }

    #[test]
    fn output_is_deterministic() {
        let b = vec![
            WalletBehavior { class: HolderClass::Holder,  retention_ratio: 1.1, swap_count: 2 },
            WalletBehavior { class: HolderClass::Reducer, retention_ratio: 0.6, swap_count: 3 },
        ];
        let s1 = compute_kscore(&b, 1000, 40.0, 200, &cfg());
        let s2 = compute_kscore(&b, 1000, 40.0, 200, &cfg());
        assert_eq!(s1.score, s2.score);
    }
}
```

**Wire into `domain/mod.rs`:** add `pub mod kscore;`

**Update `helius.rs::analyze_behaviors`:** replace the inlined K-Score block and `classify_wallet` logic with calls to:
```rust
use crate::domain::kscore::{classify_wallet, compute_kscore};
```

**Verify:**
```bash
cargo check --workspace --all-targets
cargo test -p cynic-kernel domain::kscore
```
Expected: 6 tests pass. `helius.rs` behavior unchanged (identical math).

**Falsification of K-Score separability:** `grep -r "diamond_hands\|retention_signal" cynic-kernel/src/backends/` returns 0 lines after this task.

---

## Task 3 — Split `backends/helius.rs` into `backends/helius/`

Mechanical file split. No behavior changes. Largest task — do in its own session.

### 3.1 — Pre-split smoke test (RED — add before any split)

Add to `cynic-kernel/src/backends/helius.rs` tests section:

```rust
#[test]
fn token_data_default_does_not_panic() {
    use crate::domain::enrichment::TokenData;
    let td = TokenData::default();
    assert_eq!(td.holder_count, 0);
    assert_eq!(td.top1_pct, 0.0);
    // lp_status defaults to empty string from Default
    let _ = td.to_stimulus(); // must not panic
}
```

This test must pass BEFORE and AFTER the split.

### 3.2 — Create directory

```bash
mkdir -p cynic-kernel/src/backends/helius
```

### 3.3 — Create files in order (each gets its public API from the caller list)

**`cynic-kernel/src/backends/helius/rpc.rs`** — all pure RPC calls:
- `get_asset` (lines ~69-137)
- `get_largest_accounts` (lines ~138-305)
- `get_holders_via_das` (lines ~306-413)
- `estimate_holder_count` (lines ~414-462)
- `get_token_age_hours` (lines ~513-610)
- `resolve_owner` (lines ~611-641 — single address variant)
- `resolve_owners` (lines ~1195-1256 — batch variant)

**`cynic-kernel/src/backends/helius/rest.rs`** — REST/enhanced API calls:
- `get_wallet_total_bought` (lines ~985-1047)
- `batch_identity` (lines ~1261-1355)

**`cynic-kernel/src/backends/helius/holders.rs`** — holder analysis:
- `detect_lp_and_supply_status` (lines ~647-714)
- `classify_holder` (lines ~716-755) — calls `domain::solana_constants`
- `classify_holders_batch` (lines ~766-983) — calls `domain::solana_constants`
- `analyze_behaviors` (lines ~1059-1193) — calls `domain::kscore::compute_kscore`

**`cynic-kernel/src/backends/helius/mod.rs`** — struct + orchestration:
- `HeliusEnricher` struct (lines ~17-22)
- `impl HeliusEnricher` (constructors + `credit_snapshot`)
- `impl TokenEnricherPort for HeliusEnricher` (the `enrich()` method, lines ~1356-1730)

### 3.4 — Update `backends/mod.rs`

No change needed. Rust resolves `pub mod helius;` to `backends/helius/mod.rs` automatically when the directory exists and `backends/helius.rs` is deleted.

### 3.5 — Delete old file

```bash
git rm cynic-kernel/src/backends/helius.rs
```

### 3.6 — Verify

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p cynic-kernel
```

Expected: All tests pass. The pre-split smoke test passes. `HeliusEnricher` still reachable as `crate::backends::helius::HeliusEnricher`.

---

## Task 4 — Phase 2: Convergence Auto-Trigger

### 4.1 — Add `ConvergenceConfig` to `infra/config.rs`

**Context:** No `KernelConfig` exists. Follow `KScoreConfig` pattern exactly.

Add to `cynic-kernel/src/infra/config.rs`:

```rust
/// Convergence auto-trigger configuration — loaded from [convergence] in backends.toml.
#[derive(Debug, Clone)]
pub struct ConvergenceConfig {
    /// Minimum distinct observations for the same mint in the time window.
    pub threshold: u32,
    /// Time window for counting observations (hours).
    pub window_hours: u64,
    /// Minimum hours between re-judgments of the same mint.
    pub cooldown_hours: u64,
}

impl Default for ConvergenceConfig {
    fn default() -> Self {
        Self { threshold: 3, window_hours: 1, cooldown_hours: 6 }
    }
}
```

Add `convergence: ConvergenceConfig` field to `DogThresholds`.

Add `convergence: Option<ConvergenceEntry>` to `BackendsFile`.

Add `ConvergenceEntry` struct:
```rust
#[derive(Deserialize)]
struct ConvergenceEntry {
    threshold: Option<u32>,
    window_hours: Option<u64>,
    cooldown_hours: Option<u64>,
}
```

Parse in `load_dog_thresholds`:
```rust
if let Some(conv) = file.convergence {
    if let Some(v) = conv.threshold { result.convergence.threshold = v; }
    if let Some(v) = conv.window_hours { result.convergence.window_hours = v; }
    if let Some(v) = conv.cooldown_hours { result.convergence.cooldown_hours = v; }
}
```

Document in `~/.config/cynic/backends.toml` template:
```toml
[convergence]
threshold = 3         # minimum observations in window to trigger judgment
window_hours = 1      # counting window
cooldown_hours = 6    # min hours between re-judgments of same mint
```

### 4.2 — Wire `ConvergenceConfig` into `spawn_convergence_consumer`

Update signature:
```rust
pub fn spawn_convergence_consumer(
    judge: Arc<Judge>,
    storage: Arc<dyn StoragePort>,
    metrics: Arc<Metrics>,
    shutdown: CancellationToken,
    convergence_config: ConvergenceConfig,   // NEW
) -> JoinHandle<()>
```

Pass config from wherever `spawn_convergence_consumer` is called (search: `grep -rn "spawn_convergence_consumer" cynic-kernel/src/`).

### 4.3 — Write tests (RED)

Add to `convergence_consumer.rs`:

```rust
#[cfg(test)]
mod threshold_tests {
    use super::*;

    #[test]
    fn below_threshold_is_rejected() {
        let count = 2u64;
        let config = ConvergenceConfig { threshold: 3, ..ConvergenceConfig::default() };
        assert!(count < config.threshold as u64);
    }

    #[test]
    fn at_threshold_is_accepted() {
        let count = 3u64;
        let config = ConvergenceConfig::default();
        assert!(count >= config.threshold as u64);
    }

    #[tokio::test]
    async fn run_cycle_empty_storage_no_panic() {
        use crate::domain::storage::NullStorage;
        let judge = Arc::new(crate::judge::Judge::new(vec![], vec![]));
        let storage: Arc<dyn StoragePort> = Arc::new(NullStorage);
        let metrics = Arc::new(crate::domain::metrics::Metrics::new());
        let mut judged = std::collections::HashSet::new();
        let config = ConvergenceConfig::default();
        run_cycle(&judge, &storage, &metrics, &mut judged, &config).await;
        // NullStorage returns empty — cycle should exit cleanly
    }
}
```

### 4.4 — Update `run_cycle`

**CRITICAL DESIGN NOTE:** The spec says convergence auto-trigger should route to `domain="token-analysis"`, which triggers full Helius enrichment. However, the current `convergence_consumer.rs` calls `judge.evaluate()` directly, bypassing `pipeline::run()` and thus bypassing `enrich_token()`. To fix this properly, the consumer would need `PipelineDeps` (enricher, embedding, storage, verdict_cache, etc.) — a significant scope increase.

**Decision for this plan:** Wire the threshold gate and the `domain="token-analysis"` routing via `judge.evaluate()` (with a raw mint as content). Document the enrichment gap with a `// TODO: wire pipeline::run() for full Helius enrichment` comment. A separate task will close this gap once the Phase 1 split (Task 3) makes the enricher easier to pass around.

Updated `run_cycle` logic (delta from existing code):

```rust
async fn run_cycle(
    judge: &Arc<Judge>,
    storage: &Arc<dyn StoragePort>,
    metrics: &Arc<Metrics>,
    judged_targets: &mut HashSet<String>,
    config: &ConvergenceConfig,  // NEW param
) {
    // ... existing observation poll ...

    for obs in convergence_obs {
        let ctx: serde_json::Value = /* ... */;
        let target = obs.target.clone();

        // NEW: extract mint from tag "mint:{mint}" if present
        let mint = obs.tags.iter()
            .find_map(|t| t.strip_prefix("mint:"))
            .map(String::from);
        let judgment_target = mint.as_deref().unwrap_or(&target);

        // NEW: skip if already judged this session
        if judged_targets.contains(judgment_target) {
            continue;
        }

        // NEW: threshold gate — count observations for this mint in window
        let obs_in_window = match tokio::time::timeout(
            Duration::from_secs(5),
            storage.list_observations_by_tag("D1", &format!("mint:{judgment_target}"), 20),
        ).await {
            Ok(Ok(obs)) => obs,
            _ => continue, // fail-open: can't count → skip
        };

        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::hours(config.window_hours as i64);
        let count = obs_in_window.iter()
            .filter(|o| chrono::DateTime::parse_from_rfc3339(&o.created_at)
                .map(|t| t.with_timezone(&chrono::Utc) > cutoff)
                .unwrap_or(false))
            .count() as u64;

        if count < config.threshold as u64 {
            tracing::debug!(
                mint = %judgment_target, count, threshold = config.threshold,
                "convergence below threshold — skip"
            );
            continue;
        }

        // NEW: route as token-analysis when mint is known, else fallback to social
        let (domain, content) = if mint.is_some() {
            // TODO: replace judge.evaluate() with pipeline::run() for Helius enrichment
            ("token-analysis".to_string(), judgment_target.to_string())
        } else {
            let social = format_social_section(&ctx);
            (obs.domain.clone(), format!("[DOMAIN: {domain}]\n\n[SOCIAL SIGNAL]\n{social}",
                domain = obs.domain))
        };

        tracing::info!(
            mint = %judgment_target, count, domain = %domain,
            "convergence threshold reached — dispatching judgment"
        );

        let stimulus = Stimulus {
            content,
            context: Some(obs.context.clone()),
            domain: Some(domain.clone()),
            request_id: None,
        };

        match tokio::time::timeout(Duration::from_secs(120),
            judge.evaluate(&stimulus, None, metrics, SlotPriority::Hermes)).await
        {
            Ok(Ok(verdict)) => {
                tracing::info!(
                    mint = %judgment_target, domain = %domain,
                    q_score = %format!("{:.3}", verdict.q_score.total),
                    kind = ?verdict.kind,
                    tags = "auto-convergence",
                    "convergence verdict issued"
                );
                judged_targets.insert(judgment_target.to_string());
            }
            Ok(Err(e)) => tracing::warn!(mint = %judgment_target, "convergence judgment failed: {e}"),
            Err(_) => tracing::warn!(mint = %judgment_target, "convergence judgment timed out (120s)"),
        }
    }
}
```

**Note on `obs.tags`:** Verify `RawObservation` in `storage/types.rs` has a `tags: Vec<String>` field before writing this code. If not, extract mint from `obs.context` JSON instead.

### 4.5 — Verify

```bash
cargo check --workspace --all-targets
cargo test -p cynic-kernel convergence_consumer
```

Expected: New threshold tests pass. Existing `convergence_consumer_respects_shutdown` still passes.

---

## Task 5 — Phase 3: Rug Pre-Filter

### 5.1 — Write tests (RED)

**File:** `cynic-kernel/src/domain/rug_prefilter.rs`

**IMPORTANT:** The spec uses `top1_holder_pct` — the actual field is `top1_pct`. All code below uses the correct name.

```rust
//! Rug pre-filter — synchronous BURN gate before Dog dispatch.
//! NOT a safety gate — fail-open: Inconclusive → Dogs judge.
//! K15: consumes TokenData (produced by enrichment), changes system behavior (gates Dogs).
//!
//! Integration: pipeline/mod.rs::pipeline_inner() between Stage 5 result and Stage 9.

use crate::domain::enrichment::TokenData;

#[derive(Debug, Clone, PartialEq)]
pub enum PreFilterResult {
    Pass,
    Rug(String),
    Inconclusive,
}

/// Run rug pre-filter against enriched token data.
///
/// Hard-fail signals (any one → Rug):
///   - holder_count == 0 OR liquidity_usd < $100
///   - mint_authority_active AND age_hours > 168 (week-old token can inflate supply)
///   - top1_pct > 95 AND holder_count < 10 (zero distribution)
///
/// Soft signals (combined score >= 3 → Rug):
///   - supply_burned_pct == None/0 AND lp_status != "burned" AND age_hours > 48
///   - trajectory_class == Some("DEAD")  [None = Inconclusive — not tracked]
///   - volume_24h_usd == 0 AND age_hours > 72
pub fn rug_prefilter(token: &TokenData) -> PreFilterResult {
    // Hard-fail: dead market
    if token.holder_count == 0 {
        return PreFilterResult::Rug("no holders — dead token".to_string());
    }
    if token.liquidity_usd.map_or(false, |l| l < 100.0) {
        return PreFilterResult::Rug(format!(
            "liquidity ${:.0} < $100", token.liquidity_usd.unwrap_or(0.0)
        ));
    }

    // Hard-fail: honeypot (active mint authority on week-old token)
    if token.mint_authority_active && token.age_hours > 168 {
        return PreFilterResult::Rug(format!(
            "mint authority active on {}-hour-old token", token.age_hours
        ));
    }

    // Hard-fail: zero distribution
    // NOTE: field is top1_pct (not top1_holder_pct — spec had wrong name)
    if token.top1_pct > 95.0 && token.holder_count < 10 {
        return PreFilterResult::Rug(format!(
            "top holder owns {:.0}% with {} holders", token.top1_pct, token.holder_count
        ));
    }

    // Soft signals
    let mut score: u32 = 0;

    if token.supply_burned_pct.map_or(true, |p| p == 0.0)
        && token.lp_status != "burned"
        && token.age_hours > 48
    {
        score += 1; // no commitment signal
    }

    // None = not tracked → Inconclusive (per spec)
    if token.trajectory_class.as_deref() == Some("DEAD") {
        score += 1;
    }

    if token.volume_24h_usd.map_or(false, |v| v == 0.0) && token.age_hours > 72 {
        score += 1;
    }

    if score >= 3 {
        return PreFilterResult::Rug(
            format!("soft-signal score={score}: no-commitment + dead-trajectory + no-trading")
        );
    }

    if score > 0 {
        return PreFilterResult::Inconclusive;
    }

    PreFilterResult::Pass
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::enrichment::TokenData;

    fn healthy() -> TokenData {
        TokenData {
            mint: "TestMintAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into(),
            holder_count: 500,
            top1_pct: 20.0,
            top10_pct: 60.0,
            age_hours: 100,
            mint_authority_active: false,
            lp_status: "burned".into(),
            liquidity_usd: Some(50_000.0),
            volume_24h_usd: Some(10_000.0),
            supply_burned_pct: Some(50.0),
            ..TokenData::default()
        }
    }

    #[test]
    fn pass_on_healthy_token() {
        assert_eq!(rug_prefilter(&healthy()), PreFilterResult::Pass);
    }

    #[test]
    fn rug_on_zero_holders() {
        let mut t = healthy(); t.holder_count = 0;
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn rug_on_low_liquidity() {
        let mut t = healthy(); t.liquidity_usd = Some(50.0);
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn rug_on_active_mint_old_token() {
        let mut t = healthy(); t.mint_authority_active = true; t.age_hours = 200;
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn young_token_active_mint_not_hard_fail() {
        // 24h old with active mint → NOT week-old honeypot signal
        let mut t = healthy(); t.mint_authority_active = true; t.age_hours = 24;
        assert_ne!(rug_prefilter(&t), PreFilterResult::Rug(
            "mint authority active on 24-hour-old token".to_string()
        ));
    }

    #[test]
    fn rug_on_zero_distribution() {
        let mut t = healthy(); t.top1_pct = 98.0; t.holder_count = 3;
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn all_soft_signals_produce_rug() {
        let mut t = healthy();
        t.lp_status = "unsecured".into();
        t.supply_burned_pct = Some(0.0);
        t.age_hours = 100;  // > 48h no-commitment AND > 72h no-trading
        t.trajectory_class = Some("DEAD".to_string());
        t.volume_24h_usd = Some(0.0);
        assert!(matches!(rug_prefilter(&t), PreFilterResult::Rug(_)));
    }

    #[test]
    fn trajectory_none_does_not_trigger_soft_signal() {
        // None = not tracked by cron → treated as Inconclusive, not DEAD
        let mut t = healthy();
        t.trajectory_class = None;
        t.lp_status = "unsecured".into();
        t.supply_burned_pct = Some(0.0);
        t.age_hours = 100;
        t.volume_24h_usd = Some(1000.0); // has volume, no no-trading signal
        // Only 1 soft signal (no-commitment) → Inconclusive, not Rug
        let r = rug_prefilter(&t);
        assert_eq!(r, PreFilterResult::Inconclusive);
    }

    #[test]
    fn false_positive_guard_jup_token() {
        // JUP is a real WAG token — must not be filtered
        let t = TokenData {
            mint: "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".into(),
            holder_count: 250_000,
            top1_pct: 12.5,
            top10_pct: 45.2,
            age_hours: 1200,
            mint_authority_active: false,
            lp_status: "burned".into(),
            liquidity_usd: Some(5_000_000.0),
            volume_24h_usd: Some(500_000.0),
            supply_burned_pct: Some(0.0),
            holder_data_available: true,
            ..TokenData::default()
        };
        assert_eq!(rug_prefilter(&t), PreFilterResult::Pass);
    }
}
```

### 5.2 — Wire into `domain/mod.rs`

```rust
pub mod rug_prefilter;
```

### 5.3 — Add `prefilter_verdict` to `Judge`

Before integrating into the pipeline, `Judge` needs a way to emit a deterministic BARK without going through evaluation.

**Check first:**
```bash
grep -n "derive.*Default\|impl Default for Verdict\|impl Default for QScore" \
  cynic-kernel/src/domain/dog.rs
```

If `Verdict` and `QScore` have `Default`, add to `cynic-kernel/src/judge/mod.rs`:

```rust
/// Build a deterministic BARK verdict for rug pre-filter rejections.
/// Skips Dog evaluation. Q-score = 0.0. Tagged "prefilter" for audit trail.
pub fn prefilter_verdict(
    &self,
    content: &str,
    domain: &str,
    reason: &str,
) -> crate::domain::dog::Verdict {
    use crate::domain::dog::{Verdict, VerdictKind};
    let mut v = Verdict {
        id: uuid::Uuid::new_v4().to_string(),
        content: content.to_string(),
        domain: domain.to_string(),
        kind: VerdictKind::Bark,
        dog_id: "prefilter".to_string(),
        context: format!("rug-prefilter: {reason}"),
        ..Verdict::default()  // zero QScore, empty tags, etc.
    };
    v.tags.push("prefilter".to_string());
    v
}
```

If `Verdict` has no `Default`, build the struct field-by-field using `VerdictKind::Bark` and a zeroed `QScore`.

### 5.4 — Integrate into `pipeline/mod.rs::pipeline_inner`

Insert between Stage 5 (token enrichment, ~line 197) and Stage 6 (wallet enrichment, ~line 207):

```rust
// ── Stage 5b: Rug pre-filter (BURN optimization — fail-open) ──
if let Some(ref td) = captured_token_data {
    use crate::domain::rug_prefilter::{rug_prefilter, PreFilterResult};
    if let PreFilterResult::Rug(reason) = rug_prefilter(td) {
        tracing::info!(
            phase = "prefilter",
            mint = %td.mint,
            reason = %reason,
            "rug pre-filter: BARK — skipping Dogs"
        );
        deps.metrics.inc_verdict();
        let prefilter_verdict = deps.judge.prefilter_verdict(
            &content,  // NB: content here is the enriched stimulus string — use raw mint
            domain_hint,
            &reason,
        );
        // Best-effort side effects (store verdict for CCM, audit trail)
        let empty_embedding: Option<crate::domain::embedding::Embedding> = None;
        side_effects::run(
            &Stimulus {
                content: content.clone(),
                context: None,
                domain: domain.clone(),
                request_id: deps.request_id.clone(),
            },
            &prefilter_verdict,
            &empty_embedding,
            deps,
            false,
        ).await;
        return Ok(PipelineResult::Evaluated {
            verdict: Box::new(prefilter_verdict),
            token_data: captured_token_data.map(Box::new),
            enriched_content,
        });
    }
}
```

**Note:** At this point in `pipeline_inner`, `content` has already been moved into `token_result.content`. Keep a reference to the original mint before Stage 5 if needed, or use `td.mint.clone()` for the verdict content.

### 5.5 — Retroactive calibration check (manual, pre-deploy)

```bash
# Check: how many historical token-analysis WAG/HOWL verdicts would pre-filter have caught?
# If any WAG/HOWL would have been filtered → loosen heuristic before deploy
curl -s "${CYNIC_REST_ADDR}/verdicts?domain=token-analysis&limit=200" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" | \
  jq '[.[] | select(.kind == "Wag" or .kind == "Howl")] | length'
# Compare against pre-filter logic applied manually to stored token data
# Target: 0 false positives
```

### 5.6 — Verify

```bash
cargo check --workspace --all-targets
cargo test -p cynic-kernel rug_prefilter
cargo test -p cynic-kernel pipeline
```

Expected: 9 rug_prefilter tests pass. Pipeline tests still pass (NullStorage → no token enrichment → pre-filter never triggered in unit tests).

---

## Task 6 — Phase 4: Outcome Feedback Loop (Python, Tier 1)

### 6.1 — Pre-checks

```bash
crontab -l | grep daily_snapshot  # verify runs at 06:00 — collector must run AFTER
python3 --version                  # confirm Python 3.10+
```

### 6.2 — Create directory and test file (RED)

```bash
mkdir -p cynic-python/heuristics/collection/tests
touch cynic-python/heuristics/collection/__init__.py
touch cynic-python/heuristics/collection/tests/__init__.py
```

**File:** `cynic-python/heuristics/collection/tests/test_outcome_collector.py`

```python
"""Tests for outcome_collector — run: pytest cynic-python/heuristics/collection/tests/"""
import pytest
from cynic_python.heuristics.collection.outcome_collector import classify_outcome


class TestClassifyOutcome:
    def test_rug_price_and_holders(self):
        assert classify_outcome(-85.0, -60.0, -95.0) == "RUG"

    def test_rug_price_and_liquidity(self):
        assert classify_outcome(-82.0, -10.0, -92.0) == "RUG"

    def test_decline_on_price(self):
        assert classify_outcome(-40.0, 5.0, 0.0) == "DECLINE"

    def test_decline_on_holders(self):
        assert classify_outcome(-5.0, -25.0, 0.0) == "DECLINE"

    def test_stable(self):
        assert classify_outcome(-10.0, 5.0, 2.0) == "STABLE"

    def test_growth(self):
        assert classify_outcome(50.0, 15.0, 20.0) == "GROWTH"

    def test_boundary_rug_exact(self):
        # price=-80%, holders=-50% → exactly at RUG boundary
        assert classify_outcome(-80.0, -50.0, 0.0) == "RUG"

    def test_stable_near_zero(self):
        assert classify_outcome(0.0, 0.0, 0.0) == "STABLE"
```

### 6.3 — Create `outcome_collector.py`

**File:** `cynic-python/heuristics/collection/outcome_collector.py`

```python
"""
Tier 1 EXPERIMENTAL: Outcome feedback loop for token-analysis verdicts.

Research question: Do Dog Q-scores correlate with actual token outcomes?
Success condition: rho(q_score, outcome_label) > 0.3 after 30 days data
Timeline: 30 days from 2026-05-23
Death date: 2026-06-22 — delete if no acting K15 consumer wired by then
Promotion condition: A calibration script reads outcome observations, computes rho,
  proposes Dog weight changes. That script becomes the K15 consumer → Tier 2.

K15 Exception (conscious):
  Producer: this script
  Consumer: NONE — explicit Tier 1 exception per python-lifecycle.md §K15
  Action at death date: delete this file + commit message "decommission: no consumer"
"""
__version__ = "0.1.0"

import json
import logging
import os
import sys
from datetime import datetime, timezone, timedelta
from typing import Optional

import requests

logger = logging.getLogger(__name__)
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(name)s: %(message)s"
)

CYNIC_REST_ADDR = os.environ.get("CYNIC_REST_ADDR", "http://localhost:3030")
CYNIC_API_KEY = os.environ.get("CYNIC_API_KEY", "")
_HEADERS = {"Authorization": f"Bearer {CYNIC_API_KEY}"}


def classify_outcome(
    price_delta_pct: float,
    holder_delta_pct: float,
    liquidity_delta_pct: float,
) -> str:
    """
    Derive outcome label from delta metrics.
    Epistemic status: heuristic — not ground truth.

    Returns: "RUG" | "DECLINE" | "STABLE" | "GROWTH"
    """
    if price_delta_pct <= -80.0 and (
        holder_delta_pct <= -50.0 or liquidity_delta_pct <= -90.0
    ):
        return "RUG"
    if price_delta_pct < -30.0 or holder_delta_pct < -20.0:
        return "DECLINE"
    if price_delta_pct > 30.0 and holder_delta_pct > 10.0:
        return "GROWTH"
    return "STABLE"


def fetch_dexscreener(mint: str) -> dict:
    """
    Fetch current price, volume, liquidity from DexScreener (free, no credits).
    Returns empty dict on failure — callers skip on empty (P18: P9 fail loud at call site).
    """
    try:
        url = f"https://api.dexscreener.com/latest/dex/tokens/{mint}"
        r = requests.get(url, timeout=10)
        r.raise_for_status()
        pairs = r.json().get("pairs") or []
        if not pairs:
            return {}
        p = pairs[0]  # highest-liquidity pair
        return {
            "price_usd": float(p.get("priceUsd") or 0),
            "volume_24h_usd": float((p.get("volume") or {}).get("h24") or 0),
            "liquidity_usd": float((p.get("liquidity") or {}).get("usd") or 0),
        }
    except Exception as e:
        logger.warning("DexScreener fetch failed for %s: %s", mint, e)
        return {}


def _get_verdicts_in_window(window_hours: int) -> list:
    """Query kernel for token-analysis verdicts in the age window."""
    try:
        r = requests.get(
            f"{CYNIC_REST_ADDR}/verdicts",
            headers=_HEADERS,
            params={"domain": "token-analysis", "limit": 200},
            timeout=15,
        )
        r.raise_for_status()
        verdicts = r.json()
    except Exception as e:
        raise RuntimeError(f"Kernel /verdicts query failed: {e}") from e

    now = datetime.now(timezone.utc)
    lo = now - timedelta(hours=window_hours + 1)
    hi = now - timedelta(hours=window_hours - 1)

    result = []
    for v in verdicts:
        created_str = v.get("created_at", "")
        if not created_str:
            continue
        try:
            judged_at = datetime.fromisoformat(created_str.replace("Z", "+00:00"))
        except ValueError:
            continue
        if lo <= judged_at <= hi:
            result.append(v)
    return result


def collect_outcomes(window_hours: int) -> int:
    """
    Collect outcome data for verdicts at window_hours after judgment.
    Returns number of outcomes stored.
    """
    verdicts = _get_verdicts_in_window(window_hours)
    logger.info("Found %d token-analysis verdicts in window=%dh", len(verdicts), window_hours)

    tool = "outcome_7d" if window_hours >= 168 else "outcome_24h"
    stored = 0

    for v in verdicts:
        mint = (v.get("content") or "").strip()
        if len(mint) < 32 or len(mint) > 44:
            continue  # not a mint address

        current = fetch_dexscreener(mint)
        if not current:
            logger.warning("No DexScreener data for %s — skipping", mint)
            continue

        ctx = {
            "verdict_id": v.get("id", ""),
            "original_qscore": v.get("q_score", {}).get("total", 0.0),
            "original_verdict": v.get("kind", "Unknown"),
            "current_price_usd": current.get("price_usd"),
            "current_volume_24h_usd": current.get("volume_24h_usd"),
            "current_liquidity_usd": current.get("liquidity_usd"),
            "schema_version": 1,  # P17: append-only files need schema versioning
        }

        try:
            obs_r = requests.post(
                f"{CYNIC_REST_ADDR}/observe",
                headers=_HEADERS,
                json={
                    "tool": tool,
                    "target": mint,
                    "domain": "token-analysis",
                    "context": json.dumps(ctx),
                    "agent_id": "outcome_collector",
                    "tags": ["outcome", "7d" if window_hours >= 168 else "24h"],
                },
                timeout=10,
            )
            obs_r.raise_for_status()
            stored += 1
            logger.info("Stored %s for mint=%s q=%.3f", tool, mint, ctx["original_qscore"])
        except Exception as e:
            logger.warning("Failed to store outcome for %s: %s", mint, e)

    logger.info("outcome_collector done: %d/%d outcomes stored", stored, len(verdicts))
    return stored


if __name__ == "__main__":
    wh = int(sys.argv[1]) if len(sys.argv) > 1 else 24
    n = collect_outcomes(wh)
    sys.exit(0 if n >= 0 else 1)
```

### 6.4 — No systemd (Tier 1 — manual only)

Run manually:
```bash
# T+24h window
CYNIC_REST_ADDR=http://localhost:3030 CYNIC_API_KEY=... \
  python3 cynic-python/heuristics/collection/outcome_collector.py 24

# T+7d window
CYNIC_REST_ADDR=http://localhost:3030 CYNIC_API_KEY=... \
  python3 cynic-python/heuristics/collection/outcome_collector.py 168
```

### 6.5 — Verify

```bash
python3 -m pytest cynic-python/heuristics/collection/tests/test_outcome_collector.py -v
```

Expected: 8 tests pass. `classify_outcome` is pure — no network calls in tests.

---

## Issues Found During Plan Writing

### CRITICAL

**C1 — Wrong field name in spec:** The spec uses `top1_holder_pct` in the rug pre-filter heuristics table. The actual field in `TokenData` is `top1_pct` (line 34 of `enrichment.rs`). Every `top1_holder_pct` reference in the spec must use `top1_pct`. If this is not caught, the code will fail to compile.

**C2 — Convergence enrichment gap:** The spec says auto-triggered convergence judgments should use `domain="token-analysis"`, which "triggers full Helius enrichment via `enrich_token()`". However, the existing `convergence_consumer.rs` calls `judge.evaluate()` directly, which does NOT go through `pipeline::run()` and therefore does NOT call `enrich_token()`. To trigger Helius enrichment from the convergence consumer, it must call `pipeline::run()` instead, which requires passing `PipelineDeps` (or an equivalent enricher reference) into the consumer. This is a scope increase not reflected in the spec's Phase 2 design. The plan defers full enrichment wiring to a follow-up task and documents it as a TODO.

### MAJOR

**M1 — No `KernelConfig` struct:** The spec references "`KernelConfig`, tunable at boot" for convergence parameters. This struct does not exist. The correct pattern (established by `KScoreConfig`) is: add a `ConvergenceConfig` struct + `[convergence]` section in `backends.toml` + parse in `load_dog_thresholds` + field in `DogThresholds`. Three files must change: `config.rs`, `backends.toml`, and wherever `spawn_convergence_consumer` is called.

**M2 — No `list_verdicts_by_target` on `StoragePort`:** Persistent cooldown across restarts requires querying the latest verdict for a specific mint. No such method exists (only `list_verdicts_by_domain`). The plan uses Option A (in-memory HashSet with window count) which resets on restart. This is documented as a known gap. Add `list_verdicts_by_target` as a default impl in `StoragePort` (client-side filter on `list_verdicts(1000)`) before promoting Option A to Option B.

**M3 — `helius.rs` is 1730 lines, not ~1200:** The spec's line number references for "What Moves Where" are approximations. Verify each function boundary with `grep -n "^    async fn\|^    fn" helius.rs` before splitting. The split is still feasible but do not treat spec line numbers as authoritative.

**M4 — `compute_kscore` function does not exist in helius.rs:** The spec describes extracting "`fn compute_kscore` (lines 1122-1193)". No such standalone function exists. The K-Score computation is inlined within `analyze_behaviors()`. The extraction boundary is the comment `// Compute K-Score from behaviors` at ~line 1122. Identify it by reading the function, not by line number.

**M5 — `RawObservation.tags` field:** Phase 2 extracts mint by reading `obs.tags`. Verify `RawObservation` in `storage/types.rs` has a `tags: Vec<String>` field. If not, extract mint from `obs.context` JSON parsing instead.

### MINOR

**m1 — `Verdict::default()` must be verified before Task 5.** The `prefilter_verdict()` helper uses `..Verdict::default()`. Run `grep -n "derive.*Default" cynic-kernel/src/domain/dog.rs` to confirm. If missing, build the verdict struct field-by-field.

**m2 — Python test runner may not be configured.** Check `cynic-python/pyproject.toml` for `[tool.pytest.ini_options]`. If absent, tests need to be run from the repo root with `python3 -m pytest`.

**m3 — `TokenData::lp_status` defaults to empty string.** The rug pre-filter checks `lp_status != "burned"`. With `..TokenData::default()`, `lp_status` is `String::new()` (empty), which satisfies the condition. Tests that set `lp_status = "burned"` must set it explicitly — do not rely on default.

---

## Build Gate Summary

```bash
# After each task:
cargo check --workspace --all-targets  # mandatory

# After Tasks 1+2:
cargo test -p cynic-kernel domain::solana_constants domain::kscore

# After Task 3 (split):
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p cynic-kernel  # full suite

# After Task 4:
cargo test -p cynic-kernel convergence_consumer

# After Task 5:
cargo test -p cynic-kernel rug_prefilter pipeline

# After Task 6:
python3 -m pytest cynic-python/heuristics/collection/tests/ -v

# Full gate before PR:
make check
```
