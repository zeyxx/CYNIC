# MetaDAO Submit — Step 1: Project Info

> Colle chaque champ ci-dessous dans le formulaire. Un champ = une section.

---

## Project Name

```
Talaria
```

---

## Ticker

```
TALARIA
```

---

## Short Description

```
Is the proposal sound? Are the voters human? We answer both.
```

---

## Project Description

*(Remplace tout le contenu actuel par ce qui suit)*

```markdown
## The Problem

On-chain governance is theater. One entity holds fifty wallets — it votes fifty times. One team runs twenty X and Telegram accounts — the "community" is a single actor. Governance proposals pass not because token holders deliberate, but because one player signals and others follow. Meanwhile, existing Proof of Humanity systems extract biometric data to sell it. Data is money — and today, protocols capture that value instead of users.

Two primitives are missing. First, a Proof of Humanity that works without KYC, without biometrics, without surrendering data — that proves one human, one vote, and leaves the data with the user. Second, an independent judgment layer that evaluates whether a proposal is sound before capital moves.

We built both. **CYNIC judges. Blitz & Chill proves humanity.** The AI Governance stack that connects them is the roadmap — both halves are live today.

## Why us

Nobody ships both sides of the trust problem in one stack. Existing Proof of Humanity is biometric and friction-heavy. Existing AI judgment systems are uncalibrated, cloud-dependent, single-validator. We built the intersection — game-native, sovereign, calibrated — and **both halves are already live**, not roadmap. Architecture: off-chain compute, on-chain settlement — the foundation for a ZK trust layer on Solana.

## Two Products. One Trust Layer.

### CYNIC — AI Judgment Engine *(live)*

- Heuristic validator + up to 3 independent LLM validators score proposals on 6 axioms: fidelity, phi (structural harmony), verify/falsify, culture, burn (efficiency), sovereignty
- Confidence architecturally capped at φ⁻¹ = 61.8% — the inverse of the golden ratio, hard-coded into the scoring model. The system cannot exceed this certainty threshold by design, not configuration.
- EPOCHÉ: when validators disagree beyond threshold, judgment is suspended — "I don't know" is a first-class output, not a failure state
- 3,711 verdicts rendered · Rust kernel · sovereign hardware · zero cloud dependency

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
- Every KYC/biometric database is a honeypot. In France alone, publicized crypto data leaks have already led to home invasions, kidnappings for ransom, and physical extortion targeting identified holders. The PoH verification requires no KYC, no biometrics, no identity documents. A wallet address and a chess game — nothing else. We don't know who you are, and we don't want to.
- AI agents now defeat CAPTCHAs; the human-verification industry has degraded into sweatshops where workers in low-income countries solve them by hand. Neither proves humanity.
- We verify humanity through *gameplay* — behavioral, unfakeable by bots, surrendering nothing. Sovereignty without surveillance, the way crypto was meant to work.

**M2+ (roadmap):**
- Inter-community tournaments ("$TOKEN_A vs $TOKEN_B 7-3")
- Soulbound NFT on verified completion (mainnet)
- Tribe rivalries, leaderboards, Season 1

## By the Numbers

| Metric | Value | Status |
|--------|-------|--------|
| Verdicts rendered (CYNIC) | 3,711 | ✅ live |
| Cloud dependency (AI engine) | 0% | ✅ sovereign |
| Tests (CYNIC + B&C) | 2,045 | ✅ verified |
| Commits (CYNIC + B&C) | 1,400+ | ✅ verified |
| Multiplayer chess (B&C) | live | ✅ live |
| PoH backend (anti-Sybil, permit, mint) | built | ✅ devnet |
| PoH gateway (QR → game → badge) | M1 | 🔨 shipping |
| Inter-community tournaments | M2+ | 📋 roadmap |

## $TALARIA — Ownership, Not Access

**$TALARIA is an ownership coin.** Holders own a share of a futarchy-governed treasury — value is the treasury NAV plus the market's expectation of future revenue flows. No artificial utility needed; MetaDAO's mechanism handles price discovery.

**How value accrues:**
- **Pay-per-use burn** — API fees accepted in USDC or $TALARIA. $TALARIA payers receive a 20% discount; tokens are burned on use. USDC payments flow to treasury. Usage in either currency creates protocol value — holders benefit from the discount, treasury grows regardless. Pricing is cost-plus (10× marginal inference cost): free tier (10 verdicts/month), standard per-verdict rate, volume negotiated above 1K/month. No pricing lock-in at launch — first design partners set the floor.
- **Tournament burn** — inter-community event entry burns $TALARIA (1,000/player inter-community). Communities compete; supply contracts.
- **Treasury growth** — unspent USDC stays in treasury, releasable only by market vote. The team draws a market-approved monthly allowance, not a lump sum.

There is no "hold X to access Y" tier — that model penalizes adoption precisely when the token succeeds. Revenue flows in, tokens burn out, treasury grows.

## The Upside Case

ICO price $0.005. The team package unlocks in tranches at 2× / 4× / 8× / 16× / 32× — the team is paid on exactly the price performance investors profit from.

What drives demand? Real usage. Every API call burns $TALARIA. Every inter-community tournament burns $TALARIA. The team draws an allowance only while the market approves it. There is no projected timeline for "supply halved" — that depends on adoption we haven't proven yet. What we have proven: 3,711 verdicts rendered, two live products, zero cloud dependency, 1,400+ commits. The bet is on execution, not a model.

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
| **Total allowance** | **$6,000** | **~$36,300** |

Lean by design: $3,000/builder reflects Paris cost of living, not market salary — the upside is in the price-based token package, not the allowance. At this rate, ~73% of a $50K raise is drawn over 6 months; the remainder stays treasury-controlled, releasable only by market vote.

No marketing budget. No agency. The flywheel is the marketing.

## Roadmap

*Execution baseline: ~120 commits/week combined (measured 3-month average — CYNIC 1,002 + B&C 388). Two builders, cybersecurity students, shipping daily.*

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
- ZK proof of gameplay — prove "played like a human" without revealing the game (data stays with the user)
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

## Team

**Titouan** — CYNIC kernel, judgment API, sovereign infrastructure. Cybersecurity student, shipping Rust daily since March 2026.

**Ragnar-no-sleep** — Blitz & Chill, chess platform, community. 388 commits since March 2026.

Two builders. No agency. No marketing budget. ~120 commits/week combined, measured over 3 months.

## Why TALARIA?

Talaria — the winged sandals of Hermes. Speed, trust, and the messenger who never lies.

Public on day 1. Accountable on day 1. Code is open:
- CYNIC: github.com/zeyxx/CYNIC
- Blitz & Chill: github.com/Ragnar-no-sleep/blitz-and-chill
```
