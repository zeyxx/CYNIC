# CYNIC Inference & Wisdom Synthesis Protocol v1.0

**Status:** Living document. Last updated 2026-04-27.

---

## Inference Engine Foundations

### GPU: RTX 4060 Ti 16GB

**VRAM Formula** (TheAhmadOsman, 2026):
```
VRAM ≈ Parameters (billions) × (bits_per_weight ÷ 8) + KV_cache + overhead
```

**Production Model: Qwen3.6-27B-Q2_K**
- Weights: 27 × 0.33 = 8.91 GB ✓
- Observed total: 10.6 GB (includes runtime overhead)
- KV cache (131K turbo3 fork): 1.6 GB
- **Safe config**: 64K context + q4_0 KV (12.5 GB total, 3.5 GB headroom)
- **Production config**: 131K context + turbo3 (custom fork, 12.5 GB, 3.5 GB for concurrency)

**Thinking Control Architecture** (per-request, not global):
```
Server config: --reasoning-format none  (global default OFF)
Request param: chat_template_kwargs: {"enable_thinking": false}  (Dogs)
Request param: chat_template_kwargs: {"enable_thinking": true}   (Agent)
```

Validated:
- Thinking OFF: 2.1s per inference, 100% JSON validity
- Thinking ON: 50.4s per inference, 1039 tokens, structured reasoning

**Performance Baseline** (observed, 27B Q2_K, 64K ctx + q4_0 KV):
- Prompt processing: ~400 tok/s
- Generation: ~20.8 tok/s
- Dog judgment (6 axioms): 13.9s, 271 tokens
- Agent turn (thinking ON): 50.4s, 1039 tokens

### Model Selection Decision Tree

```
Agent reasoning (60-96% of GPU time)?
  ├─ YES → 27B (quality > speed)
  └─ NO → 9B (sufficient for scoring, faster)

Diversity Dogs (multiple architectures)?
  ├─ GPU: 27B Q2_K (primary)
  ├─ Core: IQ3_XXS or Qwen3.5-9B (to test)
  └─ Fallback: Gemma 4 E4B (budget)
```

---

## Wisdom Synthesis Loop

**Philosophy:** Domain intelligence organizes knowledge by consumer. Each domain answers "what does this subsystem need to know?"

### 6 Domains (D1-D6)

| Domain | Coverage | Consumer | Example Signals |
|--------|----------|----------|-----------------|
| **D1** Solana/Tokens | 11% | Dogs (rug detection) | liquidity mechanics, contract checks |
| **D2** Inference/LLM | 118% | Inference lab (model selection) | quantization tradeoffs, VRAM math |
| **D3** Sovereignty | 0% | CYNIC identity (axiom grounding) | infrastructure independence, epistemic authority |
| **D4** Security/Scams | 70% | Dogs (exploit patterns) | fake launchpad, honeypot, social engineering |
| **D5** Macro/Politics | 30% | Market context (regulation, geopolitics) | policy, elections, macro cycles |
| **D6** Epistemology | 12% | Axiom calibration (confidence bounds) | falsification methods, calibration |

### Iteration Loop (Run Once Per Session)

```
STEP 1: Identify Gap
  gap_domain = min(coverage across all D1-D6)
  Current: D3 (0%) ← HIGHEST PRIORITY

STEP 2: Mine Raw Signals
  hermes-x scripts scan X2 for D{gap} keywords
  Example: D3 mining searches for "open-source", "self-host", "sovereign"
  Output: raw candidates + metadata

STEP 3: Curator Structures
  Claude (curator role) reads raw signals
  Creates D{N}_curated.jsonl with schema:
    {
      "signal_id": "D1_rug_001",
      "domain": "D1",
      "pattern": "...",
      "strength": 0.85,
      "sources": ["tweet_id"],
      "falsifiable_claim": "If X then Y < threshold"
    }

STEP 4: Measure Coverage
  count = lines in D{N}_curated.jsonl
  target: 150+ patterns per domain for Dogs

STEP 5: Repeat (next session)
  Next highest-priority gap becomes Step 1
```

### Execution: Python CLI

```bash
# Identify next gap and show curator task
python3 cynic-python/domain_iterator.py

# Outputs:
#   - Domain to curate: D3 (0%)
#   - Curator instructions
#   - Path: cynic-python/curation/D3_curated.jsonl
#   - Next iteration target

# Curator action: commit D3_curated.jsonl to git
git add cynic-python/curation/D3_curated.jsonl
git commit -m "feat(wisdom): D3 sovereignty patterns v1 (8 signals)"
```

### Signal Falsifiability Standard

Every pattern must have a **falsifiable claim**. Example:

**Pattern:** Liquidity locked + supply capped + multi-sig governance  
**Strength:** 0.9  
**Falsifiable claim:** "If all 3 present: rug probability < 0.1"  
**Falsify:** Find a legitimate protocol with all 3 that later rugs (updates strength)

---

## K15 Consumer: Wisdom → Dogs

**Gap:** Dogs currently receive bare text + system prompt. Dogs see no domain context.

**Fix (T#4):** Dogs fetch curated patterns matching the judgment content.

```
Dogs scoring a token judgment:
  1. Extract keywords (token name, launch pattern, social signals)
  2. Query D1_curated.jsonl: which patterns match?
  3. Inject matching patterns into scoring prompt
  4. Judge with enriched context

Expected impact:
  - Discrimination Δ before/after enrichment
  - Goal: +0.15 φ via domain grounding
```

Implementation:
```rust
// cynic-kernel/src/pipeline/wisdom_enrichment.rs
pub fn enrich_context_with_domain_signals(
    content: &str,
    domain_curations: &Map<String, Vec<Signal>>
) -> String {
    // Extract domain keywords from content
    // Fetch matching patterns from curated JSONLs
    // Return enriched prompt context
}
```

---

## Multi-Model Diversity (Future)

Current: Single GPU (27B) + single Core (9B embedding)

Future (soma orchestration):
- GPU primary: 27B Q2_K (agent + scoring)
- Core secondary: IQ3_XXS or Mistral 7B (diversity Dog)
- Cloud fallback: Gemini (hedge against local failures)

Each Dog's verdict = independent architecture → no single-model failure

---

## References

- VRAM math: @TheAhmadOsman thread (GPU Reality 2026)
- Thinking architecture: llama.cpp b8944+ (reasoning_format, enable_thinking)
- DeltaNet KV savings: Qwen3.6 architecture (25% cache cost vs pure transformer)
- Domain framework: CYNIC Constitution (K11, K12 LLM Development Principles)

---

## TODO

- [ ] Complete D1-D6 curation cycles (goal: 150 signals per domain)
- [ ] Implement K15 wisdom enrichment in Dogs
- [ ] Measure discrimination before/after enrichment
- [ ] IQ3_XXS benchmark vs Q2_K (diversity Dog candidate)
- [ ] CARNICE-V2-27B comparison (agent fine-tune validation)
- [ ] Somadel orchestrator (multi-agent resource allocation)
- [ ] Document CARNICE-V2 vs base Qwen3.6 reasoning quality
