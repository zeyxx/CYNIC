# B&C Integration Guide — Wallet Behavior Scoring

**Goal:** Add `verified_human` gate to B&C `/mint-permit` endpoint  
**Status:** Ready for integration (scorer complete, awaiting S.'s decision on May 1)  
**Effort:** 2 hours (S.)

---

## Integration Steps

### 1. Install Helius SDK (5 min)

```bash
pip install requests
# API key loaded from environment
```

### 2. Copy Scorer Module (5 min)

Copy from CYNIC repo:
```bash
cp cynic-python/wallet_behavior_scorer.py b2c/utils/
cp cynic-python/wallet_behavior_helius.py b2c/utils/
```

### 3. Update `/mint-permit` Endpoint (30 min)

**Before (current):**
```python
@app.post("/mint-permit")
async def mint_permit(body: MintPermitRequest):
    wallet = body.wallet
    game_result = body.game_result
    
    # Current: game verification only
    if game_result.verified:
        return {"mint_authorized": True}
    else:
        return {"mint_authorized": False}
```

**After (with wallet behavior):**
```python
from utils.wallet_behavior_helius import HeliusWalletCollector

collector = HeliusWalletCollector()

@app.post("/mint-permit")
async def mint_permit(body: MintPermitRequest):
    wallet = body.wallet
    game_result = body.game_result
    
    # Collect wallet behavior
    profile = collector.collect_wallet_profile(wallet)
    
    # Determine authorization and verification method
    if not game_result.verified:
        return {
            "mint_authorized": False,
            "reason": "game_not_verified"
        }
    
    if profile and profile.is_verified_human:
        return {
            "mint_authorized": True,
            "verified_by": "game + wallet_behavior",
            "authenticity_score": profile.authenticity_score,
            "metadata": {
                "wallet_age_days": profile.wallet_age_days,
                "token_count": profile.token_count,
                "program_count": profile.program_count,
            }
        }
    else:
        return {
            "mint_authorized": True,  # Still mint, but note verification method
            "verified_by": "game_only",
            "authenticity_score": profile.authenticity_score if profile else None,
            "metadata": {
                "wallet_age_days": profile.wallet_age_days if profile else None,
                "token_count": profile.token_count if profile else None,
            }
        }
```

### 4. Store Verification Metadata (30 min)

Add to personality card NFT metadata:
```json
{
  "name": "Personality Card",
  "image": "...",
  "attributes": [
    {
      "trait_type": "verified_by",
      "value": "game + wallet_behavior"  // or "game_only"
    },
    {
      "trait_type": "authenticity_score",
      "value": "0.72"
    },
    {
      "trait_type": "game_completions",
      "value": "8"
    },
    {
      "trait_type": "wallet_age_days",
      "value": "45"
    }
  ]
}
```

### 5. Add Card Filtering (20 min)

Update leaderboard query:
```python
# GET /leaderboard?verified_humans=true
def get_leaderboard(verified_humans: bool = False):
    cards = fetch_all_cards()
    
    if verified_humans:
        # Filter to cards minted with "game + wallet_behavior" verification
        cards = [c for c in cards if c.metadata["verified_by"] == "game + wallet_behavior"]
    
    return sorted_by_elo(cards)
```

---

## API Contract

### Request
```json
{
  "wallet": "G2gCo4Hx2Pq8Rm9Nz3Kp0Lk5Jq7Mn2Vt",
  "game_result": {
    "verified": true,
    "games_completed": 8,
    "elo_rating": 1450,
    ...
  }
}
```

### Response (Success)
```json
{
  "mint_authorized": true,
  "verified_by": "game + wallet_behavior",
  "authenticity_score": 0.72,
  "metadata": {
    "wallet_age_days": 45,
    "token_count": 22,
    "program_count": 8
  }
}
```

### Response (Game Not Verified)
```json
{
  "mint_authorized": false,
  "reason": "game_not_verified"
}
```

---

## Error Handling

**If Helius API fails:**
```python
try:
    profile = collector.collect_wallet_profile(wallet)
except Exception as e:
    logger.error(f"Helius error for {wallet}: {e}")
    # Graceful degradation: mint without wallet behavior
    return {
        "mint_authorized": True,
        "verified_by": "game_only",
        "authenticity_score": None,
        "error": "wallet_behavior_unavailable"
    }
```

---

## Testing

### Unit Test
```python
from utils.wallet_behavior_scorer import WalletProfile, score_wallet

def test_mint_permit_human():
    response = client.post("/mint-permit", json={
        "wallet": "test_human_1",
        "game_result": {"verified": True, "games_completed": 8}
    })
    
    assert response["mint_authorized"] == True
    assert response["verified_by"] == "game + wallet_behavior"
    assert response["authenticity_score"] > 0.6
```

### Integration Test
```python
def test_mint_permit_corpus(corpus_file):
    """Test on validation corpus (10H + 10S)"""
    corpus = load_corpus(corpus_file)
    
    for wallet, is_human in corpus:
        response = client.post("/mint-permit", json={
            "wallet": wallet,
            "game_result": {"verified": True, "games_completed": 8}
        })
        
        if is_human:
            assert response["verified_by"] == "game + wallet_behavior"
        else:
            assert response["verified_by"] == "game_only"  # Bot still mints but noted
```

---

## Timeline

| Step | Duration | Owner |
|------|----------|-------|
| Install SDK | 5 min | S. |
| Copy modules | 5 min | S. |
| Update `/mint-permit` | 30 min | S. |
| Add metadata storage | 30 min | S. |
| Add leaderboard filter | 20 min | S. |
| Test on corpus | 30 min | S. + T. |
| **Total** | ~2 hours | — |

---

## Rollback Plan

If integration causes issues:
1. Revert to game-only verification
2. Keep authenticity_score in metadata (for future use)
3. No impact to existing minted cards

---

## Questions?

- **What if Helius is slow?** → Timeout at 30s, graceful degradation (game-only)
- **What if wallet has no transactions?** → Treated as age=1, score will be low
- **What if we change heuristic weights?** → Cards already minted keep their score; future cards use new weights
- **Can players dispute their score?** → Scores are deterministic and verifiable (open-source heuristic)

---

## Next Steps

1. **May 1 EOD:** S. provides validation corpus (10H + 10S wallets)
2. **May 2-3:** T. validates on real corpus (ROC-AUC > 0.7)
3. **May 4:** S. integrates into `/mint-permit`
4. **May 5-8:** Measure impact on CYNIC + integrate into CultScreener
5. **May 9-10:** Submit unified demo

