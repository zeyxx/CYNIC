# Token Judgment Heuristics

Real-time, domain-specialized token verdict system. Multi-modal: token metrics (on-chain) + twitter signals (community) + wallet signals (behavior).

## Architecture

**Data-centric ETL pipeline:**
```
CultScreener (ground truth labels)
    ↓
Token Dataset Ingester (source → enrich → persist)
    ├→ Helius API (wallet signals)
    ├→ Hermes API (twitter signals)
    └→ Offline mock dataset (when APIs unavailable)
    ↓
Measurement Suite (compare predicted vs ground truth)
    ├→ Token Scorer (on-chain metrics)
    ├→ Twitter Scorer (community sentiment)
    ├→ Wallet Scorer (holder behavior)
    └→ Fusion engine (weighted: 60% token / 20% twitter / 20% wallet)
    ↓
Calibration Report (confusion matrix, accuracy by verdict)
    ↓
Deploy to Kernel (translate Rust thresholds, ship in deterministic-dog)
```

## Quick Start

### Option 1: Offline Testing (No APIs)
Create a synthetic ground-truth dataset locally:
```bash
cd cynic-python/heuristics
python3 create_mock_dataset.py
python3 measure_against_ground_truth.py
```

**Expected result:** 77.8% accuracy on 9 tokens (3 BARK, 3 HOWL, 3 GROWL).  
**Key finding:** Token domain 100% accurate; twitter/wallet need real data calibration.

### Option 2: Real Data (CultScreener Live)
When CultScreener API is available:
```bash
cd /home/user/Bureau/CYNIC
./scripts/calibrate-token-heuristics.sh all
```

This runs:
1. **Ingest phase:** Fetch 20 high/medium/low-risk tokens from CultScreener, enrich with Helius + Hermes
2. **Measure phase:** Score on all 3 domains, compare predicted vs ground truth
3. **Analyze phase:** Report accuracy, identify weak signals
4. **Deploy phase:** Translate calibrated thresholds to Rust, ship in kernel

## Files

### Core Heuristics
- `token_heuristics.py` — On-chain signal scorer (authority, concentration, age, liquidity, origin)
- `twitter_heuristics.py` — Community signal scorer (sentiment, engagement, red flags, decentralization)
- `wallet_heuristics.py` — Holder behavior scorer (bot %, whale count, retail %, trading activity)

### Data Pipeline
- `cultscreener_client.py` — CultScreener API wrapper (public API, no auth required)
- `token_dataset_ingester.py` — ETL: source (CultScreener) → enrich (Helius + Hermes) → persist (JSON/CSV)
- `real_data_loader.py` — Bridges to Helius (wallet signals) and Hermes (twitter signals)
- `create_mock_dataset.py` — Offline test dataset (9 tokens, all signals pre-populated)

### Validation & Measurement
- `measure_against_ground_truth.py` — Load ground-truth dataset, score each token, compare vs CultScreener labels
- `dataset_builder.py` — Dataclass definitions (TokenMetrics, TwitterSignals, WalletSignals)

### Documentation
- `QUICKSTART.md` — 3-command workflow (ingest → measure → iterate)
- `DATAFLOW.md` — Complete architecture & signal definitions
- `CALIBRATION-ANALYSIS.md` — Root cause analysis of GROWL miscalibration, next steps
- `CALIBRATION_REPORT.md` — Historical calibration data, decision rationale

### Automation
- `../scripts/calibrate-token-heuristics.sh` — Multi-phase workflow (ingest/measure/analyze/tune/deploy)

## Accuracy Targets

| Component | Metric | Target | Status |
|-----------|--------|--------|--------|
| **Token domain** | BARK/HOWL accuracy | 90%+ | ✓ 100% (baseline) |
| **Twitter domain** | GROWL identification | 70%+ | ⚠ Needs real data |
| **Wallet domain** | GROWL identification | 70%+ | ⚠ Needs real data |
| **Fused verdict** | Overall accuracy | 75%+ | ⚠ 77.8% on mock (GROWL weak) |

## Key Findings (Mock Data Analysis)

### What Works
✓ **Token domain:** 100% accurate on BARK (rugs) and HOWL (legitimate)  
✓ **Architecture:** Multi-modal fusion reduces single-domain bias  
✓ **Pipeline:** End-to-end ingestion + measurement working  

### What Needs Calibration
⚠ **GROWL (ambiguous young tokens):** Confused with WAG (marginal tokens)  
⚠ **Root cause:** Twitter/wallet scorers reward characteristics (engagement, retail holding) that are GOOD in young legitimate tokens but also HIGH in young rugs  
⚠ **Solution:** Age-stratified thresholds (tokens < 336h = 2 weeks need discount on engagement signals)

### Hidden Variables (Hypothesis)
- **Age matters:** A 5% engagement rate on a 2-day-old token = GROWL baseline, not HOWL signal
- **Origin matters:** pump.fun tokens with real communities are legitimately ambiguous (neither rug nor established)
- **Independence matters:** Twitter + wallet signals may not be independent predictors until token age > 2 weeks

## Calibration Workflow

### Phase 1: Real Data Ingestion
When CultScreener API is live:
```bash
python token_dataset_ingester.py
```
Outputs:
- `~/.cynic/datasets/tokens/ground_truth.json` (full signals, all 3 domains)
- `~/.cynic/datasets/tokens/ground_truth.csv` (summary for manual review)

### Phase 2: Measure Calibration
```bash
python measure_against_ground_truth.py
```
Outputs:
- Confusion matrix (shows GROWL→WAG misclassification)
- Per-category accuracy (BARK/GROWL/HOWL)
- Domain breakdown (token-only vs twitter vs wallet)

### Phase 3: Analyze Root Cause
Extract failing tokens from confusion matrix, plot:
- `token_age_hours` vs `fused_score`
- `engagement_rate` vs `verdict`
- `positive_pct` vs `verdict`

Expected pattern: Young tokens cluster near GROWL/WAG boundary.

### Phase 4: Tune Heuristics (if needed)
If GROWL accuracy < 70%, implement age-based adjustment:
```python
# Pseudo-code for twitter_heuristics.py
if token_age_hours < 336:  # 2 weeks
    engagement_discount = (token_age_hours / 336)  # Linear ramp 0→1
    twitter_score = twitter_score * engagement_discount
```

Re-measure after each adjustment.

### Phase 5: Deploy to Kernel
Once accuracy ≥ 75%:
1. Copy calibrated thresholds from Python to Rust
2. Update `cynic-kernel/src/dogs/deterministic/token.rs`
3. Rebuild: `make check && make build`
4. Deploy: `cp target/release/cynic-kernel ~/bin/cynic-kernel`
5. Verify: `curl ${CYNIC_REST_ADDR}/judge -d '{"domain":"token-analysis","content":"..."}'`

## Integration with Kernel

The Python heuristics are **validation & calibration tools** (tier 2 analysis).  
The Rust deterministic-dog (tier 1 kernel) embeds the calibrated thresholds.

**Deployment:** Thresholds from Python → constants in `token.rs` + `twitter.rs` (if added)

**Pattern:** Python experiments + measures, Rust ships + enforces.

## Data Format

### Ground Truth Dataset (JSON)
```json
{
  "mint": "DezXAZ8z7PnrnRJjz3wXBoRgixVqXaSo1S1zceA85q",
  "symbol": "BONK",
  "verdict": "Howl",  // CultScreener ground truth
  "cultscreener_risk": "low",
  "cultscreener_confidence": 0.97,
  "token_metrics": {
    "holders": 1200000,
    "top10_pct": 3.5,
    "herfindahl": 0.03,
    "age_hours": 10000,
    "mint_authority_active": false,
    ...
  },
  "wallet_signals": {
    "whale_count": 0,
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

### Measurement Output
```
Overall Accuracy: 77.8% (7/9)

Accuracy by Ground Truth Verdict:
  Bark  : 100.0% (3/3)
  Growl :  33.3% (1/3)
  Howl  : 100.0% (3/3)

Confusion Matrix:
  Bark→Bark: 3
  Growl→Wag: 2
  Howl→Howl: 3
```

## Dependencies

**Python packages:**
- `requests` (CultScreener, Helius, Hermes API calls)
- Standard library: `json`, `csv`, `dataclasses`, `pathlib`, `argparse`

**External APIs (optional, graceful fallback):**
- CultScreener (public, no auth required)
- Helius (requires `HELIUS_API_KEY` or `CYNIC_REST_ADDR`)
- Hermes (requires `HERMES_ADDR`)

If APIs unavailable:
- Use mock dataset (`create_mock_dataset.py`)
- Use cached real dataset (`~/.cynic/datasets/tokens/ground_truth.json` if already ingested)

## Timeline to Production

| Task | Duration | Dependencies |
|------|----------|--------------|
| Ingest real data (60 tokens) | 2-3 min | CultScreener + Helius APIs live |
| Measure calibration | <1 min | Ingestion complete |
| Root cause analysis | 5-10 min | Measurement complete |
| Age-stratified tuning (if needed) | 10-30 min | Analysis complete |
| Rust translation | 15 min | Thresholds finalized |
| Kernel test + deploy | 10 min | Rust code ready |
| **Total time to production** | **<1 hour** | **All APIs available** |

## Next Steps

1. **When CultScreener is available:** Run `./scripts/calibrate-token-heuristics.sh all`
2. **Review results:** Compare fused accuracy to 75% target
3. **If GROWL < 70%:** Implement age-based adjustments per CALIBRATION-ANALYSIS.md Phase 3
4. **Deploy to kernel:** Follow Phase 5 checklist
5. **Measure agreement:** Compare Rust deterministic-dog output to Python measurement on same tokens

## References

- **Archimedes Principle:** Three independent signal channels converge to reduce shared bias
- **Verdict Thresholds:** BARK ≤0.236, GROWL 0.236-0.382, WAG 0.382-0.528, HOWL ≥0.528
- **φ (golden ratio):** φ⁻¹ = 0.618 = max confidence, φ⁻² = 0.382 = WAG/GROWL boundary
- **CultScreener:** https://cultscreener.com (ground truth token risk assessments)
- **Helius:** https://helius.dev (on-chain Solana metrics)

## Troubleshooting

### "CultScreener API error: 429"
Rate limited. Wait 30s, or reduce `count_per_level` in ingester script.

### "Helius enrichment failed: 401"
Missing API key. Set `HELIUS_API_KEY` or configure `CYNIC_REST_ADDR` for kernel proxy.

### "Measurement shows 0% accuracy"
Check dataset validity:
```bash
python3 -c "
import json
with open('~/.cynic/datasets/tokens/ground_truth.json') as f:
    data = json.load(f)
    print(f'Tokens: {len(data)}')
    print(f'Verdicts: {set(t[\"verdict\"] for t in data)}')
    print(f'Wallet signals: {sum(1 for t in data if t.get(\"wallet_signals\"))}')
"
```

### "GROWL accuracy not improving after tuning"
Likely needs real data with more granular age distribution. Current mock dataset has only 3 GROWL tokens at limited age ranges (120h, 600h, 2000h).
