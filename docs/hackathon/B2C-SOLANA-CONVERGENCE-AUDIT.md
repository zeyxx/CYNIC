# B&C + T.'s Solana Code — Convergence Audit

**Status:** Analyzing commits + code structure to map where T.'s prior work converges with B&C J6-7  
**Updated:** 2026-04-25  
**Confidence:** 0.55 (observed commits + code structure)

---

## Current B&C Architecture (from commits + code)

### Frontend (J1-3)
```
Local Game → Zustand Store (ring buffer) → Signals (6 dimensions) → Classifier → PersonalityResult
                                                                                      ↓
                                                                                 Archetypes (9)
                                                                              Confidence (Q-Score)
```

**Tech:**
- Zustand (state)
- localStorage (`bc:local-games:v1`)
- Client-side Stockfish WASM
- Phantom/Solflare wallet integration (Exists: commit 434f41e)

### Backend (Current — `/api/personality`)
```
GET /api/personality
  ↓
  requireAuth()
  ↓
  findUserGamesForPersonality(userId)
  ↓
  computeSignals(games, userId)
  ↓
  classify(signals)
  ↓
  Return PersonalityResult
```

**Tech:**
- Next.js App Router
- Prisma (Game repository)
- Rate-limit gate (IP-based)

**Location:** `apps/web/src/app/api/personality/route.ts`

### Missing (J6-7)
No commits mention: `/nonce`, `/mint-permit`, `arweave`, `irys`, `ed25519`, `metaplex`

**Inference:** J6-7 is a greenfield sprint. S. is building from scratch (or has unpushed code).

---

## T.'s Solana Code (from GASdf audit)

### High-Confidence Patterns (Existing Code)

#### 1. **Nonce Management**
**GASdf pattern:** Anomaly detector + Redis
```typescript
// GASdf: src/services/anomaly-detector.js
// Detects: rate limits, nonce reuse, replay attacks
```

**Convergence for B&C J6-7:**
- `/nonce` endpoint: generate nonce, store in cache (Redis or in-memory)
- TTL=5min (implement LRU eviction)
- Return: `{ nonce, ttl, expiresAt }`

**T. can contribute:** LRU cache pattern + rate-limit logic

#### 2. **Ed25519 Signature Verification**
**GASdf pattern:** tweetnacl library
```typescript
// GASdf: src/services/validator.js, line 4
const nacl = require('tweetnacl');

// Verify fee payer signature on transaction
// Pattern: deserialize tx → extract signers → validate against expected feePayer
```

**Convergence for B&C J6-7:**
- Client: signs `{nonce, gameData, walletAddress}` with Ed25519 (browser)
- Backend: verify signature at `/mint-permit` gate
- Reject if: signature invalid OR nonce expired OR nonce already used

**T. can contribute:** Signature verification logic (adapt validator.js)

#### 3. **Rate-Limiting + Anti-Sybil**
**GASdf pattern:** Holder tiers + φ-weighted E-Score
```typescript
// GASdf: src/services/holder-tiers.js (146-153)
// Holder discount = log₁₀(share) + 5 / 3

// GASdf: src/services/harmony.js (126-131)
// E-Score = 1 - φ^(-E/25) — φ-weighted engagement scoring
```

**Convergence for B&C J6-7:**
- S. requires: N≥5 games + confidence≥φ⁻¹ + nonce valid + rate-limit 1/wallet/h
- **T. already has** φ-weighting logic from GASdf E-Score

**T. can contribute:** φ-weighted scoring architecture for anti-sybil gate

---

### Medium-Confidence Patterns (Partial)

#### 4. **Transaction Size Validation**
**GASdf pattern:** Pre-validation before submission
```typescript
// GASdf: src/services/validator.js (97-115)
// Check: size ≤ Solana limit (1232 bytes)
```

**Convergence for B&C J6-7:**
- If B&C builds Metaplex tx on backend, needs size validation
- If B&C builds on client, less relevant

#### 5. **Solana RPC + Circuit Breaker**
**GASdf pattern:** Multi-endpoint failover
```typescript
// GASdf: src/utils/rpc.js
// Failover pool + circuit breaker on RPC health
```

**Convergence for B&C J6-7:**
- `/mint-permit` needs to submit tx to blockchain
- Needs: RPC endpoint pool + timeout handling

**T. can contribute:** RPC failover pattern

---

### Low-Confidence / Not Found

#### 6. **Arweave / Irys Integration**
- **T.'s code:** Not found in GASdf or other audited repos
- **Status:** Learning needed OR S. has it locally

#### 7. **Metaplex Core NFT Minting**
- **T.'s code:** Not found
- **Status:** Learning needed OR S. has it locally

#### 8. **Nonce LRU Cache Specifics**
- **GASdf pattern:** Generic rate-limit, not specifically LRU with 5min TTL
- **T. can adapt:** Yes, but needs design from S.

---

## Where T.'s Code Converges with B&C J6-7

```
┌─────────────────────────────────────────────────────────────────┐
│                    B&C Personality Card Flow                     │
└─────────────────────────────────────────────────────────────────┘

STEP 1: User plays 5+ games locally
          ↓
STEP 2: Client requests nonce
          ↓
  ┌─────────────────────────────────────────────────────┐
  │ POST /nonce                   ← T. Can Help         │
  │ Returns: {nonce, ttl=5min}    (pattern exists)      │
  └─────────────────────────────────────────────────────┘
          ↓
STEP 3: Client signs {nonce, personality, wallet} with Ed25519
          ↓
STEP 4: Client submits to /mint-permit
          ↓
  ┌─────────────────────────────────────────────────────┐
  │ POST /mint-permit             ← T. Can Help         │
  │   Verify signature (tweetnacl)   (validator.js)     │
  │   Verify nonce not used (LRU)    (anomaly-detector) │
  │   Verify N≥5 games              (existing)          │
  │   Verify confidence≥φ⁻¹          (existing)         │
  │   Check rate-limit 1/wallet/h    (T. has pattern)   │
  │                                                     │
  │   If valid:                                         │
  │   - Upload metadata to Arweave/Irys  ← S.?          │
  │   - Sign Mint Permit (Ed25519)       ← T. Can Help  │
  │   - Return permit to client                         │
  └─────────────────────────────────────────────────────┘
          ↓
STEP 5: Client submits permit to Metaplex Core program ← S.?
          ↓
STEP 6: Soulbound NFT minted on-chain
```

---

## Concrete Help Offer (Based on Real Code)

### T. Should Offer to S.:

**Angle 1: Nonce + Rate-Limit Gateway**
- Pattern: GASdf anomaly-detector.js
- Implement: `/nonce` endpoint with LRU cache (TTL=5min)
- Implement: 1-wallet-per-hour check
- Effort: ~2-3 hours

**Angle 2: Ed25519 Signature Verification**
- Pattern: GASdf validator.js (line 4, tweetnacl)
- Implement: Verify signature({nonce, gameData, wallet})
- Implement: Reject if signature invalid OR nonce expired
- Effort: ~1-2 hours

**Angle 3: Rate-Limit + Anti-Sybil Scoring**
- Pattern: GASdf holder-tiers.js + harmony.js
- Implement: φ-weighted anti-sybil gate (N≥5, confidence≥φ⁻¹)
- Effort: ~1 hour (mostly copied from GASdf)

**Angle 4: RPC Failover (if needed)**
- Pattern: GASdf RPC pool + circuit breaker
- Implement: Robust Solana connection for Arweave upload / NFT mint
- Effort: ~2-3 hours

**Total:** ~6-9 hours of T.'s time (if S. agrees these angles are blockers)

### T. Should Ask S.:

1. **Do you have Arweave/Irys SDK ready?** (If not, T. needs to learn it)
2. **Do you have Metaplex Core minting code?** (If not, T. needs to learn it)
3. **Which of the above angles (1-4) are blockers for your sprint?**

---

## Document: B2C-CYNIC-INTEGRATION-SPECIFICATION.md

Already created. Will be updated once S. clarifies scope.

---

## Confidence Breakdown

| Element | Confidence | Why |
|---------|------------|-----|
| Nonce management pattern | 0.75 | GASdf has it, pattern is clear |
| Ed25519 verification | 0.80 | tweetnacl library, validator.js is concrete |
| Rate-limiting | 0.70 | GASdf has it, but LRU specifics need design |
| Anti-sybil φ-weighting | 0.65 | GASdf has φ-weighted discounts, not direct anti-sybil |
| Arweave/Irys convergence | 0.25 | Not in T.'s code, unknown if S. has it |
| Metaplex convergence | 0.30 | Not in T.'s code, unknown if S. has it |
| Overall feasibility (T. + S. pairing) | 0.60 | Depends on S.'s existing code + clarity |

---

## Next: Await S.'s Response

Once S. answers on Slack:
1. Which of angles 1-4 are blockers?
2. Do you have Arweave/Irys + Metaplex code?

Then we know if T. is helpful async or needs pairing.
