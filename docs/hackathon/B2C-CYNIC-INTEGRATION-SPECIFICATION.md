# B&C ↔ CYNIC Integration Specification

**Status:** Clarification pending (S. response on Slack expected)  
**Last updated:** 2026-04-25  
**Confidence:** 0.45 (inferred from Slack messages + code probe)

---

## What This Document Is

A living map of how Blitz & Chill Personality Cards integrate with CYNIC's judgment pipeline. Updated as S. clarifies scope and implementation proceeds.

---

## Current Understanding (from Slack 2026-04-24)

### S.'s Anti-Sybil Gate (Personality Card Minting)

**Required conditions for mint:**
- N ≥ 5 completed chess games (local, Stockfish WASM)
- Confidence (Q-Score) ≥ φ⁻¹ ≈ 0.618 from personality classifier
- Nonce LRU cache 5min (prevent re-submission replay)
- Rate-limit: 1 mint request per wallet per hour

**Delivery:** J6-7 Sprint
- `/nonce` endpoint → returns nonce + TTL
- `/mint-permit` endpoint → verifies nonce + games + confidence, signs Ed25519 permit, uploads metadata to Arweave (via Irys)
- Metaplex Core Soulbound NFT minted on-chain (Pinocchio program)

### S.'s Ecosystem Vision

**Three consumers of Personality Cards:**
1. **$ASDFASDFA** (first) — gating membership by chess archetype
2. **CYNIC** (second) — behavioral epistemic oracle on wallet reputation
3. **Hackathon demo** (proof of concept)

---

## CYNIC's Role: Validation Scope (CLARIFY)

**Pending clarification from S.:**

| Question | Option A | Option B | Option C |
|----------|----------|----------|----------|
| **What does CYNIC validate?** | Ed25519 signature + metadata integrity (cryptographic) | Wallet itself (N≥5 games + confidence ≥ φ⁻¹) | Both: crypto + wallet reputation |
| **When does validation happen?** | At mint time (pre-blockchain) | After mint (blockchain audit) | Both (pre + post) |
| **What's the verdict domain?** | `token-analysis` (existing) | `chess` (existing) | `wallet-reputation` (new) |
| **Who calls CYNIC?** | Mint Permit Service (S. backend) | External audit tool | Both |
| **Output shape** | `{isValid: bool, fidelity, phi, verify, ...}` | `{qScore: f64, verdict: HOWL/WAG/BARK}` | Full CYNIC verdict |

---

## Technical Integration Points

### Path 1: Pre-Mint Validation (Cryptographic)

```
User plays 5+ games locally
  ↓
Personality Card generated locally (9 archetypes, 6 signals, Q-score)
  ↓
Request /nonce → {nonce, ttl=5min}
  ↓
Sign nonce + card + wallet address with Ed25519 (client-side)
  ↓
POST /mint-permit {nonce, signature, card, wallet_address}
  ↓
[S. SERVICE] Verify signature (cryptographic gate)
  ↓
[CYNIC?] Validate card (reputation gate)
  ↓
If valid: upload metadata to Arweave, sign Mint Permit (S.'s Ed25519)
  ↓
Return Permit to client
  ↓
Client submits Permit to Metaplex Core program
  ↓
Soulbound NFT minted on Pinocchio PDA
```

**CYNIC entry point:** After signature verification, before Arweave upload. Could be synchronous (block if verdict fails) or async (validate post-mint).

### Path 2: Post-Mint Audit (Reputation)

```
Soulbound NFT exists on-chain
  ↓
CYNIC queries: which wallets hold chess personality cards?
  ↓
For each wallet: fetch on-chain games metadata (from Arweave)
  ↓
Validate consistency: "This wallet claims N=47 games, archetype=Philosophe, confidence=0.72"
  ↓
Anti-Sybil check: is the wallet familiar? (CCM crystals on this wallet?)
  ↓
Verdict: HOWL (legitimate) / WAG (neutral) / BARK (suspicious)
  ↓
Feed verdict back to $ASDFASDFA or community reputation system
```

**CYNIC entry point:** Autonomous, separate from mint flow. Runs on-demand or scheduled.

---

## Architecture Debt & Unknowns

### Unknowns (S. to clarify)

1. **Confidence source:** Is the "confidence ≥ φ⁻¹" check done by S.'s classifier, or does S. call CYNIC?
2. **Ed25519 library:** Which Rust crate? (ed25519-dalek? sodium? libsodium FFI?)
3. **Arweave SDK:** Irys API auth + fee model? S. has an API key or free tier?
4. **Rate-limit mechanism:** Redis? In-memory? SurrealDB?
5. **Fallback if CYNIC is down:** Does mint proceed anyway, or block?

### Technical Angles (Where I can Help)

**Ed25519 / Cryptography:**
- Signature verification at `/mint-permit` gate
- Keypair generation for S.'s backend Ed25519 key
- Security audit: nonce + signature + wallet address cannot be replayed

**Arweave / Irys Integration:**
- Metadata upload format (JSON structure)
- Irys API client (likely `irys-sdk-rs` or HTTP)
- Fee estimation & bundling
- Metadata retrieval (for post-mint audit)

**Rate-limiting / Anti-Sybil:**
- Nonce LRU cache implementation (TTL=5min, O(1) lookup)
- Wallet-per-hour rate limiter (SurrealDB or memory)
- Metrics: how many mints per day should we expect? (capacity planning)

**CYNIC Token Judgment:** (existing, but applied to wallet-reputation domain)
- Current corpus: tokens (rug-pulls, honeypots)
- New domain: wallets holding personality cards (sybil-detection)
- Dogs will judge: "Is this wallet legitimate?" based on:
  - On-chain metadata consistency
  - Game count + archetype coherence
  - Confidence score distribution
  - CCM crystal history (if the wallet has prior verdicts)

---

## Proposed Work Sequence (This Week)

### Step 1: S. Clarifies Scope (Today/Tomorrow)

Slack responses to:
1. What does CYNIC validate? (crypto only, reputation, or both)
2. When? (pre-mint, post-mint, both)
3. Blocker surface on J6-7 (Ed25519 choice, Arweave SDK, rate-limit tech)
4. Help offer: which angles above would unblock you fastest?

### Step 2: Design Integration Interface (After Step 1)

- REST endpoint spec: what CYNIC receives from S., what it returns
- Error handling: what happens if CYNIC is down, slow, or returns BARK?
- Metrics: what gets logged for post-hackathon audit

### Step 3: Implement & Test (Apr 28 — May 4)

- S.: J6-7 Mint Permit Service (with integration hook to CYNIC or integration stub)
- Me: CYNIC token → wallet-reputation domain adapter + test corpus
- Both: end-to-end test (client → S. backend → CYNIC → blockchain)

### Step 4: Demo & Submit (May 4-11)

- Live wallet mints a Personality Card
- CYNIC validates it (shows verdict on video)
- Card appears on-chain

---

## Decision Gate: What Actually Blocks?

**If S. says "CYNIC is only for post-mint audit, mint doesn't block on CYNIC verdict":**
- J6-7 Sprint can proceed fully independent
- CYNIC work is async, can happen in parallel
- Integration is lightweight (CYNIC queries Arweave, makes verdicts)

**If S. says "CYNIC must validate before mint (synchronous)":**
- CYNIC latency becomes critical (Dogs timeout budget)
- Error handling path is hard (what if Dogs fail?)
- Need to test end-to-end before May 4

---

## Next: Await S.'s Response

Once S. clarifies above, we update this doc and baseline the work.

**Questions for S. on Slack:**
1. Validation scope (crypto / reputation / both)?
2. Timing (pre / post / both)?
3. J6-7 blockers + help needed?

Confidence on full integration: **0.40** (inferred, pending clarification).
