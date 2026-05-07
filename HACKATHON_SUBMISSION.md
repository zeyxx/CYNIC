# CYNIC — Solana Frontier Hackathon Submission

**Track**: AI Platforms / Agents  
**Deadline**: May 10, 2026 23:59 PDT  
**Status**: Live, Demo Validated (May 6 2026)

---

## One-Liner

Five independent AI validators score Solana tokens across six axes — no model knows the others' verdict. Composite confidence bounded at 0.618. Verdicts settled on-chain via Pinocchio.

---

## Problem

Most token analysis collapses to social consensus (CT alpha) or single-model heuristics. Neither resists an adversarial narrator — a sophisticated actor who optimizes for passing any single evaluator.

**Before CYNIC**: Weak signals crashed the pipeline silently (DogError::DegenerateScores → 500). Humans saw nothing. Feedback loop broken.

---

## Solution: Adversarial Multi-Model Judgment

- **5 independent Dogs** (AI validators) score every token across 6 axioms: Fidelity, Phi, Verify, Culture, Burn, Sovereignty
- **Total isolation** — no Dog knows what others scored. Gaming requires corrupting multiple independent models simultaneously.
- **phi-bounded** — no verdict can claim confidence above phi^-1 = 0.618. Epistemic humility is structural, not a prompt.
- **Verdicts**: HOWL (strong), WAG (moderate), GROWL (suspicious), BARK (likely weak/rug)
- **On-chain settlement** via Pinocchio PDA on Solana devnet (tx confirmed, 3401 CU)

---

## Live Demo Results (May 6, 2026 — 09:34 UTC)

```
POST /judge
Content: "WIF token analysis: low confidence signal, weak fundamentals"
Domain: token-analysis

Result:
  Verdict: BARK
  Q-Score: 0.371
  Dogs: deterministic-dog + qwen-7b-hf + qwen-9b-core (3 voters)
  Anomaly: sovereignty axis disagreement (0.568)
  Integrity: hash-chained (prev_hash verified)

Axiom Breakdown:
  Fidelity:    0.300 (faithful to facts)
  Phi:         0.433 (structural harmony)
  Verify:      0.200 (testable/verifiable)
  Culture:     0.500 (cultural respect)
  Burn:        0.562 (efficiency)
  Sovereignty: 0.356 (agency preservation)
```

**Validation**: Weak Dogs produce BARK (audible signal), not 500 errors. Zero silent failures. Human can read the confusion.

---

## Architecture

```
User → REST/MCP API → Pipeline → [Dog 1, Dog 2, ..., Dog N] → Aggregation → Verdict
                                                                              ↓
                                                              Crystal Coherence Machine (memory)
                                                                              ↓
                                                              Pinocchio (on-chain settlement)
```

**Stack**:
- **Rust** (cynic-kernel): adversarial pipeline, axum REST + rmcp MCP, circuit breakers
- **Solana / Pinocchio**: PDA verdict settlement, 6 axiom scores per tx
- **SurrealDB**: Crystal Coherence Machine — verdicts crystallize into memory
- **Qwen 2.5 7B** (HuggingFace Inference): cloud Dog
- **Qwen 3.5 9B Q4** (local RTX 4060 Ti): sovereign Dog, 55 tok/s
- **Qwen 3.5 9B Q4** (local CPU + Vulkan): sovereign Dog, 6 tok/s
- **Gemini CLI**: non-sovereign Dog (Google subscription)
- **Deterministic Dog**: heuristic baseline (zero inference cost)

---

## Key Metrics

| Metric | Value |
|--------|-------|
| Total verdicts produced | 19,931+ |
| Dogs online | 5/5 |
| Crystal memory | 18 (3 canonical, 12 crystallized, 3 forming) |
| Kernel uptime | Continuous (systemd, auto-restart) |
| On-chain tx | Confirmed devnet (3401 CU) |
| MCP tools | 22 (all poison-hardened as of May 6) |
| Codebase | ~15K lines Rust kernel |

---

## What Makes This Different

1. **No single point of failure** — Dogs are independent. Corrupting one doesn't corrupt the verdict.
2. **Structural epistemic humility** — phi^-1 bound is mathematical, not a prompt instruction.
3. **Memory that compounds** — Crystal Coherence Machine injects past verdicts into future prompts. The organism learns.
4. **Sovereign by default** — 3/5 Dogs run on owned hardware. No cloud dependency for majority quorum.
5. **Adversarial-first** — Built for the case where the content TRIES to look good. Not a vibes checker.

---

## Evidence Chain

| Phase | What | Result |
|-------|------|--------|
| Phase 2 | Test Dogs on real organ-x data (11.8K tweets) | 20 verdicts, 95% BARK, 0 errors |
| Phase 3 | Research impact report | Zero silent failures confirmed |
| Phase 4 | Live demo (today) | BARK verdict, 3 Dogs, 6 axioms scored |
| PR#103 | MCP poison hardening | 21 regression tests, all tools resilient to garbage |
| PR#94 | Zombie process fix (K18) | kill_on_drop on all subprocesses |
| PR#93 | Auth on /health | Topology no longer exposed without Bearer token |

---

## Reproduction

```bash
# Live UI (no auth required)
open https://cynic-ui.vercel.app

# Direct API
curl -X POST https://<kernel>/judge \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"content":"<any token description>","domain":"token-analysis"}'
```

---

## GitHub

https://github.com/zeyxx/CYNIC

Monorepo: `cynic-kernel/` (Rust pipeline), `cynic-ui/` (React/Vite), `scripts/` (calibration + on-chain).

---

## Post-Hackathon Roadmap

1. Mainnet verdict settlement (Pinocchio → production)
2. Real-time token monitoring (webhook-driven judgment)
3. Fine-tuned sovereign model (crystals as training data, LoRA on 4060 Ti)
4. Public API for DeFi protocols (risk scoring as a service)
