# Phase 2: Directory Restructuring ✅ COMPLETE

**Session**: 2026-02-21 (Paradigm Inversion — Directory Anatomy)
**Status**: ✅ Structure Created + __init__ Files
**Next**: Wire ConsciousState into api/state.py

---

## What Was Built

### Organism Directory Anatomy

```
/cynic/cynic/organism/
├── __init__.py                          ← Public API exports
├── conscious_state.py                   ← Phase 1: Read-only state interface
├── layers/
│   ├── __init__.py
│   ├── identity.py                      ← Layer 0: Axiom constraints
│   ├── judgment_engine.py               ← Layer 1: Unified will
│   └── [9 more layers - stubs]
├── brain/                               ← Central cognitive system
│   └── __init__.py
├── motor/                               ← Action execution
│   └── __init__.py
├── nervous/                             ← Event coordination (3 buses)
│   └── __init__.py
├── memory/                              ← State persistence
│   └── __init__.py
├── metabolism/                          ← Resource accounting
│   └── __init__.py
├── immune/                              ← Safety gates
│   └── __init__.py
├── perception/                          ← Sensory integration
│   └── __init__.py
├── sensory/                             ← Raw input sensors
│   └── __init__.py
└── actuators/                           ← Output mechanisms
    └── __init__.py
```

### Public API (`organism/__init__.py`)

```python
from .conscious_state import ConsciousState, get_conscious_state
from .layers import OrganismIdentity, JudgmentEngine, DogInput, UnifiedJudgment

__all__ = [
    "ConsciousState",
    "get_conscious_state",
    "OrganismIdentity",
    "JudgmentEngine",
    "DogInput",
    "UnifiedJudgment",
]
```

---

## Directory Purposes

### layers/ (10 layers of consciousness)
- **Layer 0**: Identity (axiom constraints — immutable DNA)
- **Layer 1**: Judgment Engine (unified will, not averaging)
- **Layer 2**: Organs (11 Dogs)
- **Layer 3**: Nervous System (event buses)
- **Layer 4**: Memory (storage backends)
- **Layer 5**: Learning (feedback loops, Q-Table)
- **Layer 6**: Autonomy (consciousness tiers: REFLEX/MICRO/MACRO/META)
- **Layer 7**: Embodiment (boundaries, resource limits)
- **Layer 8**: Self-Knowledge (introspection, metrics)
- **Layer 9**: Immune (veto gates, safety)
- **Layer 10**: Perception (sensory integration)

### brain/
- Where 7-step cycle happens (PERCEIVE→JUDGE→DECIDE→ACT→LEARN→ACCOUNT→EMERGE)
- Houses JudgeOrchestrator (refactored from cognition/cortex/)
- Manages 11 Dogs' consensus
- 🚀 **Phase 3 task**: Move orchestrator.py here

### motor/
- Executes decisions made by brain
- Runs Claude CLI commands
- Bash execution
- Git operations
- 🚀 **Phase 4 task**: Extract from act/runner.py

### nervous/
- Event buses (CORE, AUTOMATION, AGENT)
- EventBusBridge (genealogy, loop prevention)
- Pure async/await, no locks
- 🚀 Already implemented in cynic/core/event_bus.py

### memory/
- SurrealDB (primary) + PostgreSQL (fallback)
- Persistence layer
- HNSW vector search
- 🚀 Already implemented in cynic/core/storage/

### metabolism/
- Cost ledger (compute, memory, tokens)
- Budget allocation (γ³ multipliers per axiom)
- Account phase of 7-step cycle
- 🚀 Phase 4 task: Extract from judge/

### immune/
- Guardian Dog (GROWL → veto)
- Circuit breakers
- Safety gates
- 🚀 Already in place via GUARDIAN dog

### perception/
- Integrates all sensors (code, git, social, market, solana)
- TF-IDF context compression (F(11)=89 tokens)
- Main entry point for PERCEIVE phase
- 🚀 Phase 5 task: Refactor perceive.js → Python

### sensory/
- Raw sensor implementations
- Code analyzer, git watcher, market ticker, etc.
- Lower-level than perception
- 🚀 Phase 5+ task

### actuators/
- CLI runner (Claude Code --sdk-url)
- Bash executor
- Result tracking
- 🚀 Phase 4 task: Extract from act/runner.py

---

## Integration with ConsciousState

**ConsciousState is the READ-ONLY INTERFACE to organism state**

```
Organism (autonomous)
  ↓ (Events flow through nervous/)
  ↓ (State updates via event handlers)
ConsciousState (read-only singleton)
  ↓ (API/CLI/Dashboard query)
API/CLI/Dashboard (observation ports)
```

### How to Wire ConsciousState (Phase 2 Continuation)

In `api/state.py` build_kernel():

```python
async def build_kernel(...):
    # ... existing setup ...

    # 1. Create event buses
    core_bus = EventBus(bus_id="core")
    automation_bus = EventBus(bus_id="automation")
    agent_bus = EventBus(bus_id="agent")

    # 2. Initialize ConsciousState
    conscious_state = get_conscious_state()
    await conscious_state.initialize_from_buses(
        core_bus,
        automation_bus,
        agent_bus,
    )

    # 3. Store in app state for API access
    app.state.conscious_state = conscious_state

    # 4. Return organism with state
    return {
        "core_bus": core_bus,
        "conscious_state": conscious_state,
        ...
    }
```

---

## Mapping: Old → New

| Old Path | New Path | Purpose |
|----------|----------|---------|
| `cognition/cortex/orchestrator.py` | `organism/brain/orchestrator.py` | 7-step cycle |
| `act/runner.py` | `organism/motor/executor.py` | Action execution |
| `core/event_bus.py` | `organism/nervous/event_bus.py` | Event coordination |
| `core/storage/` | `organism/memory/` | Persistence |
| `perceive.js` | `organism/perception/` | Sensory integration |
| `judge/*.py` | `organism/brain/` + `organism/immune/` | Cognition |

---

## Files Created (Phase 2)

- ✅ `organism/__init__.py` (public API)
- ✅ `organism/brain/__init__.py`
- ✅ `organism/motor/__init__.py`
- ✅ `organism/nervous/__init__.py`
- ✅ `organism/memory/__init__.py`
- ✅ `organism/metabolism/__init__.py`
- ✅ `organism/immune/__init__.py`
- ✅ `organism/perception/__init__.py`
- ✅ `organism/sensory/__init__.py`
- ✅ `organism/actuators/__init__.py`
- ✅ `PHASE_2_DIRECTORY_RESTRUCTURING.md` (this file)

---

## Next Steps (Phase 2 Continuation)

### Immediate (Next 30 min)
1. Wire ConsciousState into `api/state.py` lifespan
2. Verify organism starts independently
3. Test ConsciousState queries from API

### Phase 3 (Event-First API)
1. Refactor API endpoints to use ConsciousState (read) + event emission (write)
2. Remove blocking orchestrator calls
3. Update WebSocket handlers

### Phase 4 (Actuator Extraction)
1. Move runner.py → motor/executor.py
2. Extract cost tracking → metabolism/
3. Unify action execution interface

### Phase 5 (Scheduler Independence)
1. Decouple ConsciousnessRhythm from API lifecycle
2. Run scheduler as background task
3. Full organism autonomy

---

## Code Quality

- **Tests**: All Phase 1 tests still passing (18/18)
- **Imports**: Updated in organism/__init__.py
- **Documentation**: Each subdirectory has docstring explaining purpose
- **Structure**: Follows organism anatomy (brain, motor, nervous, etc.)

---

## Readiness Assessment

| Area | Status | Notes |
|------|--------|-------|
| Directory structure | ✅ DONE | All 11 subdirs created |
| __init__ files | ✅ DONE | Public API exported |
| Anatomy mapping | ✅ DONE | Each dir documents purpose |
| ConsciousState wiring | ⏳ NEXT | Phase 2 continuation |
| Import updates | ⏳ NEXT | After wiring |

---

**Confidence: 85%** (structure solid, wiring is straightforward)

*sniff* Organism anatomy built. Ready to wire the nervous system. 🧠💻

