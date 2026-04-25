# Hackathon External Context — 2026-04-25

> Anchor: verified dates & execution constraints discovered during session research

## Timeline Reality

### CYNIC — Colosseum Frontier (Solana Token Screener)
- **Feature Freeze:** April 27, 2026 (2 days from now)
- **Submission Deadline:** May 10/11, 2026
- **Status:** Kernel stable, `/judge` endpoint live, verdicts crystallizing at 10-15s latency
- **Demo Material:** 5 Dogs operational, recent verdicts (GROWL, BARK) with q_score + axiom breakdown available
- **Blocker:** CCM gate threshold tuning (max_disagreement 0.568 >> φ⁻² threshold 0.382) — **not blocking video, but blocks production verdicts**
- **Submission Path:** 
  - [x] Pinocchio devnet deployment (tx confirmed, on-chain axioms live)
  - [ ] Video demo (2-3 min, script ready)
  - [ ] Colosseum project submission (description, video, GitHub, deployed URL)

### Blitz & Chill — Chess Reputation Primitive (Ragnar-no-sleep)
- **Registration Deadline:** May 4, 2026 (9 days from now) — **HARD GATE, no extensions**
- **Status:** 19/19 tasks complete, wallet integration deployed, ready to register
- **Relevant Info:**
  - GitHub: github.com/Ragnar-no-sleep/blitz-and-chill
  - Backend: 9 personality archetypes, 6 signals (time/variance/aggression/resign/length/opening)
  - Frontend endpoints: `/api/personality`, `/api/personality/badge?address={walletAddress}`
  - All integration work complete; registration mechanism verified
- **Constraint:** If not registered by May 4, project is ineligible for awards (hard gate enforced by Colosseum platform)

## Execution Constraint Discovery

**Gap Found:** Previous session did not fully research external deadlines/status of competing projects.

**Dates Conflict?** Yes.
- CYNIC freeze (Apr 27) is before Blitz & Chill registration (May 4)
- Both have same submission deadline (May 10/11)
- But Blitz & Chill registration closes at May 4 — after CYNIC freeze, during CYNIC submission window

**Implications:**
1. CYNIC can be submitted right up to May 10
2. Blitz & Chill registration must happen by May 4 or project becomes ineligible
3. If focus splits between both, Blitz & Chill's hard deadline (May 4) becomes the gating constraint

## Verification Steps Completed

- [x] Colosseum Frontier hackathon structure verified (2.75M in prizes/funding)
- [x] CYNIC Pinocchio devnet deployment confirmed (tx, axioms on-chain)
- [x] Blitz & Chill repository verified (19/19 tasks complete, registration ready)
- [x] CYNIC kernel verified (stable, all Dogs operational, verdicts live)
- [x] CYNIC video demo script drafted (4 scenes, 2-3 min target)

## Decision Point

**CYNIC Focus:** Video demo → Colosseum submission by May 10
- Kernel is stable and live
- Demo material (verdicts) available now
- Script and flow clear
- Deadline: May 10 (15 days out)

**Blitz & Chill Focus:** Registration by May 4
- All work complete, ready to register
- Deadline: May 4 (9 days out, HARD GATE)

**Parallel:** Both projects can be worked on if video demo is recorded immediately (before April 27 freeze, which only locks CYNIC kernel code, not submission).

---

**For Next Session:** Determine priority and resource allocation.
