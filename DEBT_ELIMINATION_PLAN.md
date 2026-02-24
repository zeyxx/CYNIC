# CYNIC Debt Elimination Plan
> "φ distrusts φ" — Systematic consolidation toward professional clarity

**Date**: 2026-02-23
**Status**: 🟡 Phase 1 Ready to Execute
**Confidence**: 61.8% (φ⁻¹ limit)
**Duration**: 3-4 weeks total (Phase 1 = 1 week, Phase 2-3 = 2-3 weeks)

---

## Executive Summary

### Current State (Audit 2026-02-23)
```
Codebase: 303 Python files, 8,394 LOC
Test coverage: ~40% (6,278 test LOC)
Type safety: 87.5% coverage (6 routers at 0%)
Debt markers: 56 (mostly in 13 files)
God objects: 3 (>900 LOC each)
Untested critical modules: 7
Dual code paths: 1 (api/state.py vs organism/organism.py)
```

### Target State (After Debt Elimination)
```
Codebase: 303 files, ~8,500 LOC (consolidation, not bloat)
Test coverage: >70% (15,000+ test LOC)
Type safety: >95% (strict mypy mode passing)
Debt markers: <3 (all resolved or documented)
God objects: 0 (all split <600 LOC)
Untested critical modules: 0
Dual code paths: 0
Global state issues: 0
```

### Success Metrics (φ-Bounded)
| Metric | Current | Target | Phase | Confidence |
|--------|---------|--------|-------|------------|
| Debt markers | 56 | <3 | 1-2 | 61.8% |
| Type coverage | 87.5% | >95% | 1 | 58% |
| Test coverage | 40% | >70% | 2-3 | 52% |
| Max module size | 1248 LOC | <600 LOC | 2 | 55% |
| God objects | 3 | 0 | 2 | 54% |

---

## PHASE 1: CRITICAL CLEANUP (Week 1)
> Remove friction, unblock future work

### Task 1.1: Remove Dual Awakening Paths
**Objective**: Eliminate api/state.py (648 LOC, DEPRECATED)
**Blocker**: Many modules import from both old and new paths

**Steps**:
1. Audit imports: Find all `from cynic.api.state import` statements
   ```bash
   grep -r "from cynic.api.state import" cynic/ tests/
   grep -r "from cynic.api import state" cynic/ tests/
   ```
2. Create migration script: Replace imports with new path
   - OLD: `from cynic.api.state import awaken, CynicState`
   - NEW: `from cynic.organism.organism import CynicOrganism, awaken`
3. Update test files (26 files)
   - conftest.py (module-level imports)
   - All test_*.py files using state.py
4. Verify tests pass:
   ```bash
   python -m pytest tests/ -v --tb=short
   ```
5. Delete api/state.py file
6. Commit: "feat: Eliminate dual awakening paths, consolidate to organism/organism.py"

**Files to modify**: 30+ files (api/*, cognition/*, tests/*)
**Risk**: Test failures if migration incomplete
**Effort**: 4-6 hours
**Q-SCORE IMPACT**: +15 points

**Verification**:
- [ ] Zero imports from cynic.api.state remain
- [ ] All 26 tests pass
- [ ] api/state.py file deleted
- [ ] conftest.py uses only organism paths

---

### Task 1.2: Add Return Type Hints to Critical Routers
**Objective**: Unblock mypy strict mode (6 routers at 0% type coverage)

**Target files**:
1. `cynic/api/routers/actions.py` — 0% → 100%
2. `cynic/api/routers/mcp.py` — 0% → 100%
3. `cynic/api/routers/nervous.py` — 0% → 100%
4. `cynic/api/routers/organism.py` — 0% → 100%
5. `cynic/cli/deploy.py` — 0% → 100%
6. `cynic/dna/examples.py` — 0% → 100%

**Steps per file**:
1. Read function signatures (20 min)
2. Identify return types from docstrings + code (20 min)
3. Add type hints to all functions (30 min)
4. Run mypy on single file: `mypy --strict cynic/api/routers/actions.py` (10 min)
5. Fix any mypy errors (10-30 min depending on file)
6. Commit: "feat: Add return type hints to {filename}"

**Typical changes** (10-50 lines per file):
```python
# Before
def get_judgment(judgment_id):
    """Get judgment by ID."""
    result = db.get(judgment_id)
    return result

# After
from typing import Dict, Any
from cynic.core.entities import Judgment

def get_judgment(judgment_id: str) -> Judgment:
    """Get judgment by ID."""
    result = db.get(judgment_id)
    return result
```

**Verification**:
```bash
python -m mypy --strict cynic/api/routers/
python -m mypy --strict cynic/cli/deploy.py
python -m mypy --strict cynic/dna/examples.py
```

**Files affected**: 6 routers
**Effort**: 4-6 hours (1 hour per file)
**Q-SCORE IMPACT**: +10 points

**Verification**:
- [ ] `mypy --strict` passes on all 6 files
- [ ] No `Any` types (use specific types)
- [ ] All function signatures typed
- [ ] Tests still pass

---

### Task 1.3: Fix 4 Critical Bug/TODO Markers
**Objective**: Resolve high-impact debt in core modules

#### Bug 1: deployer.py (16 markers)
**File**: `cynic/cognition/neurons/deployer.py`
**Issue**: Deployment blocker detection relies on unfinished TODO patterns

**Steps**:
1. Read deployer.py (15 min) — understand deployment blocker logic
2. Find all 16 TODO/FIXME markers (5 min)
3. For each marker:
   - Understand intent from context (5-10 min)
   - Implement or collapse into existing code (10-20 min)
   - Test in isolation (5-10 min)
4. Commit: "fix: Resolve 16 TODO markers in deployer.py"

**Expected resolution** (examples):
- Line 67-75: `MAX_BLOCKERS=10` TODO → finalize threshold with rationale
- Line 101: `_TODO_PATTERNS` → collapse into actual pattern definitions
- Line 264-290: `TODO penalty logic` → implement clear scoring

**Effort**: 2-3 hours
**Q-SCORE IMPACT**: +4 points

#### Bug 2: core.py Route Bugs (lines 138, 230)
**File**: `cynic/api/routers/core.py`
**Issue**: 2 BUG markers in core routing logic

**Steps**:
1. Read lines 138, 230 + surrounding context (10 min)
2. Identify root cause: unclear context (10-20 min)
3. Add clarifying comment or fix bug (10-20 min)
4. Write test case verifying fix (20-30 min)
5. Commit: "fix: Resolve BUG markers in core.py routing"

**Effort**: 1-2 hours
**Q-SCORE IMPACT**: +3 points

#### Bug 3: mcp/server.py Judgment Routing (lines 301-302)
**File**: `cynic/mcp/server.py`
**Issue**: TODO markers for judgment_id mapping not implemented

**Steps**:
1. Understand judgment lifecycle (20 min)
   - Create judgment → get judgment_id
   - Map judgment_id → internal state_key
   - Map judgment_id → executed action
2. Implement mapping logic (30-40 min)
   - Create judgment_id → state_key lookup
   - Create judgment_id → action lookup
   - Add to persistence layer
3. Write tests (30-40 min)
4. Commit: "feat: Implement judgment_id state/action mapping"

**Effort**: 2-3 hours
**Q-SCORE IMPACT**: +3 points

#### Bug 4: claude_code_bridge.py Build/Deploy Endpoints (lines 441, 464)
**File**: `cynic/mcp/claude_code_bridge.py`
**Issue**: TODO markers - /build and /deploy endpoints stubbed

**Steps**:
1. Read stub implementations (10 min)
2. Implement or keep as documented stubs:
   - Option A: Implement /build endpoint (wire to orchestration)
   - Option B: Keep as documented TODO with clear reason
3. Commit: "feat: Implement or document build/deploy endpoints"

**Effort**: 1-2 hours
**Q-SCORE IMPACT**: +2 points

**Total Phase 1.3 Effort**: 4-6 hours
**Q-SCORE IMPACT**: +12 points

---

### Phase 1 Summary
| Task | Effort | Impact | Status |
|------|--------|--------|--------|
| 1.1: Remove dual paths | 4-6h | +15 | 🔴 Ready |
| 1.2: Type hints | 4-6h | +10 | 🔴 Ready |
| 1.3: Critical bugs | 4-6h | +12 | 🔴 Ready |
| **TOTAL PHASE 1** | **12-18h** | **+37** | 🔴 Ready |

**Week 1 Timeline**:
- Mon-Tue: Task 1.1 (dual paths removal)
- Wed-Thu: Task 1.2 (type hints)
- Fri: Task 1.3 (critical bugs) + Phase 1 verification

**Phase 1 Verification**:
```bash
# All tests pass
python -m pytest tests/ -v

# Strict mypy mode passes
python -m mypy --strict cynic/

# No debt markers in critical files
grep -r "TODO\|FIXME\|BUG" cynic/api/ cynic/cognition/neurons/deployer.py cynic/mcp/

# api/state.py deleted
ls -la cynic/api/state.py  # should fail (file not found)
```

---

## PHASE 2: ARCHITECTURAL CLEANUP (Weeks 2-3)
> Refactor god objects, establish interfaces, add tests

### Task 2.1: Split orchestrator.py (1248 LOC)
**Objective**: Break down largest module into testable units

**Current structure**:
- orchestrator.py = MCTS node orchestration + all cycle handlers
- 1248 LOC = too large, single point of failure

**Target structure**:
```
cynic/cognition/cortex/
├─ orchestrator.py (400-500 LOC) — coordinator only
├─ cycle_reflex.py (300-400 LOC) — REFLEX loop
├─ cycle_micro.py (300-400 LOC) — MICRO loop (consolidate existing)
├─ cycle_macro.py (300-400 LOC) — MACRO loop (consolidate existing)
└─ handlers/
   ├─ handler_*.py — individual handlers (unchanged)
   └─ composer.py — compose handler chains
```

**Steps**:
1. Extract REFLEX cycle logic:
   ```python
   # Move from orchestrator.py → cycle_reflex.py
   class Reflex Cycle Handler:
       async def execute(self, state):
           # Quick reaction without meta-cognition
           pass
   ```
2. Consolidate MICRO/MACRO cycles (already split, just clean imports)
3. Update orchestrator.py to delegate to cycles
4. Update all imports in api/handlers/
5. Write tests:
   - test_cycle_reflex.py (50-100 lines)
   - test_cycle_micro.py (50-100 lines)
   - test_cycle_macro.py (50-100 lines)
   - test_orchestrator.py (100-150 lines)
6. Commit: "refactor: Split orchestrator.py into cycle-specific handlers"

**Verification**:
- [ ] orchestrator.py < 500 LOC
- [ ] All cycles split into separate files
- [ ] All tests pass
- [ ] cyclomatic complexity reduced

**Effort**: 2-3 days
**Q-SCORE IMPACT**: +20 points

---

### Task 2.2: Split state_manager.py (998 LOC)
**Objective**: Extract subsystems from god object

**Current structure**:
- state_manager.py = all state coordination (brain + metabolism + immune + motor + perception)
- 998 LOC = too large, high coupling

**Target structure**:
```
cynic/organism/
├─ state_manager.py (300-400 LOC) — coordinator
├─ brain_coordinator.py (150-200 LOC) — brain state
├─ metabolic_tracker.py (150-200 LOC) — budget/energy
├─ immune_monitor.py (150-200 LOC) — safety state
├─ motor_executor.py (150-200 LOC) — action state
└─ perception_aggregator.py (100-150 LOC) — sensory state
```

**Steps per subsystem** (repeat 5 times):
1. Identify state attributes for subsystem
2. Create new file (e.g., brain_coordinator.py)
3. Extract methods handling that state
4. Define interface (get_state, update_state, get_metric)
5. Update state_manager.py to delegate
6. Write unit tests (50-100 lines per subsystem)
7. Commit: "refactor: Extract {subsystem} from state_manager"

**Example** (brain_coordinator.py):
```python
class BrainCoordinator:
    """Manages brain-specific state (decision history, confidence, etc.)"""

    def __init__(self, state_store: Storage):
        self.state_store = state_store

    def get_decision_history(self) -> List[Decision]:
        return self.state_store.get("brain.decisions")

    def record_decision(self, decision: Decision) -> None:
        history = self.get_decision_history()
        history.append(decision)
        self.state_store.set("brain.decisions", history)
```

**Verification**:
- [ ] state_manager.py < 400 LOC
- [ ] All subsystems have unit tests
- [ ] All tests pass
- [ ] No circular dependencies

**Effort**: 2-3 days
**Q-SCORE IMPACT**: +20 points

---

### Task 2.3: Refactor adapter.py (882 LOC, 0 tests)
**Objective**: Split LLM adapter + add comprehensive tests

**Current structure**:
- adapter.py = Claude + Ollama + Google in one file (no tests!)
- 882 LOC, critical path, untested = HIGH RISK

**Target structure**:
```
cynic/llm/
├─ base.py (100-150 LOC) — LLMProvider interface
├─ providers/
│  ├─ claude.py (250-300 LOC) — Claude adapter
│  ├─ ollama.py (250-300 LOC) — Ollama adapter
│  └─ google.py (150-200 LOC) — Google adapter
├─ adapter.py (100-150 LOC) — factory pattern
└─ tests/
   ├─ test_base.py (50 lines)
   ├─ test_claude.py (100-150 lines)
   ├─ test_ollama.py (100-150 lines)
   ├─ test_google.py (50-100 lines)
   └─ test_adapter.py (50 lines)
```

**Steps**:
1. Create base.py with LLMProvider protocol:
   ```python
   from typing import Protocol

   class LLMProvider(Protocol):
       async def prompt(self, text: str) -> str: ...
       async def stream(self, text: str) -> AsyncIterator[str]: ...
   ```
2. Create providers/ directory
3. Extract Claude logic → claude.py
4. Extract Ollama logic → ollama.py
5. Extract Google logic → google.py
6. Create factory in adapter.py:
   ```python
   def create_provider(name: str) -> LLMProvider:
       providers = {
           "claude": ClaudeProvider,
           "ollama": OllamaProvider,
           "google": GoogleProvider,
       }
       return providers[name]()
   ```
7. Write tests for each provider (mocking external calls)
8. Commit: "refactor: Split LLM adapter into typed provider pattern + tests"

**Test strategy** (mock external LLM services):
```python
# test_claude.py
@pytest.mark.asyncio
async def test_claude_prompt():
    provider = ClaudeProvider(api_key="test-key")
    with patch("anthropic.AsyncAnthropic") as mock_client:
        mock_client.return_value.messages.create.return_value = "response"
        result = await provider.prompt("test")
        assert result == "response"
```

**Verification**:
- [ ] adapter.py < 150 LOC
- [ ] All providers < 300 LOC each
- [ ] 80%+ test coverage for adapter module
- [ ] All tests pass (with mocking)
- [ ] No external service calls in unit tests

**Effort**: 3-4 days
**Q-SCORE IMPACT**: +25 points (from 0% → 80% coverage)

---

### Task 2.4: Establish Formal Interfaces
**Objective**: Define clear module boundaries (replace duck typing)

**Create interfaces** in `cynic/core/interfaces/`:

#### 1. EventHandler Interface
```python
# cynic/core/interfaces/event_handler.py
from typing import Protocol, Any

class EventHandler(Protocol):
    """Event bus message handler."""

    async def handle(self, event: dict[str, Any]) -> None:
        """Process event."""
        ...

    @property
    def event_types(self) -> list[str]:
        """Event types this handler subscribes to."""
        ...
```

**Replace**: inspect-based discovery in api/handlers/

#### 2. Storage Backend Interface
```python
# cynic/core/interfaces/storage.py
from typing import Protocol, Any

class StorageBackend(Protocol):
    """Persistent storage abstraction."""

    async def get(self, key: str) -> Any:
        """Retrieve value."""
        ...

    async def set(self, key: str, value: Any) -> None:
        """Store value."""
        ...
```

**Enforce**: Document postgres vs surreal compatibility

#### 3. LLMProvider Interface
```python
# Already created in Task 2.3
```

#### 4. Perception Sensor Interface
```python
# cynic/core/interfaces/sensor.py
from typing import Protocol, AsyncIterator

class Sensor(Protocol):
    """Perception sensor worker."""

    async def observe(self) -> dict:
        """Observe environment state."""
        ...

    async def stream(self) -> AsyncIterator[dict]:
        """Stream observations."""
        ...
```

**Steps**:
1. Create cynic/core/interfaces/ directory
2. Define each interface (files above)
3. Add to __init__.py exports
4. Update existing implementations to inherit interfaces
5. Update type hints in api/services/
6. Write tests:
   - test_event_handler.py (interface compliance)
   - test_storage_backend.py (interface compliance)
7. Commit: "feat: Define formal module interfaces (EventHandler, Storage, LLM, Sensor)"

**Verification**:
- [ ] All interfaces defined in cynic/core/interfaces/
- [ ] Implementations inherit interfaces
- [ ] Type hints updated to use interfaces
- [ ] Tests pass

**Effort**: 2-3 days
**Q-SCORE IMPACT**: +15 points

---

### Task 2.5: Add Comprehensive Unit Tests
**Objective**: Bring coverage from 40% → 60%+

**Priority modules** (untested or minimally tested):
1. agent_loop.py (chat) — 0 tests
2. builders/* (9 files) — minimal tests
3. orchestrator handlers (cycles) — see Task 2.1
4. senses/workers — 0 tests

**Test plan** (per module):

#### Tests for agent_loop.py
```python
# tests/chat/test_agent_loop.py (100 lines)
@pytest.mark.asyncio
async def test_agent_loop_chat_turn():
    """Agent processes user message and generates response."""
    agent = ChatAgent(llm=mock_llm, state=mock_state)
    response = await agent.turn("hello")
    assert response.text != ""

@pytest.mark.asyncio
async def test_agent_loop_error_handling():
    """Agent handles LLM failures gracefully."""
    agent = ChatAgent(llm=mock_llm_error, state=mock_state)
    with pytest.raises(LLMError):
        await agent.turn("test")
```

#### Tests for builders/*
```python
# tests/api/builders/test_builders.py (100-150 lines)
def test_cognition_builder_creates_brain():
    """CognitionBuilder creates properly wired brain."""
    builder = CognitionBuilder(storage=mock_storage)
    brain = builder.build()
    assert brain is not None
    assert brain.dogs is not None
```

#### Tests for senses/workers
```python
# tests/senses/test_workers.py (100 lines)
@pytest.mark.asyncio
async def test_code_sense_worker():
    """CodeSensor observes codebase changes."""
    sensor = CodeSensor(repo_path="/tmp/repo")
    observation = await sensor.observe()
    assert "files" in observation
    assert "changes" in observation
```

**Steps**:
1. For each untested module:
   - Read module code (20-30 min)
   - Identify main functions/classes (10 min)
   - Create test file with 50-100 line tests (30-60 min)
   - Mock external dependencies (LLM, storage, events)
   - Run: `pytest tests/{new_test}.py -v`
   - Iterate until passing (20-40 min)
2. Commit per module: "test: Add unit tests for {module}"
3. Final commit: "test: Comprehensive test suite (coverage 40% → 60%+)"

**Mocking strategy**:
```python
@pytest.fixture
def mock_llm():
    """Mock LLMProvider for testing."""
    mock = AsyncMock()
    mock.prompt.return_value = "test response"
    return mock

@pytest.fixture
def mock_storage():
    """Mock StorageBackend for testing."""
    mock = AsyncMock()
    mock.get.return_value = {}
    mock.set.return_value = None
    return mock
```

**Verification**:
- [ ] All untested modules have tests
- [ ] Coverage report shows >60%
- [ ] All tests pass (including integration tests)
- [ ] No external service calls in unit tests

**Effort**: 3-4 days
**Q-SCORE IMPACT**: +25 points

---

### Phase 2 Summary
| Task | Effort | Impact | Dependencies |
|------|--------|--------|--------------|
| 2.1: Split orchestrator | 2-3d | +20 | Phase 1 done |
| 2.2: Split state_manager | 2-3d | +20 | Phase 1 done |
| 2.3: Refactor adapter | 3-4d | +25 | Phase 1 done |
| 2.4: Interfaces | 2-3d | +15 | 2.1-2.3 done |
| 2.5: Unit tests | 3-4d | +25 | 2.1-2.4 done |
| **TOTAL PHASE 2** | **12-17 days** | **+105** | Sequential |

---

## PHASE 3: CONSOLIDATION & VALIDATION (Week 4)
> Final cleanup, documentation, deployment readiness

### Task 3.1: Remove Remaining Debt Markers
**Objective**: Resolve 15-20 medium/low priority markers

**Remaining markers after Phase 1-2** (estimated):
- orchestration/{monitor.py, versioning.py} — 4 markers
- perceive/environment.py — 1 marker
- core/storage/surreal.py — 1 marker
- llm/temporal.py — 1 marker (after Task 2.3)
- Plus any new markers discovered

**Steps**:
1. Collect all remaining markers: `grep -r "TODO\|FIXME" cynic/`
2. Categorize by priority (critical, medium, low)
3. Resolve medium/low markers (30 min - 1 hour each)
4. Document critical markers with clear rationale
5. Commit: "fix: Resolve remaining debt markers"

**Effort**: 2-3 days
**Q-SCORE IMPACT**: +5 points

---

### Task 3.2: Validate Architecture & Dependencies
**Objective**: Ensure no hidden violations

**Steps**:
1. Formal dependency analysis:
   ```bash
   # Check for circular imports
   python -m pydeps cynic/ --exclude tests --no-show
   ```
2. Module boundary enforcement:
   - api/ should not import from cognition/
   - cognition/ should not import from api/
   - All cross-module communication via interfaces
3. Global state audit:
   - Identify remaining global state
   - Move to DI pattern where possible
4. Import analysis:
   - Verify all imports use __all__ exports
   - Remove unused imports
5. Commit: "refactor: Enforce module boundaries and clean imports"

**Effort**: 2-3 days
**Q-SCORE IMPACT**: +5 points

---

### Task 3.3: Final Documentation & Changelog
**Objective**: Record all changes, consolidate learnings

**Documents to create/update**:
1. ARCHITECTURE.md — Module layout, interfaces, DI patterns
2. MODULES.md — Per-module purpose, dependencies, testing strategy
3. INTERFACES.md — Formal interfaces, protocols, contracts
4. CHANGELOG.md — All Phase 1-3 changes summarized
5. TESTING.md — How to run tests, coverage, mocking strategy

**Steps**:
1. Write ARCHITECTURE.md (1-2 hours)
2. Generate MODULES.md (1-2 hours)
3. Document INTERFACES.md (1 hour)
4. Create CHANGELOG.md (30 min)
5. Create TESTING.md (30 min)
6. Commit: "docs: Comprehensive architecture & consolidation docs"

**Effort**: 1-2 days
**Q-SCORE IMPACT**: +5 points

---

### Task 3.4: Verify Production Readiness
**Objective**: Comprehensive validation before merge

**Checks**:
1. Test suite:
   ```bash
   python -m pytest tests/ -v --cov=cynic --cov-report=html
   ```
   - Coverage > 70%
   - All tests pass
   - No flaky tests

2. Type safety:
   ```bash
   python -m mypy --strict cynic/
   ```
   - No errors
   - No Any types (except documented exceptions)

3. Code quality:
   ```bash
   python -m ruff check cynic/
   python -m ruff format cynic/ --check
   ```
   - No linting errors
   - Code formatted consistently

4. Performance:
   - Orchestrator response time < 100ms
   - Memory usage < 500MB
   - Startup time < 5 seconds

5. Security:
   - No hardcoded credentials
   - No SQL injection vectors
   - Type-safe inputs (Pydantic validation)

**Verification checklist**:
- [ ] 70%+ test coverage
- [ ] mypy --strict passes
- [ ] ruff lint/format passes
- [ ] Performance benchmarks met
- [ ] Security audit passed
- [ ] Documentation complete
- [ ] All Phase 1-3 tasks completed

**Effort**: 1-2 days
**Q-SCORE IMPACT**: +10 points

---

### Phase 3 Summary
| Task | Effort | Impact | Status |
|------|--------|--------|--------|
| 3.1: Remaining markers | 2-3d | +5 | 🟡 Dependent |
| 3.2: Architecture validation | 2-3d | +5 | 🟡 Dependent |
| 3.3: Documentation | 1-2d | +5 | 🟡 Dependent |
| 3.4: Production readiness | 1-2d | +10 | 🟡 Dependent |
| **TOTAL PHASE 3** | **6-10 days** | **+25** | Sequential |

---

## EXECUTION TIMELINE

### Week 1: Phase 1 (Critical Cleanup)
```
Monday-Tuesday:   Task 1.1 (dual paths removal)
Wednesday-Thursday: Task 1.2 (type hints)
Friday:           Task 1.3 (critical bugs) + validation
Outcome: Code compiles with strict mypy, tests pass, +37 Q-score
```

### Week 2-3: Phase 2 (Architectural Refactoring)
```
Monday-Wednesday:  Task 2.1 (orchestrator) + Task 2.2 (state_manager)
Thursday-Friday:   Task 2.3 (adapter refactoring) + Task 2.4 (interfaces)
Weekend:           Testing & iteration
Monday:            Task 2.5 (unit tests)
Tuesday-Wednesday: Phase 2 validation
Outcome: God objects split, interfaces formalized, +105 Q-score, 60%+ coverage
```

### Week 4: Phase 3 (Consolidation)
```
Monday-Tuesday:    Task 3.1 (remaining markers) + Task 3.2 (validation)
Wednesday-Thursday: Task 3.3 (documentation) + Task 3.4 (readiness)
Friday:            Final review, merge to main
Outcome: Production-ready repo, +25 Q-score, comprehensive docs
```

---

## SUCCESS CRITERIA

### Code Quality
- ✅ Type coverage: >95%
- ✅ Test coverage: >70%
- ✅ Debt markers: <3 (resolved or documented)
- ✅ Max module size: <600 LOC
- ✅ God objects: 0
- ✅ Global state: 0
- ✅ Circular dependencies: 0

### Process Quality
- ✅ All commits atomic (one feature per commit)
- ✅ All commits have clear messages
- ✅ All tests pass (unit + integration)
- ✅ Documentation complete and accurate
- ✅ Architecture verified via code review

### Team Quality
- ✅ Code follows Python best practices
- ✅ Interfaces clearly defined
- ✅ Testing strategy documented
- ✅ Onboarding docs complete
- ✅ Future developers can understand architecture

---

## RISK MITIGATION

### Risk 1: Import Migration Breaks Tests
**Probability**: High (Phase 1.1)
**Mitigation**: Run tests after each import change, use sed/script for bulk replacement

### Risk 2: Type Hints Too Verbose
**Probability**: Medium (Phase 1.2)
**Mitigation**: Allow `from __future__ import annotations` to defer parsing

### Risk 3: God Object Splits Introduce Coupling
**Probability**: Medium (Phase 2.1-2.2)
**Mitigation**: Use formal interfaces, avoid back-references, test in isolation

### Risk 4: Tests Pass Locally but Fail CI
**Probability**: Low (Phases 1-3)
**Mitigation**: Run tests in Docker (same as CI), check platform-specific code

### Risk 5: Phase Overruns Timeline
**Probability**: Medium (all phases)
**Mitigation**: Prioritize by Q-score impact, can skip Phase 3 if needed

---

## DECISION GATES

### Before Phase 2
- [ ] Phase 1 complete: mypy --strict passes
- [ ] Phase 1 complete: all tests pass
- [ ] Phase 1 complete: api/state.py deleted
- [ ] Phase 1 complete: 0 critical debt markers

### Before Phase 3
- [ ] Phase 2 complete: orchestrator < 500 LOC
- [ ] Phase 2 complete: state_manager < 400 LOC
- [ ] Phase 2 complete: adapter tests 80%+ coverage
- [ ] Phase 2 complete: interfaces formalized
- [ ] Phase 2 complete: test coverage > 60%

### Before Merge to Main
- [ ] Phase 3 complete: all debt markers resolved/documented
- [ ] Phase 3 complete: architecture validated
- [ ] Phase 3 complete: documentation complete
- [ ] Phase 3 complete: production readiness confirmed

---

## APPENDIX: QUICK REFERENCE

### Phase 1 Commands
```bash
# Find dual path imports
grep -r "from cynic.api.state import" cynic/ tests/

# Run tests
python -m pytest tests/ -v

# Mypy strict mode
python -m mypy --strict cynic/

# Find type coverage
mypy --html ./htmlcov cynic/
```

### Phase 2 Commands
```bash
# Measure module sizes
find cynic -name "*.py" -exec wc -l {} \; | sort -rn | head -20

# Find god objects (>600 LOC)
find cynic -name "*.py" -exec sh -c 'lines=$(wc -l < "$1"); if [ "$lines" -gt 600 ]; then echo "$lines $1"; fi' _ {} \;

# Cyclomatic complexity
python -m radon cc cynic/ -a
```

### Phase 3 Commands
```bash
# Coverage report
python -m pytest tests/ --cov=cynic --cov-report=html

# Dependency analysis
python -m pydeps cynic/ --exclude tests

# Format check
python -m ruff format cynic/ --check
python -m ruff check cynic/
```

---

**Document Last Updated**: 2026-02-23
**Confidence**: 61.8% (φ⁻¹ limit)
**Status**: 🟡 Ready for Phase 1 Execution
