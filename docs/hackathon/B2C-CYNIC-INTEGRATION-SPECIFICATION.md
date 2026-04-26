# Blitz & Chill ↔ CYNIC Integration Specification

**Purpose:** S. (Blitz & Chill backend) calls CYNIC to validate wallet authenticity for Personality Card mints (Option C anti-Sybil).

**Status:** Updated 2026-04-25. Reflects wallet-judgment domain + deterministic/LLM Dogs.

---

## Overview

**Option C Personality Card Mint Flow:**

User clicks "Mint Card" → Generate nonce → Sign (Ed25519) → POST /mint-permit → B&C verifies sig → Lookup games (≥5) → Call CYNIC /judge (wallet authenticity) → If CYNIC ≥ φ⁻¹ (0.618): rate-limit (1 mint/hr) + upload Arweave + return permit. Else: 403 Forbidden.

---

## Endpoint: POST `/judge` (CYNIC) — Wallet Authenticity

**Request:**
```json
{
  "stimulus": {
    "content": "Validate wallet for Personality Card mint",
    "domain": "wallet-judgment",
    "context": "WALLET_ENRICHMENT_CONTEXT_HERE",
    "wallet_address": "G2gCo4Hx2Pq8Rm9Nz3Kp0Lk5Jq7Mn2Vt"
  },
  "inject_crystals": true
}
```

**Enrichment Context (B&C builds from LocalCompletedGame):**

```
WALLET PROFILE
Address: G2gCo4Hx2Pq8Rm9Nz3Kp0Lk5Jq7Mn2Vt
Games completed: 8
Wallet age: 24 days
Archetype consistency: 85%

GAME HISTORY
Archetypes: ["The Aggressive", "The Aggressive", "The Pragmatist", ...]
Game timestamps: ["2026-04-10T14:32:00Z", "2026-04-11T15:45:00Z", ...]
Game durations (seconds): [342, 287, 405, ...]
Average: 326s, Variance: 0.18

OPENING REPERTOIRE
Hash: 0x3f7e2a1b5c8d9e4f...
Unique openings: 6

MOVE SEQUENCE
Hash: 0x7c2a9e1f3b8d4e5g...
Replay risk: false

FLAGS
Suspicious clustering: false
Age < 5 days: false
Duration variance > 0.50: false
```

**Response:**
```json
{
  "verdict": { "kind": "WAG", "q_score": 0.61 },
  "dogs": [
    { "name": "deterministic-dog", "verdict": "WAG", "q_score": 0.62 },
    { "name": "qwen-7b-hf", "verdict": "HOWL", "q_score": 0.65 },
    { "name": "qwen35-9b-gpu", "verdict": "WAG", "q_score": 0.58 },
    { "name": "gemma-4-e4b", "verdict": "GROWL", "q_score": 0.55 },
    { "name": "gemini-cli", "verdict": "WAG", "q_score": 0.63 }
  ],
  "consensus": {
    "mean_q_score": 0.61,
    "option_c_gate": "PASS"
  }
}
```

---

## Integration Checklist for J6-7 (B&C)

- [ ] **Step 1:** Build /nonce endpoint (32-byte random, 5min TTL)
- [ ] **Step 2:** Build /mint-permit endpoint (Ed25519 verify + games ≥5 + rate-limit 1/hr + Arweave upload)
- [ ] **Step 3:** Call CYNIC /judge from /mint-permit (optional for J6-7 — fallback to Ed25519-only)
- [ ] **Step 4:** Rate-limit enforcement per wallet per hour
- [ ] **Step 5:** Arweave metadata upload and URI return

**Fallback (if CYNIC not ready):** Validate Ed25519 sig + check games ≥5, ship without anti-Sybil. Add CYNIC gating post-hackathon (May 12+).

---

## Timeline

| Date | Milestone |
|------|-----------|
| Apr 25 | T. ships wallet-judgment domain + Dogs |
| Apr 26-27 | S. builds /nonce + /mint-permit (Ed25519 + Arweave) |
| Apr 28 | T. optionally deploys wallet-judgment to CYNIC |
| May 1-4 | S. optionally integrates CYNIC /judge call |
| May 4 | Registration hard gate |
| May 11 | Submission deadline |

**Critical path:** J6-7 solo-capable (Ed25519 is ~1 day). CYNIC integration async (optional before May 1).

---

## Next Sync with S.

- Confirm J6-7 scope (solo = yes, Ed25519-only?)
- Decide: integrate CYNIC now or fallback to Ed25519?
- If integrating: T. ships wallet-judgment before Apr 28 EOD.
