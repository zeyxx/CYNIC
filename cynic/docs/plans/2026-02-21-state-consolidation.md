# Day 2: State Consolidation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Consolidate 5 scattered state systems (q_table, conscious_state, dogs, residuals, actions) into unified OrganismState with 3-layer architecture (MEMORY/PERSISTENT/CHECKPOINT).

**Architecture:**
- Create `OrganismState` root class managing all internal state
- Implement 3 layers: MemoryLayer (fast, ephemeral), PersistentLayer (durable, DB), CheckpointLayer (session snapshots)
- Migrate existing subsystems one-by-one with TDD
- Maintain event-driven updates (no direct state mutations except via events)
- Add symbolic testing following Kani proof strength criteria

**Tech Stack:**
- Python dataclasses (immutable snapshots)
- asyncio locks (thread-safety)
- PostgreSQL (persistent layer)
- JSON (checkpoint serialization)
- pytest + hypothesis (property-based testing for symbolic coverage)

---

## Summary of Tasks

### Phase 1: Layer Architecture
1. Create OrganismState base class with 3-layer foundation
2. Add symbolic input testing (Hypothesis/hypothesis)

### Phase 2: Subsystem Migrations
3. Migrate Q-table to OrganismState
4. Migrate conscious_state to OrganismState
5. Migrate dogs registry to OrganismState
6. Migrate residuals to OrganismState
7. Migrate actions queue to OrganismState

### Phase 3: Integration
8. Add end-to-end integration tests (Kani criteria)
9. Update Organism class to use OrganismState
10. Add Kani audit documentation

---

## Key Files to Create

**New Files:**
- `cynic/organism/state_manager.py` (~800 LOC — OrganismState, 3 layers)
- `tests/organism/test_state_manager.py` (~500 LOC — unit + symbolic tests)
- `tests/organism/test_state_integration.py` (~300 LOC — integration tests)
- `docs/testing/KANI_AUDIT.md` (audit proof documentation)

**Modified Files:**
- `cynic/organism/organism.py` (add state property)
- `cynic/organism/__init__.py` (export OrganismState)
- `cynic/api/server.py` (pass pool to state during lifespan)

---

## Testing Strategy

**Kani Proof Strength Criteria (ALL 6 to implement):**

1. **Symbolic Input Coverage** — Use Hypothesis to generate arbitrary test values
2. **Vacuity Check** — Ensure frozen dataclasses prevent invalid states
3. **Branch Coverage** — All paths tested (default, exception, happy path)
4. **Invariant Strength** — ∀ transitions. Invariants hold (3-layer consistency)
5. **Inductive Reasoning** — Full state recovery works for all topologies
6. **Formal Spec** — No loops in state transitions; all mutations atomic

---

## Architecture Diagram

```
OrganismState (ROOT)
├─ MemoryLayer (Fast, ephemeral)
│  ├─ qtable: {state_key: {action: q_value}}
│  ├─ consciousness_level: "REFLEX"|"MICRO"|"MACRO"|"META"
│  ├─ recent_judgments: [judgment]
│  ├─ dogs: {dog_id: dog_status}
│  ├─ residuals: {residual_id: residual_state}
│  └─ pending_actions: [action]
│
├─ PersistentLayer (Durable, PostgreSQL)
│  ├─ q_learning table (async flush)
│  ├─ judgments table (important entries)
│  ├─ e_scores table (reputation)
│  └─ residuals table (tracking)
│
└─ CheckpointLayer (JSON snapshots)
   ├─ ~/.cynic/organism_state.json (full snapshot)
   ├─ ~/.cynic/organism_snapshot.json (metadata)
   └─ Recovery on startup (auto-recover from checkpoint)
```

---

## Next Step: Choose Execution Mode

**Plan complete and saved to `docs/plans/2026-02-21-state-consolidation.md`**

Two execution options:

**1. Subagent-Driven (this session, fast iteration)**
- I dispatch fresh subagent per task (1-2 tasks at a time)
- Review code between tasks
- Catch issues early
- Recommended: **Best for collaborative refinement**

**2. Parallel Session (separate, batch execution)**
- Open new Claude Code session with `superpowers:executing-plans`
- Execute full 10 tasks with checkpoints
- Returns when all tasks complete
- Recommended: **Best for autonomous completion**

**Which approach?**
