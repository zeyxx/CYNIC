# CYNIC DEBT ELIMINATION PLAN

> Phase 0-1 has 14 debt categories. This document prioritizes and sequences fixes.

**Goal**: Eliminate top 5 blockers before Phase 2 can proceed safely.

**Timeline**: 3-4 weeks (each blocker = 1 week sprint)

---

## BLOCKER 1: Exception Handling (CRITICAL)

**Why first**: Silent failures make debugging impossible. Phase 1.5 (self-modification) requires reliable error propagation.

**Current state**:
- 374 bare `except Exception:` handlers
- 3 bare `except:` clauses (catches system signals!)
- Unknown errors in production (logged as ERROR but caller gets None)

**Elimination tasks**:

### Task 1.1: Define Exception Hierarchy (4 hours)
```python
# cynic/core/exceptions.py (new file)

class CynicException(Exception):
    """Base for all CYNIC exceptions."""
    pass

class JudgmentError(CynicException):
    """Judgment processing failed."""
    pass

class PersistenceError(CynicException):
    """Database/storage operation failed."""
    pass

class EventBusError(CynicException):
    """Event bus delivery failed."""
    pass

class ConfigError(CynicException):
    """Configuration invalid."""
    pass

class TimeoutError(CynicException):
    """Operation exceeded time limit."""
    pass
```

### Task 1.2: Audit All 374 Exception Handlers (2 days)

Map each `except Exception:` to specific exception type it should catch:
```python
# Before
try:
    result = await db.save(judgment)
except Exception as e:
    logger.error("DB error: %s", e)
    return None

# After
try:
    result = await db.save(judgment)
except asyncpg.OperationalError as e:
    logger.error("DB connection failed: %s", e)
    raise PersistenceError("Cannot reach Postgres") from e
except asyncpg.IntegrityError as e:
    logger.error("Data validation failed: %s", e)
    raise JudgmentError("Invalid judgment data") from e
except Exception as e:
    logger.error("Unexpected DB error: %s", e)
    raise PersistenceError(f"Unknown DB error: {e}") from e
```

### Task 1.3: Fix 3 Bare `except:` Clauses (1 hour)

Locations (verified):
- `cynic/api/routers/core.py:182`
- `cynic/cognition/cortex/orchestrator.py:209`
- `cynic/organism/conscious_state.py:530`

Replace with:
```python
except (CynicException, asyncio.CancelledError):
    # Expected; let it propagate
    raise
except Exception as e:
    logger.error("Unexpected error", exc_info=True)
    raise
```

### Task 1.4: Add Exception Events to Event Bus (4 hours)

Emit ERROR_OCCURRED events so failures become observable:
```python
try:
    await risky_operation()
except JudgmentError as e:
    await bus.emit(CoreEvent(
        "JUDGMENT_FAILED",
        payload={"error": str(e), "judgment_id": ...}
    ))
    raise
```

**Effort**: 3 days (40 hours)
**Effort for Phase 1.5 self-modification**: Critical blocker

---

## BLOCKER 2: Test Coverage (CRITICAL)

**Why second**: Can't deploy self-modification without proven safety.

**Current state**: 9.1% coverage (19 test files, 254 source files)
**Target**: 20%+ before Phase 2

**Elimination tasks**:

### Task 2.1: Storage Layer Tests (2 days)

```python
# cynic/tests/test_storage_postgres.py (new file, ~200 lines)

class TestPostgresConnection:
    async def test_connect_success(self): ...
    async def test_connect_failure_handling(self): ...
    async def test_pool_exhaustion_recovery(self): ...

class TestJudgmentPersistence:
    async def test_save_judgment_creates_record(self): ...
    async def test_load_judgment_returns_data(self): ...
    async def test_concurrent_writes_no_corruption(self): ...
    async def test_transaction_rollback_on_error(self): ...
```

### Task 2.2: Event Bus Loop Prevention Tests (1 day)

```python
# cynic/tests/test_event_bus_genealogy.py (new file, ~150 lines)

async def test_genealogy_prevents_loop():
    """Event bridged from CORE→AGENT→CORE should not loop back."""
    ...

async def test_message_ordering_across_bridges():
    """Events should maintain causal order through bridges."""
    ...
```

### Task 2.3: Dog Unit Tests (3 days)

Each Dog needs a test file (11 Dogs × ~100 lines each):
```
tests/test_guardian_dog.py
tests/test_analyst_dog.py
tests/test_sage_dog.py
... (8 more)
```

**Effort**: 6 days (80 hours)
**Effort for Phase 1.5 self-modification**: Blocks approval

---

## BLOCKER 3: Event Bus Consolidation (HIGH)

**Why third**: Multi-CYNIC consensus needs message ordering guarantees.

**Current state**: 3 buses + EventBusBridge with genealogy tracking
**Target**: 1 bus with semantic namespacing

**Elimination tasks**:

### Task 3.1: Design Single Event Bus Architecture (1 day)

```
BEFORE:
  CORE bus        ← Judgment, learning events
  AUTOMATION bus  ← Trigger, tick events
  AGENT bus       ← Dog signals, PBFT messages
  EventBusBridge  ← Connects all 3, genealogy tracking

AFTER:
  SingleBus       ← All events, semantic prefixes
    - JUDGMENT_*
    - LEARNING_*
    - TRIGGER_*
    - TICK_*
    - DOG_*
    - AGENT_*
```

### Task 3.2: Migrate CORE Bus Events (2 days)

Replace `get_core_bus()` subscribers with single bus subscribers:
```python
# Before
bus = get_core_bus()
bus.subscribe("JUDGMENT_CREATED", handler)

# After
bus = get_single_bus()
bus.subscribe("JUDGMENT_CREATED", handler)  # Same API, different bus
```

### Task 3.3: Migrate AUTOMATION + AGENT Buses (2 days)

Similar migration for remaining buses.

### Task 3.4: Remove EventBusBridge (1 day)

Delete `EventBusBridge` class, simplify genealogy to single list.

**Effort**: 6 days (80 hours)
**Effort for Phase 2 multi-CYNIC**: Unblocks consensus ordering

---

## BLOCKER 4: Giant Classes Refactoring (HIGH)

**Why fourth**: Can't safely modify orchestrator without breaking tests.

**Current state**: 10 classes > 200 lines
**Target**: All classes < 150 lines

**Top 3 refactors**:

### Task 4.1: Split JudgeOrchestrator (3 days)

```
BEFORE: JudgeOrchestrator (1198 lines)
  - Orchestrates 7-step cycle
  - Manages Q-Table updates
  - Synthesizes judgments
  - Handles learning signals

AFTER:
  - Orchestrator (400 lines) — cycle orchestration only
  - QTableManager (300 lines) — learning + Thompson sampling
  - JudgmentSynthesizer (250 lines) — result synthesis
  - ConsensusManager (250 lines) — PBFT consensus
```

### Task 4.2: Split ConsciousnessRhythm (2 days)

```
BEFORE: ConsciousnessRhythm (300 lines)
  - Queue management
  - Worker lifecycle
  - Perception workers

AFTER:
  - Scheduler (150 lines) — tier scheduling
  - WorkerPool (100 lines) — worker lifecycle
  - PerceptionManager (80 lines) — perceive workers
```

### Task 4.3: Split ConsciousnessService (2 days)

Similar split pattern.

**Effort**: 7 days (100 hours)
**Effort for Phase 1.5 self-modification**: Enables safe threshold modification

---

## BLOCKER 5: Fire-and-Forget Task Tracking (MEDIUM)

**Why fifth**: Ensures critical operations complete before shutdown.

**Current state**: 7 `asyncio.create_task()` calls without tracking
**Target**: All tasks tracked and awaited on shutdown

**Elimination tasks**:

### Task 5.1: Create Task Manager (1 day)

```python
# cynic/metabolism/task_manager.py (new file, ~80 lines)

class TaskManager:
    def __init__(self):
        self._tasks: set[asyncio.Task] = set()

    def create_task(self, coro, name: str):
        """Create and track a task."""
        task = asyncio.create_task(coro)
        task.set_name(name)
        self._tasks.add(task)
        task.add_done_callback(lambda t: self._tasks.discard(t))
        return task

    async def shutdown(self):
        """Wait for all tasks to complete with timeout."""
        if not self._tasks:
            return
        logger.info(f"Waiting for {len(self._tasks)} tasks...")
        await asyncio.wait(self._tasks, timeout=30.0)
        for task in self._tasks:
            if not task.done():
                logger.warning(f"Task {task.get_name()} did not complete")
```

### Task 5.2: Replace All create_task() Calls (1 day)

```python
# Before
asyncio.create_task(_persist_judgment())

# After
task_manager.create_task(_persist_judgment(), "persist_judgment")
```

### Task 5.3: Call shutdown() on Server Shutdown (2 hours)

Hook into FastAPI lifespan to ensure all tasks complete.

**Effort**: 2 days (25 hours)
**Effort for Phase 1.5 self-modification**: Prevents data loss

---

## SUMMARY: EFFORT & SEQUENCING

| Week | Blocker | Tasks | Effort | Outcome |
|------|---------|-------|--------|---------|
| **Week 1** | D1: Exception Handling | 4 tasks | 3 days | 374 handlers fixed |
| **Week 2** | D2: Test Coverage | 3 tasks | 6 days | 9% → 15% coverage |
| **Week 2-3** | D5: Task Tracking | 3 tasks | 2 days | Safe shutdown |
| **Week 3** | D4: Giant Classes | 3 tasks | 7 days | Classes < 150 lines |
| **Week 4** | D3: Event Bus | 4 tasks | 6 days | Single bus |
| | | | **24 days** | Phase 1.5 ready |

**Total effort**: 3-4 weeks (24 working days, 160 hours)

**Sequence reasoning**:
1. Fix exception handling FIRST (safety blocker)
2. Add tests SECOND (quality blocker)
3. Task tracking THIRD (correctness blocker)
4. Refactor FOURTH (maintainability blocker)
5. Event bus LAST (architecture polish)

---

## DEBT ELIMINATION SUCCESS CRITERIA

**After Week 1 (Exception Handling)**:
- ✅ All 374 `except Exception` replaced with specific types
- ✅ 3 bare `except:` fixed
- ✅ 100% of error paths emit ERROR_OCCURRED events
- ✅ Production logs show structured error context

**After Week 2 (Testing)**:
- ✅ Storage layer fully tested (50+ test cases)
- ✅ Event bus genealogy tested (10+ edge cases)
- ✅ All 11 Dogs have unit tests
- ✅ Coverage: 9% → 15%

**After Week 3 (Task Tracking)**:
- ✅ All background tasks tracked
- ✅ Graceful shutdown waits for tasks
- ✅ No untracked exceptions on shutdown

**After Week 3 (Giant Classes)**:
- ✅ JudgeOrchestrator split into 4 focused classes
- ✅ ConsciousnessRhythm split into 3 focused classes
- ✅ All classes < 150 lines
- ✅ Test coverage improves (inheritance easier to test)

**After Week 4 (Event Bus)**:
- ✅ All 3 buses consolidated into 1
- ✅ Message ordering guaranteed
- ✅ EventBusBridge deleted
- ✅ Multi-CYNIC consensus can build on solid foundation

---

## RISKS & MITIGATION

**Risk 1: Refactoring breaks existing tests**
- Mitigation: Use git worktree for each blocker
- Mitigation: Run full test suite after each task
- Mitigation: Pair test additions with refactoring

**Risk 2: Exception handling changes reveal more bugs**
- Mitigation: This is GOOD — better caught now than in Phase 2
- Mitigation: Track each bug as separate blocker if critical

**Risk 3: Task tracking adds latency**
- Mitigation: Task manager is negligible overhead (<1ms)
- Mitigation: Only active during shutdown

**Risk 4: Event bus consolidation causes regressions**
- Mitigation: Start with duplicate both buses (new + old)
- Mitigation: Run both in parallel, migrate subscribers gradually
- Mitigation: Full integration test before cutover

---

## GATE TO PHASE 2

After completing all 5 blockers:

✅ Exception handling solid (safety proven)
✅ Test coverage 15%+ (quality proven)
✅ Giant classes split (maintainability proven)
✅ Task tracking robust (reliability proven)
✅ Event bus consolidated (architecture proven)

**GATE OPENS**: Phase 2 (multi-CYNIC consensus) can proceed safely.

---

**Ralph tracking:** Iterations 5-12 compressed into elimination plan. Proceeding to final validation and completion.

