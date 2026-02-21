# CYNIC Architecture Drift Analysis

**Date**: 2026-02-21
**Timespan**: commits ff84278 through 0a355f0 (20 commits)
**Status**: ⚠️ MULTIPLE UNCOORDINATED THREADS

---

## Thread Identification

### Thread 1: Immune System Framework (commits 2d56c86 → 62ed5d9 → 58f23a0)
**Purpose**: Build safety system for detecting + fixing errors
**Files Created**:
- `scripts/cynic_immune_system.py` (746 LOC)
- `scripts/test_immune_framework.py` (387 LOC)

**Assumptions**:
- Can find + fix "pathogenic" code
- Consensus engine (3/5 detector agreement)
- Safe dry-run verification
- Human checkpoint before apply

**State at This Thread**: Standalone framework, not integrated into main codebase

---

### Thread 2: Organism State Refactoring (commits c04d916 → c36a80e)
**Purpose**: Migrate state management to `OrganismState` class
**Files Modified**:
- `cynic/tests/organism/test_state_manager.py` (25 tests)
- `cynic/cynic/organism/state_manager.py` (new structure)

**Assumptions**:
- ConsciousnessScheduler replaces DogScheduler
- OrganismState holds Q-table, conscious_state, etc.
- Memory layer for state persistence

**State at This Thread**: Test suite passing, but not wired into bootstrap

---

### Thread 3: Bootstrap Fixes (commits e4e676d → 2c0cf9f → 0a355f0)
**Purpose**: Repair kernel startup path
**Files Modified**:
- `cynic/api/state.py` (scheduler init, KernelServices wiring)
- `cynic/cognition/neurons/discovery.py` (module path fix)
- `cynic/cognition/cortex/self_probe.py` (FileNotFoundError handling)
- `cynic/api/handlers/__init__.py` (exception handling)

**Assumptions**:
- AppState is the boot container
- KernelServices has cognition/metabolic/senses domains
- Bootstrap uses _create_components() → _create_services() → _make_app_state()

**State at This Thread**: ~70% working, scheduler.register_perceive_worker() missing

---

## The Core Problem: Uncoordinated Architectural Decisions

### What Each Thread Assumes About AppState

| Component | Thread 1 (Immune) | Thread 2 (Organism) | Thread 3 (Bootstrap) | Reality |
|-----------|-------------------|---------------------|----------------------|---------|
| Scheduler | Not mentioned | `ConsciousnessScheduler` | `ConsciousnessScheduler` (but missing methods) | ❓ |
| State storage | Not mentioned | `OrganismState` class | `AppState` fields | ❓ |
| Q-table | Not mentioned | In `OrganismState._qtable` | In `AppState.qtable` | ❓ |
| Event bus | Not mentioned | Assumed (scheduler subscribe) | Integrated (learning_loop.start()) | ✓ |
| Dogs | Not mentioned | Discovered + orchestrated | In `AppState.orchestrator.dogs` | ✓ |

### Missing Coordination Points

**1. OrganismState vs AppState**
```
Thread 2 writes tests for OrganismState with methods like:
  - update_qtable_entry()
  - get_conscious_state()
  - update_conscious_state()

But Thread 3's bootstrap creates AppState with:
  - orchestrator (JudgeOrchestrator)
  - qtable (QTable instance)
  - learning_loop (LearningLoop instance)

QUESTION: Is AppState supposed to BE OrganismState?
Or are they separate concepts?
```

**2. Scheduler Interface Mismatch**
```
Thread 2 assumes scheduler has these methods:
  - register_perceive_worker()
  - schedule_judgment()
  - transition_level()

Thread 3 finds scheduler (ConsciousnessScheduler) has:
  - select_level() ✓
  - (missing register_perceive_worker()) ✗

BLOCKER: _wire_perceive_workers() can't run
```

**3. Service Layer Discovery**
```
Thread 3 creates KernelServices as domain-aggregator:
  - cognition: CognitionServices
  - metabolic: MetabolicServices
  - senses: SensoryServices

But Thread 3 can't satisfy all constructor args:
  - DirectActionsHandler needs universal_actuator + qtable (not provided)
  - JudgmentExecutorHandler needs orchestrator (maybe OK?)
  - guidance_writer can't find cynic.core.events module (doesn't exist)

RESULT: Handler discovery logs warnings but continues (breaks coverage)
```

---

## Timeline of Decisions

```
1. Phase 1 Validation (old baseline)
   ↓
2. Exception Handling Refactor (workflow improvement)
   ↓
3. State Refactoring Begins (Thread 2 starts: c04d916)
   - Creates OrganismState class
   - Assumes new architecture
   ↓
4. Immune System Created (Thread 1 starts: 2d56c86)
   - Standalone framework
   - No integration planned
   ↓
5. Bootstrap Investigation (Thread 3 starts: e4e676d)
   - Discovers mismatches
   - Tries to patch AppState/KernelServices
   - Hits scheduler interface issues
```

---

## What's Actually Working?

✅ **Core kernel components**:
- 11 dogs discovered + orchestrated
- JudgeOrchestrator properly wired
- Learning loop starts
- Event bus connected (3-bus aggregate)
- Q-table exists and accessible

✅ **Phase 3 API layer** (in theory):
- /judge, /perceive endpoints implemented
- Event emission working
- Response models defined

❌ **Integration glue**:
- Bootstrap path stalls at perceive worker registration
- Handler registry warns but doesn't fail
- Service layer partially wired
- Tests can't run end-to-end

---

## The Fix Matrix

### Option A: Merge Threads (2-3 hours)
1. Decide: **Is AppState supposed to inherit from OrganismState or vice versa?**
   - If yes: Merge class hierarchies, update bootstrap
   - If no: Define clear separation of concerns

2. **Fix scheduler interface**:
   - Either find/implement `register_perceive_worker()`
   - Or remove perceive worker wiring (temporary)

3. **Reconcile service layer**:
   - Fill in missing handler kwargs
   - Find or create `cynic.core.events`

### Option B: Establish Single Authority (1 hour)
1. **One committer makes decision**: Which thread is "source of truth"?
   - Thread 2 (OrganismState) or Thread 3 (AppState)?

2. **Rebase other threads** on that decision

3. **Retire conflicting assumptions**

### Option C: Parallel Universe (risky)
- Keep all three threads but mark them explicitly
- Thread 1 = immune framework (scripts/)
- Thread 2 = organism state (draft)
- Thread 3 = bootstrap (current prod)
- Risk: Constant merge conflicts

---

## Confidence Assessment

**Can fix these drifts**: 87% (φ-bounded to 61.8% = **61%**)
- Root causes identified ✓
- Decision points clear ✓
- Coordination path visible ✓
- But requires committed choices

**Can do so without breaking**: 52% (φ-bounded)
- Multiple interdependencies
- Linters reverting manual edits
- Code path unclear in places

---

## Recommendation

**STOP integrating tests. START coordinating threads.**

1. **Immediately**: Call which is "primary" (Thread 2 or Thread 3)?
2. **Then**: Rebase/merge the losing thread into winner
3. **Then**: Check integration tests
4. **Then**: Address missing scheduler methods

*sniff* The organism has too many brains. Time for a decision.

Confidence: 52% (φ-bounded) that we can coordinate successfully with human guidance.
