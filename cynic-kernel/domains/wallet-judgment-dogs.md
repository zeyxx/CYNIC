# Wallet Judgment — Dogs' Prompts & Scoring Rules

## Context

A wallet is requesting a Personality Card mint. CYNIC Dogs must validate: **Is this wallet authentic (≥5 games, confidence ≥ φ⁻¹ = 0.618)?**

Inputs (enriched stimulus):
- `wallet_address`: Ed25519 public key (Solana address)
- `games_completed`: Integer, count of completed games
- `archetype_sequence`: List of archetype verdicts (["The Aggressive", "The Pragmatist", "The Aggressive", ...])
- `game_timestamps`: ISO timestamps of each game completion
- `game_durations_seconds`: List of integers (how long each game took)
- `average_game_duration`: Float, mean duration in seconds
- `duration_variance`: Float, coefficient of variation (σ/μ)
- `opening_repertoire_hash`: Hash of opening sequences played (for uniqueness check)
- `archetype_consistency`: Float, % of games in modal archetype
- `wallet_age_days`: Days since first on-chain activity
- `token_holdings`: List of (mint, balance, category) for enrichment (post-May 11)
- `cultscreener_signals`: Holder distribution of tokens the wallet holds (optional enrichment)
- `holdex_k_score`: HolDex quality score of wallet's token portfolio (optional enrichment, post-May 11)
- `suspicious_cluster`: Boolean, wallet shows circular funding or shared-IP signals with other wallets
- `move_sequence_hash`: Hash of game move sequence (for copy-paste detection)
- `replay_risk`: Boolean, game moves match another wallet's game exactly

## Deterministic Dog (in-kernel heuristic)

**Purpose:** Fast, reproducible, zero-model verdict on wallet authenticity.

**Algorithm:**

```
INIT:
  confidence ← 0.0
  critical_fails ← []

GATE 1: Minimum games
  IF games_completed < 5:
    critical_fails.push("insufficient_samples")
    confidence ← 0.0
    RETURN BARK (insufficient evidence)

GATE 2: Critical Sybil markers
  IF suspicious_cluster == true OR replay_risk == true:
    critical_fails.push("sybil_coordination")
    confidence ← 0.05  # Very high confidence that it IS Sybil
    RETURN BARK (likely Sybil)

FIDELITY:
  # Archetype consistency
  IF archetype_consistency >= 0.80:
    fidelity ← 0.55
  ELSE IF archetype_consistency >= 0.50:
    fidelity ← 0.35
  ELSE:
    fidelity ← 0.15

PHI:
  # Time distribution harmony
  IF duration_variance <= 0.20:  # ±20% is natural
    time_harmony ← 0.55  # Natural rhythm
  ELSE IF duration_variance <= 0.50:
    time_harmony ← 0.35  # Some variance but OK
  ELSE:
    time_harmony ← 0.15  # Chaotic (bot-like)
  
  # Temporal spread
  IF games_completed >= 20 AND wallet_age_days >= 20:
    temporal_spread ← 0.55  # Established pattern
  ELSE IF games_completed >= 5 AND wallet_age_days >= 3:
    temporal_spread ← 0.35  # Building pattern
  ELSE:
    temporal_spread ← 0.15  # Rushed/clustered
  
  phi ← mean(time_harmony, temporal_spread)

VERIFY:
  # Timestamp consistency and verifiability
  # Assume all timestamps provided are on-chain verifiable (Solana proof-of-history)
  verify ← 0.55  # Default: on-chain timestamps are truth

CULTURE:
  # Gameplay patterns
  IF replay_risk == true:  # Moves copy-pasted
    culture ← 0.15
  ELSE IF (games_completed >= 5 AND wallet_age_days >= 5):
    culture ← 0.45  # Minimal history needed for cultural participation
  ELSE:
    culture ← 0.25

BURN:
  # Wallet efficiency and dormancy
  IF wallet_age_days < 5:
    burn ← 0.15  # New wallet, high Sybil risk
  ELSE IF wallet_age_days >= 30 AND games_completed >= 10:
    burn ← 0.55  # Established use
  ELSE:
    burn ← 0.35  # Intermediate

SOVEREIGNTY:
  # Decision autonomy
  IF replay_risk == true:
    sovereignty ← 0.05  # Copy-pasted = coordinated
  ELSE IF archetype_consistency >= 0.75:
    sovereignty ← 0.55  # Consistent = autonomous decisions
  ELSE:
    sovereignty ← 0.35  # Mixed = uncertain autonomy

AGGREGATE:
  axiom_scores ← [fidelity, phi, verify, culture, burn, sovereignty]
  q_score ← mean(axiom_scores)
  
  # Apply age ceiling (asymmetric confidence)
  IF wallet_age_days < 5:
    q_score ← min(q_score, 0.45)
  ELSE IF wallet_age_days < 30:
    q_score ← min(q_score, 0.55)
  # else: no ceiling, up to 0.618

VERDICT:
  IF q_score >= 0.618:
    RETURN HOWL (confidence=high, authentic)
  ELSE IF q_score >= 0.528:
    RETURN WAG (confidence=medium, probably authentic)
  ELSE IF q_score >= 0.382:
    RETURN GROWL (confidence=medium, probably Sybil)
  ELSE:
    RETURN BARK (confidence=high, likely Sybil)
```

**Rationale:**
- Minimum 5 games is non-negotiable (Option C requirement).
- Sybil markers (replay, circular funding) are terminal failures regardless of other signals.
- Archetype consistency is proxy for autonomous decision-making (not copy-pasted).
- Age ceiling reflects that new wallets have higher Sybil base rate (95%+).
- Time variance and temporal spread measure natural gameplay rhythm (bots cluster).

---

## LLM Dogs (qwen-7b, qwen35-9b, gemma-4, gemini)

**Purpose:** Nuanced evaluation incorporating game narrative context, move quality, and emergent patterns.

**Prompt Template:**

```
You are a Sybil Detector evaluating wallet authenticity for Option C Personality Card minting.

WALLET PROFILE:
- Address: {wallet_address}
- Games played: {games_completed}
- Age: {wallet_age_days} days
- Archetype sequence: {archetype_sequence}
- Archetype consistency: {archetype_consistency}%

GAMEPLAY SIGNALS:
- Average game duration: {average_game_duration}s (variance: {duration_variance})
- Opening repertoire uniqueness: {opening_repertoire_hash[:16]}...
- Move sequence uniqueness: {move_sequence_hash[:16]}...

RED FLAGS:
{IF suspicious_cluster}
- ⚠️ Clustering detected: wallet shows coordinated behavior with other wallets
{ENDIF}
{IF replay_risk}
- 🚨 CRITICAL: Game moves appear copy-pasted from another wallet
{ENDIF}
{IF wallet_age_days < 5}
- ⚠️ New wallet (< 5 days old): elevated Sybil base rate
{ENDIF}
{IF duration_variance > 0.50}
- ⚠️ Chaotic game duration (variance > 50%): suggests automation
{ENDIF}

EVALUATION:

Score the wallet on six axioms (range 0.05-0.618, φ⁻¹ is maximum earned confidence):

1. **FIDELITY** (Archetype Consistency): Does the wallet play the same style consistently? 
   - Score how faithfully this wallet commits to an archetype. Ignore single outlier games; look for the dominant pattern.
   
2. **PHI** (Gameplay Rhythm): Is the time distribution natural or bot-like?
   - Score how well the game timing and spacing suggests human play. Natural sleep patterns, no 3am farming, variance <50% is good.
   
3. **VERIFY** (Timestamp Authenticity): Can game logs be independently verified?
   - Score trustworthiness of on-chain timestamps. Assume Solana proof-of-history makes all timestamps verifiable: high confidence here.
   
4. **CULTURE** (Ecosystem Participation): Does the wallet play like a genuine participant?
   - Score if the wallet engages naturally (diverse opponents, multiple game types, uses public UI not just API). Copy-pasted moves = lowest score.
   
5. **BURN** (Wallet Efficiency): Is this wallet dormant or actively used?
   - Score based on wallet age relative to game count. Dormant whales = low. Consistent recent activity = high.
   
6. **SOVEREIGNTY** (Decision Autonomy): Can the wallet make independent decisions?
   - Score how much the wallet shows individual agency. Unique opening repertoire, varied move sequences, no obvious coordination = high.

ASYMMETRIC CONFIDENCE RULE:
- If you score this wallet as BARK (likely Sybil): confidence in BARK is high (>0.6). Absence of red flags does not prove authenticity.
- If you score this wallet as HOWL (likely authentic): max confidence is 0.618. You CANNOT prove humanity, only absence of Sybil markers.

OPTION C GATE (mandatory reading):
- Minimum games: {games_completed} / 5 required
- Confidence floor: φ⁻¹ = 0.618 required
- **Final verdict**: Does this wallet qualify for Personality Card mint?
  - PASS if: games_completed ≥ 5 AND mean(axiom_scores) ≥ 0.618
  - FAIL if: games_completed < 5 OR mean(axiom_scores) < 0.618

OUTPUT FORMAT:

AXIOM SCORES:
- FIDELITY: [score]
- PHI: [score]
- VERIFY: [score]
- CULTURE: [score]
- BURN: [score]
- SOVEREIGNTY: [score]

CONFIDENCE: [mean of axiom scores, 0-1]

VERDICT: [HOWL | WAG | GROWL | BARK]

REASONING: [2-3 sentences explaining the verdict. Focus on load-bearing signals: age, game count, consistency, any red flags. Ignore minor noise.]

OPTION C GATE: [PASS | FAIL]
```

**Dog Specifics:**

- **qwen-7b-hf**: Fast, good at pattern recognition. Excels at detecting copy-paste move sequences and temporal anomalies. Watch for over-confidence on limited data.
- **qwen35-9b-gpu**: Nuanced reasoning, handles conflicting signals well (e.g., low game count but perfect consistency). Best for edge cases.
- **gemma-4-e4b**: Calibrated conservatism. Naturally skeptical of new wallets. Good baseline for asymmetric confidence (BARK > HOWL).
- **gemini-cli**: Cross-domain pattern matching. Can spot subtle Sybil infrastructure (e.g., wallet clustering without explicit signal).

---

## Integration with /mint-permit Gate

**Option C Flow:**

```
POST /mint-permit
├─ Input: { wallet, nonce, archetype, ed25519_sig }
├─ Verify Ed25519 sig over (nonce, wallet, archetype)
├─ IF sig invalid:
│  └─ RETURN 401 Unauthorized
├─ Lookup games_completed for wallet
├─ IF games_completed < 5:
│  └─ RETURN 400 Bad Request (insufficient games)
├─ Construct enriched_context (via wallet-judgment domain)
├─ Call /judge with stimulus:
│  ├─ content: "Wallet authenticity check"
│  ├─ context: [enriched game history + signals]
│  ├─ domain: "wallet-judgment"
│  └─ inject_crystals: true  # Use any CCM crystals on wallet behavior
├─ Run all 5 Dogs (deterministic + qwen-7b + qwen35-9b + gemma-4 + gemini)
├─ Compute verdict:
│  ├─ mean_q_score = mean of all Dogs' q_scores
│  └─ IF mean_q_score >= 0.618:
│     ├─ Rate-limit: 1 mint per wallet per hour
│     ├─ Upload Arweave metadata (Personality Card JSON)
│     └─ RETURN 200 OK + { mint_permit, arweave_uri }
│     ELSE:
│     └─ RETURN 403 Forbidden (wallet fails authenticity gate)
```

**Metadata Enrichment (post-May 11, optional):**
- Integrate CultScreener: `curl CultScreener/analyze?tokens=[wallet_holdings]` → enriched token context
- Integrate HolDex: `curl HolDex/k-score?wallet=X` → quality signal (do not gate on it, only enrich context)

---

## Falsification Test (future work)

Once CCM crystal volume on wallet behavior exists:

1. Create stimulus: "Wallet Y at age 30 days with 8 games (archetype consistent 85%, time variance 0.22)"
2. Run without crystal injection (baseline)
3. Run with crystal injection (if any "honest wallet" crystals exist)
4. Measure delta. If delta ≥ φ⁻⁴ (0.146) or verdict_kind changes → K15 is acting.
