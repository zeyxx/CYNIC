# TALARIA — Futardio Project Description (final v3)

> À coller dans le champ "Project Description" sur futard.io

---

## The Problem

Futarchy lets markets decide. But who's in the market? Bots? Sybils? Nobody checks if voters are human or if proposals are sound before the vote starts. We do.

## Why us

Nobody ships both sides of the trust problem in one stack. Existing Proof of Humanity (WorldCoin) is biometric and friction-heavy. Existing judgment engines (GPT-4 APIs) are uncalibrated, cloud-dependent, single-validator. We built the intersection — game-native, sovereign, calibrated — and **both halves are already live**, not roadmap.

## Two Products. One Trust Layer.

### CYNIC — AI Judgment Engine *(live)*

- Heuristic validator + up to 3 independent LLM validators score proposals on 6 axioms: fidelity, logic, verifiability, culture, efficiency, sovereignty
- Confidence architecturally capped at φ⁻¹ = 61.8% — the system refuses certainty beyond the evidence
- EPOCHÉ: judgment suspended when validators disagree (no forced verdicts)
- 2,195 verdicts rendered · Rust kernel · sovereign hardware · zero cloud dependency

### Blitz & Chill — Social Chess Platform *(live)*

**Live now:**
- Real-time multiplayer, matchmaking, QR challenge links
- Solo vs Stockfish 18 WASM (10 difficulty levels, multi-threaded)
- Post-game analysis, accuracy %, eval graph
- Auth (Google/Discord), Glicko-2 rating, PWA offline play

**PoH — backend complete, gateway shipping M1:**
- Anti-Sybil classifier, Ed25519 permit signer, per-wallet rate limiter: ✅ built
- Metaplex Core soulbound mint on devnet: ✅ end-to-end
- Missing piece: PoH gateway UI — scan QR → play 5-min game → 7-day human badge
- **No KYC, no biometrics, no data stored. Privacy is the moat.**

**Why privacy is the product, not a feature:**
- Every KYC/biometric database is a honeypot. A single leak exposes holders to targeted attacks — in France, crypto data leaks have already led to physical kidnappings. We store zero personal data.
- AI agents now defeat CAPTCHAs; the human-verification industry has degraded into sweatshops where workers in low-income countries solve them by hand. Neither proves humanity.
- We verify humanity through *gameplay* — behavioral, unfakeable by bots, surrendering nothing. Sovereignty without surveillance, the way crypto was meant to work.

**M2+ (roadmap):**
- Inter-community tournaments ("$TOKEN_A vs $TOKEN_B 7-3")
- Soulbound NFT on verified completion (mainnet)
- Tribe rivalries, leaderboards, Season 1

## By the Numbers

| Metric | Value | Status |
|--------|-------|--------|
| Verdicts rendered (CYNIC) | 2,195 | ✅ live |
| Cloud dependency (AI engine) | 0% | ✅ sovereign |
| Tests (CYNIC + B&C) | 2,045 | ✅ verified |
| Commits (CYNIC + B&C) | 1,404 | ✅ verified |
| Multiplayer chess (B&C) | live | ✅ live |
| PoH backend (anti-Sybil, permit, mint) | built | ✅ devnet |
| PoH gateway (QR → game → badge) | M1 | 🔨 shipping |
| Inter-community tournaments | M2+ | 📋 roadmap |

## Token Utility

**$TALARIA** has two mechanisms:

1. **Access token** — hold $TALARIA to unlock judgment API tiers beyond free quota (free: 10/month · Standard: hold 1,000 · Pro: hold 10,000)
2. **Tournament burn** — inter-community tournament entry burns $TALARIA (casual: 10 · ranked: 100 · inter-community: 1,000/player)

## The Upside Case

ICO price $0.005. The team package unlocks in tranches at 2× / 4× / 8× / 16× / 32× — the team is paid on exactly the price performance investors profit from. What drives demand toward those multiples?

- 50 API clients × 10,000 $TALARIA held = 500K tokens locked
- 10 inter-community events/month × 100 players × 1,000 $TALARIA = 1M tokens burned/month
- At that burn rate: supply halved in ~12 months

These multiples require no speculative leap — just product adoption in a space (futarchy, PoH) that is actively growing. Every governance protocol needs what we built.

## Market

**Target:** futarchy protocols, DAOs running proposal votes, crypto communities with tribal rivalries.

**GTM:** inter-community chess rivalries are self-propagating content — zero ad spend, communities onboard each other through play.

## Use of Funds

Funds are held in a futarchy-governed treasury. The team requests a **monthly allowance** (market-approved), not a salary. Unspent USDC stays in the treasury — there is no private reserve.

| Monthly allowance request | Monthly | 6-month draw |
|---------------------------|---------|--------------|
| T. — CYNIC kernel, judgment API | $3,000 | $18,000 |
| S. — Blitz & Chill, community | $3,000 | $18,000 |
| Infrastructure (sovereign hardware, electricity ~180 kWh/month) | $50 | $300 |
| **Total allowance** | **$6,050** | **~$36,300** |

Lean by design: $3,000/builder reflects Paris cost of living, not market salary — the upside is in the price-based token package, not the allowance. At this rate, ~73% of a $50K raise is drawn over 6 months; the remainder stays treasury-controlled, releasable only by market vote.

No marketing budget. No agency. The flywheel is the marketing.

## Roadmap

*Execution baseline: ~120 commits/week combined (measured 3-month average — CYNIC 1,016 + B&C 388). Two builders, cybersecurity students, shipping daily.*

**M1 (weeks 1–4)**
- CYNIC: public judgment API with quota tiers + rate limiting
- B&C: PoH gateway UI — QR → play 5-min game → 7-day human badge (backend already built)
- $TALARIA: liquidity pools seeded automatically at raise close (MetaDAO mechanism)

**M2 (weeks 5–8)**
- CYNIC: dog health stabilized (LLM validators back online), crystals pipeline active
- B&C: PoH mainnet + first inter-community tournament (2 communities)
- B&C: tournament leaderboard, tribe profiles

**M3–4**
- PoH on-chain mainnet — soulbound NFT on verified completion
- Burn-as-a-service: tournament entry burns $TALARIA (1,000/player inter-community)
- Oracle pricing: TWAP from MetaDAO liquidity pool replaces fixed token amount
- Checkers/dames as second game (PoH variant)
- First external API client

**M5–6**
- sBPF optimization (kernel performance)
- Go SDK for judgment API
- Season 1: prize pool funded from protocol fees
- $TALARIA burn rate target: 1M tokens/month

## Token Allocation

MetaDAO standard structure — the protocol fixes the float; the team only earns tokens if the price performs.

**Liquid at launch (set by protocol):**

| Bucket | Tokens | Unlock |
|--------|--------|--------|
| ICO (public raise) | 10,000,000 | 100% liquid at TGE |
| LP pool (auto-seeded) | 2,900,000 | seeded at raise close (paired with 20% of USDC) |

**Team performance package (price-based, locked):**

| Holder | Tokens | Unlock |
|--------|--------|--------|
| T. | 3,600,000 | 5 tranches at 2× / 4× / 8× / 16× / 32× ICO price · 18-month cliff |
| S. | 3,600,000 | 5 tranches at 2× / 4× / 8× / 16× / 32× ICO price · 18-month cliff |

No team tokens unlock at TGE. Nothing vests on time alone — only on sustained price performance (3-month TWAP). Funds raised are held in a futarchy-governed treasury; the team draws a market-approved monthly allowance, not a lump sum.

*Incentives aligned by design: if the token doesn't perform, the team earns nothing beyond the monthly allowance.*

## Why TALARIA?

Talaria — the winged sandals of Hermes. Speed, trust, and the messenger who never lies.

Public on day 1. Accountable on day 1. Code is open:
- CYNIC: github.com/zeyxx/CYNIC
- Blitz & Chill: github.com/Ragnar-no-sleep/blitz-and-chill
