# Calibration Corpus — Token Analysis Domain

> Created: 2026-04-22 | Purpose: Dog VERIFY (test-retest) + FIDELITY (accuracy) measurement.
> Protocol: Each stimulus × 3 runs through `/judge` with `domain="token-analysis"`.
> From 27 datapoints: compute σ_intra per Dog (VERIFY), correct ordering (FIDELITY).

## Expected ordering per format

Within each format: HOWL stimulus > AMBIGUOUS stimulus > BARK stimulus.
A Dog that inverts this ordering has FIDELITY problems.

---

## FORMAT 1 — Metadata brute (chiffres seuls)

### M-HOWL: JUP (Jupiter) — real data from Helius, 2026-04-22

Token: JUP (Jupiter). Type: FungibleToken. Program: SPL Token.
Supply: 6,863,982,190. Decimals: 6. Price: $0.177 USDC.
Mint authority: EXISTS (61aq585V8cR2sZBeawJFt2NPqmN7zDi1sws4KLs5xHXV).
Metadata: Mutable. Description: "Official governance token for Jupiter."
Top holder concentration: ~4.2% (286 JUP in top account of sampled set).
Age: 14+ months. DEX aggregator with $2B+ TVL across Solana.

**Expected: WAG or HOWL.** Strong fundamentals, governance utility, wide distribution. Mint authority active is a concern (prevents full HOWL).

### M-BARK: Synthetic rug pattern — based on SolRugDetector (arXiv 2603.24625)

Token: MOONAI. Type: FungibleToken. Program: SPL Token.
Supply: 1,000,000,000. Decimals: 9. Price: $0.00000034 USDC.
Mint authority: ACTIVE (deployer wallet, created 2 hours ago).
Freeze authority: ACTIVE. Metadata: Mutable.
Holders: 3 wallets. Top holder: 94.7% of supply. LP: None detected.
Age: 2 hours. No website, no social links. Deployer funded by mixer.

**Expected: BARK.** Every rug indicator present: concentrated supply, active authorities, no LP, no history, mixer funding.

### M-AMBIGUOUS: BONK — real data from Helius, 2026-04-22

Token: Bonk. Type: FungibleToken. Program: SPL Token.
Supply: 87,994,739,245,442. Decimals: 5. Price: $0.000006 USDC.
Mint authority: EXISTS (9AhKqLR67hwapvG8SA2JFXaCshXc9nALJjpKaHZrsbkw).
Metadata: Mutable. Description: "The Official Bonk Inu token."
Top holder: 34M BONK (~0.00004% of supply). Distribution: wide.
Age: 2+ years. Community meme token. Major exchange listings.

**Expected: WAG or GROWL.** Real community and history, but meme token with no utility. Mint authority active. Extremely high supply dilution. Mixed signals.

---

## FORMAT 2 — Narrative enrichie (projet + contexte)

### N-HOWL: Pyth Network — infrastructure oracle

Pyth Network is a first-party oracle that publishes financial market data on-chain. Over 500 price feeds across crypto, equities, FX, and commodities. Used by major Solana protocols (Jupiter, Drift, Marginfi, Kamino). Backed by Jump Trading. Governance token PYTH launched Nov 2023 with 85,000+ unique claimers. Cross-chain deployment via Wormhole to 50+ chains. Revenue model: data licensing fees from integrators. Team: public, experienced (ex-Jump, ex-FTX Research). Authorities: governed by Pyth DAO multisig, not single deployer.

**Expected: HOWL.** Infrastructure protocol with real revenue, institutional backing, cross-chain adoption, public team, governance.

### N-BARK: Synthetic — fake AI gaming token

SolanaGPT-X claims to be "the first AI-powered gaming metaverse on Solana." Website launched 3 days ago, copied template from a known rugpull. Whitepaper is 2 pages, mostly buzzwords ("quantum neural blockchain AI synergy"). Team: anonymous, no LinkedIn, no GitHub history. Token launched on pump.fun 6 hours ago. Telegram group has 12,000 members (growth from 0 to 12K in 4 hours — likely botted). Smart contract not verified. No audit. LP locked for 7 days only (minimum pump.fun default).

**Expected: BARK.** Every narrative red flag: buzzword whitepaper, anonymous team, botted social, unverified contract, 7-day LP lock.

### N-AMBIGUOUS: New DeFi protocol with mixed signals

Kamino Finance is a Solana DeFi protocol offering automated liquidity vaults, lending/borrowing, and leveraged yield strategies. $1.2B TVL. Team is pseudonymous but consistently active since 2023. Has been audited by OtterSec and Sec3. Token KMNO launched Apr 2024. Supply distribution: 30% to community via points program, 20% team (2-year vest). Criticism: some users report liquidation issues during high volatility. Protocol has survived 3 major Solana outages without fund loss. Revenue: protocol fees from lending spreads, currently $2M/month. Risk: concentrated TVL in a few vaults, smart contract upgrade authority held by 3/7 multisig.

**Expected: WAG or GROWL.** Real protocol with TVL and revenue, but pseudonymous team, liquidation concerns, concentrated risk. Not clearly HOWL, not clearly BARK.

---

## FORMAT 3 — Red flag patterns (behavioral signals)

### R-HOWL: Measured, transparent communication

"We've completed our Q1 audit with OtterSec — full report published at docs.example.com/audit. Three medium-severity findings were identified and patched before mainnet deployment. Our treasury diversification proposal (PROP-47) passed governance with 73% approval. Next milestone: v2 lending module, estimated June. We're hiring a senior Rust engineer — see our careers page. Current TVL: $340M, down 12% from ATH due to broader market conditions."

**Expected: HOWL.** Transparent about weaknesses, audit published, governance active, realistic about market conditions, hiring.

### R-BARK: Urgency, hype, and pressure

"LAUNCHING IN 30 MINUTES! This is NOT a drill! The devs are DOXXED (trust me bro). Already 50x from presale and we haven't even listed yet! Get in NOW before the CEX listing announcement tomorrow. NFA but this is the easiest 1000x of your life. Telegram link in bio. Whitelist spots almost GONE. Don't be the one who missed $PEPE. Burn mechanism activated — supply going to ZERO."

**Expected: BARK.** Urgency pressure, unverifiable claims, "trust me bro" doxxing, promises of returns, FOMO manipulation, supply burn hype.

### R-AMBIGUOUS: Good tech, concerning signals

"Our zero-knowledge proof verification module is now live on devnet. Benchmarks show 340ms verification time, 60% faster than competitors using Groth16. We've open-sourced the prover at github.com/example/zkprover (47 stars, 3 contributors). However, we should note that our mainnet launch has been delayed twice — originally planned for Q4 2025, now targeting Q3 2026. Our lead cryptographer left the project in January for personal reasons. We've hired a replacement from Trail of Bits. Current funding: $1.2M remaining from seed round, runway ~8 months at current burn rate."

**Expected: GROWL or WAG.** Real technology with benchmarks and open source. But: delayed twice, key person departure, limited runway. The transparency is good (FIDELITY+), the fundamentals are concerning (VERIFY-).

---

## Protocol

1. Run each of the 9 stimuli through `POST /judge` with `domain="token-analysis"`, `inject_crystals=false`.
2. Repeat 3 times per stimulus (27 calls total).
3. Record per-Dog scores per axiom per run.
4. Compute:
   - **σ_intra(Dog, stimulus)**: std dev of Q-scores across 3 runs of same stimulus. Target: < 0.10.
   - **Ordering accuracy(Dog)**: Does Dog rank M-HOWL > M-AMBIGUOUS > M-BARK within Format 1? Same for Format 2, 3.
   - **Format discrimination(format)**: Which format produces the largest Δ between HOWL and BARK?
   - **Cross-Dog agreement(stimulus)**: Spearman ρ between Dog pairs across the 9 stimuli.
5. Results populate the Dog axiom profile (FIDELITY from ordering, VERIFY from σ_intra).
