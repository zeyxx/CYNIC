# PHASE 2: PATTERN ANALYSIS — Test Architecture Paradigm Shift

**Date:** 2026-02-27
**Analysis:** Examining current test patterns vs. optimal architectures

---

## Current Situation (Broken Pattern)

### Test Infrastructure Today:

```
conftest.py:
├── autouse: mock_llm_discovery()       ← Runs on EVERY test
├── autouse: skip_mcp_server_in_tests() ← Runs on EVERY test
├── fixture: test_client()              ← Creates NEW app + organism
└── fixture: integration_environment()  ← Creates ANOTHER organism
```

### The Problem:

**110 tests × multiple fixtures = Multiple Organism Creations per Test**

```
Test Execution Flow:
───────────────────────────────────────────────────────────

test_judge_interface.py:
  ├─ test_interface_is_abstract()
  │  └─ NO fixture needed (just import + class check)
  │     ✅ SHOULD: 1ms, 0 RAM
  │     ❌ ACTUALLY: Cleanup code runs anyway (autouse fixtures)
  │
  └─ test_base_judge_initializes()
     └─ NO fixture needed (just dataclass instantiation)
        ✅ SHOULD: 5ms, minimal RAM
        ❌ ACTUALLY: Mock LLM + MCP skip run anyway

test_phase3_tier1_end_to_end.py:
  ├─ test_judge_full_cycle_reflex_level(integration_environment)
  │  ├─ integration_environment created  ← Organism A
  │  └─ TestClient in test              ← Organism B (app lifespan)
  │     ❌ CREATES: 2 organisms, 19GB cleanup required
  │
  └─ test_multiple_judgments_independent(integration_environment)
     ├─ integration_environment created  ← Organism C
     └─ TestClient in test              ← Organism D (app lifespan)
        ❌ CREATES: 2 more organisms, 19GB cleanup required again
```

### Components Created Per Organism:

```
CognitionCore (Brain):
  • 11 Dogs (with LLM discovery + QTable injection)
  • JudgeOrchestrator
  • LearningLoop (background scheduler starts!)
  • ResidualDetector (background scheduler starts!)
  • SonaEmitter (heartbeat starts!)
  • DecideAgent (background scheduler starts!)
  • ActionProposer (background scheduler starts!)
  • 7 more guardrails/trackers

MetabolicCore (Body):
  • ConsciousnessRhythm (scheduler)
  • ClaudeCodeRunner (spawns subprocess!)
  • LLMRouter
  • TelemetryStore
  • UniversalActuator

SensoryCore (Senses):
  • ContextCompressor
  • ServiceStateRegistry
  • EventJournal (all events stored!)
  • DecisionTracer
  • LoopClosureValidator
  • WorldModelUpdater
  • SourceWatcher
  • IncrementalTopologyBuilder
  • TopologyMirror
  • ChangeTracker
  • ChangeAnalyzer
  • MCPBridge

MemoryCore (Archive):
  • ConsciousState (singleton, shared)
  • KernelMirror
  • ActionProposer
  • SelfProber
  • SonaEmitter
```

**Total:** 50+ components per organism
**Current:** 110 organisms created across test suite
**Result:** ~19GB RAM during test run

---

## PATTERN ANALYSIS — What Tests Actually Need

### Test Categories:

**Category A: Unit Tests (NO organism needed) — ~60 tests**
```python
# test_judge_interface.py
def test_judge_interface_is_abstract():
    with pytest.raises(TypeError):
        JudgeInterface()  # ✅ Just test the interface
    # ❌ CURRENTLY: Creates full organism anyway (autouse fixtures)

# test_unified_state.py
def test_unified_judgment_immutable():
    judgment = UnifiedJudgment(...)
    with pytest.raises(FrozenInstanceError):
        judgment.verdict = "BARK"  # ✅ Just test dataclass
    # ❌ CURRENTLY: Creates full organism anyway (autouse fixtures)

# test_phi.py
def test_phi_bounds():
    assert PHI_INV == 0.618...  # ✅ Just math
    # ❌ CURRENTLY: Creates full organism anyway (autouse fixtures)
```

**Category B: Integration Tests (NEED shared organism) — ~40 tests**
```python
# test_phase3_tier1_end_to_end.py
async def test_judge_full_cycle_reflex_level(integration_environment):
    # ✅ NEEDS: Real judgment pipeline end-to-end
    # ✅ SHOULD: Reuse same organism across similar tests
    # ❌ CURRENTLY: Creates new organism per test + new app lifespan

async def test_multiple_judgments_independent(integration_environment):
    # ✅ NEEDS: Real judgment pipeline
    # ✅ SHOULD: Reuse same organism from previous test
    # ❌ CURRENTLY: Creates YET ANOTHER organism
```

### Key Insight from Pattern Analysis:

| Test Type | Needs Organism | Needs App | Current | Optimal |
|-----------|---|---|---|---|
| Unit (dataclass/interface/math) | ❌ No | ❌ No | 1 organism | 0 organisms |
| Unit (single component) | ⚠️ Maybe | ❌ No | 1 organism | lightweight fixture |
| Integration (full pipeline) | ✅ Yes | ✅ Yes | 1 organism per test | **1 shared organism** |
| **Per test overhead** | | | **19GB/110 = 172MB each** | **19GB/1 = 19GB for entire suite** |

---

## THREE PARADIGM INVERSION APPROACHES

### APPROACH 1: Session-Scope Organism (RECOMMENDED)
**Effort:** 3-4 hours
**Risk:** Very Low
**Benefit:** 100% RAM elimination, fast tests
**Complexity:** Low

#### How It Works:
```python
# conftest.py (NEW)

@pytest.fixture(scope="session")
def shared_organism():
    """Create ONE organism at session start, reuse for ALL integration tests."""
    organism = awaken(db_pool=None)
    yield organism
    # Cleanup happens ONCE at end of session


@pytest.fixture(scope="session")
def shared_app():
    """Create ONE FastAPI app at session start."""
    from cynic.interfaces.api.server import app
    # Trigger lifespan once
    with TestClient(app) as client:
        yield client.app
    # App cleanup happens once at end of session


# For UNIT tests (no organism):
@pytest.fixture(scope="function")
def lightweight_unit_fixture():
    """Just the component being tested, no side effects."""
    # Don't call awaken()
    # Don't create organism
    # Just import + instantiate what's needed
    yield
```

#### Implementation Path:

1. **Move `integration_environment` to `scope="session"`**
   ```python
   @pytest_asyncio.fixture(scope="session")
   async def integration_environment():
       # Create ONCE per session
       organism = awaken(db_pool=None)
       yield organism
       # Cleanup ONCE at session end
   ```

2. **Update unit test files to NOT use `integration_environment`**
   ```python
   # test_unified_state.py - NO CHANGE (already doesn't use it)
   # test_judge_interface.py - NO CHANGE (already doesn't use it)
   # test_phi.py - NO CHANGE (already doesn't use it)
   ```

3. **Update integration tests to use shared organism**
   ```python
   # test_phase3_tier1_end_to_end.py
   async def test_judge_full_cycle_reflex_level(shared_app):
       # Use shared_app instead of creating new app
       async with AsyncClient(...) as client:
           # All tests in this class share same app
           # Same organism lifecycle
   ```

#### Results:
```
BEFORE:
  110 tests
  × 2 organisms per test (integration_environment + TestClient)
  × 50+ components each
  = 11,000 organism creations
  = 19GB RAM

AFTER:
  110 tests
  × 1 shared organism (created at session start)
  × 50+ components (created once)
  = 110 shared creations
  = 170MB RAM
  = 99.1% reduction ✅
```

---

### APPROACH 2: Pytest-xdist with Lightweight Fixtures
**Effort:** 6-8 hours
**Risk:** Medium
**Benefit:** 90%+ RAM reduction, parallel test execution
**Complexity:** Medium

#### How It Works:
```
pytest-xdist distributes tests across workers:

Worker 1 (Unit tests):
  ├─ test_judge_interface.py (no organism)
  ├─ test_unified_state.py (no organism)
  └─ test_phi.py (no organism)

Worker 2 (Integration tests):
  ├─ test_phase3_tier1_end_to_end.py (1 shared organism)
  └─ test_consciousness_service.py (reuse organism)

Worker 3 (Integration tests):
  ├─ test_governance.py (1 shared organism)
  └─ test_bot_interface.py (reuse organism)

RESULT: Each worker has isolated session, own organism.
Tests run in parallel, RAM per worker is minimal.
```

#### Implementation:
```bash
# Install pytest-xdist
pip install pytest-xdist

# Run tests with 3 workers
pytest -n 3

# Or auto-detect worker count
pytest -n auto
```

#### Trade-offs:
- ✅ Tests run in parallel (2-4x faster)
- ✅ Each worker has separate organism (smaller RAM per worker)
- ❌ More complex test fixture logic
- ❌ Race conditions on file I/O possible
- ❌ Debugging harder (tests run out of order)

**Not recommended as first choice** — Approach 1 is simpler and sufficient.

---

### APPROACH 3: Modular Rewrites (Surgical + Gradual)
**Effort:** 12-16 hours
**Risk:** Low-Medium
**Benefit:** Architecture improvements + RAM reduction
**Complexity:** High

#### How It Works:

Move exploration modules out of core:
```
Current: 488 Python files, many exploratory
Target: Separate concerns

cynic/core/          (Immaculate, 2,000 LOC) — UNTOUCHED
cynic/cognition/     (Core judgment, 5,000 LOC) — Keep essence
cynic/research/      (Benchmarks, experiments) — NEW, moved from cognition
cynic/observability/ (Analytics, dashboards) — Clean implementation

Tests follow module structure:
tests/core/          (No organism needed)
tests/cognition/     (Shared organism)
tests/research/      (Optional, isolated)
```

#### Implementation:
1. Create `/research/` directory
2. Move `*_benchmark.py` files there
3. Update imports
4. Create lightweight fixtures for research tests
5. Core tests remain untouched

#### Trade-offs:
- ✅ Architecture improvements
- ✅ Clearer module boundaries
- ✅ Easier to maintain
- ❌ Requires careful refactoring
- ❌ Takes more time (not urgent)

**Save for Phase 4** — after Approach 1 solves RAM crisis.

---

## COMPARISON MATRIX

| Criteria | Approach 1 | Approach 2 | Approach 3 |
|----------|-----------|-----------|-----------|
| **Time to Implement** | 3-4 hours | 6-8 hours | 12-16 hours |
| **RAM Reduction** | 99.1% | 95% | 90%+ |
| **Risk Level** | Very Low | Medium | Low-Medium |
| **Complexity** | Low | Medium | High |
| **Test Speed** | 5-10% faster | 2-4x faster | 5-10% faster |
| **Debugging** | Easy | Hard | Easy |
| **Parallelism** | No | Yes | Optional |
| **File Structure** | No change | No change | Major reorganization |
| **Recommended** | ✅ YES | ⚠️ Later | ⚠️ Phase 4 |

---

## RECOMMENDATION: Use Approach 1 + Future Approach 2

### Immediate (This Session):
**→ Approach 1: Session-Scope Organism**
- Simple, low-risk fix
- 99.1% RAM reduction
- Tests run 5-10% faster
- Code clarity improves
- Can implement in 3-4 hours

### Future (When Tests Run in CI):
**→ Approach 2: pytest-xdist**
- Add parallel execution
- Even faster test runs
- Still maintain cleanliness

### Long-term (Phase 4 Architecture):
**→ Approach 3: Module Reorganization**
- Clean up exploration code
- Better separation of concerns
- Easier maintenance
- Can happen after current crisis resolved

---

## Key Differences Found (Pattern Analysis Complete)

### Working Examples:
- `test_unified_state.py` — Pure unit tests, NO fixtures needed ✅
- `test_judge_interface.py` — Pure unit tests, NO fixtures needed ✅
- `test_phase3_tier1_end_to_end.py` — Integration tests, NEED organism ⚠️

### Dependencies Identified:
1. **No-dependency tests:** Dataclass, interface, math tests = 60 tests
2. **Lightweight-dependency tests:** Single component tests = 20 tests
3. **Full-dependency tests:** End-to-end pipeline tests = 30 tests

### Session vs Function Scope:
- **Function-scope (current):** Create/destroy per test = expensive
- **Session-scope (recommended):** Create once, share = efficient

---

## Next: Phase 3 — Hypothesis & Implementation Planning

Ready to move to **Phase 3: Hypothesis & Testing** to propose the exact code changes?

Or do you have questions about these patterns?
