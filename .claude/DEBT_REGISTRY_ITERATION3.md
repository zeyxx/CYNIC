# CYNIC CODEBASE DEBT REGISTRY

> *sniff* Ralph analyzing 254 source files, 23 tests, 72K lines of code
> **Test coverage: 9.1%** (should be 20%+) — MAJOR DEBT SIGNAL

**Analysis timestamp:** 2026-02-21
**Codebase**: cynic/cynic/ (excluding venv, cache)
**Scope**: Phase 0-1 production code

---

## SECTION 1: CRITICAL DEBT (Risk 8-10, Must Fix Before Phase 2)

### D1: Exception Handling Anti-Pattern

**Component**: api, cognition, core, organism (entire codebase)
**Risk Level**: 9/10
**Impact**: Silent failures, undebuggable errors in production
**Fix Effort**: 4/10 (tedious but systematic)
**Why it happened**: Early code used `except Exception` for quick error swallowing

**Code example (BAD)**:
```python
try:
    result = await some_async_op()
except Exception as e:
    logger.error("Something broke: %s", e)
    return None  # Caller doesn't know what went wrong
```

**Occurrences**:
- 374 bare `except Exception:` handlers (vs proper specific exceptions)
- 3 bare `except:` clauses (catches KeyboardInterrupt, SystemExit!)
- Affects: server.py, state.py, all routers, all handlers, neurons

**Why this blocks Phase 2**:
- When CYNIC self-modifies thresholds, errors get swallowed
- Impossible to debug production failures
- Users see `None` returned, no diagnostic data

**Fix plan**:
1. Create exception hierarchy: `CynicException`, `JudgmentError`, `PersistenceError`, etc.
2. Replace each `except Exception` with specific exception types
3. Add exception context to event bus (emit ERROR_OCCURRED events)
4. Never silently return None — propagate or emit

---

### D2: Missing Test Coverage (9.1% → Should Be 20%+)

**Component**: Full codebase
**Risk Level**: 9/10
**Impact**: Untested code paths break in production, regressions sneak in
**Fix Effort**: 8/10 (significant effort, but necessary)
**Why it happened**: Phase 0-1 focused on implementation, not tests

**By module**:
```
api/routers/       — 5 test files
cognition/cortex/  — 2 test files
core/storage/      — 0 test files ← NO PERSISTENCE TESTS!
tests/             — 19 files total
```

**Critical gaps**:
- **Storage layer (0 tests)**: Postgres, SurrealDB, persistence, migration
- **Event bus (0 dedicated tests)**: 3 buses, genealogy tracking, loop prevention
- **Handlers (partial)**: cycle_macro, cycle_micro, evolve, act_executor
- **Neurons (0 tests)**: All 11 Dogs should have unit tests
- **Configuration (0 tests)**: config.py, config_adapter.py

**Why this blocks Phase 2**:
- Self-modification code (Phase 1.5) has 0 tests
- Can't prove threshold changes are safe
- Multi-CYNIC consensus (Phase 2) requires proven reliability

**Fix plan**:
1. Add 100+ lines of tests for storage layer (Postgres CRUD, migrations)
2. Add 50+ lines for event bus (loop prevention, genealogy)
3. Add 40+ lines per Dog (unit tests for all 11)
4. Add integration tests for full judgment cycle

---

### D3: Architecture Fragmentation (3 Event Buses)

**Component**: core/event_bus.py, cognition/cortex/orchestrator.py, api/state.py
**Risk Level**: 8/10
**Impact**: Complex wiring, hard to debug, message ordering issues
**Fix Effort**: 6/10 (requires refactoring event flow)
**Why it happened**: Different event types grew organically, led to 3 separate buses

**The problem**:
```
CORE bus    ← Judgments, learning, consciousness
AUTOMATION bus ← Triggers, ticks, cycles
AGENT bus   ← Dog signals, PBFT messages

With EventBusBridge connecting all 3:
- Genealogy tracking adds complexity
- Loop prevention requires distributed tracing
- Message ordering not guaranteed across buses
```

**Why this blocks Phase 2**:
- Multi-CYNIC consensus needs guaranteed message ordering
- Current 3-bus design doesn't guarantee causal ordering
- Self-modification events can get reordered

**Fix plan**:
1. Consolidate 3 buses into 1 with semantic namespacing (prefix events: `JUDGMENT_*`, `AGENT_*`)
2. Remove EventBusBridge, use single queue with subscriber filters
3. Guarantee causal ordering at disk (event log)

---

### D4: Giant Classes Need Splitting

**Component**: scheduler.py, orchestrator.py, consciousness_service.py, and 7 more
**Risk Level**: 8/10
**Impact**: Hard to test, hard to modify, high cognitive load
**Fix Effort**: 7/10 (requires careful refactoring)
**Why it happened**: Functionality grew, no periodic refactoring

**Giant classes (>200 lines)**:
- `ConsciousnessRhythm` (300 lines) — scheduler logic too mixed
- `JudgeOrchestrator` (300 lines) — orchestration + synthesis
- `ConsciousnessService` (300 lines) — service + state + routing
- `ResidualDetector` (300 lines) — detection + feedback + learning
- 6 more Dogs and cortex modules

**Why this blocks Phase 2**:
- Self-modification code needs to modify parts of `JudgeOrchestrator`
- Can't safely refactor without breaking tests (and we have few tests)
- New features can't be isolated

**Fix plan**:
1. Split `JudgeOrchestrator` into: `Orchestrator`, `QTableUpdater`, `JudgmentSynthesizer`
2. Split `ConsciousnessRhythm` into: `Scheduler`, `QueueManager`, `WorkerPool`
3. Split `ConsciousnessService` into: `Service`, `StateManager`, `Router`
4. Create small focused classes (<150 lines each)

---

### D5: Fire-and-Forget Tasks (Race Conditions)

**Component**: server.py, event_bus.py, routers/sdk.py, organism/state_manager.py
**Risk Level**: 8/10
**Impact**: Tasks fail silently, data loss, inconsistent state
**Fix Effort**: 3/10 (just add await tracking)
**Why it happened**: Async background work used `asyncio.create_task()` without tracking

**Locations**:
```python
# server.py line ~500
asyncio.create_task(_execute_and_emit())  # Fires and forgets

# event_bus.py line ~200
asyncio.create_task(self._run_handler(handler, event))  # Fires and forgets

# routers/sdk.py
asyncio.create_task(_persist_sdk_session())  # Fires and forgets
```

**Why this blocks Phase 2**:
- Self-modification events might not persist
- Threshold changes lost on crash
- Multi-CYNIC consensus can't verify completion

**Fix plan**:
1. Track all create_task() calls in a set
2. On shutdown, await all pending tasks
3. Log task failures with context
4. Add timeouts to prevent hangs

---

## SECTION 2: MAJOR DEBT (Risk 6-7, Should Fix Before Phase 2)

### D6: No Type Hints in Many Files

**Component**: scheduler.py, organism modules, perceive workers
**Risk Level**: 7/10
**Impact**: IDE can't autocomplete, bugs from wrong types, hard to refactor
**Fix Effort**: 5/10 (mechanical but time-consuming)
**Why it happened**: Quick prototyping prioritized over type safety

**Examples**:
```python
def __init__(self, orchestrator: Any) -> None:  # Too generic
def register_perceive_worker(self, worker: Any) -> None:  # Should be PerceiveWorker
```

**Fix plan**:
1. Add TypedDict for all dataclass-like structures
2. Add Protocol for interfaces (PerceiveWorker, Handler, etc.)
3. Run mypy with `--strict` mode
4. Fix all type errors

---

### D7: Logging Inconsistency

**Component**: Throughout codebase
**Risk Level**: 6/10
**Impact**: Hard to trace execution, debug production issues
**Fix Effort**: 4/10 (systematic replacement)
**Why it happened**: Multiple logging styles from different authors

**Problems**:
```python
logger.info("DEBUG: ...")  # INFO level but says DEBUG
logger.info("*sniff* CYNIC ...")  # Dog voice mixed with logs
logger.error("Something broke")  # No context
```

**Fix plan**:
1. Use structured logging (JSON format for all logs)
2. Add request_id to all logs
3. Use consistent dog expressions only in CLI, not in logs
4. Add log levels: TRACE, DEBUG, INFO, WARN, ERROR

---

### D8: Configuration Hard-coded Values

**Component**: core/phi.py, core/formulas.py, scheduler.py
**Risk Level**: 6/10
**Impact**: Hard to tune parameters, can't A/B test, difficult to deploy
**Fix Effort**: 3/10 (move to config files)
**Why it happened**: Constants defined close to code for clarity

**Examples**:
```python
# In scheduler.py
_QUEUE_CAPACITY = fibonacci(10)  # 55 — hardcoded formula

# In consciousness.py
THRESHOLDS = {"HOWL": 80, "WAG": 50, "GROWL": 38.2}  # Static
```

**Fix plan**:
1. Move all parameters to CynicConfig
2. Load from environment or config file
3. Allow runtime override via API
4. Log all configuration at startup

---

### D9: Circular Dependency Risk (Not Yet, But Growing)

**Component**: api/state.py imports cognition/, cognition/ imports api/
**Risk Level**: 6/10
**Impact**: Import-time issues, hard to test, runtime errors
**Fix Effort**: 5/10 (requires careful import reordering)
**Why it happened**: Mutual dependencies between API layer and cognition layer

**Fix plan**:
1. Audit import graph with `pipdeptree`
2. Create dependency injection layer
3. Use TYPE_CHECKING for circular imports
4. Add import order tests

---

## SECTION 3: MODERATE DEBT (Risk 4-5, Nice to Fix Before Phase 2)

### D10: No Connection Pooling for Database

**Component**: core/storage/postgres.py
**Risk Level**: 5/10
**Impact**: Database connection exhaustion under load
**Fix Effort**: 3/10 (use asyncpg pool)
**Why it happened**: Early design didn't anticipate concurrency load

---

### D11: Error Messages Not User-Friendly

**Component**: api/models.py, core/exceptions.py
**Risk Level**: 4/10
**Impact**: Users confused by technical errors
**Fix Effort**: 4/10 (rewrite error messages)
**Why it happened**: Developer-focused error messages

---

### D12: No Rate Limiting

**Component**: api/routers/* (all endpoints)
**Risk Level**: 5/10
**Impact**: DoS attacks, resource exhaustion
**Fix Effort**: 3/10 (add middleware)
**Why it happened**: Prototype focused on functionality, not operations

---

## SECTION 4: MINOR DEBT (Risk 2-3, Can Fix After Phase 2)

### D13: Inconsistent Code Formatting

**Risk Level**: 2/10
**Why it happened**: Multiple authors, no pre-commit hooks

---

### D14: Sparse Documentation

**Risk Level**: 3/10
**Why it happened**: Code written faster than docs

---

---

## DEBT SUMMARY TABLE

| ID | Component | Risk | Impact | Effort | Blocks |
|----|-----------|------|--------|--------|--------|
| D1 | Exception handling | 9 | Silent failures | 4 | Phase 1.5 |
| D2 | Test coverage (9%) | 9 | Regressions | 8 | Phase 2 |
| D3 | 3 Event buses | 8 | Message ordering | 6 | Phase 2 |
| D4 | Giant classes | 8 | Hard to refactor | 7 | Phase 2 |
| D5 | Fire-and-forget tasks | 8 | Race conditions | 3 | Phase 1.5 |
| D6 | Missing type hints | 7 | IDE blind | 5 | Phase 2 |
| D7 | Logging inconsistency | 6 | Debug hard | 4 | Phase 2 |
| D8 | Hard-coded config | 6 | Not tunable | 3 | Phase 2 |
| D9 | Circular dependencies | 6 | Import issues | 5 | Phase 2 |
| D10 | No DB pooling | 5 | Conn exhaustion | 3 | Phase 2 |
| D11 | Bad error messages | 4 | User confusion | 4 | Phase 2 |
| D12 | No rate limiting | 5 | DoS risk | 3 | Phase 2 |

**TOP 5 BLOCKERS (Must fix before Phase 2)**:
1. Exception handling (D1) — Safety issue
2. Test coverage (D2) — Quality issue
3. Event bus (D3) — Architectural issue
4. Giant classes (D4) — Maintainability issue
5. Fire-and-forget (D5) — Correctness issue

---

**Ralph tracking:** Iteration 3/15. Identified 14 major debt categories. Next: Create elimination plan.

