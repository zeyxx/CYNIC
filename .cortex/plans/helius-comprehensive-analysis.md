# Helius Ground Truth Analysis — Comprehensive Landscape

**Dates:** 2026-05-07 through 2026-05-16. Observations about Helius APIs, HolDex precedent, SolRPDS dataset, and current kernel enrichment pipeline.

---

## 1. Helius API Landscape (Live Probed)

### Reliable Endpoints (tested, working)

| Endpoint | Cost | Rate Limit (Dev plan) | Use Case | Status |
|----------|------|----------------------|----------|--------|
| `getAsset` (DAS) | 10 cr | 10 req/s | Token metadata, supply, authorities, price | ✓ LIVE |
| `getTokenLargestAccounts` (RPC) | 1 cr | 50 req/s | Top holders, concentration | ✓ LIVE (fails on overload) |
| `getTokenAccounts` (DAS) | 10 cr | 10 req/s | All holders (random sample) | ✓ LIVE (unsorted, fallback) |
| `getAccountInfo` (RPC) | 1 cr | 50 req/s | Token account → owner resolution | ✓ LIVE |
| `getSignaturesForAddress` (RPC) | 1 cr | 50 req/s | Tx history (newest first) | ✓ LIVE |
| Wallet API identity | 100 cr | 10 req/s | Resolve exchange/protocol wallets | ✓ LIVE |
| Wallet API funded-by | 100 cr | 10 req/s | Sybil detection: funding source | ✓ LIVE |
| Priority Fee API | 1 cr | 50 req/s | Transaction priority fee estimation | ✓ LIVE |

### Paid-only (Developer plan +$49/mo)

| Endpoint | Cost | Notes |
|----------|------|-------|
| `getTransactionsForAddress` | 10 cr | Sorted transaction history. Enables exact age in 1 call (sortOrder: "asc"). |
| `getTransfersByAddress` | 10 cr | Parsed token transfers only (10× cheaper than Enhanced Transactions). |
| Enhanced Transactions API | 100 cr | Parsed txns for multiple sigs. **Slow**: 10s+ hang on inactive wallets. |
| Enhanced WebSockets | 2 cr/0.1MB | Real-time account monitoring. |
| LaserStream gRPC | 2 cr/0.1MB | Lowest-latency streaming. |

### Unreliable / Problematic

| Endpoint | Issue | Workaround |
|----------|-------|-----------|
| `getTokenLargestAccounts` (high-cap tokens) | "account index service overloaded" on JUP/BONK/WIF | Fallback to `getTokenAccounts` (DAS) but returns random sample, not top holders. |
| `getTokenAccounts` | Returns unsorted random sample, not top-20 | Cannot use for concentration metrics directly; need to fetch all + sort locally. |
| Enhanced Transactions API | Hangs 10s+ on wallets with sparse SWAP history | 4s timeout needed; 100 credits wasted per timeout. |

### Developer Plan Budget (10M credits/month)

**Basic enrichment (no behavioral):**
- getAsset: 10 cr
- getTokenLargestAccounts: 1 cr
- Wallet identity (top holder): 100 cr
- **Total per token:** ~111 cr → ~90K tokens/month

**K-Score behavioral (as implemented in HolDex):**
- getTokenAccounts (all holders): 10 cr
- getEnhancedTransactions (5 wallets × 20 txs): 5 × 100 = 500 cr
- **Total per token:** ~610 cr → ~16K tokens/month

**Optimized behavioral (using `getTransfersByAddress`):**
- getTokenAccounts: 10 cr
- getTransfersByAddress (5 wallets × 1 call): 5 × 10 = 50 cr
- **Total per token:** ~60 cr → ~166K tokens/month (10× better)

---

## 2. HolDex Precedent (Observed Implementation)

HolDex (JavaScript/Node, /home/user/Bureau/CYNIC/external/HolDex) solves K-Score calculation and is **production-ready**:

### K-Score Formula (3-Pillar, HolDex v8)
```
K = (DiamondHands^0.5) × (OrganicGrowth^0.35) × (Longevity^0.15)

where:
  DiamondHands = conviction (% accumulators + holders in top 20) × accExtRatio
  OrganicGrowth = real_holders ($1+ balance) / total_holders
  Longevity = token_age_days / 365
```

### Data Collection (HolDex process)
1. **getTokenAccounts** → all holders (paginated)
2. **Pool filtering** → removes DEX protocols (Raydium, Orca, PumpFun, etc.) from analysis
3. **Top-20 analysis** → for each holder, fetch retention ratio via **Enhanced Transactions** (100 cr/wallet)
4. **Wallet classification** → retention_ratio >= 1.5 → accumulator; >= 1.0 → holder; etc.
5. **Conviction score** → (accumulators + holders) / top_20_analyzed
6. **Real holders count** → (balance in USD) >= $1 filter on all holders

### Cost Breakdown (HolDex)
- getTokenAccounts: 10 cr
- Enhanced Transactions (5 wallets × 20 txs paginated): 500 cr
- **Total: ~510 cr/token** (matches Helius memory: ~527 cr observed)

### Why HolDex is Optimal
- ✓ Eliminates manipulable metrics (volume, liquidity, price)
- ✓ On-chain behavior only (SWAP history + wallet balances)
- ✓ Pool filtering prevents exchange custody from biasing conviction
- ✓ Retention ratio = (current_balance / total_bought) is hard to fake
- ✓ Tested in production for 6+ months

---

## 3. SolRPDS Dataset (Liquidity Analysis)

**SolRPDS** = Solana Rug Pull Detection System, from 3.69B blockchain transactions.

### Data Structure
```json
{
  "LIQUIDITY_POOL_ADDRESS": "...",
  "MINT": "...",
  "TOTAL_ADDED_LIQUIDITY": 338.59,
  "TOTAL_REMOVED_LIQUIDITY": 233.04,
  "NUM_LIQUIDITY_ADDS": 14,
  "NUM_LIQUIDITY_REMOVES": 2,
  "ADD_TO_REMOVE_RATIO": 1.45,
  "FIRST_POOL_ACTIVITY_TIMESTAMP": "2023-12-30 22:24:29",
  "LAST_POOL_ACTIVITY_TIMESTAMP": "2023-12-30 23:03:40",
  "LAST_SWAP_TIMESTAMP": "2023-12-31 00:00:00",
  "INACTIVITY_STATUS": "Active"
}
```

### What's Useful for CYNIC
- **ADD_TO_REMOVE_RATIO**: rug pull indicator (high add, sudden remove = rug)
- **INACTIVITY_STATUS**: pool dead vs active
- **FIRST/LAST_POOL_ACTIVITY**: token age from liquidity perspective
- **NOT in enrichment scope**: SolRPDS is LP-level, not token-level behavioral

**Integration point**: Could augment token profile with LP age/activity signal, but not part of current K-Score.

---

## 4. Current CYNIC Enrichment Pipeline (Kernel + Python)

### What's Implemented

**cynic-kernel/src/backends/helius.rs (1456 lines):**
- `get_asset()` → metadata, supply, authorities, price (10 cr)
- `get_largest_accounts()` → top holders, concentration (1 cr, fallback to DAS on overload)
- `get_holders_via_das()` → fallback when RPC overloaded
- `analyze_behaviors()` → K-Score + wallet behavior classification
- `batch_identity()` → resolve holder addresses to exchanges/protocols

**TokenData struct (80+ fields):**
- Basic: mint, name, symbol, supply, decimals, price
- Distribution: holder_count, top1_pct, top10_pct, herfindahl
- Age: age_hours, age_is_exact
- Authorities: mint_authority_active, freeze_authority_active
- Behavior: kscore, wallet_behaviors, buy_sell_ratio, divergence_class
- Quality flags: holder_data_available, age_is_exact, holder_count_is_exact

**cynic-python/heuristics/:**
- `helius_token_profiler.py`: TokenProfile + to_stimulus() formatting
- `wallet_behavior_helius.py`: HeliusWalletCollector + wallet profile scoring

### What's **NOT** Persisted
- **Critical**: All enriched TokenData is discarded after verdict
- No table for API calls themselves (cost tracking)
- No log of which calls succeeded/failed/timed out
- No correlation between K-Score signal strength and conviction accuracy

### Current Pipeline Flow
```
/judge (domain=token-analysis, content=mint)
  → pipeline/enrichment.rs:enrich_token()
    → HeliusEnricher::enrich()
      → get_asset (10 cr) → name, supply, authorities, price
      → get_largest_accounts (1 cr) → concentration
      → analyze_behaviors() → K-Score (500 cr) if enabled
    → TokenData computed
  → TokenData used in stimulus → Dogs evaluate
  → VerdictLodge stores verdict
  → TokenData **DISCARDED** ❌
```

**Problem**: No acting K15 consumer. Data is produced, stimulus built, verdict rendered, but enrichment data never stored.

---

## 5. Data Quality Issues Discovered

### Issue #1: `getTokenLargestAccounts` Overload on High-Cap Tokens
**Symptom**: 8/23 calibration tokens marked `holder_data_available: false`.
**Root cause**: Free plan 2 req/s DAS rate limit + high-cap tokens (JUP, BONK, WIF) trigger "account index service overloaded".
**Workaround in kernel**: Fallback to `getTokenAccounts` (DAS), but returns unsorted random sample.
**Fix**: Developer plan 10 req/s unblocks the high-cap case (observed 2026-05-08).

### Issue #2: Enhanced Transactions Hangs on Sparse Wallets
**Symptom**: 10s+ wait on wallets with no SWAP history.
**Cost**: 100 cr wasted per timeout.
**Fix**: `getTransfersByAddress` (10 cr, Developer plan) is 10× cheaper and doesn't hang.

### Issue #3: Age Estimation via Pagination Heuristic
**Current (free plan)**: Use `getSignaturesForAddress` newest-first pagination, estimate 720h floor from newest sig.
**Actual cost for 10K token age check**: 1 cr each = 10K cr/month.
**Better (Developer plan)**: `getTransactionsForAddress` with `sortOrder: "asc"` gets exact age in 1 call (same 10 cr).

---

## 6. Ground Truth Requirements

User stated: **"Il est impératif d'avoir la ground truth sur tout ça avant de créé le registre Solana"**
(CRITICAL to have ground truth on everything before building Solana registry.)

### What We Need to Answer
1. **Which Helius endpoints are reliable?** → getAsset yes, getTokenLargestAccounts unreliable on high-cap
2. **Which signals correlate with conviction?** → K-Score, concentration, buy/sell divergence vs CultScreener labels
3. **What's the true API cost profile?** → Currently estimated 527 cr/token (K-Score); can drop to 150 cr with getTransfersByAddress
4. **How does divergence_class map to verdicts?** → EARLY_ACCUM → HOWL? DISTRIBUTION → BARK?
5. **What's the quality delta between free & Developer plans?** → Free: 8/23 tokens have zero holder data. Dev: 100% coverage.

---

## 7. Proposed Ground Truth Schema

**Principle**: Minimal initial persistence layer that captures **what Helius actually returns** and **how it correlates with conviction**.

### Tables (5 total)

**1. helius_snapshots** (central table, denormalized for query efficiency)
```sql
CREATE TABLE helius_snapshots (
  id UUID PRIMARY KEY,
  request_id TEXT,          -- kernel request_id for tracing
  mint TEXT NOT NULL,
  call_timestamp DATETIME,
  
  -- Raw Helius API responses (denormalized)
  api_endpoint TEXT,        -- "getAsset" | "getTokenLargestAccounts" | ...
  http_status INT,          -- 200 | 429 | 503
  helius_cost_credits INT,
  helius_latency_ms INT,
  
  -- Parsed response data
  name TEXT,
  symbol TEXT,
  supply BIGINT,
  decimals INT,
  price_usd FLOAT,
  holder_count INT,
  top1_pct FLOAT,
  top10_pct FLOAT,
  herfindahl FLOAT,
  mint_authority_active BOOL,
  freeze_authority_active BOOL,
  age_hours BIGINT,
  
  -- Behavioral signals (if computed)
  kscore FLOAT,
  buy_sell_ratio FLOAT,
  divergence_class TEXT,
  
  -- Data quality
  error_occurred BOOL,
  error_message TEXT,
  holder_data_available BOOL,
  
  UNIQUE(mint, call_timestamp),
  INDEX(mint),
  INDEX(call_timestamp),
  INDEX(api_endpoint)
);
```

**2. conviction_labels** (ground truth from calibration_results_real.json)
```sql
CREATE TABLE conviction_labels (
  mint TEXT PRIMARY KEY,
  symbol TEXT,
  cultscreener_conviction FLOAT,  -- 0.0 to 1.0
  conviction_tier TEXT,            -- "strong" | "mixed" | "weak"
  expected_verdict TEXT,           -- "Howl" | "Wag" | "Growl" | "Bark"
  notes TEXT,
  created_at DATETIME
);
```

**3. api_audit** (every RPC call for cost analysis)
```sql
CREATE TABLE api_audit (
  id UUID PRIMARY KEY,
  endpoint TEXT,
  mint TEXT,
  call_timestamp DATETIME,
  helius_cost_credits INT,
  helius_latency_ms INT,
  http_status INT,
  retry_count INT,
  success BOOL,
  
  INDEX(endpoint),
  INDEX(call_timestamp),
  INDEX(http_status)
);
```

### Schema Design Principles
- **Denormalization** (helius_snapshots): One row = one RPC response. Query convenience > normalized form.
- **Append-only**: No mutations. Audit trail preserved.
- **Temporal joins**: call_timestamp ties snapshots to conviction_labels for before/after analysis.
- **Minimal compute**: Store raw Helius data; let query layer derive signals.

---

## 8. Phase 1 Implementation Plan

### Phase 1a: Schema DDL + Initial Capture
1. Add 3 tables to kernel storage schema
2. Wire `enrich_token()` → persist helius_snapshots + api_audit on every call
3. Seed conviction_labels from calibration_results_real.json

**Cost**: 2-3 hours kernel work + 1 hour backfill script

### Phase 1b: Backfiller
1. Fetch historical Helius data for 33 calibration tokens (4-week lookback)
2. Hydrate helius_snapshots table
3. Track API costs + coverage gaps

**Cost**: 2-3 hours Python + $10-20 in Helius credits (reuse Developer plan budget)

### Phase 1c: Query Tools
1. Correlation analysis: divergence_class vs conviction_tier
2. K-Score accuracy: kscore (from snapshot) vs expected_verdict (from labels)
3. API cost profile: total_credits / token / endpoint

**Cost**: 3-4 hours SQL + Python visualization

### Timeline
- **Week 1**: Phase 1a DDL + wiring
- **Week 2**: Phase 1b backfill + validation
- **Week 3**: Phase 1c analysis

---

## 9. Key Unknowns (To Be Answered by Ground Truth)

| Question | How to Answer | Why It Matters |
|----------|---------------|----------------|
| Does divergence_class predict verdict accuracy? | Correlation test: divergence_class vs expected_verdict | K-Score is 35% of enrichment cost; if it doesn't improve accuracy, drop it. |
| Is concentration (herfindahl) more predictive than K-Score? | Regression: herfindahl, kscore vs verdict accuracy | May eliminate 500 cr behavioral data fetch entirely. |
| Do wallet identities (exchange custody) explain misclassifications? | Cohort analysis: holder_identity="CEX" subset vs retail | Privacy concern or genuine signal? |
| How much does free → Developer plan improve coverage? | Query: holder_data_available=false before/after | Determines if free plan is viable for production. |
| What's the buy/sell ratio distribution? | Histogram: buy_sell_ratio across 33 tokens | Calibrate divergence_class thresholds. |

---

## 10. Non-Goals (Explicitly Out of Scope for Phase 1)

- ❌ MCTS state/reward design (depends on Phase 1 data)
- ❌ Liquidity pool analysis (SolRPDS is LP-level, not token-level)
- ❌ Sentiment analysis (social signals outside Helius scope)
- ❌ Real-time monitoring (Enhanced WebSockets is paid, not tested yet)
- ❌ Historical backtesting >4 weeks (Helius API costs prohibitive)

---

## Recommendation

**Start with Phase 1a this week**: Add the 3-table schema to kernel, wire persistence into enrichment pipeline, and backfill 33 calibration tokens. This costs <$100 in Helius credits and answers "What does Helius actually return?"

**Then Phase 1b/1c**: Build the query tools to answer correlation questions. No code changes needed; just SQL + Python.

**Then MCTS**: Once we have ground truth showing which signals matter, design MCTS state/reward based on observed signal strength, not theoretical importance.
