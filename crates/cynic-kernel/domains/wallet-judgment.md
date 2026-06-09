# Wallet Judgment Domain — Anti-Sybil Evaluation Criteria

Evaluate the WALLET'S ON-CHAIN GAME HISTORY and CONSISTENCY — not the claims about who is playing. A coordinated Sybil attack with clean game logs is still Sybil. A legitimate player with noisy metrics is still legitimate if the archetype consistency is honest.

Judge the BEHAVIORAL SIGNATURE encoded in game signals and on-chain activity. Game history reveals WHETHER the wallet is playing genuinely, HOW CONSISTENTLY, and WHETHER patterns suggest coordination or deception. This is what you score.

## ASYMMETRIC CONFIDENCE

Negative assessments (BARKs) have higher confidence than positive assessments (HOWLs). Zero red flags ≠ proof of humanity. Game logs reveal consistency but CANNOT prove intent without off-chain context.

- Games completed < 5: max axiom score = 0.45 (insufficient sample, cannot HOWL yet)
- Games completed 5-20: max axiom score = 0.55 (building track record)
- Games completed > 20 with stable archetype: max axiom score = 0.618 (established pattern)

## FIDELITY

Does the wallet's game history match what an honest player would show? Is the wallet faithful to a single archetype or play style?

- HIGH (0.50-0.618): Archetype consistent across ≥80% of games. Game signals (time-to-decision, piece sacrifice patterns, openings) show coherent style. Creator wallet age > 30 days. No abrupt style switches. Example: Wallet that plays "The Aggressive" across 15 games with <0.15 variance on aggression_signal.
- MEDIUM (0.25-0.45): Archetype consistent 50-80% of games. Some variance within a style. Wallet age 5-30 days. Minor style drift that could indicate learning. Example: Wallet that oscillates between "The Aggressive" and "The Pragmatist" but mostly stays in one, showing consistent decision-making.
- LOW (0.05-0.20): Archetype inconsistent (<50% consistency). Abrupt style switches. Wallet age < 5 days. Game signals contradict claimed play style. Example: Wallet that plays "The Analytical" with deep calculations but then makes random moves in next game.

## PHI

Is the wallet's time and resource allocation proportional to genuine play? Is the game engagement harmonious or concentrated?

- HIGH (0.50-0.618): Games spread across ≥7 days. Time between games shows natural rhythm (not all clustered). Game duration consistent (±20% variance). No suspicious time patterns (3am spam grinding). Example: Wallet that plays 1-2 games per day over 2 weeks, each game 5-15 min, natural sleep/activity pattern.
- MEDIUM (0.25-0.45): Games spread across 3-7 days. Some clustering but explainable (weekend grinding). Game duration variance 20-50%. Minor time pattern anomalies. Example: Wallet that plays 5 games in one day then nothing for 2 days, then 3 more.
- LOW (0.05-0.20): Games clustered in <48 hours. Game duration wildly inconsistent (30s to 45min). Suspicious time pattern (all games at 3-5am UTC). Humans don't play like this. Example: Wallet that plays 20 games in 6 hours with highly variable times, suggesting bot farming.

## VERIFY

Can these game signals be independently verified on-chain or via game logs? Are the red flags or green flags trustworthy?

- HIGH (0.50-0.618): Game logs match Solana on-chain timestamps. Archetype scores reproducible (hash of game moves reproducible). Cross-source consistency (client logs = server logs). All claim timestamps verifiable. Example: Wallet where game move sequence matches server hash, timestamps match on-chain proof, signals fully auditable.
- MEDIUM (0.25-0.45): Most game properties verifiable. Some signals require client-side trust (self-reported game duration, client-side archetype score). Minor timestamp discrepancies (<5min). Example: Game logs mostly match server, but client reports slightly different end-time.
- LOW (0.05-0.20): Game logs don't match server timestamps. Archetype claims cannot be reproduced. Client and server data conflict. Timestamps suspiciously rounded or off. Example: Wallet claiming Analytical archetype but game moves show no deep calculation, timestamps are all exact minutes (xx:00).

## CULTURE

Does this wallet follow the norms of legitimate gameplay? Is the participation model consistent with ecosystem culture?

- HIGH (0.50-0.618): Wallet participates in multiple game types (blitz, rapid, classical). Uses standard UI (not API grinding). Engages with features naturally (opening encyclopedia, not copy-pasting moves). Plays against diverse opponents (not same wallet repeatedly). Example: Wallet that plays blitz and rapid, uses the web UI, searches for openings naturally, no suspicious opponent concentration.
- MEDIUM (0.25-0.45): Plays primarily one game type. Occasional API usage explainable (mobile fallback). Repeats some opponents but not excessively. Example: Wallet that plays mostly blitz, some API usage, plays same opponent 3-4 times total.
- LOW (0.05-0.20): API-only usage (never uses web UI). Plays exclusively against 1-2 opponent wallets (bot farm setup). Game type unnaturally narrow (only fastest games). Exploits edge-case rules. Example: Wallet that calls /play only via API, plays same two wallets 20 times each, always picks same archetype (unconstrained).

## BURN

Is the wallet efficiently structured for gaming? No unnecessary complexity, no dormant coin holdings, no sybil infrastructure?

- HIGH (0.50-0.618): Wallet has been active. Holds tokens naturally consistent with stake (not dormant whale). No obvious multi-wallet dependency (not clustering with other wallets). Clean transaction history (game moves, not suspicious transfers). Example: Wallet with ~100 games, some SOL, natural token holdings, no coordinated activity with related wallets.
- MEDIUM (0.25-0.45): Wallet has activity. Holds more tokens than necessary for staking but not extreme. Minor dormancy (gaps >5 days). Some connection to other wallets but explainable. Example: Wallet with 10 games, large token holdings, one gap for 2 weeks, but then active again naturally.
- LOW (0.05-0.20): Wallet is largely dormant (created, never touched for months, sudden activity). Holds massive tokens (whale behavior) but plays casual games (unusual). Directly connected to other wallets via transfers (Sybil network marker). Example: Wallet created 1 year ago, never used, suddenly plays 5 games, or wallet holding 1M tokens plays casually.

## SOVEREIGNTY

Can this wallet act independently, or does it show coordinated behavior with other wallets? Is the decision-making autonomous?

- HIGH (0.50-0.618): All games initiated independently. Move sequences show individual decision-making (not copy-pasted from other wallet's game). Opening repertoire unique to this wallet (not shared with N other wallets). No suspicious clustering with related wallets. Example: Wallet with unique opening preferences, individual move patterns, no coordinate transfers with other wallets.
- MEDIUM (0.25-0.45): Mostly independent games. Some opening overlap with ecosystem (normal, standard openings). Occasional transfer connections to other wallets (explainable — friends, exchanges). Example: Wallet that uses popular openings but applies them differently, has 1-2 transfer connections to other wallets.
- LOW (0.05-0.20): Move sequences appear copy-pasted or nearly identical to other wallet games (exact same opening, exact same decisions). Clustered transfers (A→B→C→D→E→A, circular). Shared infrastructure signals (same IP cluster, same user agent). Example: Wallet where 5 games match another wallet's move sequence step-for-step, or wallet that receives funds from 20 coordinated sources.

## PRIOR CALIBRATION

The base rate for new wallets in Option C is skepticism, not trust:
- 95%+ of Sybil attacks are discovered eventually. Default prior for new wallet: BARK until proven otherwise.
- Evidence that OVERRIDES the prior: ≥5 games with consistent archetype + stable time patterns + independent move sequences + age >30d = strong evidence of authenticity, sufficient to upgrade verdict.
- Exception awareness: genuine humans sometimes have inconsistent patterns (travel, context-switching) — the prior is strong but not absolute.

When signals conflict (e.g., consistent archetype but bot-like clustering), score the WEAKER signal. A single critical red flag (moves copy-pasted from another wallet, circular funding network) should suppress the overall assessment regardless of game count.

## INTEGRATION WITH TOKEN-JUDGMENT

Wallet judgment does NOT replace token judgment. The two are orthogonal:
- **Token judgment**: Is the token itself legitimate? (FIDELITY, PHI, VERIFY, CULTURE, BURN, SOVEREIGNTY on-chain)
- **Wallet judgment**: Is the wallet honest in gameplay? (FIDELITY, PHI, VERIFY, CULTURE, BURN, SOVEREIGNTY in behavior)

A wallet can be honest but hold a scam token. A scam wallet can hold legitimate tokens. Option C gates Personality Card mints with BOTH:
1. Wallet authenticity ≥ φ⁻¹ (from wallet-judgment Dogs)
2. Ed25519 signature verification (cryptographic authenticity)

Optional (post-May 11): integrate token holdings via CultScreener (what tokens does this wallet hold?) and HolDex K-Score (quality of the wallet's token portfolio) as enrichment signals, but do NOT gate on token quality — only wallet behavior + cryptographic verification.
