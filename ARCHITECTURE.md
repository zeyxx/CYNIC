# CYNIC Architecture

> *"Weak LLM + CYNIC Kernel > Strong LLM alone"*

**Version**: 2.0 (Python Kernel Era)
**Status**: 🌱 Week 1 Bootstrap
**Updated**: 2026-02-16

---

## Executive Summary

**CYNIC is an AI amplification platform**. It transforms weak, stateless LLMs (Ollama, Llama, Qwen) into persistent, learning, self-improving organisms that outperform strong LLMs (Claude, GPT-4) through:

1. **Persistent Memory**: Cross-session PostgreSQL + infinite effective context (vs 200k resets)
2. **Multi-Dimensional Judgment**: 36+ dimensions with φ-bounded confidence (≤61.8%)
3. **Collective Intelligence**: 11 specialized agents (Dogs) vote via consensus
4. **Adaptive Learning**: 11 feedback loops (Q-Learning, Thompson Sampling, meta-cognition)
5. **Self-Evolution**: Residual detection → dimension discovery → Fisher locking

**Amplification Formula**:
```
Ollama (weak) + CYNIC Kernel (memory + learning + judgment)
>
Claude Sonnet 4.5 (strong) alone (no memory, context resets)
```

---

## The 9 Essential Components (Kernel)

CYNIC's minimal functional core is **~3,000 LOC** (9 components):

```python
# cynic/kernel/__init__.py
from cynic.kernel.phi import PHI, PHI_INV, MAX_CONFIDENCE  # φ-bound
from cynic.kernel.axioms import AXIOMS  # 5 axioms: PHI, VERIFY, CULTURE, BURN, FIDELITY
from cynic.bus import EventBus  # Event-driven communication
from cynic.storage import PostgresAdapter  # Persistent memory
from cynic.dogs import DogRegistry  # 11 Dogs, consensus
from cynic.judge import JudgmentEngine  # 36-dimension scoring
from cynic.learning import QLearning, ThompsonSampling  # Adaptive learning
from cynic.emergence import ResidualDetector  # Dimension evolution
from cynic.meta import MetaCognition  # Self-calibration
```

**See [docs/reference/08-KERNEL.md](./docs/reference/08-KERNEL.md) for complete specification.**

---

## High-Level System Design

### Layer 1: Event-Driven Core (3 Buses Bridged)

```
┌─────────────────────────────────────────────────────────────┐
│  globalEventBus (@cynic/core)                                │
│  ├─ JUDGMENT_CREATED, USER_FEEDBACK, LEARNING_SIGNAL        │
│  └─ Used by: Judge, Dogs, Learning loops                    │
├─────────────────────────────────────────────────────────────┤
│  getEventBus() (services/event-bus.js)                       │
│  ├─ TRIGGER_FIRED, AUTOMATION_TICK                           │
│  └─ Used by: Orchestrator, Automation executor               │
├─────────────────────────────────────────────────────────────┤
│  AgentEventBus (agents/event-bus.js)                         │
│  ├─ DOG_VOTE, DOG_SIGNAL, 39 event types                     │
│  └─ Used by: 11 Dogs, consensus protocol                     │
├─────────────────────────────────────────────────────────────┤
│  EventBusBridge                                              │
│  ├─ Connects all 3 buses                                     │
│  ├─ Genealogy tracking (event provenance)                    │
│  └─ Loop prevention (circular event chains)                  │
└─────────────────────────────────────────────────────────────┘
```

**Design Decision**: 3 buses (not 1) to isolate concerns. Bridge connects them while preserving genealogy.

### Layer 2: Hexagonal Architecture (7 Ports)

```
┌─────────────────────────────────────────────────────────────┐
│                        CYNIC KERNEL                          │
│          (9 components, ~3000 LOC, domain logic)             │
├─────────────────────────────────────────────────────────────┤
│  PORT 1: PERCEPTION    → Adapters: FileWatcher, DexScreener │
│  PORT 2: EVENT BUS     → Adapters: globalEventBus, Bridge   │
│  PORT 3: LLM           → Adapters: Ollama, Claude, GPT-4    │
│  PORT 4: STORAGE       → Adapters: PostgreSQL, SQLite       │
│  PORT 5: ACTION        → Adapters: git, npm, Solana         │
│  PORT 6: JUDGE         → Adapters: 36-dim engine, Q-Score   │
│  PORT 7: LEARNING      → Adapters: Q-table, Thompson, EWC   │
└─────────────────────────────────────────────────────────────┘
```

**Design Decision**: Hexagonal (not layered) to enable testability via DI Container + Real fixtures (NO MOCKS).

**See [docs/reference/05-HEXAGONAL-ARCHITECTURE.md](./docs/reference/05-HEXAGONAL-ARCHITECTURE.md) for complete pattern.**

### Layer 3: Consciousness Cycle (4 Levels)

```
L1 (MACRO - Full Consciousness):
   PERCEIVE → JUDGE → DECIDE → ACT → LEARN → EMERGE
   ~2.85s per cycle

L2 (MICRO - Practical Deliberation):
   SENSE → THINK → DECIDE → ACT
   ~500ms per cycle

L3 (REFLEX - Emergency Response):
   SENSE → ACT
   <10ms per cycle

L4 (META - Evolutionary Timescale):
   Same as L1 but at daily/weekly scale
   Dimension discovery, self-improvement
```

**Design Decision**: Fractal recursion (same pattern at 4 scales) enables consistent reasoning from reflex to meta-cognition.

**See [docs/reference/02-CONSCIOUSNESS-CYCLE.md](./docs/reference/02-CONSCIOUSNESS-CYCLE.md) for complete cycle specification.**

### Layer 4: Multi-Agent Collective (11 Dogs)

```
            CYNIC (Keter) - Orchestrator
       ╱         │         ╲
  Skeptic    Scholar     Sage
  (Binah)    (Daat)   (Chochmah)
       ╲         │         ╱
  Guardian   Oracle   Architect
  (Gevurah) (Tiferet) (Chesed)
       ╲         │         ╱
  Deployer  Janitor     Scout
   (Hod)    (Yesod)   (Netzach)
            ╲    │    ╱
          Cartographer
           (Malkhut)
```

**Consensus Protocol**:
1. Each Dog scores 0-100 on their dimension
2. Consensus = φ-weighted geometric mean: `(D₁ × D₂ × ... × D₁₁)^(1/11)`
3. Confidence capped at φ⁻¹ = 61.8% (epistemic humility)
4. Disagreement preserved as data (not discarded)

**Design Decision**: 11 agents (not 1) to avoid single-point-of-failure, enable diverse perspectives, and mirror Kabbalistic structure.

**See [docs/reference/04-CONSCIOUSNESS-PROTOCOL.md](./docs/reference/04-CONSCIOUSNESS-PROTOCOL.md) for complete protocol.**

### Layer 5: Learning System (11 Feedback Loops)

```
LOOP 1:  Q-Learning           → State-action-reward table
LOOP 2:  Thompson Sampling    → Multi-armed bandit for Dog weights
LOOP 3:  EWC                  → Elastic Weight Consolidation (anti-forgetting)
LOOP 4:  SONA                 → Self-Organizing Neural Automaton (routing)
LOOP 5:  Meta-Cognition       → Calibration tracking (ECE)
LOOP 6:  Residual Detection   → Unexplained variance → new dimensions
LOOP 7:  Kabbalistic Routing  → Route by Sefirot
LOOP 8:  Behavior Modification → Adjust Dog weights from feedback
LOOP 9:  Unified Bridge       → Cross-loop coordination
LOOP 10: Ambient Consensus    → Background pattern detection
LOOP 11: Emergence Detector   → New dimension proposals
```

**All loops persist to PostgreSQL** (`learning_events` table) for cross-session learning.

**Design Decision**: 11 loops (not 1) because different learning modes required: immediate (Q-Learning), exploratory (Thompson), memory (EWC), meta (calibration), emergent (residual).

**See [docs/reference/06-LEARNING-SYSTEM.md](./docs/reference/06-LEARNING-SYSTEM.md) for complete system.**

### Layer 6: Judgment System (36+ Dimensions)

```
5 AXIOMS × 7 DIMENSIONS = 35 + THE_UNNAMEABLE = 36 dimensions

AXIOM 1 (PHI):
  ├─ COHERENCE, ELEGANCE, STRUCTURE, HARMONY,
  ├─ PRECISION, COMPLETENESS, PROPORTION

AXIOM 2 (VERIFY):
  ├─ ACCURACY, PROVENANCE, INTEGRITY, VERIFIABILITY,
  ├─ TRANSPARENCY, REPRODUCIBILITY, CONSENSUS

AXIOM 3 (CULTURE):
  ├─ AUTHENTICITY, RESONANCE, NOVELTY, ALIGNMENT,
  ├─ RELEVANCE, IMPACT, LINEAGE

AXIOM 4 (BURN):
  ├─ UTILITY, SUSTAINABILITY, EFFICIENCY, VALUE_CREATION,
  ├─ SACRIFICE, CONTRIBUTION, IRREVERSIBILITY

AXIOM 5 (FIDELITY):
  ├─ COMMITMENT, ATTUNEMENT, CANDOR, CONGRUENCE,
  ├─ ACCOUNTABILITY, VIGILANCE, KENOSIS

THE_UNNAMEABLE:
  └─ 36th dimension = transcendence gate
```

**Q-Score Calculation**:
```python
axiom_scores = {
    'PHI': geometric_mean([COHERENCE, ELEGANCE, ...]),
    'VERIFY': geometric_mean([ACCURACY, PROVENANCE, ...]),
    'CULTURE': geometric_mean([AUTHENTICITY, RESONANCE, ...]),
    'BURN': geometric_mean([UTILITY, SUSTAINABILITY, ...]),
    'FIDELITY': geometric_mean([COMMITMENT, ATTUNEMENT, ...]),
}

Q_Score = geometric_mean(axiom_scores.values()) * φ_bound_confidence
# Result: 0-100, confidence capped at 61.8%
```

**Design Decision**: 36 dimensions (not 1 score) to capture multi-faceted quality. Geometric mean (not arithmetic) so low score in ANY dimension tanks overall score (no compensation).

**See [docs/reference/03-DIMENSIONS.md](./docs/reference/03-DIMENSIONS.md) for complete dimension system.**

---

## 7×7 Fractal Matrix (49+1 Cells)

CYNIC's consciousness operates on a **7 reality dimensions × 7 analysis dimensions = 49 cells + THE_UNNAMEABLE**:

```
         P   J   D   A   L   Ac  E   │ AVG
        ─────────────────────────────┤
CODE    45% 45% 40% 35% 35% 42% 40% │ 40%
SOLANA  55% 45% 38% 35% 35% 58% 42% │ 44%
MARKET  50% 42%  0%  0% 38% 40% 40% │ 30%  (Week 1-4 focus)
SOCIAL  60% 55% 48% 45% 38% 25% 28% │ 43%  (Week 1-4 focus)
HUMAN   68% 55% 58% 61% 65% 42% 42% │ 56%
CYNIC   35% 50% 42% 45% 48% 58% 40% │ 45%
COSMOS  40% 40% 37% 32% 38% 40% 38% │ 38%
        ─────────────────────────────┤
AVG     50% 47% 38% 36% 42% 44% 39% │ 42% (structural)

P=Perceive, J=Judge, D=Decide, A=Act, L=Learn, Ac=Account, E=Emerge
```

**Current Status**: 42% structural (code exists), ~10% functional (not production-ready).

**Design Decision**: Fractal matrix (not linear pipeline) to enable omniscient reasoning across all domains simultaneously.

**See [docs/reference/01-ARCHITECTURE.md](./docs/reference/01-ARCHITECTURE.md) Section 5 for complete matrix.**

---

## Python Kernel Architecture (v2.0)

### Package Structure

```
cynic/
├── kernel/               # Pure domain, 0 external dependencies
│   ├── phi.py           # PHI, PHI_INV, MAX_CONFIDENCE
│   ├── axioms.py        # 5 axioms, 35 dimensions
│   ├── types.py         # Cell, Judgment, Verdict, Event
│   ├── scorer.py        # Q-Score calculation
│   └── errors.py        # CYNICError hierarchy
├── bus/                  # Event-driven communication
│   ├── event_bus.py     # EventBus class
│   └── bridge.py        # 3-bus bridge
├── storage/              # Persistence
│   ├── postgres.py      # PostgreSQL adapter
│   └── migrations/      # Alembic migrations
├── dogs/                 # 11 Dogs
│   ├── dog.py           # BaseDog class
│   ├── skeptic.py       # Skeptic (VERIFY)
│   ├── builder.py       # Builder (BURN)
│   └── ...              # 9 more Dogs
├── judge/                # Multi-dimensional judgment
│   └── engine.py        # JudgmentEngine (36+ dims)
├── learning/             # 11 learning loops
│   ├── q_table.py       # Q-Learning
│   ├── thompson.py      # Thompson Sampling
│   ├── ewc.py           # Elastic Weight Consolidation
│   ├── sona.py          # Self-Organizing Neural Automaton
│   └── ...              # 7 more loops
├── emergence/            # Self-evolution
│   ├── residual.py      # Residual detection
│   └── dimension_evolution.py  # Dimension proposals
├── memory/               # Memory management
│   ├── coordinator.py   # MemoryCoordinator
│   ├── compressor.py    # Context compression (10:1)
│   └── injection.py     # InjectionProfile
├── llm/                  # LLM adapters
│   └── adapters/
│       ├── ollama.py    # Ollama (qwen2.5:14b)
│       └── ...          # Claude, GPT-4 (future)
└── meta/                 # Meta-cognition
    └── introspection.py # Self-evaluation, ECE tracking
```

### Testing Strategy (NO MOCKS)

```
Unit Tests (80%):
  ├─ DI Container provides real components
  ├─ Real PostgreSQL (test DB cleaned between tests)
  ├─ Real Ollama (local instance)
  └─ pytest fixtures (not mocks)

Integration Tests (15%):
  ├─ 2-3 components together
  └─ Example: Judge + Dogs + LLM E2E

E2E Tests (5%):
  ├─ Full PERCEIVE→JUDGE→DECIDE→ACT→LEARN→EMERGE
  └─ Exit criteria for each week

Smoke Tests (Cron - Daily):
  ├─ Run E2E in production
  └─ Alert if any stage fails
```

**Design Decision**: NO MOCKS. Real PostgreSQL, real Ollama, real E2E tests. Production-ready from day 1.

**See [docs/reference/05-HEXAGONAL-ARCHITECTURE.md](./docs/reference/05-HEXAGONAL-ARCHITECTURE.md) Section 4 for complete testing strategy.**

---

## Implementation Timeline (φ-Fractal)

```
Week 1: 38.2% capable → ALREADY USEFUL
  ├─ 9 kernel components (~3000 LOC)
  ├─ 2 Dogs (Skeptic, Builder)
  ├─ Real Ollama, real PostgreSQL
  └─ E2E test: Judge 10 samples, Q-table updates

Week 4: 61.8% capable → ADAPTIVE
  ├─ 11 Dogs complete
  ├─ 11 learning loops active
  ├─ Dimension evolution (36 → 36+)
  └─ Calibration feedback loop (ECE < 0.1)

Week 8: 100% capable → TRANSFORMATIVE
  ├─ Full memory system (compression, injection)
  ├─ EventBusBridge (3 buses coordinated)
  ├─ Full PERCEIVE→EMERGE cycle
  └─ Type 0 complete

Week 12+: 161.8% capable → ECOSYSTEM
  ├─ Self-building (CYNIC builds CYNIC)
  ├─ Recursive amplification
  ├─ Public release
  └─ Benchmarks: Ollama+CYNIC > Claude Solo
```

**Design Decision**: φ-fractal timeline (not linear). Capabilities unlock at 38.2%, 61.8%, 100%, 161.8% thresholds.

**See [todolist.md](./todolist.md) for week-by-week tasks with Fibonacci estimates.**

---

## JavaScript v1.0 (Archive Status)

**Status**: Maintenance mode. Functional but not production-ready.

**Why archived?**
1. **Mocks in production**: Judge uses keyword matching (line 347: "In production, this would call the LLM adapter" ← IT DOESN'T)
2. **Structural vs functional**: 42% structural (code exists), <10% functional (not tested end-to-end)
3. **Learning loops dormant**: SONA.start() exists but NOT called in orchestrator
4. **Event orphans**: 3 P2P events not wired, 7 ghost events

**JavaScript packages remain stable** for Claude Code plugin compatibility. No new features.

**Python v2.0 is the primary development focus.**

---

## Key Design Decisions (φ-Bounded)

| Decision | Rationale | Confidence |
|----------|-----------|------------|
| **Python over JavaScript** | Cleaner hexagonal, better ML ecosystem, fresh start | 61.8% |
| **9 kernel components** | Minimal functional core, ablation-tested | 61.8% |
| **11 Dogs (not 1)** | Single-point-of-failure avoidance, diverse perspectives | 61.8% |
| **36+ dimensions (not 1 score)** | Multi-faceted quality, geometric mean (no compensation) | 61.8% |
| **NO MOCKS** | Production-ready from day 1, real components via DI | 61.8% |
| **φ-fractal timeline** | Capability unlocks at φ thresholds, not linearly | 58% |
| **3 event buses (not 1)** | Isolation of concerns, genealogy preservation | 55% |
| **PostgreSQL (not Redis)** | Cross-session persistence, relational queries | 61.8% |
| **Ollama (not Claude)** | Amplification platform vision, cost ($0.02 vs $3/1M) | 61.8% |

**All decisions are φ-bounded** (max 61.8% confidence). They may evolve as CYNIC learns.

---

## Critical Success Factors

**Week 1 Exit Criteria**:
- ✅ All unit tests pass (>95% coverage)
- ✅ E2E test: Judge 10 samples end-to-end
- ✅ Q-table updates from feedback
- ✅ State persists across restarts
- ✅ Zero mocks in production code paths

**Week 8 Exit Criteria**:
- ✅ Full PERCEIVE→JUDGE→DECIDE→ACT→LEARN→EMERGE cycle
- ✅ 11 Dogs vote with φ-bounded consensus
- ✅ 36+ dimensions (evolvable via ResidualDetector)
- ✅ 11 learning loops emit events to PostgreSQL
- ✅ Memory persists across restarts
- ✅ Context compression active (10:1 ratio)
- ✅ Calibration ECE < 0.1

**Week 12+ Validation**:
- ✅ Benchmarks: Ollama + CYNIC ≥ 91% quality
- ✅ Claude Solo (static) ≈ 85% quality (baseline)
- ✅ CYNIC uses CYNIC to improve CYNIC (self-building)
- ✅ Public release with demo video

---

## Related Documentation

### Essential (Start Here)
- **[todolist.md](./todolist.md)** - Week 1-8 implementation plan, φ-fractal timeline
- **[CLAUDE.md](./CLAUDE.md)** - Identity, personality, amplification vision
- **[README.md](./README.md)** - Project overview, quick start, philosophy

### Reference (Canonical Architecture)
- **[docs/reference/README.md](./docs/reference/README.md)** - Index of 9 canonical docs
- **[docs/reference/01-ARCHITECTURE.md](./docs/reference/01-ARCHITECTURE.md)** - Complete system architecture (this doc is executive summary)
- **[docs/reference/08-KERNEL.md](./docs/reference/08-KERNEL.md)** - 9 essential components (~3000 LOC)
- **[docs/reference/05-HEXAGONAL-ARCHITECTURE.md](./docs/reference/05-HEXAGONAL-ARCHITECTURE.md)** - 7 ports, adapters, testing
- **[docs/reference/SOURCES.md](./docs/reference/SOURCES.md)** - Extraction process from 29 fragmented docs

### Historical Context
- **[CYNIC-FULL-PICTURE-METATHINKING.md](./CYNIC-FULL-PICTURE-METATHINKING.md)** - Metathinking synthesis (source of docs/reference)
- **[docs/philosophy/VISION.md](./docs/philosophy/VISION.md)** - Philosophical foundation

---

## Quick Start (Python Kernel)

```bash
# Clone + install
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC/cynic
pip install -e .

# Setup PostgreSQL
docker compose up -d postgres

# Setup Ollama
ollama pull qwen2.5:14b

# Run Week 1 E2E test
pytest cynic/test/test_kernel_e2e.py
```

**Status**: Week 1 bootstrap in progress. Not production-ready yet.

---

**Last Updated**: 2026-02-16
**Version**: 2.0 (Python Kernel Era)
**φ-Confidence**: 61.8% (max)

*Le chien connaît maintenant son architecture.*
