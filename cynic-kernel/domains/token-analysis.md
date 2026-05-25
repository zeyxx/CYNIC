# Token Analysis Domain — Axiom Evaluation Criteria

Evaluate the TOKEN'S ON-CHAIN REALITY — not the quality of its description. A scam described with technical accuracy is still a scam. A legitimate token with a garbage name is still legitimate if the on-chain structure is sound.

Judge the POWER STRUCTURE encoded in the metrics. On-chain data reveals WHO has power, HOW MUCH power, and WHETHER they can use it. This is what you score.

## ASYMMETRIC CONFIDENCE

Negative assessments (BARKs) have higher confidence than positive assessments (HOWLs). Absence of red flags ≠ presence of legitimacy. On-chain data reveals power concentration but CANNOT prove intent.

- Token age <30 days: max axiom score = 0.45 (untested, cannot HOWL yet)
- Token age 30-365 days: max axiom score = 0.55 (building track record)
- Token age >365 days with growing holders: max axiom score = 0.618 (established)

## TOKEN CLASS AWARENESS

Not all tokens are growth tokens. Authority interpretation depends on the token's role in the ecosystem:

- **Infrastructure/Stablecoin** (e.g. USDC, USDT, wSOL): Active mint and freeze authorities are EXPECTED. Circle MUST control mint (pegging) and freeze (compliance). Score stability and reliability, not growth. DO NOT penalize active authorities. Signals: market_cap >$500M, age >365d, active mint+freeze, holders >100K.
- **Governance/Utility** (e.g. JUP, RAY, JTO): May retain active mint for scheduled emissions. Evaluate whether the authority has a documented purpose. Reduced penalty if project is established (age >180d, multiple exchange listings).
- **Growth/Meme** (e.g. BONK, WIF): Standard scoring — revoked authorities = trust signal.
- **New/Unclassified** (pump.fun, <30d): 98.6% prior applies. Maximum skepticism.

When scoring, FIRST identify the token class from the metrics, THEN apply the appropriate interpretation. A stablecoin with active freeze authority is functioning correctly — penalizing it would be a false signal.

## FIDELITY
Does the on-chain state match what a legitimate project would show? Is the token faithful to its claimed purpose? Consider the token class — a stablecoin with active mint is faithful to its purpose (pegging), not dishonest.
- HIGH (0.50-0.618): On-chain state matches the token's role. For growth tokens: authorities revoked, metadata immutable. For stablecoins: active authorities with massive holder base and proven track record. For governance: documented emission schedule. Example: JUP — governance token with DAO treasury; USDC — stablecoin with active mint (Circle must mint/burn for peg).
- MEDIUM (0.25-0.45): Some authorities retained with plausible reason given the token class. Metadata mutable but consistent over time. Example: New DeFi token with active mint authority (for inflation schedule) but transparent vesting and clear documentation.
- LOW (0.05-0.20): Claims contradict on-chain state regardless of class. "Community token" with one wallet holding 90%. "Deflationary" with active mint authority and no burn mechanism. Metadata recently changed. Example: pump.fun token claiming "SafeMoon 2.0" with active freeze authority and 3 holders.

## PHI
Is the holder distribution proportional? Is the tokenomics structure harmonious or concentrated?
- HIGH (0.50-0.618): HHI < 0.15 (excluding LP/burn addresses). Top-1 wallet < 10% of circulating supply. Distribution follows organic growth pattern. Example: BONK — community airdrop to millions, extremely distributed, no single wallet dominates.
- MEDIUM (0.25-0.45): HHI 0.15-0.40. Top-1 wallet 10-30%. Distribution shows some concentration but not extreme. Example: Mid-cap token where top holder is a known treasury/staking contract.
- LOW (0.05-0.20): HHI > 0.40. Top-1 wallet > 50%. Distribution shows clear whale dominance. Few real holders. Example: Token where 3 wallets hold 95% of supply, most "holders" have dust amounts (0.000001).

## VERIFY
Can these metrics be independently verified on-chain? Are there verifiable red flags or green flags?
- HIGH (0.50-0.618): All token properties verifiable on-chain. Cross-source consistency (Helius, DexScreener, RugCheck agree). Verified contract source. No discrepancies between data sources. Example: Standard SPL token with immutable metadata, all state readable from Solana explorer.
- MEDIUM (0.25-0.45): Most properties verifiable. Some claims require off-chain trust (team identity, roadmap). Minor cross-source disagreements. Example: Token with verifiable on-chain state but unverifiable team claims.
- LOW (0.05-0.20): Key claims cannot be verified. Metadata URI broken or pointing to mutable content. Cross-source disagreements on fundamental metrics. Example: Token where metadata image has changed, holder data is inconsistent across explorers, or supply doesn't match stated tokenomics.

## CULTURE
Does this token follow established Solana token standards? Is the authority model consistent with good practices?
- HIGH (0.50-0.618): Standard SPL token, proper metadata (name, symbol, image), accepted launchpad (Raydium, Jupiter LFG, pump.fun graduated). Follows established patterns for its category. No impersonation. Example: Raydium-listed token following all standard practices — proper decimals, metadata, verified creator.
- MEDIUM (0.25-0.45): Minor deviations from standard (unusual decimals, Token-2022 extensions with plausible reason). Legitimate pump.fun origin that graduated to Raydium. Example: Token with non-standard decimals but clear technical reason.
- LOW (0.05-0.20): Impersonates established token (copycat name/symbol). Violates ecosystem norms. Non-standard authority patterns without justification. Example: Token named "JUPITOR" or "B0NK" — name similarity to established tokens is a manipulation signal.

## BURN
Is the token efficiently structured? Burned supply, minimal waste, no unnecessary authorities retained?
- HIGH (0.50-0.618): LP burned (permanent, irreversible). Authorities revoked (no unnecessary power retained). Clean token design — standard SPL, no bloat. If supply is burned, it's verifiable. Example: Token with burned LP, revoked mint/freeze, and verifiable burn address.
- MEDIUM (0.25-0.45): LP locked (temporary — depends on lock duration). Some authorities retained with stated reason. Token structure is clean but with unnecessary extensions. Example: Token with 6-month LP lock, active mint authority for documented inflation schedule.
- LOW (0.05-0.20): LP unsecured (creator can pull liquidity at any time). Unnecessary authorities retained. Wasteful structure. No skin in the game — creator has sacrificed nothing irreversible. Example: pump.fun token with LP in creator wallet, active mint AND freeze authority. Zero irreversible commitments.

## SOVEREIGNTY
Is control distributed or concentrated? Can individual holders act freely without one wallet dominating? For infrastructure tokens (stablecoins), centralized control is the design — score whether that control is exercised by a known, regulated entity (high sovereignty through accountability) vs an anonymous actor (low sovereignty).
- HIGH (0.50-0.618): For growth tokens: freeze_authority revoked, distributed holders, functions independently. For stablecoins: freeze authority held by regulated entity (Circle, Tether), massive holder base, exchange-listed — sovereignty through regulatory accountability, not through absence of control. Example: USDC — Circle can freeze but is regulated, audited, and accountable; holders trust the institution.
- MEDIUM (0.25-0.45): freeze_authority revoked but moderate concentration. Some admin functions governed by multisig or DAO. Example: Token with governance multisig, revoked freeze, but significant treasury allocation.
- LOW (0.05-0.20): freeze_authority ACTIVE on a growth/meme token (anonymous actor can freeze any wallet). Single wallet controls >50%. Token entirely dependent on creator's continued good behavior. Example: pump.fun token where anonymous creator can freeze any holder's wallet.

## EMPIRICAL SIGNAL CALIBRATION (from n=30 token correlation study, 2026-05)

The following signals have measured correlations with token survival outcomes. Use these to weight your judgment — signals near the top of this list are more predictive than those at the bottom.

| Signal | rho | Direction | Interpretation |
|--------|-----|-----------|----------------|
| supply_burned_pct | +0.672 | positive | Strongest structural commitment signal |
| longevity (K-Score) | +0.632 | positive | Age-adjusted survival — primary positive predictor |
| accumulator_ratio | -0.622 | **negative** | Many accumulators = FOMO buying, NOT healthy growth |
| diamond_hands | -0.396 | **negative** | High DH = bag-holding, NOT genuine conviction |
| k_score (composite) | -0.327 | **negative** | Composite destroys signal — use sub-components instead |
| organic_growth | ~+0.3 | positive | Distribution quality — moderate positive signal |
| market_cap | -0.083 | noise | Not predictive — ignore for scoring |
| liquidity | +0.038 | noise | Not predictive — wash trading makes this unreliable |

**Key inversions** (counterintuitive but empirically measured):
- **diamond_hands**: High values mean FOMO buyers accumulating, not holders with conviction. Low diamond_hands = passive holders who stayed = actual fidelity. Treat high diamond_hands as a WARNING, not a positive signal.
- **accumulator dominance**: When accumulators >> extractors, insiders or bots are buying aggressively. This correlates with WORSE outcomes. Extractor dominance (mature holders who took some profit but core stayed) is actually healthier.
- **k_score composite**: The composite DH * OG * Longevity mixes inverted signals, destroying information. Score longevity and organic_growth INDIVIDUALLY instead.

## BUY/SELL DIVERGENCE

Holder behavior reveals what insiders know. When buy/sell ratios diverge from conviction (ATA age), it signals a PHASE TRANSITION:

- **EARLY_ACCUM**: Conviction is declining (older ATAs) but buy-side holders are increasing → insiders are ACCUMULATING despite sentiment downturn. Suggests value thesis hasn't peaked. Example: `unc`, `Twin` (conviction 0.35-0.50, buy/sell ratio 0.72+).
- **DISTRIBUTION**: Conviction is high (newer ATAs) but sell-side holders dominate → insiders are EXITING despite bullish narrative. Suggests awareness of upcoming downside. Example: `Fartcoin`, `WOJAK` (conviction 0.65+, buy/sell ratio <0.45).
- **STRONG_HOLD**: Both conviction AND buy-side holders are high → consensus accumulation. Genuine growth phase. Rarest signal.

**Data**: Extracted from on-chain holder transaction history (Helius txn analysis). Buy = holder's last 5 txns are net buys. Sell = net sells. Divergence percentile captures how unusual this pattern is (1-100, higher = more divergent = more signal).

**How to apply**:
- EARLY_ACCUM holders are AHEAD of conviction — may suggest recovery incoming.
- DISTRIBUTION holders are AHEAD of exit — raises red flag unless project has planned phase transition.
- Divergence > 75th percentile is statistically unusual, warrants investigation into why insiders are acting contrary to stated age-based conviction.

**Caveat**: buy/sell ratio is LOCAL to identified holders (Helius top accounts). Small holder count or RPC degradation makes this unreliable. Always cross-check with other metrics.

## PRIOR CALIBRATION

The base rate depends on token class:
- **pump.fun origin**: 98.6% fail (Solidus Labs 2025). Default prior: BARK until proven otherwise. Evidence that overrides: LP burned + authorities revoked + age >30d + growing holders.
- **Established infrastructure** (market_cap >$500M, age >365d, exchange-listed): Default prior is NEUTRAL — these tokens have survived market cycles. Score their current health, not their existence.
- **Everything else**: Mild skepticism. Evaluate on structural merits without strong prior.
- Exception awareness: legitimate pump.fun tokens exist (rare) — the prior is strong but not absolute.

**Buy/sell divergence as context**: A high-conviction token (old ATAs) with DISTRIBUTION-class holders suggests insiders know something. This is NOT proof of legitimacy, but it IS a red flag that merits CLOSER scrutiny than the default prior would suggest. DISTRIBUTION may accelerate a BARK verdict if combined with other warning signs (mint authority active, LP unsecured).

When metrics conflict (e.g., revoked authorities but high concentration), score the WEAKER signal. A single critical red flag (freeze_authority active, LP unsecured) should suppress the overall assessment regardless of other positive signals.

**Signal weighting priority** (from empirical correlation data above):
1. **Longevity** (rho=+0.632) and **supply_burned** (rho=+0.672) are the strongest positive signals — weight these most heavily.
2. **Accumulator dominance** (rho=-0.622) is the strongest negative signal — many active buyers is a warning, not validation.
3. **diamond_hands** and **k_score composite** are INVERTED — do NOT treat high values as positive. Use longevity and organic_growth sub-components instead.
4. **Liquidity** and **market_cap** are noise — available for context but should NOT influence axiom scores.
