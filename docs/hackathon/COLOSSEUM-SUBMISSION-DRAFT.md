# CYNIC: Sovereign Token Intelligence via Calibrated Doubt

## Pitch (1-2 sentences)
CYNIC is a multi-Dog consensus engine that judges Solana tokens on six axioms (Fidelity, Structure, Verifiability, Culture, Efficiency, Sovereignty) and crystallizes disagreement into immutable verdicts on-chain via Pinocchio.

## Long Description
### The Problem
Token safety = social consensus today. No mechanical guarantee. Rugpulls, honeypots, and graduated scams extract $B annually because judgment defaults to narrative, not evidence.

### The Solution
CYNIC deploys five independent Dogs (LLMs + heuristics) to score every token independently on six axioms:
- **Fidelity** — faithful to observable reality
- **Phi (φ)** — structurally harmonious & proportional
- **Verify** — testable & falsifiable claims
- **Culture** — honors tradition & convention
- **Burn** — minimal waste & efficiency
- **Sovereignty** — preserves human agency & freedom

### How It Works
1. **Input:** Token address + domain context
2. **Deliberation:** 5 independent Dogs score on 6 axioms (5-15 seconds, real-time latency)
3. **Crystallization:** CCM (Crystal Coherence Machine) learns consensus boundaries, producing BARK/GROWL/WAG/HOWL verdicts
4. **Immutability:** Verdict + axiom scores + Dog reasoning lands on-chain via Pinocchio, verifiable forever

### Key Achievement
- **Detection Rate:** 75.5% on 53 confirmed rug-pulls (baseline)
- **Latency:** 10-15 seconds (3-4 Dogs respond before verdict issued)
- **Infrastructure:** Sovereign (Tailscale mesh, no external APIs for judgment)
- **Transparency:** Every verdict shows which Dogs voted, their scores, their reasoning

### Dogs (Independent Validators)
- qwen-7b-hf (HuggingFace Inference)
- qwen35-9b-gpu (RTX 4060 Ti, 55 tok/s)
- gemma-4-e4b-core (CPU + Vulkan iGPU)
- deterministic-dog (instant heuristic baseline)
- gemini-cli (Google/Gemini, optional fallback)

### On-Chain
- **Program:** Pinocchio (A4QK3jj2kDx6w3da7FF3wxiBMnD2NrDsL1F7RCJA5NXx)
- **Community PDA:** 8Pyd1hqd6jTX2jR8YvCAjnd3cyP5qB7XaxzwAGtHCSFD
- **Devnet Status:** Verdict submission live, axiom scores confirmed on-chain

### Philosophy
Sovereignty = making the cost of lying visible. CYNIC doesn't predict price. It reveals structure. The six axioms are not ML artifacts — they are load-bearing principles that Dogs must justify in writing, every verdict.

---

## Demo Video Script (2-3 min)
See `/tmp/video-demo-script.md`

## GitHub Link
https://github.com/zeyxx/CYNIC (public repo, code + docs)

## Deployed URL
REST API: https://residents-administrator-temperature-boc.trycloudflare.com (temporary tunnel)
UI: https://cynic-ui.vercel.app

---

**Status:** Devnet ready. Mainnet deployment pending on May 10 post-hackathon.
