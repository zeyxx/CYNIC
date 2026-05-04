# CYNIC — Sovereign Token Judge
## Colosseum Frontier Submission (Updated May 4, 2026)

---

## Problem

Token evaluation on Solana is fragmented. Rugs, honeypots, and slow-bleeds are indistinguishable from legitimate projects using current tooling. Retail investors rely on contradictory signals: whale wallets, social media hype, audit reports from incentivized firms. No tool produces reproducible, auditable consensus on token safety.

## Solution

CYNIC is a **multi-Dog consensus engine** that judges Solana tokens on six indivisible axioms:

- **FIDELITY:** Is the claim faithful to truth and sound principles?
- **PHI:** Is the structure harmonious and proportional?
- **VERIFY:** Is the claim testable and refutable?
- **CULTURE:** Does it honor tradition and pattern?
- **BURN:** Is it efficient and minimal-waste?
- **SOVEREIGNTY:** Does it preserve agency and freedom?

Five independent Dogs score each token:
1. **Deterministic Dog** — heuristic rules (rug indicators, liquidity locks, holder concentration) ✓ **Online**
2. **Qwen 7B HF** — lightweight inference validator (HuggingFace) ✓ **Online**
3. **Qwen 3.5 9B Core** — sovereign CPU inference ✓ **Online**
4. **Qwen 3.5 9B GPU** — high-throughput GPU inference (55 tok/s on RTX 4060 Ti) ✓ **Online**
5. **Gemini CLI** — philosophical synthesis validator ⚠️ **Degraded (circuit-breaker open)**

Dogs vote independently. Consensus emerges as **φ-bounded confidence** (max q_score = 0.618 ≈ golden ratio inverse). This is not a bug—it's epistemic honesty. On controversial tokens (rugged projects, experimental DEXes, high-risk arbitrage), Dogs should diverge, not converge to false certainty. When three independent validators agree a token is HOWL, we report φ⁻¹ (earned confidence). When they split 3-2 on a borderline token, we report WAG (low confidence). The ceiling prevents Colosseum gaming: a "perfect" unanimous verdict is mathematically impossible, forcing judges to acknowledge uncertainty.

## Differentiation

**Reproducible:** Deterministic Dog logic is pure heuristics—no proprietary black box. Every decision is auditable. Regression test (commit 9bfba2d) validates forced consensus architectural bias was removed; filter respects explicit requests only.

**Wisdom-Injected (Phase 1):** Hermes sensor captures 4,088 tweets via passive proxy. High-signal tweets (score ≥3) route to Dogs; observations stored via K15 producer. Cross-session learning via SKILL.md. Dogs receive falsifiable claims alongside token data.

**On-Chain Proofs:** `submit_verdict.ts` encodes verdicts to Solana community PDA. Every judge call produces an immutable record.

**K15 Status (Producer-Consumer):** Producer operational and validated—observations created via `/observe` endpoint after each verdict, stored in SurrealDB. Consumer layer documented and tested. Honest about Phase 1 scope: we prove judgment is truth-bearing; automated recovery actions (Phase 2) follow post-hackathon. Circuit-breaker architecture prevents silent failures on degraded Dogs.

## Why This Matters

Token safety is a **coordination problem**. CYNIC doesn't replace human judgment—it makes disagreement visible and measurable. When Dogs diverge, the axioms pinpoint why. When they converge, confidence is earned through independent validation.

On Solana's 50K+ tokens, this is the difference between noise and signal.

---

## Technical Details

**API:** REST (Cloudflare tunnel) + MCP
- `POST /judge {content}` → Verdict with q_score + axiom breakdown
- `GET /health` → Dog status, circuit breaker state, kernel version

**On-Chain Contract:** Community PDA stores verdict hash + timestamp + Dog votes

**Data & Falsification:**
- 4,088 captured tweets (6.6MB)
- 124 computed verdicts
- 14 routed observations
- K15 validation: Producer (✓), Consumer gap (documented for Phase 2)
- Regression tests: forced-consensus removal (✓), circuit-breaker skip on degraded Dogs (✓), filter respect (✓)

**Dogs Status (May 4, 09:34 UTC):**
- Deterministic: ✓ 0ms latency
- Qwen-7B HF: ✓ ~6.5s latency
- Qwen-35B GPU: ✓ ~30s latency
- Qwen-35B Core: ✓ (available, tested)
- Gemini-CLI: ⚠️ Circuit-breaker OPEN (3 consecutive failures, terminal capability issue, recoverable)

Quorum = 2+ Dogs (deterministic + 1 language model) → verdict issued. System designed to degrade gracefully; continues operating with 4/5 Dogs. No single-point-of-failure.

---

## Team

**Zey** (@zeyxx) — Architecture, Dogs, axiom design, wisdom pipeline, on-chain integration

---

## Deployed

- **REST API:** Cloudflare tunnel (private IP: <TAILSCALE_CORE>:3030)
- **Web UI:** https://cynic-ui.vercel.app
- **GitHub:** https://github.com/zeyxx/CYNIC
- **On-Chain:** Pinocchio contract deployed to Solana devnet

---

## Status (as of May 4, 2026, 09:34 UTC)

**Kernel:** ✓ Live and stable (PID 3160167, 6h+ uptime)
**Storage:** ✓ SurrealDB running, recovered from lock issue
**Dogs:** ✓ 4/5 operational (Gemini in circuit-breaker)
**K15 Producer:** ✓ Observations routed and persisted
**Falsification Tests:** ✓ All pass (regression, circuit-breaker, filter)

**Ready for:**
- Live token judging (via `/judge` endpoint)
- On-chain verdict submission (via Pinocchio)
- Demo recording (script prepared, infrastructure live)

**Not yet ready for:**
- Video demo (prepared but not recorded; can be done before May 10 deadline)
- Colosseum platform submission (form not yet filled)

**Honest Assessment:** Judgment engine is production-grade. Infrastructure is stable. One Dog (Gemini) is temporarily degraded but recoverable. The submission is technically complete; execution (video + form) remains.

---

## Submission Links (to be completed)

- **Video Demo:** [To be uploaded to Colosseum]
- **GitHub:** https://github.com/zeyxx/CYNIC
- **Live API:** [Cloudflare tunnel URL, updated before submission]
- **UI:** https://cynic-ui.vercel.app

---

**Deadline:** May 10, 2026 23:59 PDT (6 days remaining)
