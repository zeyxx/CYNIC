# Wallet Behavior Analysis — Status & Integration Plan

**Date:** 2026-04-30  
**Deadline:** May 2-3 (real corpus validation)  
**Integration:** May 4 (B&C /mint-permit)

---

## Current State: INTEGRATION-READY

### ✓ Completed

1. **Heuristic Design & Implementation** (wallet_behavior_scorer.py)
   - Four-tier scoring: age (25%), diversity (30%), temporal (25%), anomalies (20%)
   - Gate threshold: φ⁻¹ = 0.618 → "verified human"
   - Synthetic validation: ROC-AUC = 1.0, accuracy = 100% on 4-wallet corpus

2. **Data Collection** (wallet_behavior_helius.py)
   - Dual-endpoint architecture: RPC + REST
   - Methods: getBalance, getTokenBalances, getTransactionHistory, getAccountInfo
   - Error handling: graceful degradation on API failures

3. **Integration Module** (wallet_behavior_integration.py)
   - drop-in function: `check_wallet_verified_human(wallet: str)`
   - B&C contract: returns (is_verified: bool, profile: WalletProfile)
   - Timeout handling: 30s default, fails safely to game_only verification

4. **Validation Framework** (wallet_behavior_validator.py)
   - ROC-AUC computation, confusion matrix, threshold optimization
   - Falsification test: target ROC-AUC > 0.7 on real corpus

---

## Known Blockers (API Collection)

| Blocker | Status | Impact | Mitigation |
|---------|--------|--------|-----------|
| **Helius rate limiting** | 429 errors after ~15 wallets | Can't collect 20-wallet corpus in one session | Distribute across 2-3 API keys, batch over 12h |
| **Protocol accounts (no txs)** | Token Program, Orca, Raydium have 0 transactions | No signal separation between humans/bots | Use actual trader/bot addresses, not program accounts |
| **CultScreener API** | No explicit creator field | Can't extract sybil wallets directly | Query token mint → lookup on-chain metadata |
| **DexScreener API** | No direct trade endpoint | Can't extract recent traders | Use pair-specific swap history queries |
| **Organ X tweets** | Few mentions of actual wallet addresses | Limited corpus from Twitter captures | Focused search on contest/airdrop tweets |

---

## Plan for May 2-3 (Real Corpus Collection)

### Phase 1: Data Source Preparation (May 2, morning)

1. **CultScreener Rugs** (sybil candidates)
   ```bash
   # Query rug tokens (conviction < 25)
   curl "https://cultscreener-api.onrender.com/api/tokens/leaderboard/conviction?conviction_max=25&limit=10"
   # Extract mint → query on-chain token authority
   ```

2. **DexScreener Traders** (human candidates)
   ```bash
   # For major tokens, fetch recent swap transactions
   # Parse for unique trader addresses with activity span > 7d
   ```

3. **Manual Curation** (1-2 hours)
   - Identify 5 documented rugs from public sources
   - Identify 5 known traders from Organ X / Twitter
   - Total: 10H + 10S corpus = 20 wallets

### Phase 2: Helius Collection (May 2, afternoon)

```bash
# Script: collect_real_corpus.py
# For each wallet:
#   1. call HeliusWalletCollector.collect_wallet_profile()
#   2. Retry on 429 with exponential backoff (1.5s → 3s → 6s)
#   3. Skip wallet if all retries fail
#   4. Save to validation_corpus_real_YYYY_MM_DD.json

# Expected: 15-18 of 20 succeed (rate limit handling)
```

### Phase 3: Validation & Reporting (May 2-3, evening)

```bash
# Run validator
python3 wallet_behavior_validator.py validation_corpus_real_YYYY_MM_DD.json

# Report:
#   - ROC-AUC (target > 0.7)
#   - Accuracy by class (humans vs sybils)
#   - Confusion matrix
#   - Gate threshold optimization
```

**Falsification:** If ROC-AUC < 0.7, identify which signal (age/diversity/temporal/anomalies) is weak and iterate.

---

## Integration Readiness (May 4)

### B&C Endpoint: `/mint-permit` (30 min)

**Current:**
```python
@app.post("/mint-permit")
async def mint_permit(body: MintPermitRequest):
    if body.game_result.verified:
        return {"mint_authorized": True}
    else:
        return {"mint_authorized": False}
```

**Updated:**
```python
from cynic_python.wallet_behavior_integration import mint_permit_response

@app.post("/mint-permit")
async def mint_permit(body: MintPermitRequest):
    return mint_permit_response(body.wallet, body.game_result.verified)
```

### Storage: NFT Metadata

Add traits:
```json
{
  "trait_type": "verified_by",
  "value": "game + wallet_behavior" // or "game_only"
}
{
  "trait_type": "authenticity_score",
  "value": "0.72"
}
```

### Testing

```bash
# Unit test (synthetic)
python3 -m pytest tests/test_wallet_integration.py

# Integration test (real corpus after May 2-3)
python3 -m pytest tests/test_wallet_corpus.py \
  --corpus validation_corpus_real_YYYY_MM_DD.json
```

---

## Costs

| Task | API | Credits | Cost |
|------|-----|---------|------|
| Collect 20 wallets (Helius) | 120 credits/wallet | 2,400 | $0.60 |
| Validation iterations (2x) | 50 credits/run | 100 | $0.02 |
| Integration testing | 10 credits/test | 100 | $0.02 |
| **Total** | — | ~2,600 | **~$0.65** |

(Helius Developer plan: $50/month = 1M credits)

---

## Timeline

| Date | Task | Owner | Status |
|------|------|-------|--------|
| Apr 30 | Design + heuristic validation | T. | ✓ DONE |
| May 1 | S. decision on co-submission (needed) | S. | ⏳ PENDING |
| May 2am | Real corpus curation | T. | → READY |
| May 2pm | Helius collection | T. | → READY |
| May 2-3 | Validation (ROC-AUC > 0.7) | T. | → READY |
| May 4 | B&C integration | S. | → READY |
| May 5-6 | Measure CYNIC impact | T. | Blocked |
| May 7-8 | CultScreener display integration | T. | Blocked |

---

## Files

| File | Purpose | Status |
|------|---------|--------|
| `wallet_behavior_scorer.py` | Core heuristic | ✓ Production |
| `wallet_behavior_helius.py` | Data collection | ✓ Production (rate limits documented) |
| `wallet_behavior_validator.py` | Falsification test | ✓ Production |
| `wallet_behavior_integration.py` | B&C integration | ✓ Production (no API key needed) |
| `real_wallet_corpus_builder.py` | Corpus extraction helpers | ✓ Ready (needs real data sources) |
| `validation_corpus_hybrid.json` | Test corpus (synthetic) | ✓ ROC-AUC 1.0 |
| `validation_corpus_real_YYYY_MM_DD.json` | Real corpus | → May 2 |

---

## Next Actions

1. **Today (Apr 30):** Push code, document integration contract for S.
2. **May 1:** Await S. decision on co-submission + corpus input
3. **May 2 morning:** Curate real addresses (rugs + traders)
4. **May 2 afternoon:** Collect profiles via Helius
5. **May 2-3 evening:** Validate, iterate if needed
6. **May 4:** Integrate into B&C

---

## Notes

- **Synthetic data validated:** Heuristic works correctly on engineered profiles
- **Real data delayed:** API friction (rate limits, account structure) → collect May 2-3 instead
- **Graceful degradation:** B&C mints with game_only if Helius unavailable
- **Falsifiable:** ROC-AUC > 0.7 is the gate; if not met, adjust weights and retry

**Epistemic status:** Heuristic correctness = HIGH (observed on synthetic). Real-world performance = CONJECTURE until May 3 (ρ~0.618 due to limited training data).
