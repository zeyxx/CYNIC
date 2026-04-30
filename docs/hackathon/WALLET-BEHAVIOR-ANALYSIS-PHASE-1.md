# Phase 1: Wallet Behavior Analysis — Human vs Bot Detection

**Status:** Design (pre-implementation)  
**Deadline:** May 2-4 (implementation + validation)  
**Falsification Criterion:** ROC-AUC > 0.7 on validation corpus (10 human + 10 Sybil wallets)  
**Output:** `wallet_behavior_scorer.py` module + integration into B&C `/mint-permit`

---

## Problem Statement

B&C's Personality Card (chess proof-of-play) verifies humans through gameplay. But **wallet holder analysis** for CYNIC's token judgment requires distinguishing humans from bots **without game data**—purely from on-chain behavior.

**Use case:** Filter CYNIC token verdicts by "% verified humans in holder set" → reweight conviction scores → demonstrate compound proof (game verification + wallet authenticity).

---

## Design: Four Tiers of Signals

### Tier 1: Wallet Age & Tenure (Foundation)

**Signal:** How long has this wallet existed and actively used?

| Age | Signal | Confidence | Rationale |
|-----|--------|-----------|-----------|
| < 1 week | Fresh | Low | Bot farms spin up wallets daily |
| 1-4 weeks | Established | Medium | Humans create accounts and persist |
| > 4 weeks | Proven | High | Long-term commitment increases authenticity |

**Metric:** `wallet_age_days` (from `/getTokenBalances` or `/getAccount` first_activity_block)

**Scoring:**
```
age_score = {
  < 7: 0.15
  7-28: 0.40
  > 28: 0.65
}
```

---

### Tier 2: Transaction Diversity (Activity Pattern)

**Signal:** Do interactions span multiple projects/tokens, or are they clustered on one?

Bot signature: Heavy concentration on single token/program (pump & dump, artificial volume).
Human signature: Varied interactions (swaps, transfers, staking, multiple tokens).

**Metrics** (via Helius):
- `token_count` — # unique token mints interacted with (via getTokenBalances)
- `program_count` — # unique programs called (via getTransactionHistory)
- `unique_swap_pairs` — # distinct token pairs swapped (parse /judge txs)

**Scoring:**
```
diversity_score = {
  token_count < 3: 0.15        (appears pump-focused)
  token_count 3-10: 0.40       (moderate spread)
  token_count > 10: 0.65       (healthy exploration)
  
  Multiply by program_count_factor:
    < 2 programs: 0.7
    2-5 programs: 0.9
    > 5 programs: 1.0
}
```

Combined: `diversity = token_score × program_factor`

---

### Tier 3: Temporal Spread (Consistency)

**Signal:** Are transactions clustered in time (bot bursts) or smoothly distributed (human)?

Human signature: Regular activity over weeks (20-30 day spread, roughly daily)  
Bot signature: Concentrated within hours or days (pump window)

**Metrics:**
- `activity_span_days` — days between first and last transaction
- `transaction_density` — txs per day in that span
- `gap_max_days` — longest gap between consecutive transactions

**Scoring:**
```
temporal_score = {
  span < 3 days: 0.15           (concentrated bot window)
  span 3-14 days: 0.35          (emerging)
  span > 14 days: 0.65          (sustained)
}

Adjust by gap_max_days:
  max_gap > 7 days: ×0.7        (dormant periods suggest inactive bot)
  max_gap 2-7 days: ×0.9        (normal sleeping)
  max_gap < 2 days: ×1.0        (active engagement)
```

---

### Tier 4: Behavioral Anomalies (Red Flags)

**Signal:** Markers of coordinated attack or unnatural patterns.

**Critical Failures** (instant BARK):
- `first_activity_recent && massive_gas_burn` → recent whale (possible MEV bot)
- `all_txs_same_time_window` → coordinated pump
- `single_token_concentration > 95%` → suspicious focus
- `transaction_frequency anomaly` — 100+ txs in <1 hour (bot flooding)

**Scoring:**
```
If ANY critical failure: return 0.10 (bot)
Otherwise: proceed to composite
```

---

## Composite Score (Human Authenticity)

```python
def score_wallet(wallet_profile: WalletProfile) -> float:
    """
    Returns: 0.0-1.0 confidence in human authenticity
    φ⁻¹ = 0.618 is the gate for "verified human"
    """
    
    # Check critical failures first
    if profile.has_critical_failure():
        return 0.10
    
    # Four-tier composite
    w_age = 0.25
    w_diversity = 0.30
    w_temporal = 0.25
    w_anomaly = 0.20
    
    score = (
        age_score(profile) * w_age +
        diversity_score(profile) * w_diversity +
        temporal_score(profile) * w_temporal +
        anomaly_penalty(profile) * w_anomaly
    )
    
    # Clamp to [0.05, 0.95] to avoid false certainty
    return max(0.05, min(0.95, score))

# Gate: is_verified_human = score >= φ⁻¹ = 0.618
```

---

## Data Sources (Helius API)

| What | Helius Method | Cost | Data |
|-----|--------|------|------|
| Wallet age, SOL balance | `getBalance` | 1 cr | `lamports`, account metadata |
| Token holdings | `getTokenBalances` | 10 cr | `mint`, `amount`, implicit token_count |
| Transaction history | `getTransactionHistory` (parsed mode) | ~110 cr | timestamp, instruction type, accounts touched |
| Account info | `getAccountInfo` | 1 cr | owner program, executable flag |
| Program accounts | `getProgramAccounts` | 10 cr | (fallback: list all accounts created by a program) |

**Cost per wallet:** ~130 credits (1 balance + 10 token + 110 history + 1 account + 8 misc)  
**Cost for validation corpus:** 10 humans × 130 + 10 sybils × 130 = 2,600 credits (~$0.65)

---

## WalletProfile Dataclass

```python
@dataclass
class WalletProfile:
    """On-chain wallet behavior extracted from Helius."""
    wallet_address: str
    
    # Tier 1: Age
    wallet_age_days: int
    first_activity_block: int
    
    # Tier 2: Diversity
    token_count: int           # unique mints
    program_count: int         # unique programs
    unique_swap_pairs: int     # distinct [token_a, token_b] pairs
    sol_balance: float
    
    # Tier 3: Temporal
    activity_span_days: int    # first to last tx
    total_transactions: int
    transaction_density: float  # txs per day
    gap_max_days: int          # longest gap between consecutive txs
    
    # Tier 4: Anomalies
    all_txs_same_hour: bool    # all within 1 hour
    single_token_pct: float    # % of interactions on single token
    recent_whale_flag: bool    # age < 1 day AND balance > threshold
    transaction_frequency_anomaly: bool  # > 100 txs in 1 hour
    
    # Composite
    authenticity_score: float  # 0.0-1.0
    is_verified_human: bool    # score >= 0.618
```

---

## Falsification Tests

### Test 1: Corpus Separation (ROC-AUC)

**Data:** 10 verified humans (from B&C game history) + 10 Sybil wallets (known pump.fun bots, MEV bots, etc.)

**Metric:** ROC-AUC when scoring all 20 wallets

**Falsification Criterion:** ROC-AUC > 0.7 (0.5 = random, 1.0 = perfect separation)

**How to get Sybil corpus:**
- Pump.fun liquidations (public rugs, identifiable from CultScreener)
- MEV bot addresses (Jito bundles, observable via Helius transaction patterns)
- Known coordinated attacks (e.g., moonshot sybil farms from Discord leaks)
- Synthetic: create fresh wallet, 50 transactions in 2 hours to single token → instant Sybil profile

---

### Test 2: Signal Stability (Re-Score Variance)

**Method:** Score same 10 wallets at T₁ and T₂ (1 day apart)

**Falsification Criterion:** Pearson r > 0.85 (scores should be stable over 24h)

**Expected:** Tier 1 (age) and Tier 3 (temporal) drift slowly; Tier 2 (diversity) can vary if new txs appear.

---

### Test 3: Domain Transfer (Token vs Wallet)

**Method:** Apply same heuristic framework to token holder validation ("is this token's holder set human-dominated?")

**Design:** 
- For each token, sample top-50 holders
- Score each holder as human/bot
- Compute % verified humans in top-50
- Correlation with CYNIC token verdict: verified_human_pct vs q_score

**Falsification Criterion:** Positive correlation (r > 0.3) — tokens with more human holders → higher CYNIC confidence

---

## Integration Points

### Integration A: B&C `/mint-permit` Gate

```rust
// pseudocode
POST /mint-permit {
  wallet: "...",
  game_result: {...}
}

// Before minting:
wallet_profile = fetch_from_helius(wallet)
authenticity_score = score_wallet(wallet_profile)

if authenticity_score >= 0.618 && game_result.verified {
    mint_card(wallet)      // Verified human + verified game
} else if game_result.verified {
    mint_card(wallet)      // Game verified only
    // metadata.verified_by = "game_only"
} else {
    reject()               // Neither game nor wallet verified
}
```

**Output:** Card metadata includes `verified_human: bool` + `authenticity_score: float`

---

### Integration B: CYNIC Token Reweighting

```python
# After scoring a token with Dogs:

# Sample top-100 holders
holders = helius.getTokenLargestAccounts(mint, limit=100)

# Score each holder
verified_count = sum(
    score_wallet(fetch_profile(h)) >= 0.618
    for h in holders
)

verified_human_pct = verified_count / len(holders)

# Adjust token verdict confidence
if verified_human_pct > 0.70:
    q_score_adjusted = q_score * 1.15  # boost confidence
elif verified_human_pct < 0.30:
    q_score_adjusted = q_score * 0.85  # reduce confidence
else:
    q_score_adjusted = q_score          # neutral

# Display in CultScreener
return {
    "conviction": conviction_score,
    "verified_humans_pct": verified_human_pct,
    "verified_wallets_pct": verified_wallets_pct,  # from B&C
    "cynic_q_score": q_score_adjusted
}
```

---

## Implementation Roadmap

| Phase | Task | Owner | Deadline |
|-------|------|-------|----------|
| 1a | Design heuristics (THIS DOC) | T. | Apr 30 ✓ |
| 1b | Implement `wallet_behavior_scorer.py` | T. | May 2 |
| 1c | Collect validation corpus (10H + 10S) | S. | May 2 |
| 1d | Falsification Test 1 (ROC-AUC > 0.7) | T. | May 3 |
| 1e | Integrate into B&C `/mint-permit` | S. + T. | May 4 |
| 2a | Measure impact on CYNIC Dogs | T. | May 5-6 |
| 2b | Integrate into CultScreener display | S. + T. | May 7-8 |
| 3 | Record unified demo | T. | May 9-10 |

---

## Open Questions

1. **What defines a "Sybil" wallet for test purposes?**
   - Suggested: Recent age + single-token concentration + coordinated timing
   - Alternative: Provide known-bad wallets from CultScreener leaderboard

2. **Should temporal signals account for crypto market hours?**
   - Conjecture: Weekend/night activity is human-like; 3am GMT pump windows are bots
   - Falsifiable: Compare actual human (T.'s wallet) vs known bot addresses

3. **How to handle "sleeping" wallets?**
   - Old wallet (age=180 days) but zero activity in last 60 days
   - Proposal: Decay score by (1 - activity_recency_score) after 30 days dormancy
   - Falsify: Test on abandoned wallets vs newly-resumed accounts

4. **Should we penalize high-frequency traders?**
   - Observation: Some humans day-trade (100+ txs/week)
   - Conjecture: Transaction frequency alone is not a signal; diversity matters more
   - Falsifiable: Compare score of HFT humans vs Sybil farms on same corpus

---

## Cost & Timeline Summary

| Item | Cost | Time |
|------|------|------|
| Validate on 20 wallets | 2,600 credits | 1 hour (API calls) |
| Implement scorer module | — | 4 hours |
| Integrate into B&C | — | 2 hours (S.) |
| Measure CYNIC impact (20-30 tokens) | ~3,000 credits | 3 hours |
| Record demo | — | 1 hour |
| **Total** | **~5,600 credits** | **~11 hours** |

**Credits:** ~$1.40 (Helius Developer plan: 1M credits/month for $50)

---

## Epistemic Status

- **Observed:** B&C game verification works; Helius data available; pump.fun wallets identifiable (confirmed via CultScreener API)
- **Deduced:** Transaction diversity + temporal spread correlate with human behavior (from Titouan's observation data: 93 WPM, burst scrolls, deliberation patterns)
- **Inferred:** ROC-AUC > 0.7 achievable with four-tier heuristic (pattern from token-domain Dogs: achieved 0.95 on Rug prediction)
- **Conjecture:** Verified human filter will boost CYNIC confidence on legitimate tokens by 15%+ (hypothesis; falsifiable via A/B on 20 tokens)

**Falsifiable:** All criteria in Test 1-3 sections above.

---

## Next Step

**Decision Gate: May 1 EOD**

- S. confirms: (1) willing to co-own narrative? (2) integrate wallet behavior score? (3) validation corpus ready (10H + 10S)?
- If YES → start Phase 1b implementation May 2
- If NO → skip wallet behavior, go separate submissions path

