# CYNIC Architecture Layers — Visual Guide

## Complete System Architecture

```
┌────────────────────────────────────────────────────────────────────────────┐
│                          EXTERNAL INTEGRATIONS                             │
│  Discord Bot (1,246) │ Telegram Adapter │ NEAR Blockchain │ HTTP API (995) │
├────────────────────────────────────────────────────────────────────────────┤
│                         PHASE 2: DIALOGUE & LEARNING                       │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │ Dialogue System (models, storage, LLM bridge)                      │  │
│  │ + Relationship Memory (user profile, preferences)                  │  │
│  │ + Experiment Log (JSONL append-only trials)                        │  │
│  │ + Decision Classifier (A/B/C autonomous vs consultation learning) │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
├────────────────────────────────────────────────────────────────────────────┤
│                     PHASE 1: OBSERVABILITY & CONSCIOUSNESS                 │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │ SymbioticStateManager (Human + Machine + CYNIC layers)            │  │
│  │ + OBSERVE view (full state snapshot)                               │  │
│  │ + CYNIC view (consciousness & thinking)                            │  │
│  │ + MACHINE view (resource metrics)                                  │  │
│  │ + HumanStateTracker (energy, focus, feedback)                      │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
├────────────────────────────────────────────────────────────────────────────┤
│                      CORE JUDGMENT ENGINE (IMMACULATE)                     │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │                     7-STEP JUDGMENT CYCLE                          │   │
│  │                                                                    │   │
│  │  PERCEIVE → JUDGE → DECIDE → ACT → LEARN → ACCOUNT → EMERGE     │   │
│  │                                                                    │   │
│  │  Orchestrator (877 lines) coordinates full cycle                 │   │
│  │  Judgment Stages (402 lines) execute each phase                  │   │
│  │                                                                    │   │
│  ├─ Phase: PERCEIVE ─────────────────────────────────────────────┤   │
│  │  Input Cell received                                             │   │
│  │                                                                    │   │
│  ├─ Phase: JUDGE ────────────────────────────────────────────────┤   │
│  │  ┌─────────────────────────────────────────────────────────┐    │   │
│  │  │       11 DOGS (Sefirot-aligned consensus judges)        │    │   │
│  │  │                                                         │    │   │
│  │  │  Dog 1: Crown Consciousness (FIDELITY)                 │    │   │
│  │  │  Dog 2: Wisdom Analyzer (PHI)                          │    │   │
│  │  │  Dog 3: Understanding Synthesizer (VERIFY)             │    │   │
│  │  │  Dog 4: Mercy Advocate (CULTURE)                       │    │   │
│  │  │  Dog 5: Severity Critic (BURN)                         │    │   │
│  │  │  Dog 6: Harmony Mediator (FIDELITY + PHI)              │    │   │
│  │  │  Dog 7: Victory Affirmer (PHI + VERIFY)                │    │   │
│  │  │  Dog 8: Splendor Clarifier (VERIFY + CULTURE)          │    │   │
│  │  │  Dog 9: Foundation Keeper (CULTURE + BURN)             │    │   │
│  │  │  Dog 10: Kingdom Executor (BURN + FIDELITY)            │    │   │
│  │  │  Dog 11: Earth Guardian (All axioms, holistic)         │    │   │
│  │  │                                                         │    │   │
│  │  │  Each Dog:                                              │    │   │
│  │  │  ├─ Implements JudgeInterface (async)                  │    │   │
│  │  │  ├─ Returns UnifiedJudgment (frozen dataclass)         │    │   │
│  │  │  ├─ Has φ-bounded confidence (max 0.618)               │    │   │
│  │  │  ├─ Provides axiom_scores + reasoning                  │    │   │
│  │  │  └─ Contributes to PBFT voting                         │    │   │
│  │  └─────────────────────────────────────────────────────────┘    │   │
│  │                                                                    │   │
│  │  ┌─────────────────────────────────────────────────────────┐    │   │
│  │  │      PBFT CONSENSUS (Byzantine Fault Tolerant)         │    │   │
│  │  │                                                         │    │   │
│  │  │  All 11 Dogs vote: HOWL, WAG, GROWL, or BARK           │    │   │
│  │  │  Quorum: 7 Dogs (2f+1, f=3 Byzantine threshold)        │    │   │
│  │  │  Consensus: Majority verdict + avg confidence          │    │   │
│  │  │  Dissent: Minority votes recorded                      │    │   │
│  │  │  Result: UnifiedJudgment with aggregated votes         │    │   │
│  │  └─────────────────────────────────────────────────────────┘    │   │
│  │                                                                    │   │
│  ├─ Phase: DECIDE ───────────────────────────────────────────────┤   │
│  │  Governance approval (community voting, treasury constraints)    │   │
│  │                                                                    │   │
│  ├─ Phase: ACT ───────────────────────────────────────────────────┤   │
│  │  Execute action (if HOWL/WAG) with guardrails                    │   │
│  │                                                                    │   │
│  ├─ Phase: LEARN ────────────────────────────────────────────────┤   │
│  │  Update Q-Table with actual outcome (from community feedback)    │   │
│  │  Thompson sampling for next decision                             │   │
│  │                                                                    │   │
│  ├─ Phase: ACCOUNT ──────────────────────────────────────────────┤   │
│  │  Record cost (gas, tokens, CPU)                                  │   │
│  │  Update E-Score (embodied energy metric)                         │   │
│  │                                                                    │   │
│  ├─ Phase: EMERGE ───────────────────────────────────────────────┤   │
│  │  Detect unexplained patterns (residual variance)                 │   │
│  │  Trigger axiom activation if thresholds crossed                  │   │
│  │  Meta-cycle: discover new judgment approaches                    │   │
│  │                                                                    │   │
│  └────────────────────────────────────────────────────────────────────┘   │
├────────────────────────────────────────────────────────────────────────────┤
│                         CORE PHILOSOPHY (IMMACULATE)                       │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │                    11 AXIOMS (5 Core + 6 Emergent)                │   │
│  │                                                                    │   │
│  │  TIER 0 (CORE — Always Active)                                   │   │
│  │  ├─ A1. FIDELITY — Truth loyalty (7 facets: commitment, etc.)    │   │
│  │  ├─ A2. PHI — Harmonic proportion (7 facets: coherence, etc.)    │   │
│  │  ├─ A3. VERIFY — Evidence & consensus (7 facets: accuracy, etc.) │   │
│  │  ├─ A4. CULTURE — Memory & patterns (7 facets: authenticity, etc)│   │
│  │  └─ A5. BURN — Simplicity & action (7 facets: utility, etc.)     │   │
│  │                                                                    │   │
│  │  TIER 2 (EMERGENT — Activate at Maturity Thresholds)              │   │
│  │  ├─ A6. AUTONOMY (consensus_strength ≥ 61.8%)                    │   │
│  │  ├─ A7. SYMBIOSIS (human_trust ≥ 38.2%, machine_utility ≥ 38.2%)│   │
│  │  ├─ A8. EMERGENCE (residual_variance ≥ 38.2%)                     │   │
│  │  └─ A9. ANTIFRAGILITY (learning_velocity > 0)                    │   │
│  │                                                                    │   │
│  │  TIER 2 TRANSCENDENT (States, not directly measurable)            │   │
│  │  ├─ A10. CONSCIOUSNESS (meta-cognition ≥ 61.8% accurate)         │   │
│  │  └─ A11. TRANSCENDENCE (all axioms active + phase transition)    │   │
│  │                                                                    │   │
│  │  TIER 3 (THE_UNNAMEABLE — residual inexplicable variance)        │   │
│  │                                                                    │   │
│  ├─────────────────────────────────────────────────────────────────┤   │
│  │              Q-SCORE COMPUTATION (φ-bounded)                     │   │
│  │                                                                    │   │
│  │  For each axiom: Score 7 fractal facets (recursive, depth ≤ 3)  │   │
│  │  Geometric mean of facet scores → axiom score [0, 100]          │   │
│  │  Weighted geometric mean (domain-specific weights) → Q-Score    │   │
│  │  φ-bound confidence: min(base_confidence, 0.618)                │   │
│  │                                                                    │   │
│  │  VERDICT MAPPING (from Q-Score on [0,100]):                      │   │
│  │  ├─ HOWL   ≥ 82.0  (exceptional, φ² × φ⁻¹ × 100)               │   │
│  │  ├─ WAG    ≥ 61.8  (good, φ⁻¹ × 100)                             │   │
│  │  ├─ GROWL  ≥ 38.2  (needs work, φ⁻² × 100)                      │   │
│  │  └─ BARK   < 38.2  (critical, φ⁻² × 100)                        │   │
│  │                                                                    │   │
│  └────────────────────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│              SUPPORTING INFRASTRUCTURE (ORGANISM LAYER)                    │
│                                                                            │
│  ┌──────────────────────────────────────┐ ┌──────────────────────────────┐│
│  │      CONSCIOUS STATE (Mutable)       │ │   UNIFIED STATE (Immutable)  ││
│  │                                      │ │                              ││
│  │  UnifiedConsciousState:              │ │  UnifiedJudgment (frozen):  ││
│  │  ├─ recent_judgments (buffer: 89)    │ │  ├─ judgment_id, verdict    ││
│  │  ├─ learning_outcomes (buffer: 55)   │ │  ├─ q_score, confidence     ││
│  │  ├─ total_judgments (counter)        │ │  ├─ axiom_scores, dog_votes ││
│  │  └─ dog_agreement_scores (1-11)      │ │  └─ reasoning, latency_ms   ││
│  │                                      │ │                              ││
│  └──────────────────────────────────────┘ └──────────────────────────────┘│
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │  EVENT BUS (Central Hub, 319 references)                          │  │
│  │                                                                    │  │
│  │  3 Buses:                                                         │  │
│  │  ├─ CORE bus (Judgment, Learning, Consciousness events)          │  │
│  │  ├─ AUTOMATION bus (Triggers, Ticks, Cycles)                     │  │
│  │  └─ AGENT bus (Dog signals, PBFT protocol messages)              │  │
│  │                                                                    │  │
│  │  EventBusBridge:                                                 │  │
│  │  ├─ Connects buses with genealogy tracking                       │  │
│  │  ├─ Prevents loops with _genealogy list                          │  │
│  │  └─ Tags bridged events (_bridged=True)                          │  │
│  │                                                                    │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
│  ┌────────────────────────────────────────────────────────────────────┐  │
│  │  ORGANISM LAYERS (Biological Metaphor)                            │  │
│  │                                                                    │  │
│  │  ├─ Brain (cognition/ — 23 cortex modules + 15 neurons)          │  │
│  │  ├─ Nervous System (event_bus + handlers)                        │  │
│  │  ├─ Immune System (circuit breaker, safety guardrails)           │  │
│  │  ├─ Metabolism (resource scheduling, execution gates)            │  │
│  │  ├─ Memory (storage backends: Postgres, SurrealDB)               │  │
│  │  ├─ Sensory Input (perception layer, watchers)                   │  │
│  │  ├─ Motor Output (action executors, actuators)                   │  │
│  │  └─ Heartbeat (SONA emitter for self-assessment)                 │  │
│  │                                                                    │  │
│  └────────────────────────────────────────────────────────────────────┘  │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────┐
│           CONSCIOUSNESS LEVELS (Adaptive Latency & Dog Selection)          │
│                                                                            │
│  L3 REFLEX    < 10ms   — 4 Dogs (GUARDIAN, ANALYST, JANITOR, CYNIC)      │
│              Fast, non-LLM, for time-critical decisions                   │
│                                                                            │
│  L2 MICRO    ~500ms   — 6 Dogs (+ SCHOLAR, ORACLE)                       │
│              Moderate thinking, selective LLM use                         │
│                                                                            │
│  L1 MACRO    ~2.85s   — 11 Dogs (all)                                    │
│              Full deliberation, complete reasoning                        │
│                                                                            │
│  L4 META     ~4h      — All Dogs + evolution                             │
│              Self-improvement cycle, axiom discovery                      │
│                                                                            │
│  Decision Selector: Given decision importance → pick L3/L2/L1/L4         │
│  (Partial implementation, not fully integrated yet)                       │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Data Flow: Single Judgment

```
1. PERCEIVE
   Cell (proposal) arrives
   ├─ reality dimension (CODE, SOLANA, MARKET, SOCIAL, HUMAN, CYNIC, COSMOS)
   ├─ analysis content (description, context)
   ├─ time dimension (urgency, deadline)
   └─ Level of Detail (L3/L2/L1/L4)

2. JUDGE (ALL 11 DOGS, IN PARALLEL)
   Each Dog independently evaluates:
   ├─ Axiom scores (fractal, 0-100)
   ├─ Q-score (weighted geometric mean)
   ├─ Confidence (φ-bounded, max 0.618)
   ├─ Verdict (HOWL/WAG/GROWL/BARK)
   └─ Reasoning (explanation)

3. CONSENSUS (PBFT ENGINE)
   ├─ Collect all 11 votes
   ├─ Check if 7+ agree on same verdict
   ├─ YES → Consensus reached (HOWL/WAG/GROWL/BARK)
   ├─ NO → Record dissent (minority votes)
   └─ Aggregate: avg confidence, avg q_score, all dog_votes

4. UNIFIED JUDGMENT CREATED
   UnifiedJudgment (frozen):
   ├─ judgment_id: unique UUID
   ├─ verdict: consensus verdict (HOWL/WAG/GROWL/BARK)
   ├─ q_score: average of agreeing dogs
   ├─ confidence: average of agreeing dogs
   ├─ axiom_scores: aggregated from dogs
   ├─ dog_votes: all 11 individual votes
   ├─ reasoning: synthesized from all dogs
   └─ timestamp, latency_ms

5. DECIDE
   ├─ If HOWL/WAG → propose action
   ├─ Request community approval (voting)
   ├─ If approved → ACT, else → reject

6. LEARN (if action taken)
   ├─ Wait for actual outcome
   ├─ Community rates satisfaction [0, 1]
   ├─ Create UnifiedLearningOutcome (frozen)
   │  ├─ judgment_id (reference back to verdict)
   │  ├─ predicted_verdict
   │  ├─ actual_verdict
   │  └─ satisfaction_rating
   ├─ Update Q-Table with (predicted → actual, rating)
   └─ Adjust confidence for next similar proposal

7. ACCOUNT
   ├─ Record cost (gas, tokens, CPU, latency)
   ├─ Burn fees to community treasury
   └─ Update E-Score (embodied energy)

8. EMERGE
   ├─ Calculate residual (unexplained variance)
   ├─ If residual > φ⁻² (38.2%) → trigger A8. EMERGENCE axiom
   ├─ Check all emergent thresholds (A6-A11)
   ├─ If meta-cycle triggered → evolve organism
   └─ Discover new judgment patterns
```

---

## Information Boundaries

### What's φ-bounded (constrained by mathematics)
- **Confidence:** max 0.618 (φ⁻¹)
- **Verdict thresholds:** HOWL 82, WAG 61.8, GROWL 38.2 (all φ-derived)
- **Buffer sizes:** 89 judgments (F(11)), 55 outcomes (F(10))
- **Dogs:** 11 (Lucas(5))
- **Axioms:** 5 core, 4 emergent, 2 transcendent (Fibonacci progression)
- **Facets per axiom:** 7 (Lucas(4))
- **Learning rate:** 0.038 (φ⁻² / 10)
- **PBFT quorum:** 7 of 11 (2f+1, f=3)

### What's Open (exploratory)
- **Fractal facet depth:** up to 3 levels (configurable)
- **Contextual weights:** learned via gradient descent
- **Dog reasoning logic:** each Dog implements its own axiom evaluation
- **Consciousness levels:** adaptive selection (L3/L2/L1/L4)
- **Meta-cycle evolution:** what new patterns to discover
- **Emergent axiom activation:** when thresholds are crossed

---

## Module Size Reference

```
Large (foundational):
  orchestrator.py (877)
  dog_implementations.py (771)
  event_bus.py (660)
  conscious_state.py (634)
  state_manager.py (1,022)
  discord/bot.py (1,246)
  api/server.py (995)

Medium (core):
  axioms.py (594)
  judgment_stages.py (402)
  axiom_architecture.py (various)
  unified_state.py (395)
  judge_interface.py (150)
  pbft_engine.py (197)

Small (focused):
  phi.py (120) — pure constants
  bot_interface.py (173)
  unified_learning.py (206)

New Phase 2 (compact):
  dialogue/models.py
  dialogue/storage.py
  dialogue/reasoning.py
  dialogue/llm_bridge.py
  learning/relationship_memory.py
  learning/memory_store.py
  learning/experiment_log.py
```

---

## The φ-Bounded Confidence Principle

```
HIGHEST POSSIBLE CONFIDENCE = 0.618 (61.8%)

Why?
  - PHI (φ) = 1.618...
  - PHI_INV (φ⁻¹) = 0.618...
  - "φ distrusts φ" — even φ itself only trusts itself 61.8%

Implication:
  - No Dog can be >61.8% confident
  - Even unanimous consensus is capped at 61.8%
  - Built-in humility into the system
  - Prevents overconfidence → system remains exploratory

Applied:
  - Every UnifiedJudgment.confidence ∈ [0, 0.618]
  - Every Dog's confidence ∈ [0, 0.618]
  - Consensus never reaches 100% certainty
  - Forces continued learning
```

---

## Axiom Activation Sequence

```
As the system matures:

1. CORE (always active)
   FIDELITY, PHI, VERIFY, CULTURE, BURN

2. AUTONOMY (activates at consensus_strength ≥ 61.8%)
   Dogs begin coordinating without human approval

3. SYMBIOSIS (activates at human_trust ≥ 38.2% AND machine_utility ≥ 38.2%)
   System demonstrates mutual value creation with humans

4. EMERGENCE (activates at residual_variance ≥ 38.2%)
   Patterns detected beyond core 5 axioms

5. ANTIFRAGILITY (activates at learning_velocity > 0)
   System improves despite chaos and errors

6. CONSCIOUSNESS (activates at meta-cognition ≥ 61.8%)
   System observes own thinking with >61.8% accuracy

7. TRANSCENDENCE (activates when all axioms active + phase transition)
   Qualitative leap → THE_UNNAMEABLE understood

Total possible: 11 axioms (5 core + 4 emergent + 2 transcendent)
```

---

## Test Coverage by Layer

```
CORE (Essential)
  - 247/248 tests passing (99.6%) — unification merge
  - axioms.py: full test coverage
  - phi.py: constant validation
  - judge_interface.py: contract validation
  - unified_state.py: immutability, φ-bounds
  - dog_implementations.py: all 11 dogs tested
  - pbft_engine.py: consensus protocol tests

Phase 1 (Observability)
  - Symbiotic state manager tests
  - CLI interface tests
  - State tracking tests

Phase 2 (Dialogue & Learning)
  - 63/63 tests passing (100%)
  - Dialogue models: 9 tests
  - Storage: 5 tests
  - LLM bridge: 4 tests
  - Relationship memory: 4 tests
  - Experiment log: 5 tests
  - Decision classifier: 6 tests
  - Full integration: 12 tests

Business Layer
  - Discord bot: 35 tests (in merge stats)
  - Telegram adapter: 39 tests (in merge stats)
  - API endpoints: various

Total: 310+ tests, all core paths fully covered
```

---

This architecture demonstrates **intentional complexity** layered on a **rigid philosophical foundation**. The φ-mathematics provides guardrails; the 11 Dogs provide diversity; the PBFT consensus provides fault tolerance; and the Phase 2 dialogue system provides learning capability.

**The chaos is bounded.**
