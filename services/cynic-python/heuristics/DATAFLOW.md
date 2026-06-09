# Token Judgment Data Flow

Data-centric architecture for multi-modal token calibration. Three independent signal channels converge to reduce uncertainty (Archimedes principle).

## Data Pipeline

```
CultScreener API (ground truth labels)
    ↓
[cultscreener_client.py] ← fetch_token_risk()
    ↓
Token with risk label (Bark/Growl/Howl)
    ↓
[token_dataset_ingester.py] ← enrich_token()
    ├→ Helius [wallet_signals] ← on-chain holder behavior
    ├→ Hermes [twitter_signals] ← community sentiment/engagement
    └→ (token_metrics inferred from wallet data)
    ↓
Enriched token (3 signal channels + ground truth label)
    ↓
[measure_against_ground_truth.py] ← measure_token()
    ├→ TokenScorer → token_verdict + token_score
    ├→ TwitterScorer → twitter_verdict + twitter_score
    ├→ WalletScorer → wallet_verdict + wallet_score
    └→ Fusion (60% token / 20% twitter / 20% wallet)
    ↓
Measurement result (ground_truth vs predicted)
    ↓
Confusion matrix + accuracy report
```

## Modules

### 1. `cultscreener_client.py` — Ground Truth Source

Fetches token risk assessments from CultScreener API.

**Public API** (no auth required):
- `get_token_risk(mint)` — Single token assessment
- `search_tokens(risk_level, limit)` — Fetch by risk level (high/low/medium)
- `batch_get_risks(mints)` — Multiple tokens

**Risk Levels & Mapping**:
```
CultScreener      CYNIC Verdict
───────────────────────────
high       →      Bark
medium     →      Growl
low        →      Howl
verified   →      Howl
unknown    →      Bark (assume rug until verified)
```

**Output**:
```python
TokenRiskAssessment(
    mint="...",
    symbol="BONK",
    risk_level=RiskLevel.LOW,
    confidence=0.95,
    reasons=["low_concentration", "active_trading", ...],
    last_updated="2026-04-29T12:34:56Z",
)
```

### 2. `token_dataset_ingester.py` — ETL Pipeline

Orchestrates source → enrich → persist workflow.

**Steps**:
1. **Source**: Fetch tokens from CultScreener by risk level
2. **Enrich**: For each token, call Helius for wallet signals + Hermes for twitter signals
3. **Transform**: Map responses to our `WalletSignals` and `TwitterSignals` dataclasses
4. **Persist**: Save as JSON (full signals) and CSV (summary)

**Usage**:
```python
ingester = TokenDatasetIngester()

# Fetch 20 high-risk + 20 low-risk + 20 medium tokens from CultScreener
dataset = ingester.ingest_cultscreener(
    risk_levels=["high", "low", "medium"],
    count_per_level=20,
)

# Export
ingester.save_json(dataset, "~/.cynic/datasets/tokens/ground_truth.json")
ingester.save_csv(dataset, "~/.cynic/datasets/tokens/ground_truth.csv")
```

**Output Format** (JSON):
```json
{
  "mint": "...",
  "symbol": "BONK",
  "name": "Bonk, Solana's Dogecoin",
  "verdict": "Howl",  // Ground truth label
  "cultscreener_risk": "low",
  "cultscreener_confidence": 0.95,
  "cultscreener_reasons": ["low_concentration", ...],
  "wallet_signals": {
    "whale_count": 0,
    "top_10_hold_pct": 3.5,
    "bot_score": 0.02,
    "daily_active_traders": 1000,
    ...
  },
  "twitter_signals": {
    "follower_count": 50000,
    "engagement_rate": 0.08,
    "positive_pct": 0.75,
    ...
  }
}
```

### 3. `measure_against_ground_truth.py` — Validation

Measures how well heuristics align with ground truth.

**Workflow**:
1. Load ground-truth dataset (JSON from ingester)
2. For each token:
   - Score on token domain (TokenScorer)
   - Score on twitter domain (TwitterScorer)
   - Score on wallet domain (WalletScorer)
   - Fuse with weights (60% token / 20% twitter / 20% wallet)
   - Compare predicted verdict to ground truth
3. Report accuracy, confusion matrix, domain breakdown

**Output**:
```
GROUND TRUTH CALIBRATION MEASUREMENT
=====================================

Overall Accuracy: 78.3% (47/60)

Accuracy by Ground Truth Verdict:
  Bark  :  95.0% (19/20)
  Growl :  60.0% (12/20)
  Howl  :  80.0% (16/20)

Confusion Matrix (ground_truth→predicted):
  Bark→Bark: 19
  Bark→Growl: 1
  Growl→Growl: 12
  Growl→Wag: 8
  Howl→Howl: 16
  Howl→Wag: 4
```

## Data Requirements

### Token Count (Balanced Dataset)
For statistical significance:
- **Minimum**: 10-15 tokens per category (30-45 total)
- **Recommended**: 20-30 tokens per category (60-90 total)
- **Gold standard**: 50+ per category (150+ total)

### Quality
- **Labels**: CultScreener (crowd-sourced, on-chain verifiable)
- **Wallet data**: Helius API (real Solana ledger)
- **Twitter data**: Hermes (real social signal)

### Cost
Using Helius for wallet enrichment:
- `getTokenHolders`: ~20 credits per token
- Full dataset (60 tokens): ~1,200 credits (~$0.30 at standard pricing)

## Workflow (Next Steps)

### Session 1: Bootstrap (Measured Run)
```bash
# Run ingester (uses real APIs)
python token_dataset_ingester.py \
  --risk-levels high low medium \
  --count-per-level 10

# This writes:
#   ~/.cynic/datasets/tokens/ground_truth.json (full signals)
#   ~/.cynic/datasets/tokens/ground_truth.csv (summary)

# Measure calibration against ground truth
python measure_against_ground_truth.py

# Output: confusion matrix showing which verdicts we get wrong
```

Expected outcome:
- Token domain accuracy: 90%+ (on-chain signals are reliable)
- Twitter domain accuracy: 50-70% (needs real calibration)
- Wallet domain accuracy: 60-80% (needs real calibration)
- Fused accuracy: 70-85% (should improve with good calibration)

### Session 2: Iterate (If Needed)
If fused accuracy < 75%, identify weak domain:
```bash
# Analyze confusion matrix from Session 1
# → E.g., "Twitter consistently over-scores GROWL tokens"

# Retune twitter_heuristics.py thresholds
# Measure again

python measure_against_ground_truth.py
```

Repeat until ≥75% accuracy on ground truth.

### Session 3: Production Deploy
Once calibrated on ground truth (≥75% accuracy):
1. Translate thresholds to Rust `deterministic-dog`
2. Add to kernel binary
3. Benchmark on live judgment requests
4. Monitor confusion matrix post-deployment

## Known Limitations

### 1. Token Metrics Are Estimated
`measure_against_ground_truth.py` reconstructs TokenMetrics from wallet signals because the ingester doesn't have original on-chain token metadata. This introduces noise.

**Fix** (if needed): Add `fetch_token_metrics()` to `real_data_loader.py` using Helius `getAsset()`.

### 2. Twitter/Wallet Data May Be Stale
CultScreener assessments are 1-30 days old. Helius data is live. Hermes data is 1-7 days old.

**Acceptable** because:
- Convergence test is about signal independence, not absolute accuracy
- Stale data is still better than synthetic data
- Real uncertainty ≠ measurement error

### 3. Hermes May Not Have Twitter Data for All Tokens
Small/pump.fun tokens may have no Hermes coverage. In that case, `twitter_signals` will be `None`.

**Handle**: Fused scoring skips missing domains (uses token_score 3× in that case).

## Archimedes Principle (The Test)

Three independent measurements should converge:
- If all three say BARK → high confidence rug
- If all three say HOWL → high confidence legitimate
- If they diverge → ambiguous case (needs deeper analysis)

**Expected behavior** (with real data):
```
Token=Bark, Twitter=Bark, Wallet=Bark → Fused=Bark (confidence: φ⁻¹ = 0.618)
Token=Bark, Twitter=Growl, Wallet=Bark → Fused=Bark (confidence: 0.5)
Token=Bark, Twitter=Howl, Wallet=Howl  → Fused=Howl (confidence: 0.4, suspicious)
```

If we see the last pattern frequently, one domain is fundamentally broken (or the token genuinely diverged).

## References

- **CultScreener**: https://cultscreener.com
- **Helius**: https://helius.dev (wallet enrichment)
- **Hermes**: Local organ-x deployment (twitter signals)
- **Calibration Theory**: `CALIBRATION_REPORT.md`
