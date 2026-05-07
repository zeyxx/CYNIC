# Token-Analysis Pipeline Upgrade — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade token-analysis from RugCheck-level heuristics to behavioral multi-signal analysis with K-Score, LP burn detection, and honest holder count — differentiating CYNIC from every existing screener.

**Architecture:** Two phases. Phase 1 fixes data quality (LP status derived from burn addresses, honest holder_count labeling). Phase 2 adds behavioral wallet analysis via Helius Enhanced Transactions, computes K-Score (DiamondHands × OrganicGrowth × Longevity) with configurable weights from `backends.toml`, and integrates into both the stimulus (for LLM Dogs) and the deterministic-dog scorer.

**Tech Stack:** Rust (cynic-kernel), Helius DAS/RPC/Enhanced Transactions APIs, TOML config

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `cynic-kernel/src/domain/enrichment.rs` | Modify | Add `WalletBehavior`, `HolderClass`, `KScore` structs + `holder_count_is_exact` field |
| `cynic-kernel/src/backends/helius.rs` | Modify | LP burn detection, `LargestAccount.address` field, behavioral enrichment via Enhanced Transactions |
| `cynic-kernel/src/domain/stimulus.rs` | Modify | Add K-Score + behavioral section to token stimulus |
| `cynic-kernel/src/dogs/deterministic/token.rs` | Modify | Parse + score K-Score metrics, add `k_score` to `TokenMetrics` |
| `cynic-kernel/src/infra/config.rs` | Modify | Load K-Score weights from `backends.toml` |
| `backends.toml` | Modify | Add `[kscore]` section with configurable weights |

---

### Task 1: Fix LargestAccount to capture addresses (prerequisite)

**Files:**
- Modify: `cynic-kernel/src/backends/helius.rs:501-505` (LargestAccount struct)
- Modify: `cynic-kernel/src/backends/helius.rs:513-521` (HolderConcentration struct)

Currently `LargestAccount` only deserializes `ui_amount`. We need the `address` field for burn detection and later for behavioral analysis (fetching wallet SWAP history).

- [ ] **Step 1: Add `address` field to `LargestAccount` and return addresses in `HolderConcentration`**

In `helius.rs`, update the structs:

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LargestAccount {
    address: String,
    ui_amount: Option<f64>,
}

#[derive(Debug, Clone)]
struct HolderConcentration {
    accounts_seen: u64,
    top1_pct: f64,
    top10_pct: f64,
    herfindahl: f64,
    /// Token account addresses of top holders (needed for LP burn detection + behavioral analysis).
    holder_addresses: Vec<String>,
}
```

Update `get_largest_accounts` to collect addresses:

```rust
// After the for loop that computes hhi/top1/top10:
let holder_addresses: Vec<String> = accounts.iter().map(|a| a.address.clone()).collect();
```

And include in the return:

```rust
Ok(Some(HolderConcentration {
    accounts_seen: accounts.len() as u64,
    top1_pct,
    top10_pct,
    herfindahl: hhi,
    holder_addresses,
}))
```

- [ ] **Step 2: Verify build compiles**

Run: `cargo check --workspace`
Expected: PASS (no consumers of `holder_addresses` yet, struct is private)

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/src/backends/helius.rs
git commit -m "feat(enrichment): capture holder addresses from getTokenLargestAccounts"
```

---

### Task 2: LP Status Burn Detection

**Files:**
- Modify: `cynic-kernel/src/backends/helius.rs` (new method `detect_lp_status`, update `enrich()`)

Detect LP burn by resolving token account owners and checking against known burn addresses. Cost: 1-3 RPC credits (getAccountInfo per top holder).

- [ ] **Step 1: Add burn address constants and LP detection method**

```rust
/// Known Solana burn addresses — tokens sent here are irrecoverable.
const BURN_ADDRESSES: &[&str] = &[
    "1nc1nerator11111111111111111111111111111111",
    "1111111111111111111111111111111111111111111",
    // Raydium burn vault
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
];

/// Known Solana locker programs — LP tokens held by these are locked, not burned.
const LOCKER_PROGRAMS: &[&str] = &[
    // Streamflow
    "8e72pYCDaxu3GqMfeQ5r8wFgoZSYk6oua1Qo9XpsZjX",
    // Uncx/Team Finance
    "2r5VekMNiWPzi1pWwvJczrdPaZnJG59u91unSrTunwJg",
];

/// Resolve token account → owner, check if owner is a burn/locker address.
/// Returns "burned" | "locked" | "unsecured".
async fn detect_lp_status(
    &self,
    holder_addresses: &[String],
    total_supply: Option<f64>,
) -> String {
    // Check top-5 holders for burn/lock patterns
    let check_count = holder_addresses.len().min(5);
    let mut burned_supply_pct = 0.0;
    let mut locked_supply_pct = 0.0;

    for addr in &holder_addresses[..check_count] {
        // getAccountInfo to resolve owner of this token account
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "getAccountInfo",
            "params": [addr, {"encoding": "jsonParsed"}]
        });

        let Ok(resp) = self.client.post(&self.rpc_url).json(&body).send().await else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        let Ok(rpc) = resp.json::<serde_json::Value>().await else {
            continue;
        };

        self.credits.record_call(0, true, 1); // 1 credit per getAccountInfo

        // Extract owner from parsed account data
        let owner = rpc
            .pointer("/result/value/data/parsed/info/owner")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if BURN_ADDRESSES.contains(&owner) {
            return "burned".into();
        }
        if LOCKER_PROGRAMS.contains(&owner) {
            return "locked".into();
        }
    }

    "unsecured".into()
}
```

- [ ] **Step 2: Wire LP detection into `enrich()`**

In `TokenEnricherPort::enrich()`, replace the hardcoded `lp_status`:

```rust
// After get_largest_accounts call, extract holder_addresses
let (holder_count, top1_pct, top10_pct, herfindahl, holder_addresses) =
    if let Ok(Some(conc)) = self.get_largest_accounts(mint_address, real_supply).await {
        (conc.accounts_seen, conc.top1_pct, conc.top10_pct, Some(conc.herfindahl), conc.holder_addresses)
    } else {
        (0, 0.0, 0.0, None, vec![])
    };

// Detect LP status from holder owners
let lp_status = if !holder_addresses.is_empty() {
    self.detect_lp_status(&holder_addresses, real_supply).await
} else {
    "unsecured".into()
};
```

Replace `lp_status: "unsecured".into()` with `lp_status,` in the `TokenData` construction.

- [ ] **Step 3: Verify build**

Run: `cargo check --workspace`

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/backends/helius.rs
git commit -m "feat(enrichment): detect LP burn/lock from holder account owners"
```

---

### Task 3: Honest Holder Count

**Files:**
- Modify: `cynic-kernel/src/domain/enrichment.rs` (add `holder_count_is_exact` field)
- Modify: `cynic-kernel/src/domain/stimulus.rs` (label holder count honestly)
- Modify: `cynic-kernel/src/backends/helius.rs` (set the flag)

- [ ] **Step 1: Add `holder_count_is_exact` to `TokenData`**

In `enrichment.rs`, add after `holder_count`:

```rust
/// True if holder_count is exact (< 20 accounts returned).
/// False if it's a lower bound (20 accounts returned = likely more exist).
pub holder_count_is_exact: bool,
```

Update `Default` impl (it's derived, so add `holder_count_is_exact: false` in test constructors).

- [ ] **Step 2: Set the flag in Helius enricher**

In `helius.rs` `enrich()`, after getting `holder_count`:

```rust
let holder_count_is_exact = holder_count < 20;
```

Add `holder_count_is_exact,` to the `TokenData` construction.

- [ ] **Step 3: Update stimulus to label honestly**

In `stimulus.rs` `build_token_stimulus()`, change the holders line:

```rust
if data.holder_count_is_exact {
    s.push_str(&format!("holders: {} (exact)\n", data.holder_count));
} else {
    s.push_str(&format!("holders: {}+ (top accounts seen, likely more)\n", data.holder_count));
}
```

- [ ] **Step 4: Fix all test constructors that build `TokenData` manually**

Search for `TokenData {` in tests and add `holder_count_is_exact: true,` (or appropriate value).

- [ ] **Step 5: Verify build + tests**

Run: `cargo check --workspace && cargo test --workspace`

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/domain/enrichment.rs cynic-kernel/src/domain/stimulus.rs cynic-kernel/src/backends/helius.rs
git commit -m "fix(enrichment): honest holder count labeling (exact vs lower-bound)"
```

---

### Task 4: K-Score Config in backends.toml

**Files:**
- Modify: `backends.toml` (add `[kscore]` section)
- Modify: `cynic-kernel/src/infra/config.rs` (add `KScoreConfig` struct + loader)

- [ ] **Step 1: Add `[kscore]` section to backends.toml**

```toml
# K-Score behavioral composite weights (configurable, not magic numbers)
# Source: HolDex K-Score formula, adapted for CYNIC pipeline.
# K = DiamondHands^w_dh × OrganicGrowth^w_og × Longevity^w_lon
# Weights MUST sum to 1.0. Tune via measurement, not intuition.
[kscore]
weight_diamond_hands = 0.50     # Retention is strongest signal
weight_organic_growth = 0.35    # Distribution quality
weight_longevity = 0.15         # Age alone is weak
# Wallet classification thresholds (retention_ratio = current_balance / total_bought)
accumulator_threshold = 1.5     # >= 1.5x = buying more
holder_threshold = 1.0          # >= 1.0x = holding all
reducer_threshold = 0.5         # >= 0.5x = sold some
# extractor = below reducer_threshold
# How many top holders to analyze for behavioral signals
top_n_wallets = 10              # 10 wallets × 50 credits = 500 credits per judgment
swap_history_limit = 100        # Max SWAP transactions to fetch per wallet
```

- [ ] **Step 2: Add `KScoreConfig` struct to config.rs**

```rust
/// K-Score behavioral composite weights — loaded from backends.toml [kscore].
#[derive(Debug, Clone)]
pub struct KScoreConfig {
    pub weight_diamond_hands: f64,
    pub weight_organic_growth: f64,
    pub weight_longevity: f64,
    pub accumulator_threshold: f64,
    pub holder_threshold: f64,
    pub reducer_threshold: f64,
    pub top_n_wallets: usize,
    pub swap_history_limit: usize,
}

impl Default for KScoreConfig {
    fn default() -> Self {
        Self {
            weight_diamond_hands: 0.50,
            weight_organic_growth: 0.35,
            weight_longevity: 0.15,
            accumulator_threshold: 1.5,
            holder_threshold: 1.0,
            reducer_threshold: 0.5,
            top_n_wallets: 10,
            swap_history_limit: 100,
        }
    }
}
```

- [ ] **Step 3: Load from TOML in `load_dog_thresholds`**

Add to the `DogThresholds` struct:

```rust
pub kscore: KScoreConfig,
```

In `load_dog_thresholds()`, parse the `[kscore]` table:

```rust
if let Some(ks) = file.other.get("kscore").and_then(|v| v.as_table()) {
    if let Some(v) = ks.get("weight_diamond_hands").and_then(|v| v.as_float()) {
        result.kscore.weight_diamond_hands = v;
    }
    if let Some(v) = ks.get("weight_organic_growth").and_then(|v| v.as_float()) {
        result.kscore.weight_organic_growth = v;
    }
    if let Some(v) = ks.get("weight_longevity").and_then(|v| v.as_float()) {
        result.kscore.weight_longevity = v;
    }
    if let Some(v) = ks.get("accumulator_threshold").and_then(|v| v.as_float()) {
        result.kscore.accumulator_threshold = v;
    }
    if let Some(v) = ks.get("holder_threshold").and_then(|v| v.as_float()) {
        result.kscore.holder_threshold = v;
    }
    if let Some(v) = ks.get("reducer_threshold").and_then(|v| v.as_float()) {
        result.kscore.reducer_threshold = v;
    }
    if let Some(v) = ks.get("top_n_wallets").and_then(|v| v.as_integer()) {
        result.kscore.top_n_wallets = v as usize;
    }
    if let Some(v) = ks.get("swap_history_limit").and_then(|v| v.as_integer()) {
        result.kscore.swap_history_limit = v as usize;
    }
}
```

- [ ] **Step 4: Verify build**

Run: `cargo check --workspace`

- [ ] **Step 5: Commit**

```bash
git add backends.toml cynic-kernel/src/infra/config.rs
git commit -m "feat(config): K-Score configurable weights in backends.toml"
```

---

### Task 5: Behavioral Wallet Analysis (Helius Enhanced Transactions)

**Files:**
- Modify: `cynic-kernel/src/domain/enrichment.rs` (add `WalletBehavior`, `HolderClass`, `KScore` structs)
- Modify: `cynic-kernel/src/backends/helius.rs` (add `get_wallet_swaps`, `analyze_wallet_behavior`, `compute_kscore`)

- [ ] **Step 1: Add behavioral types to enrichment.rs**

```rust
/// Classification of a token holder based on buy/sell behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HolderClass {
    /// Bought more since initial purchase (retention >= accumulator_threshold)
    Accumulator,
    /// Holding all or most (retention >= holder_threshold)
    Holder,
    /// Sold some (retention >= reducer_threshold)
    Reducer,
    /// Sold most or all (retention < reducer_threshold)
    Extractor,
}

impl std::fmt::Display for HolderClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Accumulator => write!(f, "accumulator"),
            Self::Holder => write!(f, "holder"),
            Self::Reducer => write!(f, "reducer"),
            Self::Extractor => write!(f, "extractor"),
        }
    }
}

/// Per-wallet behavioral analysis from SWAP transaction history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBehavior {
    pub class: HolderClass,
    /// current_balance / total_bought (> 1.0 means bought more, < 1.0 means sold)
    pub retention_ratio: f64,
    /// Number of SWAP transactions found for this wallet+token
    pub swap_count: u32,
}

/// K-Score composite — behavioral health metric for a token.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KScore {
    /// Final composite score (0.0 to 1.0)
    pub score: f64,
    /// DiamondHands pillar (conviction × retention)
    pub diamond_hands: f64,
    /// OrganicGrowth pillar (holder distribution × inverse concentration)
    pub organic_growth: f64,
    /// Longevity pillar (age-based survival)
    pub longevity: f64,
    /// Wallets analyzed
    pub wallets_analyzed: u32,
    /// Breakdown: how many of each class
    pub accumulators: u32,
    pub holders: u32,
    pub reducers: u32,
    pub extractors: u32,
}
```

Add to `TokenData`:

```rust
/// K-Score behavioral composite (None if behavioral analysis unavailable).
pub kscore: Option<KScore>,
/// Per-wallet behavioral breakdown (top-N holders).
pub wallet_behaviors: Vec<WalletBehavior>,
```

- [ ] **Step 2: Add `get_wallet_swaps` to HeliusEnricher**

Fetches Enhanced Transactions filtered by SWAP type for a wallet address, then filters for the target mint.

```rust
/// Fetch SWAP transactions for a wallet, filtered to a specific token mint.
/// Returns (total_bought, total_sold, swap_count) in token units.
/// Cost: 50 credits per call (Enhanced Transactions).
async fn get_wallet_swaps(
    &self,
    wallet_owner: &str,
    target_mint: &str,
    limit: usize,
) -> Result<(f64, f64, u32), EnrichmentError> {
    let api_key = self.rpc_url.split("api-key=").nth(1).unwrap_or_default();
    let url = format!(
        "https://api.helius.xyz/v0/addresses/{}/transactions?api-key=$KEY&limit={}&type=SWAP",
        wallet_owner, api_key, limit
    );

    let resp = self.client.get(&url).send().await
        .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

    if !resp.status().is_success() {
        return Ok((0.0, 0.0, 0));
    }

    self.credits.record_call(0, true, 50); // Enhanced Transactions: 50 credits

    let txs: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| EnrichmentError::RequestFailed(e.to_string()))?;

    let mut total_bought = 0.0_f64;
    let mut total_sold = 0.0_f64;
    let mut swap_count = 0_u32;

    for tx in &txs {
        let transfers = tx.get("tokenTransfers").and_then(|v| v.as_array());
        let Some(transfers) = transfers else { continue };

        for transfer in transfers {
            let mint = transfer.get("mint").and_then(|v| v.as_str()).unwrap_or("");
            if mint != target_mint { continue; }

            let amount = transfer.get("tokenAmount")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let to = transfer.get("toUserAccount").and_then(|v| v.as_str()).unwrap_or("");
            let from = transfer.get("fromUserAccount").and_then(|v| v.as_str()).unwrap_or("");

            if to == wallet_owner {
                total_bought += amount;
            } else if from == wallet_owner {
                total_sold += amount;
            }
            swap_count += 1;
        }
    }

    Ok((total_bought, total_sold, swap_count))
}
```

- [ ] **Step 3: Add `resolve_token_account_owner` helper**

We need to resolve token account address → wallet owner for behavioral lookups:

```rust
/// Resolve a token account address to its owner wallet address.
/// Cost: 1 credit (getAccountInfo).
async fn resolve_owner(&self, token_account: &str) -> Option<String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "getAccountInfo",
        "params": [token_account, {"encoding": "jsonParsed"}]
    });
    let resp = self.client.post(&self.rpc_url).json(&body).send().await.ok()?;
    if !resp.status().is_success() { return None; }
    let rpc: serde_json::Value = resp.json().await.ok()?;
    self.credits.record_call(0, true, 1);
    rpc.pointer("/result/value/data/parsed/info/owner")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
```

- [ ] **Step 4: Add `analyze_behaviors` method**

```rust
/// Analyze top-N holders' SWAP behavior for a token.
/// Returns per-wallet classifications and K-Score composite.
/// Cost: ~N×51 credits (1 resolve + 50 enhanced transactions per wallet).
pub async fn analyze_behaviors(
    &self,
    mint: &str,
    holder_addresses: &[String],
    config: &crate::infra::config::KScoreConfig,
    holder_count: u64,
    top10_pct: f64,
    age_hours: u64,
) -> (Vec<crate::domain::enrichment::WalletBehavior>, crate::domain::enrichment::KScore) {
    use crate::domain::enrichment::{WalletBehavior, HolderClass, KScore};

    let n = holder_addresses.len().min(config.top_n_wallets);
    let mut behaviors = Vec::with_capacity(n);

    for addr in &holder_addresses[..n] {
        // Resolve token account → wallet owner
        let Some(owner) = self.resolve_owner(addr).await else { continue };

        // Fetch SWAP history for this wallet
        let Ok((bought, sold, swaps)) = self.get_wallet_swaps(&owner, mint, config.swap_history_limit).await else { continue };

        if bought <= 0.0 && swaps == 0 { continue; }

        let retention = if bought > 0.0 { (bought - sold) / bought } else { 0.0 };
        let class = if retention >= config.accumulator_threshold {
            HolderClass::Accumulator
        } else if retention >= config.holder_threshold {
            HolderClass::Holder
        } else if retention >= config.reducer_threshold {
            HolderClass::Reducer
        } else {
            HolderClass::Extractor
        };

        behaviors.push(WalletBehavior { class, retention_ratio: retention, swap_count: swaps });
    }

    // Compute K-Score from behaviors
    let total = behaviors.len() as f64;
    if total == 0.0 {
        return (behaviors, KScore::default());
    }

    let acc = behaviors.iter().filter(|b| b.class == HolderClass::Accumulator).count() as f64;
    let hld = behaviors.iter().filter(|b| b.class == HolderClass::Holder).count() as f64;
    let red = behaviors.iter().filter(|b| b.class == HolderClass::Reducer).count() as f64;
    let ext = behaviors.iter().filter(|b| b.class == HolderClass::Extractor).count() as f64;

    // DiamondHands = sqrt(conviction * retention_avg)
    let conviction = (acc + hld) / total;
    let retention_avg = (acc / ext.max(1.0) / 2.0).tanh();
    let diamond_hands = (conviction * retention_avg).sqrt();

    // OrganicGrowth = sqrt(holder_norm * inv_concentration)
    let holder_norm = 1.0 - 1.0 / (1.0 + (1.0 + holder_count as f64 / 100.0).ln());
    let inv_concentration = 1.0 - top10_pct / 100.0;
    let organic_growth = (holder_norm * inv_concentration.max(0.0)).sqrt();

    // Longevity = 1 - e^(-age_days/21)
    let age_days = age_hours as f64 / 24.0;
    let longevity = 1.0 - (-age_days / 21.0).exp();

    // K = DH^w1 * OG^w2 * L^w3
    let score = diamond_hands.powf(config.weight_diamond_hands)
        * organic_growth.powf(config.weight_organic_growth)
        * longevity.powf(config.weight_longevity);

    let kscore = KScore {
        score,
        diamond_hands,
        organic_growth,
        longevity,
        wallets_analyzed: total as u32,
        accumulators: acc as u32,
        holders: hld as u32,
        reducers: red as u32,
        extractors: ext as u32,
    };

    (behaviors, kscore)
}
```

- [ ] **Step 5: Verify build**

Run: `cargo check --workspace`

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/domain/enrichment.rs cynic-kernel/src/backends/helius.rs
git commit -m "feat(enrichment): K-Score behavioral wallet analysis via Helius Enhanced Transactions"
```

---

### Task 6: Wire Behavioral Analysis into Enrichment Pipeline

**Files:**
- Modify: `cynic-kernel/src/backends/helius.rs` (`enrich()` method)
- Modify: `cynic-kernel/src/backends/helius.rs` (add `kscore_config` field to `HeliusEnricher`)
- Modify: `cynic-kernel/src/main.rs` (pass KScoreConfig to enricher)

- [ ] **Step 1: Add KScoreConfig to HeliusEnricher**

```rust
#[derive(Debug)]
pub struct HeliusEnricher {
    client: Client,
    rpc_url: String,
    credits: Arc<HeliumsCreditTracker>,
    kscore_config: crate::infra::config::KScoreConfig,
}
```

Update `new()` and `from_env()` to accept/use default config. Add `with_kscore_config(mut self, config: KScoreConfig) -> Self` builder method.

- [ ] **Step 2: Call `analyze_behaviors` in `enrich()`**

After LP detection, before constructing `TokenData`:

```rust
// Behavioral analysis (K-Score)
let (wallet_behaviors, kscore) = self.analyze_behaviors(
    mint_address,
    &holder_addresses,
    &self.kscore_config,
    holder_count,
    top10_pct,
    age_hours,
).await;

let kscore = if kscore.wallets_analyzed > 0 { Some(kscore) } else { None };
```

Add `kscore,` and `wallet_behaviors,` to the `TokenData` construction.

- [ ] **Step 3: Pass KScoreConfig from main.rs**

In `main.rs` where `HeliusEnricher::from_env()` is called, chain:

```rust
let enricher: Option<Arc<dyn domain::enrichment::TokenEnricherPort>> =
    match backends::helius::HeliusEnricher::from_env() {
        Some(h) => {
            let h = h.with_kscore_config(dog_thresholds.kscore.clone());
            klog!("[Boot] Helius enricher configured — token-analysis will use on-chain data + K-Score");
            Some(Arc::new(h))
        }
        // ...
    };
```

- [ ] **Step 4: Verify build**

Run: `cargo check --workspace`

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/backends/helius.rs cynic-kernel/src/main.rs
git commit -m "feat(enrichment): wire K-Score behavioral analysis into enrichment pipeline"
```

---

### Task 7: Add K-Score to Stimulus

**Files:**
- Modify: `cynic-kernel/src/domain/stimulus.rs` (`build_token_stimulus`)

- [ ] **Step 1: Add behavioral section to stimulus**

After the existing `[METRICS]` section, before `[BASELINES]`:

```rust
// ── Behavioral signals (K-Score) ──
if let Some(ref ks) = data.kscore {
    s.push_str(&format!("\n[BEHAVIORAL]\n"));
    s.push_str(&format!("k_score: {:.3}\n", ks.score));
    s.push_str(&format!("diamond_hands: {:.3} (conviction of top holders)\n", ks.diamond_hands));
    s.push_str(&format!("organic_growth: {:.3} (distribution quality)\n", ks.organic_growth));
    s.push_str(&format!("longevity: {:.3} (age-adjusted survival)\n", ks.longevity));
    s.push_str(&format!(
        "wallet_breakdown: {} analyzed — {} accumulators, {} holders, {} reducers, {} extractors\n",
        ks.wallets_analyzed, ks.accumulators, ks.holders, ks.reducers, ks.extractors
    ));
}
```

Update `[BASELINES]` to include K-Score context:

```rust
s.push_str("k_score_baseline: healthy>0.6, moderate 0.3-0.6, rug<0.3. Dominated by diamond_hands (retention).\n");
```

Update `[AXIOM EVIDENCE]` to reference behavioral data:

```rust
s.push_str("FIDELITY: ... Diamond hands signal genuine conviction vs exit liquidity.\n");
s.push_str("PHI: ... K-Score organic_growth measures distribution quality beyond static HHI.\n");
```

- [ ] **Step 2: Verify build + existing tests**

Run: `cargo check --workspace && cargo test --workspace -- stimulus`

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/src/domain/stimulus.rs
git commit -m "feat(stimulus): add K-Score behavioral section to token-analysis prompt"
```

---

### Task 8: Integrate K-Score into Deterministic Dog Scorer

**Files:**
- Modify: `cynic-kernel/src/dogs/deterministic/token.rs` (parse + score K-Score)

- [ ] **Step 1: Add K-Score fields to `TokenMetrics`**

```rust
pub(super) struct TokenMetrics {
    // ... existing fields ...
    k_score: Option<f64>,
    k_diamond_hands: Option<f64>,
    k_accumulators: u32,
    k_extractors: u32,
    k_wallets_analyzed: u32,
}
```

- [ ] **Step 2: Parse K-Score from stimulus**

In `parse()`, after the existing metrics parsing, add a second pass for `[BEHAVIORAL]`:

```rust
// Parse [BEHAVIORAL] section if present
if let Some(beh_start) = content.find("[BEHAVIORAL]") {
    let beh_rest = &content[beh_start..];
    let beh_end = beh_rest[12..]
        .find("\n[")
        .map(|i| beh_start + 12 + i)
        .unwrap_or(content.len());
    let beh_section = &content[beh_start..beh_end];

    for line in beh_section.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("k_score: ") {
            m.k_score = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("diamond_hands: ") {
            m.k_diamond_hands = v.split_whitespace().next().and_then(|s| s.parse().ok());
        } else if let Some(v) = line.strip_prefix("wallet_breakdown: ") {
            // Parse "N analyzed — X accumulators, Y holders, Z reducers, W extractors"
            // Extract accumulators and extractors counts
            for part in v.split(',') {
                let part = part.trim();
                if part.ends_with("accumulators") {
                    m.k_accumulators = part.split_whitespace().next()
                        .and_then(|s| s.parse().ok()).unwrap_or(0);
                } else if part.ends_with("extractors") {
                    m.k_extractors = part.split_whitespace().next()
                        .and_then(|s| s.parse().ok()).unwrap_or(0);
                }
            }
            m.k_wallets_analyzed = v.split_whitespace().next()
                .and_then(|s| s.parse().ok()).unwrap_or(0);
        }
    }
}
```

- [ ] **Step 3: Score K-Score in axiom evaluations**

In `score()`, add K-Score boosts/penalties to relevant axioms:

```rust
// FIDELITY: diamond hands = genuine conviction
if let Some(dh) = m.k_diamond_hands {
    if dh > 0.6 {
        fidelity += ADJUST_MEDIUM; // strong conviction signal
    } else if dh < 0.2 {
        fidelity -= ADJUST_SMALL; // mostly extractors
    }
}

// PHI: K-Score reflects distribution health
if let Some(ks) = m.k_score {
    if ks > 0.6 {
        phi += ADJUST_MEDIUM;
    } else if ks < 0.3 {
        phi -= ADJUST_SMALL;
    }
}

// SOVEREIGNTY: extractor dominance = centralized exit
if m.k_wallets_analyzed > 0 && m.k_extractors > m.k_accumulators {
    sovereignty -= ADJUST_SMALL; // more extractors than accumulators = exodus
}
```

Update reasoning strings to include K-Score data when available.

- [ ] **Step 4: Add tests for K-Score parsing and scoring**

Add to the test module:

```rust
#[test]
fn parse_token_with_kscore() {
    let mut content = make_token_stimulus(&HEALTHY_MINT);
    content.push_str("\n[BEHAVIORAL]\nk_score: 0.650\ndiamond_hands: 0.720 (conviction)\norganic_growth: 0.580\nlongevity: 0.900\nwallet_breakdown: 10 analyzed — 4 accumulators, 3 holders, 2 reducers, 1 extractors\n\n[BASELINES]\n");
    let m = parse(&content).expect("should parse with kscore");
    assert!((m.k_score.unwrap() - 0.65).abs() < 0.01);
    assert!((m.k_diamond_hands.unwrap() - 0.72).abs() < 0.01);
    assert_eq!(m.k_accumulators, 4);
    assert_eq!(m.k_extractors, 1);
}

#[tokio::test]
async fn kscore_boosts_healthy_token() {
    let mut content = make_token_stimulus(&HEALTHY_MINT);
    content.push_str("\n[BEHAVIORAL]\nk_score: 0.700\ndiamond_hands: 0.800 (conviction)\nwallet_breakdown: 10 analyzed — 5 accumulators, 3 holders, 1 reducers, 1 extractors\n\n[BASELINES]\n");
    let m = parse(&content).unwrap();
    let scores_with = score(&m);

    let m_without = parse(&make_token_stimulus(&HEALTHY_MINT)).unwrap();
    let scores_without = score(&m_without);

    assert!(scores_with.fidelity > scores_without.fidelity,
        "K-Score should boost fidelity: with={}, without={}", scores_with.fidelity, scores_without.fidelity);
    assert!(scores_with.phi > scores_without.phi,
        "K-Score should boost phi: with={}, without={}", scores_with.phi, scores_without.phi);
}
```

- [ ] **Step 5: Verify build + all tests**

Run: `cargo check --workspace && cargo test --workspace`

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/dogs/deterministic/token.rs
git commit -m "feat(deterministic-dog): integrate K-Score behavioral signals into token scoring"
```

---

### Task 9: Full Pipeline Verification

**Files:** None (verification only)

- [ ] **Step 1: Run full `make check`**

Run: `make check`
Expected: All gates pass (build + test + clippy + lint-rules + lint-drift)

- [ ] **Step 2: Live test with real token (JUP)**

```bash
source ~/.cynic-env
curl -s -X POST -H "Authorization: Bearer ${CYNIC_API_KEY}" \
  -H "Content-Type: application/json" \
  ${CYNIC_REST_ADDR}/judge \
  -d '{"content":"JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN","domain":"token-analysis"}' \
  | python3 -m json.tool
```

Verify:
- `lp_status` is NOT "unsecured" for JUP (should be "burned" or "locked")
- `kscore` section present in enriched stimulus
- Dogs score higher for JUP than for a known rug token
- Verdict is WAG or HOWL (not BARK/GROWL)

- [ ] **Step 3: Live test with pump.fun rug token**

Find a recent pump.fun token and verify it gets BARK with low K-Score.

- [ ] **Step 4: Commit any test fixes**

```bash
git add -A && git commit -m "fix: pipeline verification adjustments"
```

---

### Task 10: Branch + PR

- [ ] **Step 1: Create branch (if not already on feature branch)**

```bash
git checkout -b feat/token-analysis-kscore-$(date +%Y-%m-%d)-$(head -c4 /dev/urandom | xxd -p)
```

- [ ] **Step 2: Push + PR**

```bash
git push -u origin HEAD
gh pr create --base main --title "feat: K-Score behavioral analysis + LP burn detection for token-analysis" --body "..."
```
