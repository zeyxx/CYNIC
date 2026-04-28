# CYNIC — Sovereign Token Judge
## Colosseum Frontier Submission

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
1. **Deterministic Dog** — heuristic rules (rug indicators, liquidity locks, holder concentration)
2. **Qwen 7B HF** — lightweight inference validator
3. **Qwen 3.5 9B Core** — sovereign CPU inference
4. **Qwen 3.5 9B GPU** — high-throughput GPU inference (55 tok/s)
5. **Gemini CLI** — philosophical synthesis validator

Dogs vote independently. Consensus emerges as **φ-bounded confidence** (max 0.618 ≈ golden ratio inverse), preventing overconfidence on ambiguous tokens.

## Differentiation

**Reproducible:** Deterministic Dog logic is pure heuristics—no proprietary black box. Every decision is auditable.

**Wisdom-Injected:** Hermes sensor extracts 225+ domain signals (D1–D6: Tokens, Inference, Security, Macro, Epistemology) from high-signal tweets. Dogs receive falsifiable claims alongside token data (e.g., "if liquidity permanently locked, rug probability <10%").

**On-Chain Proofs:** `submit_verdict.ts` encodes verdicts to Solana community PDA `8DVUKmJa…`. Every judge call produces an immutable record.

**K15 Validated:** Empirical testing (April 2026) shows:
- D1 (token rugs): 0.090 BARK — correct low confidence on confirmed rugs
- D6 (predictions): 0.454 GROWL — stronger confidence on empirical, falsifiable claims
- Dogs floor-score confirmed negatives conservatively; improve precision on positive signals

## Why This Matters

Token safety is a **coordination problem**. CYNIC doesn't replace human judgment—it makes disagreement visible and measurable. When Dogs diverge, the axioms pinpoint why. When they converge, confidence is earned through independent validation.

On Solana's 50K+ tokens, this is the difference between noise and signal.

---

## Technical Details

**API:** REST (Cloudflare tunnel) + MCP
- `POST /judge {content}` → Verdict with q_score + axiom breakdown
- `GET /health` → Dog status, circuit breaker state, kernel version

**On-Chain Contract:** Community PDA `8DVUKmJa…` stores verdict hash + timestamp + Dog votes

**Data:** 2,007 high-signal tweets, 225+ curated domain signals, 141 crystallized observations

---

## Team

**Zey** (@zeyxx) — Architecture, Dogs, wisdom pipeline, on-chain integration

---

## Deployed

- **REST API:** `${CYNIC_REST_ADDR}` (Cloudflare tunnel)
- **Web UI:** https://cynic-ui.vercel.app
- **GitHub:** https://github.com/zeyxx/CYNIC

---

**Status:** Live. 5/5 Dogs online. K15 validation complete. Ready for production use.
