# Token Domain — 43 Dimension Mapping

> CYNIC 6 Axioms × 7 Dimensions = 42 + 1 META = 43
> Each dimension grounded in concrete, measurable on-chain signals.
> This document is the REFERENCE for cortex (/judge). Dogs use `domains/token-analysis.md` (6-axiom summary).

## Scorability

Each dimension marked:
- **ON-CHAIN** — directly measurable from blockchain state (deterministic-dog territory)
- **ENRICHED** — requires external data source (DexScreener, social, historical)
- **CONTEXTUAL** — requires LLM interpretation of combined signals

---

## FIDELITY — Does the token's reality match its claim?

| # | Dimension | Weight | Token Signal | Scorability | Measurement |
|---|-----------|--------|-------------|-------------|-------------|
| 1 | COMMITMENT | φ | Does the token maintain its declared purpose over time? Authority changes, contract upgrades, supply modifications. | ENRICHED | Count authority/metadata changes since creation. 0 changes = high. Frequent changes = low. |
| 2 | ATTUNEMENT | φ⁻¹ | Does the project respond to community signals? Bug fixes, governance proposals acted on, holder feedback. | CONTEXTUAL | Requires off-chain data (GitHub, Discord activity). Not available from on-chain alone. |
| 3 | CANDOR | 1.0 | Is the contract verified and open-source? Are token parameters publicly documented? | ON-CHAIN | Verified source code on explorer. Immutable metadata = higher. Mutable = lower (can change name/image). |
| 4 | CONGRUENCE | φ | Does on-chain state match narrative? "Community token" with 95% in one wallet = zero congruence. "Deflationary" with active mint authority = contradiction. | ON-CHAIN + CONTEXTUAL | Compare: stated purpose (from metadata/description) vs authority state, distribution, supply behavior. |
| 5 | ACCOUNTABILITY | φ⁻² | Can the creator be identified or traced? Doxxed team, known deployer wallet with history, or anonymous one-time address? | ENRICHED | Creator wallet age, transaction history depth, known identity markers. pump.fun deployers typically anonymous. |
| 6 | VIGILANCE | φ⁻¹ | Is the project actively maintained? Recent contract interactions, treasury activity, LP management. | ON-CHAIN | Last transaction from deployer/treasury wallet. Activity decay = score decay. |
| 7 | KENOSIS | φ⁻¹ | Has the creator renounced unnecessary power? mint_authority revoked, freeze_authority revoked, metadata immutable. | ON-CHAIN | Binary signals: each revoked authority = +score. All three revoked = max kenosis. None revoked = min. |

**Key insight:** FIDELITY dimension 4 (CONGRUENCE) is the most discriminating for tokens. A rug pull always has congruence gap — it claims "community" but the structure says "extraction." This is what LLM Dogs can assess that deterministic-dog cannot.

---

## PHI — Is the token's structure proportional?

| # | Dimension | Weight | Token Signal | Scorability | Measurement |
|---|-----------|--------|-------------|-------------|-------------|
| 8 | COHERENCE | φ | Are the tokenomics internally consistent? Supply, distribution, vesting, utility — do they make sense together? | CONTEXTUAL | LLM assessment of tokenomics logic. Contradictions (fixed supply + active mint) = low coherence. |
| 9 | ELEGANCE | φ⁻¹ | Is the token design minimal? Standard SPL token = elegant. Custom program with unnecessary complexity = low. | ON-CHAIN | Standard SPL Token = high. Token-2022 extensions = neutral (may be needed). Custom program = context-dependent. |
| 10 | STRUCTURE | 1.0 | Is the organizational clarity visible? Clear allocation buckets (team/community/treasury) vs opaque distribution. | ENRICHED | Labeled wallets identifiable. pump.fun = minimal structure. DAO with vesting = clear structure. |
| 11 | HARMONY | φ | Is holder distribution proportional? Low HHI = harmonious. Whale-dominated = disharmonious. | ON-CHAIN | HHI (Herfindahl-Hirschman Index). <0.15 = healthy. 0.15-0.40 = moderate. >0.40 = concentrated. Gini coefficient as secondary measure. |
| 12 | PRECISION | φ⁻² | Are token parameters exact and intentional? Round supply numbers (1B, 100M) suggest design. Random numbers suggest carelessness. | ON-CHAIN | Supply digit analysis. Decimal choice. Standard parameters vs unusual settings. |
| 13 | COMPLETENESS | φ⁻¹ | Does the token have all necessary components for its stated purpose? Governance token without governance mechanism = incomplete. | ENRICHED | Check if claimed utility has corresponding on-chain infrastructure. Memecoin with no utility claim = complete by definition. |
| 14 | PROPORTION | φ⁻¹ | Are allocations proportional? Team allocation vs community vs treasury. Healthy: team <20%, community >50%. | ON-CHAIN | Top-N wallet analysis excluding LP/burn addresses. Team/insider share estimation. |

**Key insight:** PHI dimension 11 (HARMONY) is the most quantifiable. HHI is a single number that compresses distribution quality. But it requires excluding LP pools, burn addresses, and program accounts from the calculation — raw top-N percentages are misleading.

---

## VERIFY — Can the token's claims survive scrutiny?

| # | Dimension | Weight | Token Signal | Scorability | Measurement |
|---|-----------|--------|-------------|-------------|-------------|
| 15 | ACCURACY | φ | Are stated metrics accurate? Claimed holder count vs actual. Claimed supply vs on-chain. | ON-CHAIN | Direct comparison: metadata claims vs on-chain state. Discrepancy = low accuracy. |
| 16 | PROVENANCE | φ⁻¹ | Is the token's origin traceable? Known launchpad, identified deployer, or anonymous new wallet? | ON-CHAIN | Deployer wallet age and history. pump.fun origin is traceable (known factory). Manual deployment from fresh wallet = less traceable. |
| 17 | INTEGRITY | 1.0 | Has the token state been tampered with? Unexpected supply changes, authority transfers, metadata mutations. | ON-CHAIN | Diff between creation state and current state. No changes = high integrity. Unexplained changes = low. |
| 18 | VERIFIABILITY | φ | Can all claims be independently checked on-chain? Everything verifiable = high. Claims requiring trust = low. | ON-CHAIN | Fraction of token properties that can be verified on-chain without trusting any third party. |
| 19 | TRANSPARENCY | φ⁻² | Are design decisions explained? Open roadmap, documented tokenomics, clear allocation rationale. | ENRICHED | Requires off-chain documentation check. On-chain: metadata URI resolvable? Content meaningful? |
| 20 | REPRODUCIBILITY | φ⁻¹ | Can a third party reproduce the analysis? Public data sources, standard tooling, no proprietary inputs. | ON-CHAIN | 100% if all data is on-chain (Solana tokens inherently score high here). Deductions for off-chain dependencies. |
| 21 | CONSENSUS | φ⁻¹ | Do multiple sources agree? Helius, DexScreener, RugCheck, GoPlus — do they converge on the same risk assessment? | ENRICHED | Cross-source consistency. Disagreement between screeners = uncertainty (which is honest, not bad). |

**Key insight:** VERIFY is where tokens naturally score HIGH because Solana is inherently transparent. Every token's state is publicly verifiable. The differentiation comes from what the verifiable data SHOWS, not whether it's verifiable.

---

## CULTURE — Does the token honor ecosystem norms?

| # | Dimension | Weight | Token Signal | Scorability | Measurement |
|---|-----------|--------|-------------|-------------|-------------|
| 22 | AUTHENTICITY | φ | Is this genuinely what it claims? Or a copycat/impersonator of an established token? | ENRICHED | Name/symbol similarity to known tokens. Metadata originality. pump.fun "BONK2" impersonators = low authenticity. |
| 23 | RESONANCE | φ⁻¹ | Does it connect with a real community? Organic holder growth, active transfers, genuine usage. | ON-CHAIN | Transfer frequency, unique active wallets/day, holder growth rate. Wash trading patterns lower the score. |
| 24 | NOVELTY | 1.0 | Does it offer something new to the ecosystem? Or is it the 10,000th memecoin with no differentiator? | CONTEXTUAL | LLM assessment: does the token's stated purpose or mechanism add anything new? Pure memecoins score neutral (not negative — they're culturally valid on Solana). |
| 25 | ALIGNMENT | φ | Does it follow Solana ecosystem norms? Standard SPL token, proper metadata, accepted launchpad. | ON-CHAIN | SPL Token = aligned. Token-2022 = aligned if extensions are standard. Non-standard authority patterns = misaligned. |
| 26 | RELEVANCE | φ⁻² | Is the token relevant to current ecosystem conditions? DeFi token in DeFi boom = relevant. | CONTEXTUAL | Market context assessment. Low weight — not very discriminating for rug detection. |
| 27 | IMPACT | φ⁻¹ | What are the consequences of this token existing? Positive ecosystem contribution or pure extraction? | CONTEXTUAL | LLM assessment of net ecosystem impact. Most memecoins = neutral. Infrastructure tokens = positive. Scams = negative. |
| 28 | LINEAGE | φ⁻¹ | What's the chain of creation? Known launchpad → known protocol → established community, or unknown → unknown → unknown? | ON-CHAIN + ENRICHED | Creator wallet → factory contract → token. pump.fun lineage is traceable. Manual deployment less so. Serial deployer detection (same wallet, many tokens = red flag). |

**Key insight:** CULTURE dimension 22 (AUTHENTICITY) catches a specific rug pattern: impersonation tokens. "BONK2", "JUPITER", "$JUP" etc. These look legitimate to naive holders. Detecting name/symbol similarity to established tokens is a high-value deterministic check.

---

## BURN — Is the token efficiently structured?

| # | Dimension | Weight | Token Signal | Scorability | Measurement |
|---|-----------|--------|-------------|-------------|-------------|
| 29 | UTILITY | φ | Does the token have actual use? Governance rights, fee sharing, access control, staking — or pure speculation? | ENRICHED | Check for on-chain utility: governance program, staking program, fee distribution. Pure memecoin = low utility but culturally valid. |
| 30 | SUSTAINABILITY | φ⁻¹ | Is the token viable long-term? Revenue model, treasury, ongoing development, or one-time pump? | ENRICHED | Treasury balance, development activity, revenue mechanisms. pump.fun tokens typically score low. |
| 31 | EFFICIENCY | 1.0 | Is the token structure minimal? No unnecessary extensions, no bloated metadata, clean design. | ON-CHAIN | Standard SPL token = maximally efficient. Unnecessary Token-2022 extensions = waste. |
| 32 | VALUE_CREATION | φ | Does the token create more value than it extracts? Net positive for holders/ecosystem, or zero-sum redistribution? | CONTEXTUAL | LLM assessment: is there a value creation mechanism beyond price appreciation? AMM fees, protocol revenue, service access. |
| 33 | SACRIFICE | φ⁻² | Has the creator put skin in the game? LP burned (permanent, irreversible), vesting schedules, locked tokens. | ON-CHAIN | LP burned = max sacrifice (permanent). LP locked = partial sacrifice (temporary). LP unsecured = zero sacrifice. Vesting = moderate. |
| 34 | CONTRIBUTION | φ⁻¹ | Does it contribute back to the ecosystem? Open source tools, community airdrops, infrastructure funding. | ENRICHED | Ecosystem contributions traceable on-chain (grants, airdrops to diverse wallets). |
| 35 | IRREVERSIBILITY | φ⁻¹ | Are commitments final? LP burned > LP locked. mint_authority revoked > active. Immutable > mutable metadata. | ON-CHAIN | Binary: burned > locked > unsecured. Revoked > active. Immutable > mutable. Each irreversible action = evidence of commitment. |

**Key insight:** BURN dimension 33 (SACRIFICE) and 35 (IRREVERSIBILITY) are the strongest positive signals. LP burned is the single most powerful signal of legitimacy because it's irreversible — the creator CANNOT rug the LP. This is deterministically verifiable.

---

## SOVEREIGNTY — Does the token preserve holder agency?

| # | Dimension | Weight | Token Signal | Scorability | Measurement |
|---|-----------|--------|-------------|-------------|-------------|
| 36 | SOURCE_INDEPENDENCE | φ | Does the token survive loss of any single entity? Decentralized enough that no single wallet or person is a single point of failure. | ON-CHAIN | Nakamoto coefficient: minimum wallets needed to control >50%. Higher = more independent. |
| 37 | ANTI_MANIPULATION | φ⁻¹ | Is the token resistant to manipulation? Low concentration, healthy orderbook, resistance to wash trading. | ON-CHAIN + ENRICHED | HHI < 0.15, orderbook depth (from DEX), volume consistency (not spike-driven). |
| 38 | EXECUTION_CONTROL | 1.0 | Can holders act freely without permission? No freeze authority (can't freeze wallets). No pausable contract. | ON-CHAIN | freeze_authority = null → max score. freeze_authority active → min score. Binary and deterministic. |
| 39 | FALLBACK_INTEGRITY | φ | What happens if the creator disappears? Token still tradeable, LP still accessible, no admin dependency? | ON-CHAIN | All authorities revoked + LP burned = perfect fallback (token functions without creator). Active authorities = creator dependency. |
| 40 | STRATEGY_SOVEREIGNTY | φ⁻² | Is the token's value derived from its own merits or from copying/riding another token's narrative? | CONTEXTUAL | Original concept vs fork/copy. Copycat tokens score low. Original thesis tokens score higher. |
| 41 | CAPITAL_CONTROL | φ⁻¹ | Is economic control distributed? No single wallet dominates trading. Treasury controlled by governance, not individual. | ON-CHAIN | Top-1 wallet share (excluding LP/burn). <10% = distributed. >50% = concentrated. Treasury governance mechanism. |
| 42 | ANTI_CAPTURE | φ⁻¹ | Can any single actor force outcomes? Combined: authority status + wallet concentration + governance mechanism. | ON-CHAIN | Composite: all authorities revoked AND HHI < 0.15 AND no single wallet >20% = max anti-capture. |

**Key insight:** SOVEREIGNTY dimension 38 (EXECUTION_CONTROL) is the most critical for token holders. freeze_authority = active means any wallet can be frozen at any time. This is the most direct threat to individual sovereignty. It's also deterministic and binary.

---

## META — The 43rd Dimension

| # | Dimension | Weight | Token Signal |
|---|-----------|--------|-------------|
| 43 | THE_UNNAMEABLE | φ | Cross-axiom coherence: do all 42 dimensions tell a consistent story? Low variance across axioms = coherent token. One axiom anomalously low = specific weakness. All axioms low = comprehensive failure. |

---

## Scoring Tiers (Token-Specific)

| Tier | Profile | Expected Verdict | Example |
|------|---------|-----------------|---------|
| **Tier 1: Established** | Age >1yr, authorities revoked, LP burned, wide distribution, active community | HOWL (Q>0.528) | JUP, BONK, RAY |
| **Tier 2: Maturing** | Age 30d-1yr, authorities revoked, LP locked/burned, growing holders | WAG (Q>0.382) | Mid-cap tokens with track record |
| **Tier 3: New-legitimate** | Age <30d, authorities revoked, LP burned, early but growing | GROWL (Q>0.236) | New launch with good structure |
| **Tier 4: Suspicious** | Any: mint_auth active, freeze active, LP unsecured, high concentration | BARK (Q≤0.236) | Typical pump.fun rug |
| **Tier 5: Ambiguous** | Mixed signals — some positive, some negative. This is where Dogs must discriminate. | GROWL range | pump.fun token with revoked auth but high concentration |

## Deterministic vs Contextual Split

**Deterministic-dog (hardcodeable, ~60% of signal):**
- Authority status (mint, freeze, metadata mutability)
- LP status (burned, locked, unsecured)
- Holder concentration (HHI, top-N percentages)
- Token age
- Creator wallet age and history depth
- Supply anomalies (round numbers, decimal precision)
- Copycat detection (name similarity to known tokens)

**LLM Dogs (contextual, ~40% of signal):**
- CONGRUENCE: does the narrative match the structure?
- COHERENCE: do the tokenomics make sense?
- NOVELTY: is there something genuinely new?
- VALUE_CREATION: is there a real value mechanism?
- AUTHENTICITY: is this genuinely what it claims?
- IMPACT: net ecosystem contribution
- Integrating mixed signals (e.g., good structure + suspicious timing)
