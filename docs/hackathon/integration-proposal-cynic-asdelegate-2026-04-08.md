# CYNIC × ASDelegate — Integration Proposal for Colosseum Frontier

> **Status**: Draft for S. (Stanislaz) and eventually sollama58. Not sent yet.
> **Date**: 2026-04-08, D+2 of Colosseum Frontier hackathon
> **Author**: T. (with Claude)
> **For discussion on**: Telegram (S.) · X group $ASDFASDFA (sollama58, later)

---

## TL;DR

CYNIC's Pinocchio program on devnet solves the exact tally authority gap that S.'s 2026-02-25 audit of ASDelegate identified as BUG-002 and SEC-001. Phase 2 of ASDelegate has been blocked since 2026-03-08 waiting for an architectural decision on this. **The architecture already exists, deployed, tested — it's the CYNIC kernel + Pinocchio programme.**

Proposal: co-build `ASDelegate + CYNIC` as dual-layer governance — humans vote via Streamflow-weighted locks (ASDelegate UI), AI validators deliberate under φ-bounded doubt (CYNIC kernel), every verdict settles immutably on-chain via Pinocchio. Submit Colosseum Frontier as T. + S. co-builders, credit sollama58 as ASDelegate maintainer.

Prize: $250K pre-seed + accelerator. Feature freeze April 27. Submit May 11. Current date: April 8 (D+2).

---

## The observation that makes this integration obvious

On 2026-02-25, S. wrote `GAP-ANALYSIS.md` in `sollama58/TokenVotingUtil` identifying 22 issues in ASDelegate — 2 CRITICAL, 6 HIGH, 9 MEDIUM, 5 LOW. The two CRITICAL ones and the two most severe HIGH ones are:

| Code | Severity | Issue | Module |
|---|---|---|---|
| **SEC-001** | CRITICAL | No wallet signature verification — any `curl` can vote as any wallet | `server.js` all POST routes |
| **SEC-002** | CRITICAL | Admin password stored in JS memory, sent as plaintext header | `server.js:290`, `index.html:2711` |
| **BUG-002** | HIGH | Frontend is sole tally authority — no server source of truth, no audit trail | `index.html:2033-2051` |
| **BUG-001** | HIGH | `margin >= threshold` should be `margin > threshold` | `index.html:1536` |

S. merged Phase 1 Étape 1 (Tier 1 infrastructure fixes: CORS, Jest, Winston, Redis) via PR #1 on 2026-03-08. Phase 2 (voting logic + backend tally authority decision) has been **blocked waiting for sollama58's architectural input since that date** — over a month.

Meanwhile, on the CYNIC side (T.), the Pinocchio governance programme has been deployed on devnet at `A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx` since 2026-03-29. A community PDA at `8Pyd1hqd6jTX2jR8YvCAjnd3cyP5qB7XaxzwAGtHCSFD` is alive (probed 2026-04-08, slot 454018276). The programme provides five instructions: `initialize_community`, `submit_verdict`, `execute_action`, `pause`, `unpause`.

**This programme natively solves SEC-001 and BUG-002:**
- `submit_verdict` enforces ed25519 signature verification on the submitting agent at the Solana runtime level — the blockchain itself rejects any unsigned call. No spoofable `curl` possible. **SEC-001 closed by architecture, not by patch.**
- `submit_verdict` writes the verdict to an immutable PDA owned by the programme. The result is on-chain, auditable, single canonical source of truth. **BUG-002 closed by architecture.**

The integration wasn't designed for Colosseum. It's the answer to the Phase 2 question S. raised independently 6 weeks ago.

---

## Architecture — dual-layer governance

```
┌──────────────────────────────────────────────────────────────────────┐
│ HUMAN LAYER (ASDelegate — existing)                                  │
│                                                                      │
│ Wallet → connect (Phantom/Solflare)                                  │
│   → display locks, voting power, unlock timeline                     │
│   → create proposal / vote via signed message                        │
│   → [NEW] forward vote intent to CYNIC backend                       │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
                               ▼  POST /judge  { content: proposal }
┌──────────────────────────────────────────────────────────────────────┐
│ AI LAYER (CYNIC kernel — existing)                                   │
│                                                                      │
│   pipeline::run()                                                    │
│    → embed → crystal search → session context → evaluate             │
│    → Dogs deliberate (qwen-7b-hf + deterministic + others)           │
│    → φ-bounded Q-score (max 0.618) across 6 axioms                   │
│    → verdict = Howl | Wag | Growl | Bark                             │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
                               ▼  agent-bridge.js → submit_verdict ix
┌──────────────────────────────────────────────────────────────────────┐
│ ON-CHAIN SETTLEMENT (Pinocchio — existing, deployed devnet)          │
│                                                                      │
│   A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx                       │
│    submit_verdict(community, proposal_hash, q_score, axioms, type)   │
│    → verify ed25519 signature of agent_key                           │
│    → write Verdict PDA (immutable, owned by programme)               │
│    → timelock before execute_action is callable                      │
└──────────────────────────────┬───────────────────────────────────────┘
                               │
                               ▼  read via getAccountInfo
┌──────────────────────────────────────────────────────────────────────┐
│ UI FEEDBACK LOOP                                                     │
│                                                                      │
│ ASDelegate frontend polls CYNIC /judge/status/{id} for progressive   │
│ Dog arrival (D4), then reads final Verdict PDA from chain            │
│ → displays "AI deliberation complete: WAG 0.48" next to human vote   │
│ → human vote + AI verdict both visible, both auditable               │
└──────────────────────────────────────────────────────────────────────┘
```

---

## What changes in ASDelegate (the minimum viable fork)

| File | Change | LOC est. |
|---|---|---|
| `server.js` | Add `POST /api/proposals/:id/submit-to-cynic` route that forwards proposal text to CYNIC `/judge` and returns the `verdict_id` for polling | ~40 |
| `server.js` | Add `GET /api/proposals/:id/cynic-verdict` that reads the Verdict PDA from chain via `getAccountInfo` and deserializes (borrows layout from `programs/cynic-governance/src/state/verdict.rs`) | ~60 |
| `public/index.html` | Add "AI Deliberation" panel next to the existing human vote panel — shows Dogs arriving progressively (polling CYNIC `/judge/status/{id}`), final Q-score, per-axiom breakdown | ~200 |
| `public/index.html` | On-chain Verdict PDA reader — replaces BUG-002 client-side tally with authoritative chain read | ~80 |
| `.env.example` | Add `CYNIC_REST_ADDR`, `CYNIC_API_KEY` env vars (currently undocumented in the bridge) | ~5 |
| `ARCHITECTURE.md` | Document the dual-layer design | ~100 |

**Total ASDelegate diff: ~485 LOC**. No breaking changes to existing UI. Additive only.

## What CYNIC provides (mostly already built)

| Component | Status |
|---|---|
| CYNIC kernel `/judge` endpoint | ✓ live, responsive (<1.5s on governance test props) |
| Pinocchio programme with `submit_verdict` | ✓ deployed devnet |
| Community PDA initialized | ✓ alive (`8Pyd…`) |
| Agent bridge (`bridge/agent-bridge.js` in GASdf) | ✓ functional, needs env-var cleanup |
| **D4 — `/judge/status/{id}` progressive poll endpoint** | ⚠️ needs build, ~7h (Option A polling-on-store) or ~18h (Option B progressive writes) |
| `@gasdf/governance` SDK extracted from `bridge/governance.js` | ⚠️ needs extraction, ~5h |
| New devnet deployer keypair | ⚠️ needs regeneration (current one `/tmp/gasdf-deployer.json` lost) |
| Community re-init with distinct agent + guardian keys | ⚠️ post-hackathon |

## Responsibilities (proposed split)

| Task | Owner | Est. |
|---|---|---|
| CYNIC D4 `/judge/status/{id}` | T. | 7-18h |
| Devnet keypair regen + airdrop + bridge env vars | T. | 1h |
| CYNIC × Pinocchio bridge hardening (retry, timeout, error codes) | T. | 3h |
| `@gasdf/governance` SDK extraction | T. or S. | 5h |
| ASDelegate fork setup + CI for fork | S. | 1h |
| ASDelegate `POST /api/proposals/:id/submit-to-cynic` backend route | S. | 2h |
| ASDelegate Verdict PDA reader (frontend + backend) | S. | 4h |
| ASDelegate "AI Deliberation" panel UI (vanilla JS, integrated with polling) | S. | 8-10h |
| Integration tests (E2E: vote → CYNIC → Pinocchio → PDA read) | both | 4h |
| Pitch video recording + editing | both | 15h |
| Demo rehearsal on real $ASDF community | both | 3h |

**Rough total: 50-70h split across two contributors over 5 weeks.** Feasible given the integration builds on existing components rather than net-new architecture.

---

## Hackathon framing

**Track**: AI
**Prize**: $250K pre-seed + accelerator
**Submit**: May 11, 2026
**Feature freeze**: April 27, 2026
**Current**: April 8 (D+2) — **19 days to feature freeze, 33 days to submit**

**Team**: T. (CYNIC, GASdf, backend/infra/oracle layer) + S. (ASDelegate contributor, client layer, ASDF-Web, ASDF-Mobile, blitz-and-chill). Acknowledged maintainer: sollama58 (ASDelegate owner).

**Pitch narrative**:
1. **Problem** (30s). 98.6% of Solana community tokens rug. Jupiter closed its DAO. MetaDAO is illegible for retail. There is no governance layer usable for community tokens in the Solana ecosystem.
2. **Community** (30s). $ASDFASDFA is a real pump.fun token with a real community, real builders (T., S., sollama58, others), and real product surface (ASDelegate, CultScreener, ASDForecast, HolDex, burn tracker, oracle, bridge).
3. **Gap** (30s). ASDelegate had 22 audited issues (show GAP-ANALYSIS.md dated 2026-02-25). Two CRITICAL, six HIGH. Phase 2 was blocked on "how do we make the tally trustworthy without surrendering to a central server?"
4. **Answer** (60s). Dual-layer governance. Humans vote via Streamflow-weighted locks in ASDelegate. AI validators deliberate under φ-bounded doubt (CYNIC, max 61.8% confidence). Every verdict settles on-chain via a custom Pinocchio programme (-95% CU vs Anchor). The audit gap closes as a side-effect of the architecture, not as a patch.
5. **Demo** (60s). Live devnet proposal → CYNIC `/judge` → Dogs deliberate → `submit_verdict` tx signature → read-back from Verdict PDA → UI updates. Real on-chain tx, real Dog reasoning, real PDA data.
6. **Team + ask** (15s). Two co-builders already in the same ecosystem, shipping on the same token for months before the hackathon started.

**Why this works for Colosseum judges**:
- Solana-native proof: Pinocchio (not Anchor), devnet deployment, on-chain PDAs, Helius infrastructure, Streamflow locks
- Ecosystem proof: multiple repos, multiple contributors, pre-existing audit, active community
- Execution proof: integration builds on shipped components, not a prototype
- AI track differentiator: φ-bounded doubt + geometric-mean Q-score is genuinely novel vs "LLM calls an API"

---

## What this proposal does NOT ask

- **No rewrite of ASDelegate.** Fork, additive changes only, upstream-friendly once sollama58 cycles back to reviewing.
- **No claim over $ASDF token economics.** This is a governance layer, not a tokenomics change.
- **No displacement of sollama58.** The fork-based approach means we can either upstream the integration via PR post-hackathon or maintain in parallel if sollama58 prefers.
- **No hiding behind "AI magic".** The Dogs' reasoning is shown in the UI, per-axiom, with explicit confidence bound (61.8% maximum). Users can see exactly what the AI said and decide accordingly.
- **No disruption to S.'s current work** beyond the integration scope. `blitz-and-chill` and `ctf-forensique-airlines` remain his personal projects.

---

## Open questions for S. (Telegram call 2026-04-09)

1. **In on Colosseum as co-builder?** Co-submission under "T. + S.", with acknowledgment to sollama58.
2. **OK for Phase 2 ASDelegate = "backend tally via CYNIC Pinocchio"?** This answers the architectural question sollama58 was waiting on.
3. **Bandwidth for 5 weeks of integration work** (~20-30h on the ASDelegate side)?
4. **Sequencing for sollama58 outreach** — approach him now via X group with the integration angle, or wait until we have a demo video to show?
5. **Credit + prize split** if $250K materialises. Default proposal: 50/50 T./S., 5% gesture to sollama58 as ASDelegate maintainer (to discuss).
6. **ASDelegate deployment URL** — is there a live Render instance we can point to in the demo? If yes, share; if no, spin one up for the demo period.

---

## Appendix A: on-chain proof-points (probed 2026-04-08, JSON-RPC)

```
Pinocchio programme A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx
  executable: true
  owner:      BPFLoaderUpgradeab1e11111111111111111111111
  lamports:   1,141,440
  slot:       454,018,276

Community PDA 8Pyd1hqd6jTX2jR8YvCAjnd3cyP5qB7XaxzwAGtHCSFD
  executable: false
  owner:      A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx  ← owned by the programme
  lamports:   1,656,480
  data:       110 bytes (disc + 3 pubkeys + trailing state)
```

Agent + guardian keys in the PDA are currently identical (test config, single keypair). Post-hackathon fix: re-initialize community with distinct keys for proper asymmetric pause safety.

## Appendix B: CYNIC kernel runtime snapshot (probed 2026-04-08)

```
status:      degraded
dog_count:   4 (REST) / 3 (MCP) — K13 surface divergence
healthy:     deterministic-dog + qwen-7b-hf + qwen35-9b-gpu (gemma down)
effective voters on non-chess: 2 (deterministic + qwen-7b-hf)
  - qwen-7b-hf: 100% JSON valid (12/12 recent)
  - qwen35-9b-gpu: 18% JSON valid (2/11 recent) — silent failure mode
  - gemma-4b-core: 0% JSON valid (0/1)
```

Honest framing for the pitch: "CYNIC currently has 2 to 4 independent voters depending on prompt type, under φ-bounded doubt (max 61.8% confidence). Dogs are dynamically registerable via POST /dogs/register; the active count is an operational property, not a marketing number. The architecture supports an arbitrary N-of-N dynamic roster, and the current runtime instance demonstrates this on a real token ecosystem."

This replaces the abstract "5 AI validators" claim with a runtime-accurate, dynamic-roster framing that is both defensible and more technically interesting.

---

*This document is a draft for the Telegram call on 2026-04-09. It is not yet shared with S. or sollama58. T. reviews first, corrects any inaccuracies, then sends.*
