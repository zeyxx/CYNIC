# Wallet Behavior Analysis — Human vs Bot Detection

**Status:** Phase 1 Complete (design + implementation + validation framework)  
**Falsification Criterion:** ROC-AUC > 0.7 on 10H + 10S validation corpus  
**Next:** Real corpus collection May 2-3 (CYNIC data-centric, independent of B&C)

---

## What It Does

Scores Solana wallets 0.0-1.0 on authenticity (human vs bot).

**Gate:** score ≥ φ⁻¹ = 0.618 → "verified human"

**Use case:** Filter CYNIC token verdicts by "% verified humans in holder set" → reweight conviction.

---

## Four-Tier Heuristic

| Tier | Signal | Examples |
|------|--------|----------|
| **Age (25%)** | Wallet tenure | Fresh < 7d (0.15) vs Proven > 28d (0.65) |
| **Diversity (30%)** | Token/program spread | Single token concentration (0.15) vs 10+ tokens (0.65) |
| **Temporal (25%)** | Transaction spread over time | Clustered < 3 days (0.15) vs Sustained > 14d (0.65) |
| **Anomalies (20%)** | Red flags | All txs in 1 hour → 0.10; no flags → 1.00 |

---

## Modules

### 1. `wallet_behavior_scorer.py` (Core)

**Pure function** that scores a WalletProfile.

```python
from wallet_behavior_scorer import WalletProfile, score_wallet

profile = WalletProfile(
    wallet_address="...",
    wallet_age_days=45,
    token_count=22,
    program_count=8,
    activity_span_days=40,
    total_transactions=87,
    transaction_density=2.17,
    gap_max_days=3,
    all_txs_same_hour=False,
    single_token_pct=8.5,
    recent_whale_flag=False,
    transaction_frequency_anomaly=False,
)

profile = score_wallet(profile)

print(f"Score: {profile.authenticity_score:.3f}")
print(f"Verified: {profile.is_verified_human}")
```

**Run unit tests:**
```bash
python3 wallet_behavior_scorer.py
# ✓ All tests passed! (synthetic data: 1 human, 1 sybil, 1 emerging, 1 whale)
```

---

### 2. `wallet_behavior_helius.py` (Data Collection)

**Fetches on-chain data from Helius API** and builds WalletProfile.

Requires: `HELIUS_API_KEY` env var

```python
from wallet_behavior_helius import HeliusWalletCollector

collector = HeliusWalletCollector()
profile = collector.collect_wallet_profile("G2gCo4Hx2Pq8Rm9Nz3Kp0Lk5Jq7Mn2Vt")

print(f"Score: {profile.authenticity_score:.3f}")
```

**CLI usage:**
```bash
# Load API key from environment
python3 wallet_behavior_helius.py G2gCo4Hx2Pq8Rm9Nz3Kp0Lk5Jq7Mn2Vt
```

**Cost per wallet:** ~120 credits (~$0.03 on Developer plan)

---

### 3. `wallet_behavior_validator.py` (Validation)

**Measures ROC-AUC and confusion matrix** on labeled corpus.

```python
from wallet_behavior_validator import WalletValidator

profiles, labels = WalletValidator.load_corpus("corpus.json")
result = WalletValidator.validate(profiles, labels)

print(result)
# Validation Results
# ==================
# ROC-AUC: 0.95
# ✓ PASS (target > 0.7)
```

**Corpus format** (`corpus.json`):
```json
[
    {
        "wallet_address": "...",
        "is_human": true,
        "wallet_age_days": 45,
        "token_count": 22,
        "program_count": 8,
        ...
    },
    ...
]
```

**Run with synthetic test corpus:**
```bash
python3 wallet_behavior_validator.py
# ROC-AUC: 1.000
# ✓ PASS (target > 0.7)
```

---

## Validation Workflow

### Step 1: Collect Corpus (S.'s Responsibility)

Provide `validation_corpus.json` with 20 wallets:
- 10 verified humans (from B&C game history)
- 10 known Sybils (pump.fun bots, MEV addresses, etc.)

Each entry must include:
```json
{
  "wallet_address": "...",
  "is_human": true/false,
  "wallet_age_days": <int>,
  "token_count": <int>,
  "program_count": <int>,
  "activity_span_days": <int>,
  "total_transactions": <int>,
  "transaction_density": <float>,
  "gap_max_days": <int>,
  "all_txs_same_hour": <bool>,
  "single_token_pct": <float>,
  "recent_whale_flag": <bool>,
  "transaction_frequency_anomaly": <bool>
}
```

### Step 2: Validate (T.'s Responsibility)

```bash
python3 wallet_behavior_validator.py validation_corpus.json
```

**Falsification criterion:** ROC-AUC > 0.7

**If PASS:**
- Integration into B&C `/mint-permit` by May 4
- Measure human-filter impact on CYNIC Dogs (May 5-6)
- Integrate into CultScreener display (May 7-8)

**If FAIL:**
- Debug signal separation
- Adjust heuristic weights or thresholds
- Re-test on corpus

---

## Integration Points

### B&C `/mint-permit` Gate

```python
# Before minting card:
wallet_profile = collector.collect_wallet_profile(wallet)
if wallet_profile.is_verified_human and game_result.verified:
    mint_card(wallet, "verified_human + verified_game")
else:
    mint_card(wallet, "verified_game_only")
    # metadata.verified_by = "game_only"
    # metadata.authenticity_score = wallet_profile.authenticity_score
```

### CYNIC Token Reweighting

```python
# After scoring token with Dogs:
top_100_holders = helius.getTokenLargestAccounts(mint, limit=100)
verified_human_count = sum(
    collector.collect_wallet_profile(h).is_verified_human
    for h in top_100_holders
)
verified_human_pct = verified_human_count / 100

# Adjust confidence
if verified_human_pct > 0.70:
    q_score_adjusted = q_score * 1.15  # boost
elif verified_human_pct < 0.30:
    q_score_adjusted = q_score * 0.85  # reduce
else:
    q_score_adjusted = q_score  # neutral

return {"conviction": conviction, "verified_humans_pct": verified_human_pct, "q_score": q_score_adjusted}
```

---

## Epistemic Status

- **Observed:** B&C game verification works; Helius API data available; pump.fun wallets identifiable
- **Deduced:** Four-tier heuristic achieves ROC-AUC=1.0 on synthetic data (2H + 2S)
- **Inferred:** Real corpus will achieve ROC-AUC > 0.7 (pattern from token-domain Dogs: achieved 0.95 on rug prediction)
- **Conjecture:** Verified human filter will boost CYNIC confidence on legitimate tokens by 15%+ (falsifiable via A/B on 20 tokens)

**Falsifiable:**
1. ROC-AUC > 0.7 on real 20-wallet corpus → Tier 1 test (May 3)
2. Δ > 5% in CYNIC verdict distribution when filtering by verified humans → Tier 2 test (May 6)
3. CultScreener display metrics render live → Tier 3 test (May 8)

---

## Timeline

| Date | Task | Status |
|------|------|--------|
| Apr 30 | Design + implement scorer, collector, validator | ✓ COMPLETE |
| May 1 | S. confirms co-submit + provides corpus | ⏳ PENDING |
| May 2-3 | Run validation on real corpus (ROC-AUC > 0.7) | Blocked |
| May 4 | Integrate into B&C `/mint-permit` | Blocked |
| May 5-6 | Measure CYNIC impact (Δ > 5%) | Blocked |
| May 7-8 | Integrate into CultScreener display | Blocked |

---

## Cost

| Task | Credits | Cost |
|------|---------|------|
| Validate 20 wallets (Helius fetch) | ~2,400 | $0.60 |
| Measure impact on 20-30 tokens | ~3,000 | $0.75 |
| **Total** | ~5,400 | **$1.35** |

(Helius Developer plan: $50/month = 1M credits)

