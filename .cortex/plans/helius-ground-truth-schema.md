# Helius Ground Truth Schema — Data Infrastructure Phase 1

## Rationale
Current enrichment pipeline is **ephemeral**: Helius RPC calls → TokenData computed → stimulus built → verdict rendered → **all data discarded**. 

This prevents:
- Understanding which Helius endpoints/responses correlate with CultScreener conviction
- Recomputing signals without re-fetching (expensive, rate-limited)
- Building MCTS state/reward structure (no historical data to learn from)
- Auditing what Helius actually returned (black box)

**User requirement**: "Il est impératif d'avoir la ground truth sur tout ça avant de créé le registre Solana" — CRITICAL to have ground truth on everything before building Solana registry.

## Solution: Five-Table Snapshot Architecture

All tables append-only (no mutations), indexed for correlation queries. Each entry tagged with `call_timestamp` for temporal joins.

---

## Table 1: `token_snapshots`

**Purpose**: Token-level metadata from Helius getAsset + getTokenLargestAccounts.

**Schema**:
```sql
DEFINE TABLE IF NOT EXISTS token_snapshots;
DEFINE FIELD IF NOT EXISTS mint ON token_snapshots TYPE string;
DEFINE FIELD IF NOT EXISTS call_timestamp ON token_snapshots TYPE datetime;
DEFINE FIELD IF NOT EXISTS helius_api_cost ON token_snapshots TYPE int;  -- Credits used
DEFINE FIELD IF NOT EXISTS helius_latency_ms ON token_snapshots TYPE int;  -- RPC latency
DEFINE FIELD IF NOT EXISTS supply ON token_snapshots TYPE string;  -- BigInt, stored as string
DEFINE FIELD IF NOT EXISTS decimals ON token_snapshots TYPE int;
DEFINE FIELD IF NOT EXISTS mint_authority ON token_snapshots TYPE string;
DEFINE FIELD IF NOT EXISTS freeze_authority ON token_snapshots TYPE string;
DEFINE FIELD IF NOT EXISTS frozen ON token_snapshots TYPE bool;
DEFINE FIELD IF NOT EXISTS holder_count_estimated ON token_snapshots TYPE int;  -- Exponential probe result
DEFINE FIELD IF NOT EXISTS top1_pct ON token_snapshots TYPE float;  -- Top holder concentration
DEFINE FIELD IF NOT EXISTS top10_pct ON token_snapshots TYPE float;  -- Top 10 concentration
DEFINE FIELD IF NOT EXISTS herfindahl_index ON token_snapshots TYPE float;  -- Concentration metric
DEFINE FIELD IF NOT EXISTS request_id ON token_snapshots TYPE string;  -- Kernel request_id for correlation
DEFINE FIELD IF NOT EXISTS error_occurred ON token_snapshots TYPE bool;
DEFINE FIELD IF NOT EXISTS error_message ON token_snapshots TYPE string;
DEFINE INDEX IF NOT EXISTS token_snap_mint_ts_idx ON token_snapshots FIELDS mint, call_timestamp;
DEFINE INDEX IF NOT EXISTS token_snap_ts_idx ON token_snapshots FIELDS call_timestamp;
```

---

## Table 2: `holder_snapshots`

**Purpose**: Per-wallet holder data from getTokenLargestAccounts + behavioral analysis.

**Schema**:
```sql
DEFINE TABLE IF NOT EXISTS holder_snapshots;
DEFINE FIELD IF NOT EXISTS mint ON holder_snapshots TYPE string;
DEFINE FIELD IF NOT EXISTS call_timestamp ON holder_snapshots TYPE datetime;  -- Must match parent token_snapshot
DEFINE FIELD IF NOT EXISTS holder_address ON holder_snapshots TYPE string;
DEFINE FIELD IF NOT EXISTS current_balance ON holder_snapshots TYPE string;  -- BigInt as string
DEFINE FIELD IF NOT EXISTS total_bought_estimate ON holder_snapshots TYPE string;  -- From SWAP history
DEFINE FIELD IF NOT EXISTS retention_ratio ON holder_snapshots TYPE float;  -- current / total
DEFINE FIELD IF NOT EXISTS wallet_class ON holder_snapshots TYPE string;  -- "Accumulator" | "Holder" | "Reducer" | "Extractor"
DEFINE FIELD IF NOT EXISTS holder_identity ON holder_snapshots TYPE string;  -- "CEX" | "DEX" | "Mint" | "Bot" | "Whale" | "Retail"
DEFINE FIELD IF NOT EXISTS swap_history_available ON holder_snapshots TYPE bool;
DEFINE FIELD IF NOT EXISTS swap_count ON holder_snapshots TYPE int;
DEFINE FIELD IF NOT EXISTS last_swap_timestamp ON holder_snapshots TYPE datetime;
DEFINE FIELD IF NOT EXISTS helius_api_cost ON holder_snapshots TYPE int;  -- Credits for SWAP history fetch
DEFINE FIELD IF NOT EXISTS request_id ON holder_snapshots TYPE string;  -- Kernel request_id
DEFINE FIELD IF NOT EXISTS error_occurred ON holder_snapshots TYPE bool;
DEFINE INDEX IF NOT EXISTS holder_snap_mint_ts_idx ON holder_snapshots FIELDS mint, call_timestamp;
DEFINE INDEX IF NOT EXISTS holder_snap_address_idx ON holder_snapshots FIELDS holder_address;
DEFINE INDEX IF NOT EXISTS holder_snap_class_idx ON holder_snapshots FIELDS wallet_class;
```

---

## Table 3: `behavioral_snapshots`

**Purpose**: Aggregated behavioral signals derived from holder population.

**Schema**:
```sql
DEFINE TABLE IF NOT EXISTS behavioral_snapshots;
DEFINE FIELD IF NOT EXISTS mint ON behavioral_snapshots TYPE string;
DEFINE FIELD IF NOT EXISTS call_timestamp ON behavioral_snapshots TYPE datetime;  -- Must match token_snapshot
DEFINE FIELD IF NOT EXISTS buy_sell_ratio ON behavioral_snapshots TYPE float;  -- Buy volume / Sell volume
DEFINE FIELD IF NOT EXISTS divergence_class ON behavioral_snapshots TYPE string;  -- "EARLY_ACCUM" | "STRONG_HOLD" | "DISTRIBUTION" | "AMBIGUOUS"
DEFINE FIELD IF NOT EXISTS percentile_divergence ON behavioral_snapshots TYPE float;  -- (0.0 to 1.0) signal strength
DEFINE FIELD IF NOT EXISTS accumulator_count ON behavioral_snapshots TYPE int;  -- Wallets in accumulation phase
DEFINE FIELD IF NOT EXISTS holder_count ON behavioral_snapshots TYPE int;  -- Stable holders
DEFINE FIELD IF NOT EXISTS reducer_count ON behavioral_snapshots TYPE int;  -- Taking profits
DEFINE FIELD IF NOT EXISTS extractor_count ON behavioral_snapshots TYPE int;  -- Exiting
DEFINE FIELD IF NOT EXISTS k_score ON behavioral_snapshots TYPE float;  -- Composite scoring
DEFINE FIELD IF NOT EXISTS k_score_diamond_hands ON behavioral_snapshots TYPE float;  -- Component
DEFINE FIELD IF NOT EXISTS k_score_organic_growth ON behavioral_snapshots TYPE float;  -- Component
DEFINE FIELD IF NOT EXISTS k_score_longevity ON behavioral_snapshots TYPE float;  -- Component
DEFINE FIELD IF NOT EXISTS estimated_total_helius_cost ON behavioral_snapshots TYPE int;  -- Total credits for all API calls
DEFINE FIELD IF NOT EXISTS holder_sample_size ON behavioral_snapshots TYPE int;  -- How many holders analyzed
DEFINE FIELD IF NOT EXISTS holder_data_available ON behavioral_snapshots TYPE bool;  -- Was data degraded?
DEFINE FIELD IF NOT EXISTS request_id ON behavioral_snapshots TYPE string;  -- Kernel request_id
DEFINE INDEX IF NOT EXISTS behav_snap_mint_ts_idx ON behavioral_snapshots FIELDS mint, call_timestamp;
DEFINE INDEX IF NOT EXISTS behav_snap_divergence_idx ON behavioral_snapshots FIELDS divergence_class;
DEFINE INDEX IF NOT EXISTS behav_snap_kscore_idx ON behavioral_snapshots FIELDS k_score;
```

---

## Table 4: `helius_api_calls`

**Purpose**: Audit trail of every RPC call made during enrichment.

**Schema**:
```sql
DEFINE TABLE IF NOT EXISTS helius_api_calls;
DEFINE FIELD IF NOT EXISTS endpoint ON helius_api_calls TYPE string;  -- "getAsset" | "getTokenLargestAccounts" | "getTokenAccounts" | "getMultipleAccounts" | "getSignaturesForAsset"
DEFINE FIELD IF NOT EXISTS mint ON helius_api_calls TYPE string;
DEFINE FIELD IF NOT EXISTS holder_address ON helius_api_calls TYPE string;  -- Only set for holder-specific calls
DEFINE FIELD IF NOT EXISTS call_timestamp ON helius_api_calls TYPE datetime;
DEFINE FIELD IF NOT EXISTS request_id ON helius_api_calls TYPE string;  -- Kernel request_id for correlation
DEFINE FIELD IF NOT EXISTS helius_cost_credits ON helius_api_calls TYPE int;
DEFINE FIELD IF NOT EXISTS latency_ms ON helius_api_calls TYPE int;
DEFINE FIELD IF NOT EXISTS http_status ON helius_api_calls TYPE int;  -- 200 | 429 (rate limit) | 503 (overloaded)
DEFINE FIELD IF NOT EXISTS response_size_bytes ON helius_api_calls TYPE int;
DEFINE FIELD IF NOT EXISTS error_message ON helius_api_calls TYPE string;  -- If non-200
DEFINE FIELD IF NOT EXISTS retry_count ON helius_api_calls TYPE int;  -- How many retries before success/failure
DEFINE FIELD IF NOT EXISTS helius_request_id ON helius_api_calls TYPE string;  -- Helius-side request ID for debugging
DEFINE INDEX IF NOT EXISTS api_call_ts_idx ON helius_api_calls FIELDS call_timestamp;
DEFINE INDEX IF NOT EXISTS api_call_mint_idx ON helius_api_calls FIELDS mint;
DEFINE INDEX IF NOT EXISTS api_call_endpoint_idx ON helius_api_calls FIELDS endpoint;
DEFINE INDEX IF NOT EXISTS api_call_status_idx ON helius_api_calls FIELDS http_status;
```

---

## Table 5: `conviction_ground_truth`

**Purpose**: CultScreener conviction labels linked to token snapshots.

**Schema**:
```sql
DEFINE TABLE IF NOT EXISTS conviction_ground_truth;
DEFINE FIELD IF NOT EXISTS mint ON conviction_ground_truth TYPE string;
DEFINE FIELD IF NOT EXISTS calibration_batch ON conviction_ground_truth TYPE string;  -- "calibration_results_real" or batch name
DEFINE FIELD IF NOT EXISTS symbol ON conviction_ground_truth TYPE string;
DEFINE FIELD IF NOT EXISTS cultscreener_conviction ON conviction_ground_truth TYPE float;  -- User conviction rating (0.0 to 1.0)
DEFINE FIELD IF NOT EXISTS conviction_tier ON conviction_ground_truth TYPE string;  -- "strong" | "mixed" | "weak"
DEFINE FIELD IF NOT EXISTS expected_verdict ON conviction_ground_truth TYPE string;  -- "Howl" | "Wag" | "Growl" | "Bark" (ground truth)
DEFINE FIELD IF NOT EXISTS notes ON conviction_ground_truth TYPE string;  -- Why this conviction?
DEFINE FIELD IF NOT EXISTS first_snapshot_timestamp ON conviction_ground_truth TYPE datetime;  -- When we first captured token_snapshot for this mint
DEFINE FIELD IF NOT EXISTS last_snapshot_timestamp ON conviction_ground_truth TYPE datetime;  -- Most recent snapshot
DEFINE FIELD IF NOT EXISTS snapshot_count ON conviction_ground_truth TYPE int;  -- How many enrichment calls made for this token
DEFINE FIELD IF NOT EXISTS created_at ON conviction_ground_truth TYPE datetime;
DEFINE INDEX IF NOT EXISTS conviction_mint_idx ON conviction_ground_truth FIELDS mint UNIQUE;
DEFINE INDEX IF NOT EXISTS conviction_batch_idx ON conviction_ground_truth FIELDS calibration_batch;
DEFINE INDEX IF NOT EXISTS conviction_tier_idx ON conviction_ground_truth FIELDS conviction_tier;
```

---

## Integration Points

### 1. Pipeline Modification: `enrich_token()`
In `cynic-kernel/src/pipeline/enrichment.rs`, after enrichment completes:

```rust
if let Some(token_data) = &enriched_result.token_data {
    // Persist snapshots (async, fire-and-forget)
    let _ = store_enrichment_snapshots(
        &token_data,
        &stimulus.request_id,
        deps.storage,
    ).await;
}
```

### 2. New Function: `store_enrichment_snapshots()`
Writes to all 5 tables in a single SurrealQL batch:

```
BEGIN TRANSACTION;
  INSERT INTO token_snapshots { ... };
  INSERT INTO holder_snapshots { ... } (multiple rows, one per holder);
  INSERT INTO behavioral_snapshots { ... };
  INSERT INTO helius_api_calls { ... } (multiple rows, one per RPC call);
COMMIT;
```

### 3. Backfiller: `helius_backfill.py` (Tier 2 INFRASTRUCTURE)
Reads `calibration_results_real.json`, fetches + stores snapshots for all 33 tokens (4-week lookback).

---

## Query Examples (Post-Schema)

### Q1: Divergence Signal Strength vs Conviction
```sql
SELECT 
  g.mint,
  g.symbol,
  g.cultscreener_conviction,
  g.conviction_tier,
  b.divergence_class,
  b.percentile_divergence,
  b.k_score
FROM conviction_ground_truth g
INNER JOIN behavioral_snapshots b ON g.mint = b.mint
WHERE g.mint IN (SELECT mint FROM conviction_ground_truth WHERE calibration_batch = 'calibration_results_real')
ORDER BY g.conviction_tier, b.k_score DESC;
```

### Q2: Concentration vs Expected Verdict Accuracy
```sql
SELECT
  g.conviction_tier,
  t.top1_pct,
  COUNT(*) as token_count,
  AVG(CASE WHEN ... THEN 1 ELSE 0 END) as verdict_accuracy
FROM token_snapshots t
INNER JOIN conviction_ground_truth g ON t.mint = g.mint
GROUP BY g.conviction_tier, t.top1_pct;
```

### Q3: Helius API Cost Profile
```sql
SELECT
  endpoint,
  COUNT(*) as call_count,
  SUM(helius_cost_credits) as total_credits,
  AVG(latency_ms) as avg_latency_ms,
  COUNT(CASE WHEN http_status != 200 THEN 1 END) as failures
FROM helius_api_calls
GROUP BY endpoint
ORDER BY total_credits DESC;
```

---

## Implementation Plan

1. **Week 1 (Phase 1.1)**: Schema DDL + integration with `enrich_token()`
2. **Week 2 (Phase 1.2)**: Backfiller script + historical fetch
3. **Week 3 (Phase 1.3)**: Query tool + correlation analysis UI
4. **Week 4+ (Phase 2)**: MCTS state/reward design (requires Phase 1 data)

---

## Cost & Trade-Offs

**Storage**: ~100MB for 33 tokens × 30 days × hourly snapshots = ~23K rows
**Helius API**: Backfill = 33 tokens × 10-40 credits per token = ~500 credits ($0.50)
**Insight Gain**: Complete audit trail of what Helius returns; can correlate signals with conviction

**K15 Consumer**: Query tool (interactive or automated correlation report) driven by this data.

---

## Why This First?

Without this data layer:
- ❌ Testing enrichment accuracy is circular (no ground truth)
- ❌ Can't measure Helius API cost/benefit trade-off
- ❌ MCTS learning loop has no historical state→reward examples
- ❌ Can't audit which RPC calls failed or were rate-limited

With this data layer:
- ✅ Can correlate divergence signals with CultScreener conviction
- ✅ Can measure optimal Helius API call strategy
- ✅ Can build MCTS training corpus
- ✅ Have auditable, reproducible enrichment pipeline
