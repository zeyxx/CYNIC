# Hackathon Ground Truth — 2026-04-30

> **Crystallized reality:** Two separate competitions, same platform, different gates. This doc resolves all prior confusion.

---

## The Two Competitions

Both run on **Colosseum** (solana-colosseum.com or equivalent) — Solana Foundation hackathon platform with 2.75M in prizes.

### 1. CYNIC — Colosseum Frontier (Token Screener)
- **What:** Epistemic oracle for Solana token safety. Five Dogs (deterministic, qwen7b, qwen35b-gpu, qwen35b-core, gemini-cli) judge tokens on six axioms.
- **Owner:** T. (zeyxx)
- **Feature Freeze:** April 27, 2026 ← **kernel code locked, submission continues**
- **Submission Deadline:** May 10/11, 2026
- **Submission Requirements:**
  - Video demo (2-3 min) showing Dogs deliberating + verdict
  - GitHub link
  - Deployed URL (Vercel UI + kernel tunnel)
  - Written description
- **Status (as of April 30):**
  - Kernel: stable, /judge endpoint live, all Dogs operational
  - UI: Vercel deployed, Cloudflare tunnel active
  - Pinocchio contract: devnet live, axioms on-chain
  - Video: script drafted, not yet recorded
- **Blocker:** None. Ready to demo + submit.

### 2. Blitz & Chill — Chess Reputation Primitive
- **What:** Personality card minting based on chess game history. Nine archetypes derived from 6 signals (time, variance, aggression, resign, length, opening).
- **Owner:** S. (Ragnar-no-sleep, via GitHub Ragnar-no-sleep/blitz-and-chill)
- **Registration Deadline:** May 4, 2026 ← **HARD GATE. No extensions. After this = ineligible.**
- **Submission Deadline:** May 11, 2026
- **Registration Mechanism:** Must be registered on Colosseum platform by May 4 23:59 to compete.
- **Why May 4?** Colosseum runs separate competition tracks. B&C is a track with early registration (possibly to coordinate team formation). CYNIC is a different track with May 10/11 submission window.
- **Status (as of April 25 report):**
  - 19/19 integration tasks complete
  - Ready to register
  - Wallet integration deployed
  - All endpoints live
- **Integration with CYNIC:** B&C calls CYNIC /judge endpoint to validate wallet authenticity for Personality Card mints (Option C anti-Sybil gate). Score ≥ φ⁻¹ (0.618) required to mint.

---

## Timeline (Consolidated)

| Date | Event | Owner | Impact |
|------|-------|-------|--------|
| **Apr 27** | CYNIC kernel feature freeze | T. | Code locked; submission work continues |
| **May 1-4** | (Optional) S. integrates CYNIC /judge call | S. | If not done: fallback to Ed25519-only anti-Sybil |
| **May 4 23:59** | **Blitz & Chill Registration Hard Gate** | Colosseum | After this: B&C ineligible for awards |
| **May 4 EOD** | T. ships wallet-judgment domain (if not done) | T. | Unblocks S. optional integration |
| **May 10 23:59** | CYNIC submission deadline | T. | Must submit video + description |
| **May 11** | Both submission deadlines close | T. + S. | Final submission processing |

---

## Dependency Map

```
CYNIC (T.)
├─ Kernel: stable ✓
├─ /judge endpoint: live ✓
├─ Pinocchio contract: devnet ✓
├─ Video demo: [NOT YET RECORDED]
├─ Submission: [NOT YET SUBMITTED]
└─ Dependencies: None blocking

Blitz & Chill (S.)
├─ Core: 19/19 tasks ✓
├─ Registration: [NOT YET DONE]
│   └─ Hard gate: May 4 23:59
├─ (Optional) Integration: CYNIC /judge for wallet anti-Sybil
│   ├─ Requires: T. ships wallet-judgment (Apr 28 EOD preferred)
│   ├─ Fallback: Ed25519-only (fully solo-capable)
│   └─ Timeline: May 1-4 optional window
└─ Submission: [NOT YET SUBMITTED]

Colosseum Platform
└─ Two separate competition tracks
    ├─ Frontier (CYNIC): May 10/11 submission
    └─ Blitz & Chill: May 4 registration + May 11 submission
```

---

## What Needs to Happen (for T. and S.)

### T. (CYNIC)
1. **Immediate (before May 10):** Record video demo
   - Script exists: 4 scenes, ~2 min
   - Setup: tunnel active, kernel live, Dogs responding
   - Record when rested (no artifacts)
2. **Before May 10 23:59:** Submit to Colosseum
   - Video + GitHub link + Vercel URL + description
3. **Optional (before Apr 28 EOD):** Ship wallet-judgment to CYNIC
   - Enables S.'s optional integration
   - If not shipped: S. falls back to Ed25519-only (still ships on time)

### S. (Blitz & Chill)
1. **Before May 4 23:59:** Register on Colosseum platform
   - This is the hard gate
   - If missed: project ineligible, no do-over
2. **May 1-4 (optional):** Integrate CYNIC /judge for wallet anti-Sybil
   - If T. ships wallet-judgment by Apr 28: proceed with integration
   - If not: use Ed25519-only fallback (fully capable solo)
3. **Before May 11 23:59:** Submit to Colosseum

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| S. misses May 4 registration | **CRITICAL** | Mark calendar now. Confirm Colosseum platform access. |
| T. doesn't record video by May 10 | Medium | Video is non-blocking submission component. Can submit text-only if needed (check Colosseum rules). |
| T. doesn't ship wallet-judgment by Apr 28 | Low | S. falls back to Ed25519-only. Still ships on time. |
| CYNIC integration not ready by May 4 | Low | B&C is fully solo-capable without it. Ships either way. |

---

## Reality Check

**Two separate competitions, not one.**

- CYNIC (T.) is judged on: token screening capability, Dogs voting consensus, on-chain proof, axiom soundness
- B&C (S.) is judged on: chess personality prediction, wallet anti-Sybil, user UX

They can integrate (B&C calls CYNIC for wallet validation), but they're independent projects entering different competition tracks.

**May 4 is NOT a shared deadline.** It's B&C's registration gate. CYNIC has until May 10.

---

## Next Steps (Decision)

**For T. (CYNIC):**
- Confirm: will you record video before May 10?
- Confirm: will you ship wallet-judgment by Apr 28 EOD (to help S.)?

**For S. (Blitz & Chill):**
- **ACTION ITEM:** Register on Colosseum platform (May 4 hard gate)
- Decide: will you integrate CYNIC /judge (May 1-4 window)?
  - If yes: wait for T. to ship wallet-judgment (Apr 28 EOD)
  - If no: proceed with Ed25519-only (solo-capable)

