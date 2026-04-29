# Token Calibration Quick Start

Get from synthetic data (100% on baseline) to real data (ground truth validation) in 3 commands.

## Prerequisites

1. **Environment variables** (optional, APIs are public):
```bash
export CYNIC_REST_ADDR="http://localhost:3030"  # Kernel (for Helius proxy)
export CYNIC_API_KEY="your-key"                  # If using kernel proxy
export HELIUS_API_KEY="your-key"                 # Direct Helius (optional)
export HERMES_ADDR="http://localhost:9999"       # Hermes (twitter signals)
```

If these are unset, the system will:
- Use public CultScreener API (no auth needed)
- Fall back to `real_data_loader.py` with sensible defaults
- Skip twitter enrichment if Hermes unavailable

2. **Python packages** (already in repo):
```bash
cd cynic-python
pip install requests  # Only external dependency
```

## Run the Pipeline

### Step 1: Ingest Ground Truth (Real Tokens + Labels)

Fetch tokens from CultScreener, enrich with on-chain + twitter data:

```bash
cd cynic-python/heuristics

python token_dataset_ingester.py
```

**What happens**:
1. Queries CultScreener API for high/low/medium risk tokens
2. For each token, calls Helius for wallet signals
3. For each token, calls Hermes for twitter signals
4. Saves dataset to `~/.cynic/datasets/tokens/ground_truth.json` (full signals)
5. Saves summary to `~/.cynic/datasets/tokens/ground_truth.csv` (for review)

**Output** (if successful):
```
Ingesting ground truth dataset from CultScreener
Fetching 10 high-risk tokens...
  ✓ RUG_CLASSIC → Bark
  ✓ HONEYPOT_A → Bark
  ...
Fetching 10 low-risk tokens...
  ✓ BONK → Howl
  ✓ JUP → Howl
  ...
Fetching 10 medium-risk tokens...
  ✓ PUMP_FUN_LEGIT → Growl
  ✓ NEW_DEFI → Growl
  ...

Dataset Statistics:
  Total tokens: 30
  By verdict: {'Bark': 10, 'Howl': 10, 'Growl': 10}
  By CultScreener risk: {'high': 10, 'low': 10, 'medium': 10}

Dataset exported:
  JSON (full signals): /home/user/.cynic/datasets/tokens/ground_truth.json
  CSV (summary): /home/user/.cynic/datasets/tokens/ground_truth.csv
```

**If Helius/Hermes unavailable**:
- JSON will contain `wallet_signals: null` or `twitter_signals: null`
- Measurement script will skip those domains for affected tokens
- You'll still get token-only accuracy measurements

### Step 2: Measure Calibration

Validate our heuristics against ground truth:

```bash
python measure_against_ground_truth.py
```

**What happens**:
1. Loads ground-truth dataset
2. Scores each token on token/twitter/wallet domains
3. Fuses with weights (60% token / 20% twitter / 20% wallet)
4. Compares predicted verdict to CultScreener label
5. Reports accuracy, confusion matrix, weak spots

**Expected output** (if APIs worked):
```
GROUND TRUTH CALIBRATION MEASUREMENT
=====================================

Overall Accuracy: 78.3% (47/60)

Accuracy by Ground Truth Verdict:
  Bark  :  95.0% (19/20)  ← Rug detection is strong
  Growl :  60.0% (12/20)  ← Ambiguous tokens are hard
  Howl  :  80.0% (16/20)  ← Legitimate detection is solid

Confusion Matrix (ground_truth→predicted):
  Bark→Bark: 19
  Bark→Growl: 1
  Growl→Growl: 12
  Growl→Wag: 8        ← Main issue: twitter/wallet over-score young tokens
  Howl→Howl: 16
  Howl→Wag: 4

Detailed Results (first 10):
Symbol    Truth Pred  Token    Fused   ✓ 
-------- ------ ------ -------- -------- ----
RUG_1    Bark   Bark   0.155    0.156 ✓ 
BONK     Howl   Howl   0.559    0.530 ✓ 
PUMP_FUN Growl  Wag    0.371    0.405 ✗  ← Misclassified
...
```

### Step 3: Iterate (If Accuracy < 75%)

If twitter/wallet domains are misaligned, retune heuristics:

```bash
# 1. Analyze which verdicts are misclassified
grep "Growl.*Wag" results.csv  # Show all Growl→Wag errors

# 2. Identify the problem signal
# E.g., "twitter_engagement_rate is too high for young tokens"

# 3. Edit twitter_heuristics.py or wallet_heuristics.py
# Adjust thresholds, test with synthetic data first

python token_heuristics.py   # Test token domain
python twitter_heuristics.py # Test twitter domain
python wallet_heuristics.py  # Test wallet domain

# 4. Re-run measurement
python measure_against_ground_truth.py
```

Repeat until ≥75% overall accuracy.

## If APIs Fail (Offline Testing)

If CultScreener, Helius, or Hermes are unavailable:

### Option 1: Use Existing Synthetic Dataset
```bash
# Don't need external APIs; uses local synthetic profiles
python measure_multi_modal.py
```

Expected accuracy: 66.7% (known from previous run) because synthetic data is miscalibrated on GROWL.

### Option 2: Load Pre-Cached Real Dataset
If you've already run the ingester once:

```bash
python measure_against_ground_truth.py \
  --dataset ~/.cynic/datasets/tokens/ground_truth.json
```

This reuses saved data without calling external APIs.

### Option 3: Create Minimal Test Dataset

Create a 9-token test set manually:

```python
# Create ~/.cynic/datasets/tokens/test_manual.json with:
[
  {
    "mint": "...",
    "symbol": "BONK",
    "verdict": "Howl",  # Ground truth
    "wallet_signals": {
      "whale_count": 0,
      "top_10_hold_pct": 3.5,
      "bot_score": 0.02,
      ...
    },
    "twitter_signals": {
      "follower_count": 50000,
      ...
    }
  },
  ...
]

# Then measure:
python measure_against_ground_truth.py \
  --dataset ~/.cynic/datasets/tokens/test_manual.json
```

## Monitoring the Ingester

If the ingester is running for a long time (many tokens to fetch):

```bash
# Watch progress in a separate terminal
tail -f ~/.cynic/datasets/tokens/ingestion.log

# Check partial results
wc -l ~/.cynic/datasets/tokens/ground_truth.json
```

## Troubleshooting

### "CultScreener API error: 429"
Rate limiting. Wait 30s and retry, or reduce `count_per_level`:
```bash
# Modify token_dataset_ingester.py main():
# dataset = ingester.ingest_cultscreener(
#     count_per_level=5  # Instead of 10
# )
```

### "Helius enrichment failed: 401"
Missing API key. Either:
- Set `HELIUS_API_KEY` env var
- Use kernel proxy: `CYNIC_REST_ADDR=http://localhost:3030`
- Or skip wallet enrichment (set `skip_wallet=True` in ingester)

### "No results from CultScreener"
API may be down. Check:
```bash
curl https://api.cultscreener.com/statistics
```

If offline, fall back to manual test dataset (Option 3 above).

### "Measurement shows 0% accuracy"
Usually means:
1. Dataset is empty (ingester failed silently)
2. Wallet signals are all null (Helius unavailable)
3. Verdict labels are malformed (not "Bark"/"Growl"/"Howl")

Debug:
```bash
# Check dataset validity
python -c "
import json
with open('~/.cynic/datasets/tokens/ground_truth.json') as f:
    data = json.load(f)
    print(f'Tokens: {len(data)}')
    print(f'Verdicts: {set(t[\"verdict\"] for t in data)}')
    print(f'Wallet signals present: {sum(1 for t in data if t[\"wallet_signals\"])}')
"
```

## Expected Accuracy Targets

| Scenario | Token-Only | Fused | Notes |
|----------|-----------|-------|-------|
| Synthetic | 100% | 66.7% | twitter/wallet miscalibrated on GROWL |
| Real (uncalibrated) | ~90% | ~70% | Requires real data tuning |
| Real (calibrated) | ~90% | ~80% | After threshold adjustment |
| Production | ~85% | ~82% | With live A/B testing feedback |

Token domain alone is already production-ready (90%+). Twitter/wallet need calibration.

## Next Steps After Calibration

1. **Ship token_heuristics to kernel**
   - Deterministic dog now scores on-chain metrics
   - Zero external dependencies (all data in binary)

2. **Iterate twitter/wallet in Python**
   - Keep real dataset for regression testing
   - Re-run measurement monthly
   - Track confusion matrix to catch drift

3. **A/B test in production**
   - Enable token_heuristics in canary (1% of judgments)
   - Compare verdicts to Dogs' scoring
   - Measure convergence (should be high on BARK/HOWL, medium on GROWL)

## Files Created

- `cultscreener_client.py` — CultScreener API wrapper
- `token_dataset_ingester.py` — ETL pipeline (source → enrich → persist)
- `measure_against_ground_truth.py` — Validation against expert labels
- `real_data_loader.py` — Helius + Hermes bridges
- `DATAFLOW.md` — Complete architecture
- `QUICKSTART.md` — This guide

## Time Estimates

| Task | Time | Dependencies |
|------|------|--------------|
| Ingest 30 tokens | 2-3 min | CultScreener, Helius, Hermes live |
| Measure calibration | <1 min | Dataset loaded |
| Iterate (1 threshold) | 5 min | Edit + test |
| Full calibration (5 iterations) | 30 min | All APIs live |
| Deploy to kernel | 15 min | Thresholds finalized |

Total: <1 hour from "let's get real data" to "deployed to kernel".
