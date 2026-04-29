# To S. (Blitz & Chill) — Wallet-Judgment Integration Data Request

**From:** T. (CYNIC)  
**Date:** 2026-04-30  
**Re:** Optional wallet-judgment integration for B&C Personality Card anti-Sybil gate  
**Deadline:** May 1 23:59 UTC (S. needs data by this time to integrate by May 4 registration)

---

## Status: Code Ready, Untested on Real Data

CYNIC's wallet-judgment domain is **code-complete and production-ready**, but has **never been tested with real game history data from B&C**. The implementation includes:

- ✅ Deterministic algorithm (anti-Sybil scoring based on 6 game signals)
- ✅ 11 unit tests passing on synthetic data
- ✅ Pipeline integration in place (fast-path, 0ms latency)
- ❌ **No integration test with real B&C WalletProfile data**

---

## What We Need (To Validate Integration)

**3-5 sample WalletProfile JSON objects** from actual game histories. These serve as integration test data.

### Example Structure

```json
{
  "wallet_address": "G2gCo4Hx2Pq8Rm9Nz3Kp0Lk5Jq7Mn2Vt",
  "games_completed": 8,
  "archetype_consistency": 0.85,
  "wallet_age_days": 24,
  "average_game_duration": 326,
  "duration_variance": 0.18,
  "opening_repertoire_hash": "0x3f7e2a1b5c8d9e4f...",
  "move_sequence_hash": "0x7c2a9e1f3b8d4e5g...",
  "suspicious_cluster": false,
  "replay_risk": false
}
```

**From where:** Extract from B&C LocalCompletedGame records or in-memory game state. You have this data already.

**How many:** 3-5 examples, ideally:
- 2-3 legitimate players (high archetype consistency, normal variance, 8+ games)
- 1-2 suspicious profiles (low consistency, high variance, or sybil markers like replay_risk=true)

---

## What T. Will Do (With Your Data)

1. **Parse** the JSON samples into WalletProfile structs
2. **Call** `deterministic_dog(profile)` on each
3. **Verify** the output verdicts are sensible:
   - Legitimate players: WAG or GROWL (score ≥ φ⁻¹ = 0.618)
   - Suspicious players: BARK (score < 0.382)
4. **Add integration test** to `cynic-kernel/tests/` so the verdict is automated
5. **Confirm it works** (test passes)

**This takes ~30 minutes once you provide data.**

---

## Your Options

### Option A: Send Data (Recommended)
- Provide 3-5 WalletProfile JSON samples by **May 1 23:59**
- T. validates, adds integration test, confirms working
- You integrate CYNIC /judge call into B&C /mint-permit by May 1-4 (optional)
- Submit with full anti-Sybil gate enabled

### Option B: Skip Integration (Fallback)
- You proceed with Ed25519-only anti-Sybil (wallet signature verification + games ≥5 gate)
- No CYNIC /judge call needed
- Code is fully capable solo, ships on time (no new blocker)
- Option to integrate post-hackathon (May 12+)

**Either way, you're not blocked. Option A just adds confidence.**

---

## Timeline

| Date | Milestone |
|------|-----------|
| **May 1 23:59** | S. sends 3-5 WalletProfile JSON samples (or signals "skip integration") |
| **May 1-4** | (Optional) T. validates + S. integrates CYNIC /judge call |
| **May 4 23:59** | **HARD GATE: B&C must register on Colosseum platform** |
| **May 11 23:59** | Submission deadline |

---

## Next Step

**Reply with:**
1. Yes, sending data by May 1 → provide JSON samples
2. No, using Ed25519-only → confirm, proceed with core integration

This unblocks both projects.

