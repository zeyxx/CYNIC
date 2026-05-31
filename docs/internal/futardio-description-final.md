# TALARIA — Futardio Project Description (final v2)

> À coller dans le champ "Project Description" sur futard.io

---

## The Problem

Futarchy lets markets decide. But who's in the market? Bots? Sybils? Nobody checks if voters are human or if proposals are sound before the vote starts. We do.

## Two Products. One Trust Layer.

### CYNIC — AI Judgment Engine *(live)*

- Heuristic validator + up to 2 LLM validators score proposals on 6 axioms: fidelity, logic, verifiability, culture, efficiency, sovereignty
- Confidence architecturally capped at φ⁻¹ = 61.8% — the system refuses certainty beyond the evidence
- EPOCHÉ: judgment suspended when validators disagree (no forced verdicts)
- 1,874+ verdicts rendered · Rust kernel · sovereign hardware · zero cloud dependency

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
- No KYC, no biometrics. Behavioral proof through play.

**M2+ (roadmap):**
- Inter-community tournaments ("$TOKEN_A vs $TOKEN_B 7-3")
- Soulbound NFT on verified completion (mainnet)
- Tribe rivalries, leaderboards, Season 1

## By the Numbers

| Metric | Value | Status |
|--------|-------|--------|
| Verdicts rendered (CYNIC) | 1,874+ | ✅ live |
| Observations processed | 56,000+ | ✅ live |
| Tests (CYNIC + B&C) | 2,000+ | ✅ verified |
| Commits (CYNIC + B&C) | 1,400+ | ✅ verified |
| Cloud dependency (AI engine) | 0% | ✅ live |
| Multiplayer chess (B&C) | live | ✅ live |
| PoH backend (anti-Sybil, permit, mint) | built | ✅ devnet |
| PoH gateway (QR → game → badge) | M1 | 🔨 shipping |
| Inter-community tournaments | M2+ | 📋 roadmap |

## Token Utility

**$TALARIA** has two mechanisms:

1. **Access token** — hold $TALARIA to unlock judgment API tiers beyond free quota (free: 10/month · Standard: hold 1,000 · Pro: hold 10,000)
2. **Tournament burn** — inter-community tournament entry burns $TALARIA (casual: 10 · ranked: 100 · inter-community: 1,000/player)

## The 10x Upside Case

$90K FDV today. 10x = $900K. What gets us there?

- 50 API clients × 10,000 $TALARIA held = 500K tokens locked
- 10 inter-community events/month × 100 players × 1,000 $TALARIA = 1M tokens burned/month
- At that burn rate: supply halved in ~12 months

$900K FDV requires no speculative leap — just product adoption in a space (futarchy, PoH) that is actively growing. Every governance protocol needs what we built.

## Market & Differentiation

**Target:** futarchy protocols, DAOs running proposal votes, crypto communities with tribal rivalries.

**Edge:** nobody ships both sides of the trust problem in one stack. Existing PoH (Proof of Humanity, WorldCoin) is biometric and friction-heavy. Existing judgment engines (GPT-4 APIs) are uncalibrated, cloud-dependent, and single-validator. We built the intersection: game-native, sovereign, calibrated.

**GTM:** inter-community chess rivalries are self-propagating content. Zero ad spend. Communities onboard each other.

## Use of Funds

Monthly burn: **$6,050/month**

| Allocation | Monthly | 6 months |
|------------|---------|----------|
| T. — CYNIC kernel, judgment API | $3,000 | $18,000 |
| S. — Blitz & Chill, community | $3,000 | $18,000 |
| Infrastructure (sovereign hardware, electricity ~180 kWh/month) | $50 | $300 |
| **Total burn** | **$6,050** | **$36,300** |
| Reserve (contingency + future hire) | — | $13,700 |
| **Raise** | | **$50,000** |

No marketing budget. No agency. The flywheel is the marketing.

## Roadmap

*Execution baseline: ~100 commits/week, 30+ features/week (3-month average). Two full-time engineers.*

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

| Bucket | Tokens | % | Unlock |
|--------|--------|---|--------|
| ICO (public raise) | 10,000,000 | ~39% | at raise close |
| LP pool (auto-seeded) | 2,900,000 | ~11% | at raise close |
| T. — performance package | 3,600,000 | ~14% | price-based: 2×/4×/8×/16×/32× ICO price, min 18 months |
| S. — performance package | 3,600,000 | ~14% | price-based: 2×/4×/8×/16×/32× ICO price, min 18 months |
| Protocol treasury | 900,000 | ~3% | governance-controlled |
| **Total** | **~21,000,000** | **100%** | |

*MetaDAO standard structure. Team tokens unlock only if the token performs — aligned incentives by design.*

## Why TALARIA?

Talaria — the winged sandals of Hermes. Speed, trust, and the messenger who never lies.

Public on day 1. Accountable on day 1. Code is open:
- CYNIC: github.com/zeyxx/CYNIC
- Blitz & Chill: github.com/Ragnar-no-sleep/blitz-and-chill
