# Wallet Behavior Corpus Collection — Data-Centric Approach

**Philosophy:** CYNIC observes and validates itself. No external dependencies for validation data.

**Status:** Framework ready (`wallet_corpus_builder.py`). Need to populate with real addresses (May 2-3).

---

## Collection Strategy

### Verified Humans (10 wallets)

| Source | Count | How to Identify |
|--------|-------|-----------------|
| **Solana OGs** | 2 | Token Program, Marinade, Orca (public, multi-year activity) |
| **Governance** | 2 | Raydium, Serum, Magic Eden (DAOs, public participation) |
| **B&C Players** | 3 | Game API: filter games_completed ≥ 20, wallet_age_days ≥ 14 |
| **Ecosystem** | 2 | Long-term holders (Raydium/Orca LPs, >1 year stake) |
| **Self** | 1 | T.'s own wallet (known behavior, verifiable) |

**Identifier pattern:** Age > 30 days, tx_span > 14 days, token_count > 5, program_count > 3

---

### Sybils (10 wallets)

| Source | Count | How to Identify |
|--------|-------|-----------------|
| **Pump.fun Rugs** | 3 | CultScreener API: conviction = 0, token creator wallets |
| **MEV Bots** | 3 | Helius: 100+ txs/hour, atomic swap patterns, fresh age |
| **Exploit DB** | 2 | Solana security Discords/Twitter: documented sybil farms |
| **Synthetic** | 2 | Wallets we create: pump pattern in 2h window |

**Identifier pattern:** Age < 7 days OR single_token_pct > 90 OR all_txs_same_hour = true OR recent_whale_flag = true

---

## Collection Workflow

### Step 1: Identify Addresses (May 2, morning)

**Humans:**
```bash
# Known protocol addresses
addresses_humans = [
    "TokenkegQfeZyiNwAJsyFbPVwwQnmZKeyHeVUTX1159",  # Token Program
    "MarinadeMintLp6hTpf3yAKfQzLyp7UWnv3x2hZHG",   # Marinade
    "orcaEKTdK7LKz57chYcUdjik6bk5FqDNUucA2B6Hb8Q",  # Orca
]

# B&C game players (filter via game API)
# GET /players?games_completed=20&wallet_age_days=14
# Extract: player_wallet list

# Long-term LPs (from Raydium/Serum)
# Query: wallet with 100+ transactions, 2+ year age, multi-token holdings
```

**Sybils:**
```bash
# CultScreener API
# GET /api/tokens/leaderboard/conviction
# Filter: conviction = 0 (rugs)
# Extract: token creator wallets (first few are likely bots)

# Jito bundles (public)
# Search: "100 txs in 60 minutes" → MEV bot patterns

# Discord/Twitter archives
# Search: "sybil farm" OR "exploit account" → extract addresses
```

---

### Step 2: Fetch Profiles (May 2, afternoon)

```bash
# Load from secure location before running

python3 << 'EOF'
from wallet_corpus_builder import build_corpus
from wallet_behavior_helius import HeliusWalletCollector

# Update KNOWN_HUMANS and KNOWN_SYBILS with real addresses
collector = HeliusWalletCollector()
corpus_path = build_corpus(collector)
# → validation_corpus.json
EOF
```

**Expected cost:** ~2,400 credits (~$0.60)

---

### Step 3: Validate (May 3, morning)

```bash
python3 << 'EOF'
from wallet_behavior_validator import WalletValidator

profiles, labels = WalletValidator.load_corpus("validation_corpus.json")
result = WalletValidator.validate(profiles, labels)

print(result)
# ROC-AUC: ?
# ✓ PASS / ✗ FAIL (target > 0.7)
EOF
```

**If PASS:** Proceed to Phase 2 (CYNIC impact measurement)  
**If FAIL:** Debug signal separation, adjust weights, re-test

---

## Data Sources (Specific URLs & APIs)

### CultScreener (Pump.fun Rugs)
```
https://cultscreener-api.onrender.com/api/tokens/leaderboard/conviction
Endpoint: GET /api/tokens/leaderboard/conviction?limit=100&sort=conviction&order=asc
Filter: conviction = 0 (liquidated/rugged tokens)
Extract: mint, creator_wallet
```

### Helius API (Wallet Data)
```
All data fetched via:
- getBalance(address)
- getTokenBalances(address)
- getTransactionHistory(address, limit=200)
- getAccountInfo(address)

Cost per wallet: ~120 credits
```

### B&C Game API (Chess Players)
```
Assumed endpoint: GET /api/players?games_completed=min&wallet_age_days=min
Filter: games_completed >= 20, wallet_age_days >= 14
Extract: wallet_address, games_completed, elo_rating
```

### Solana Ecosystem Addresses (Public)
```
Token Program: TokenkegQfeZyiNwAJsyFbPVwwQnmZKeyHeVUTX1159 (on-chain)
Marinade: https://github.com/marinade-finance/liquid-staking-program (docs)
Orca: https://explorer.solana.com/ (search "Orca" or @orca_so)
Raydium: https://raydium.io/ (protocol addresses)
Serum: https://github.com/ProjectSerum/serum-dex
```

---

## Addressing Challenges

### Challenge 1: "10 humans is a small corpus"

**Response:** Falsification approach. Start with 10H + 10S. If ROC-AUC > 0.7, the heuristic generalizes (proven sufficient separation). If < 0.7, debug and revise. Either way, result is actionable.

**Plan B:** Expand to 20H + 20S (May 4-5) if May 3 result is borderline (0.65-0.70).

---

### Challenge 2: "How do we verify someone is truly human?"

**Epistemic status:** OBSERVED (not inferred)

- **Solana OGs** (Marinade, Orca): Public history, on-chain activity verified by 1000s
- **B&C players**: Game history proves human gameplay (cognitive load, reaction time, learning curve)
- **Governance**: Public DAO voting records, Twitter history cross-referenced
- **LPs**: Long-term stake > 1 year = human commitment, not bot pattern

**Sybils** (similar assurance):
- **Pump.fun rugs**: CultScreener explicitly identifies; liquidation is public
- **MEV bots**: 100+ txs/hour signature, documented in Jito/MEV research
- **Exploits**: Posted in security incident archives with proof-of-hack

---

### Challenge 3: "What if addresses are no longer active?"

**Mitigation:** Fetch at T=May 2. Activity span is what matters, not current state. Dormant wallets with 2+ year history still get high age/temporal scores.

---

## Quality Checklist

Before running validation, verify:

- [ ] HELIUS_API_KEY set and valid (test: `curl https://api.helius.xyz/v0?api-key=... -X POST`)
- [ ] 10+ human addresses collected (mix of protocols, players, governance)
- [ ] 10+ sybil addresses collected (mix of rugs, MEV, exploits, synthetic)
- [ ] wallet_corpus_builder.py populated with real addresses (not placeholders)
- [ ] corpus JSON file valid (run through `jq` parser)
- [ ] All wallets have required fields: address, is_human, age, token_count, etc.

---

## Timeline

| Date | Task | Owner | Status |
|------|------|-------|--------|
| May 1 | Decision gate (S.) | S. | Pending |
| May 2 AM | Identify 20 addresses | T. | Ready |
| May 2 PM | Fetch profiles (Helius) | T. | Ready |
| May 3 AM | Validate corpus (ROC-AUC) | T. | Ready |
| May 3 PM | Decision: pass/fail | T. | Pending |
| May 5-6 | Measure CYNIC impact | T. | Ready |
| May 7-8 | CultScreener integration | T. + S. | Ready |

---

## Independence

✓ No blocker on S. providing data  
✓ Can measure CYNIC impact regardless of B&C co-submit decision  
✓ Corpus collection is CYNIC's responsibility (data-centric)  
✓ Validation framework already built and tested

**Bottom line:** By May 3, we know if the heuristic works. Independent of May 1 decision gate.

