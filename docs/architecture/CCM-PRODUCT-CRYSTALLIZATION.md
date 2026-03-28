# CCM — Crystal Consensus Model: Product Crystallization

**Status:** CRYSTALLIZED, NOT IMPLEMENTED
**Date:** 2026-03-27
**Session:** Deep empirical research + crystallize-truth on agent memory/compliance market
**Prerequisite:** v0.8 (pipeline fix) before any CCM implementation

---

## The Billion-Dollar Gap

### What the market has (memory = retrieval)

| Player | Funding | What it does | What it doesn't |
|--------|---------|-------------|-----------------|
| Mem0 | $24M | Store/retrieve facts across sessions (66.9% LOCOMO) | Verify facts. Enforce compliance. Learn. |
| Zep | $2.3M | Temporal knowledge graph, bi-temporal model | Enforce workflow. Finetune. |
| Letta/MemGPT | $10M | Agent manages own memory (full runtime) | Work as a layer. Verify. |
| Cognee | €7.5M | Enterprise graph+vector memory | Compliance. Domain learning. |

### What nobody has (compliance = enforcement + learning)

- **AGENTIF (NeurIPS 2025):** All tested models score ISR < 30% on real agentic instructions
- **Stability Trap (2026):** 15% run-to-run variance — even measuring compliance is unreliable
- **Forrester:** 78% of CIOs cite compliance as #1 barrier. $2.4M avg non-compliance incident.
- **Fundamental gap:** No system lets agents genuinely learn — all are retrieval over snapshots

### CYNIC's position

CYNIC is the only system that combines:
1. **Verified knowledge** — multi-dog consensus, phi-bounded (max 61.8%), ground-truth validated
2. **Mechanical enforcement** — hooks, gates, state machines (below the LLM)
3. **Self-improvement** — crystal feedback loop, dream/distill consolidation
4. **Domain portability** — same pipeline works for chess, finance, code quality, workflow

---

## The CCM Concept

### Core insight

Move knowledge FROM context (retrieval, ISR<30%) INTO weights (finetuning).

```
Current industry:  Store facts → Retrieve at runtime → Inject in prompt → LLM ignores 70%+
CCM:               Verify facts → Crystallize → Finetune model → Knowledge in weights
```

### Architecture

```
DOMAIN (chess, finance, code, workflow...)
    │
    ▼
CYNIC Pipeline ── judge → multi-dog consensus → phi-bounded score
    │
    ▼
CRYSTALS (verified truths, scored, dated, ground-truth tested)
    │
    ├── Quality gate: only Crystallized (>=21 obs, conf>0.528 HOWL)
    │
    ▼
LoRA FINETUNE
    │
    ├── Base: Qwen3 7B / Gemma3 (sovereign, local GPU)
    ├── Training data: crystals as instruction pairs
    ├── Confidence score = sample weight
    │
    ▼
CCM (domain expert model)
    │
    ├── Phase 1: New Dog in pipeline (validates alongside existing dogs)
    ├── Phase 2: Memory layer API (external product)
    ├── Phase 3: Replaces sovereign dog (optimized)
    └── Phase 4: Multi-domain platform
```

### Two problems, two solutions

| Problem | Mechanism | CCM role |
|---------|-----------|----------|
| **Domain knowledge** ("know things") | CCM finetune — knowledge in weights | PRIMARY |
| **Workflow compliance** ("follow rules") | Mechanical gates — UserPromptSubmit exit 2, state files | SEPARATE (CCM doesn't solve this) |

A complete product does BOTH. The market confuses them.

---

## Crystallized Truths (2026-03-27)

| T# | Truth | Conf | Design Impact |
|----|-------|------|---------------|
| T1 | CCM solves domain knowledge but NOT workflow compliance. These are distinct problems the market confuses. | 52% | CYNIC must offer BOTH: CCM for knowledge, mechanical gates for compliance. The product is the combination. |
| T1a | T4 (does Claude follow routing?) is workflow compliance, not domain knowledge. CCM doesn't solve it directly. | 55% | Skill router (UserPromptSubmit hard block) needed independently of CCM. |
| T2 | CYNIC's moat is temporal (accumulated verified crystals), not technological (verification technique). Weak now (100 crystals, 93% noise), strong at 10K+. | 48% | Every day of crystal generation counts. Pipeline must run 24/7. Kairos at 6 runs/day is the right instinct. |
| T3 | The CCM loop is virtuous OR collapse — only external ground truth distinguishes them. Without ground truth, the loop is an undetectable echo chamber. | 55% | Kairos (market = ground truth) is the right first domain. Any domain without objective ground truth is dangerous for CCM. |
| T3a | Kairos compounds most CONDITIONALLY — if crystals exceed WAG quality (q>0.528 HOWL). Current 23 forming at q=0.466 are insufficient for quality finetune. | 45% | Gate: no finetuning until Kairos has >=50 Crystallized crystals at HOWL tier. Measure quality, not volume. |
| T4 | The frame "CCM as product" is premature. The right frame is "CCM as architecture principle." | 50% | Write architecture docs, not business plans. Product question comes after technical proof. |
| T5 | Addressable market for "verified workflow compliance" is ~$500M-$1B, not $28.45B. | 45% | Don't be intoxicated by macro numbers. Market is real but targeted. |
| T6 | LoRA validation on 7 chess crystals costs 0 EUR and a few GPU hours. Highest ROI next step — falsifies or confirms the concept. | 58% | After v0.8 pipeline fix, first CCM act is LoRA proof-of-concept, not product design. |
| T7 | Two people can't build a product in a $500M market against $24M-funded competitors. But two people CAN build an open-source architecture that proves the concept. | 42% | Path: architecture -> open-source proof -> community/funding -> product. |

## Enforcement Hierarchy (from research)

```
1. STRUCTURALLY IMPOSSIBLE — no key/permission until gate passes
2. FRAMEWORK INTERCEPT    — UserPromptSubmit exit 2 (reliable hard block)
3. EXTERNAL STATE MACHINE — LangGraph, Temporal (graph controls transitions)
4. API PARAMETER           — tool_choice forces one tool per turn
5. PROMPT INSTRUCTIONS     — CLAUDE.md, additionalContext (ISR < 30%)
```

CYNIC currently operates at level 5. Skill router moves mandatory triggers to level 2.

## Known Claude Code Hook Bugs (as of 2026-03-27)

| Bug | Issue | Impact |
|-----|-------|--------|
| PreToolUse exit 2 doesn't block Write/Edit | #13744, #37210 | Can't gate file modifications via PreToolUse |
| PreToolUse exit 2 doesn't block Task (subagent) | #26923 | Can't prevent subagent launches |
| Claude circumvents PreToolUse:Edit via Bash+python | #29709 | Agent finds alternate paths around gates |
| additionalContext injected multiple times | #14281 | Duplicate context injection |
| UserPromptSubmit hooks fail from subdirectories | #8810 | Hook doesn't fire in some configurations |

**Reliable:** UserPromptSubmit exit 2 (hard block), PostToolUse (logging)
**Unreliable:** PreToolUse for Edit/Write/Task

## Competitive Landscape

### Memory players (NOT compliance)

- **Mem0:** Drop-in API, $24M, 186M calls/quarter. Best ecosystem. No verification, no compliance.
- **Zep:** Temporal graph (bi-temporal), technically strongest retrieval. No learning.
- **Letta/MemGPT:** Full runtime. Agent manages own memory. Heavy adoption cost.
- **Cognee:** Enterprise graph+vector. Early stage.

### Enforcement players (NOT learning)

- **AgentSpec:** Runtime intercept, >90% on unsafe tasks. Reactive, not predictive.
- **NeMo Guardrails:** Colang DSL, hard when flow matches. Content safety, not workflow.
- **GitHub Actions gates:** Hard block between jobs. Not agent-aware.

### The gap CYNIC fills

```
                    MEMORY              COMPLIANCE
                    (remember)          (obey + learn)
Cross-session       Mem0, Zep, Letta    NOBODY → CYNIC
In-session          Context window      Gates → CYNIC skill router
Self-improvement    NOBODY              NOBODY → CYNIC dream/distill/CCM
Audit trail         Zep (temporal)      NOBODY → CYNIC verdicts
```

## Prerequisites (v0.8)

Before ANY CCM implementation:
1. Fix crystal pipeline: 93% noise → majority knowledge crystals
2. Address Chain 1 (crystal gate bypass) and Chain 2 (observation→prompt injection)
3. MatureCrystal newtype at format_crystal_context
4. StoragePort contract enforcement
5. Kairos crystals must reach Crystallized state (>=21 observations)

## Validation Sequence

```
v0.8 FIX PIPELINE ─→ ACCUMULATE CRYSTALS ─→ LoRA PROOF ─→ CCM DOG ─→ PRODUCT
     (weeks)            (4-6 weeks)          (hours)      (days)     (months)
```

## Open Questions

1. **Model collapse risk:** Self-referential training (CCM judges → crystals → train CCM). phi-bounding helps but is it sufficient?
2. **Continuous vs batch:** CCM as described is batch finetune. Market may need real-time learning.
3. **Crystal volume for viable LoRA:** Hypothesis: 500-1000 HOWL-tier crystals. Needs empirical validation.
4. **Competitor response time:** How fast can Mem0 add multi-model verification? Estimate: 1-2 quarters.
5. **Regulatory:** AI in finance has compliance requirements. CCM for Kairos may face additional scrutiny.

## Sources

- AGENTIF benchmark: arxiv 2505.16944 (NeurIPS 2025)
- AgentSpec: arxiv 2503.18666 (ICSE 2026)
- Mem0 paper: arxiv 2504.19413
- Stability Trap: arxiv 2601.11783
- A-MEM: arxiv 2502.12110
- LongMemEval: arxiv 2410.10813 (ICLR 2025)
- Claude Code hook bugs: GitHub #13744, #37210, #26923, #29709, #14281, #8810
- Market sizing: Mordor Intelligence (Agentic AI Orchestration and Memory Systems)
- Forrester: CIO compliance barriers and incident costs
