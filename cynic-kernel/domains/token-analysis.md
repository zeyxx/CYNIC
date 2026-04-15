# Token Analysis Domain — Axiom Evaluation Criteria

Evaluate the TOKEN'S ON-CHAIN REALITY — not the quality of its description. A scam described with technical accuracy is still a scam. A legitimate token with a garbage name is still legitimate if the on-chain structure is sound.

Judge the POWER STRUCTURE encoded in the metrics. On-chain data reveals WHO has power, HOW MUCH power, and WHETHER they can use it. This is what you score.

## ASYMMETRIC CONFIDENCE

Negative assessments (BARKs) have higher confidence than positive assessments (HOWLs). Absence of red flags ≠ presence of legitimacy. On-chain data reveals power concentration but CANNOT prove intent.

- Token age <30 days: max axiom score = 0.45 (untested, cannot HOWL yet)
- Token age 30-365 days: max axiom score = 0.55 (building track record)
- Token age >365 days with growing holders: max axiom score = 0.618 (established)

## FIDELITY
Does the on-chain state match what a legitimate project would show? Is the token faithful to its claimed purpose?
- HIGH (0.50-0.618): Authorities revoked, metadata immutable, on-chain state matches narrative. No contradictions between claimed purpose and actual structure. Creator wallet has history. Example: JUP — governance token with actual governance mechanism, revoked mint authority, DAO treasury, years of consistent operation.
- MEDIUM (0.25-0.45): Some authorities revoked, some retained with plausible reason. Metadata mutable but consistent over time. Minor discrepancies between claim and state. Example: New DeFi token with active mint authority (for inflation schedule) but transparent vesting and clear documentation.
- LOW (0.05-0.20): Claims contradict on-chain state. "Community token" with one wallet holding 90%. "Deflationary" with active mint authority and no burn mechanism. Metadata recently changed. Example: pump.fun token claiming "SafeMoon 2.0" with active freeze authority and 3 holders.

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
Is control distributed or concentrated? Can individual holders act freely without one wallet dominating?
- HIGH (0.50-0.618): freeze_authority revoked (wallets cannot be frozen). No single wallet > 10% of circulating supply. No admin functions that can override holder actions. Token functions independently of creator. Example: Fully decentralized token — all authorities revoked, distributed holders, LP burned, would continue functioning if creator wallet disappeared.
- MEDIUM (0.25-0.45): freeze_authority revoked but moderate concentration. Some admin functions exist but governed by multisig or DAO. Token is partially creator-dependent. Example: Token with governance multisig, revoked freeze, but significant treasury allocation.
- LOW (0.05-0.20): freeze_authority ACTIVE (any wallet can be frozen — most direct threat to individual sovereignty). Single wallet controls >50%. Token entirely dependent on creator's continued good behavior. Example: Token where freeze authority can lock any holder's wallet at any time, and one wallet dominates supply.

## PRIOR CALIBRATION

The base rate for new tokens is skepticism, not trust:
- 98.6% of pump.fun tokens fail (Solidus Labs 2025). Default prior for pump.fun: BARK until proven otherwise.
- Evidence that OVERRIDES the prior: LP burned + authorities revoked + age >30d + growing holders = strong evidence of legitimacy, sufficient to upgrade verdict.
- Exception awareness: legitimate pump.fun tokens exist (rare) — the prior is strong but not absolute.

When metrics conflict (e.g., revoked authorities but high concentration), score the WEAKER signal. A single critical red flag (freeze_authority active, LP unsecured) should suppress the overall assessment regardless of other positive signals.
