# Wallet Behavior Analysis — Quick Reference

**Goal:** Score wallets 0.0-1.0 (human authenticity). Gate at φ⁻¹ = 0.618.

---

## Four-Tier Heuristic (25% + 30% + 25% + 20%)

### 1. Age (25%)
| Wallet Age | Score |
|------------|-------|
| < 7 days   | 0.15  |
| 7-28 days  | 0.40  |
| > 28 days  | 0.65  |

**Why:** Bots farm wallets daily; humans create accounts and persist weeks.

---

### 2. Diversity (30%)
| Token Count | Program Count Factor | Combined |
|-------------|---------------------|----------|
| < 3 tokens  | (varies)             | 0.15     |
| 3-10 tokens | (varies)             | 0.40     |
| > 10 tokens | (varies)             | 0.65     |

**Why:** Bots concentrate on single token (pump & dump). Humans explore multiple projects.

---

### 3. Temporal Spread (25%)
| Activity Span | Gap Max | Score |
|---------------|---------|-------|
| < 3 days      | —       | 0.15  |
| 3-14 days     | > 7d    | 0.25  |
| 3-14 days     | < 7d    | 0.35  |
| > 14 days     | > 7d    | 0.45  |
| > 14 days     | < 7d    | 0.65  |

**Why:** Bots cluster in hours (pump windows). Humans spread over weeks (regular activity).

---

### 4. Anomalies (Red Flags)
| Marker | Score | Reason |
|--------|-------|--------|
| All txs in 1 hour | 0.10 | Coordinated pump |
| Single token > 95% | 0.10 | Pump & dump focus |
| 100+ txs in 1 hour | 0.10 | Bot flooding |
| Recent whale (age < 1d, balance > 100 SOL) | 0.10 | MEV bot |

**Rule:** If ANY critical failure → return 0.10 (bot) immediately.

---

## Composite Score

```
score = (
    age_score × 0.25 +
    diversity_score × 0.30 +
    temporal_score × 0.25 +
    (1.0 - anomaly_penalty) × 0.20
)
```

**Gate:** `is_verified_human = score >= 0.618`

**Confidence bounds:** [0.05, 0.95] to avoid false certainty.

---

## Example Wallets

### Human Profile (Authentic)
- Age: 45 days
- Tokens: 22 unique
- Programs: 8 unique
- Span: 40 days, max gap 3 days
- No red flags
- **Score: 0.68 → VERIFIED HUMAN** ✓

### Sybil Profile (Pump Bot)
- Age: 3 days
- Tokens: 1 (single concentration 98%)
- Programs: 2
- Span: 2 hours (all txs within 1 hour window)
- Red flag: single_token_pct > 95%
- **Score: 0.10 → BOT** ✗

### Ambiguous Profile (Emerging)
- Age: 8 days
- Tokens: 4
- Programs: 3
- Span: 7 days, max gap 5 days
- No red flags
- **Score: 0.38 → NOT VERIFIED** (below gate)

---

## Data Collection (Helius API)

| Signal | Method | Cost |
|--------|--------|------|
| wallet_age_days | getAccountInfo | 1 cr |
| token_count, token list | getTokenBalances | 10 cr |
| program_count, tx details | getTransactionHistory | ~110 cr |
| anomalies (detect) | Parse history | 0 cr |

**Total per wallet:** ~120 credits (~$0.03)

---

## Validation (May 2-3)

**Corpus:** 10 verified humans (from B&C game history) + 10 known Sybils (pump.fun bots, MEV addresses)

**Falsification Test:** ROC-AUC > 0.7 when scoring all 20

**Success:** Clearly separates humans (high scores) from bots (low scores)

---

## Integration Points

### B&C Personality Card
```
if score >= 0.618 && game_verified:
    mint_card(wallet, "verified_human + verified_game")
elif game_verified:
    mint_card(wallet, "verified_game_only")
```

### CYNIC Token Judgment
```
top_100_holders = fetch_holders(token_mint)
verified_human_pct = count_verified(top_100_holders) / 100

if verified_human_pct > 0.70:
    boost CYNIC confidence by 15%
elif verified_human_pct < 0.30:
    reduce CYNIC confidence by 15%
```

### CultScreener Display
```json
{
  "conviction": 68,
  "verified_humans_pct": 74,
  "verified_wallets_pct": 62,
  "cynic_q_score": 0.72
}
```

---

## Timeline

| Date | Task | Owner | Status |
|------|------|-------|--------|
| May 1 | S. confirms co-submit + provides corpus | S. | **PENDING** |
| May 2 | Implement scorer module | T. | Blocked on May 1 |
| May 3 | Run ROC-AUC test (20 wallets) | T. | Blocked on May 1 |
| May 4 | Integrate into B&C `/mint-permit` | S. + T. | Blocked on May 1 |

**Decision Gate: May 1 EOD** → S. answers (1) co-own narrative? (2) integrate wallet score? (3) corpus ready?

