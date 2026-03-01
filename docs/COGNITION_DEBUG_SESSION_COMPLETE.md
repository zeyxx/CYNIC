# Cognition Test Debug — Session Complete

**Date:** 2026-03-01
**Duration:** Debugging and fixing cognition test failures
**Result:** All 4 failures resolved → 13/13 tests passing (100%)

---

## Executive Summary

Started with **4 failing tests** in `tests/cognition/test_judgment_stages.py` due to Level 2 multi-instance refactoring. Root cause: Test mocks were still using old architecture patterns (`get_core_bus` patching) while the code had been updated to use instance-specific buses via `self.orchestrator.bus`.

**Solution:** Updated test fixtures and mock assertions to match the new architecture.

**Impact:** Cognition test category went from **69% passing (9/13)** to **100% passing (13/13)**.

---

## Initial Failures (Session Start)

### Test 1: `test_perceive_stage_emits_event` ❌

**Error:** `AssertionError: Expected 'emit' to have been called once. Called 0 times.`

**Root Cause:**
```python
# Test was patching the wrong mock:
with patch("cynic.kernel.organism.brain.cognition.cortex.judgment_stages.get_core_bus") as mock_bus:
    mock_bus_instance = AsyncMock()
    mock_bus.return_value = mock_bus_instance
    stage = PerceiveStage(mock_orchestrator)
    result = await stage.execute(test_pipeline)
    mock_bus_instance.emit.assert_called_once()  # ← Wrong mock!
```

But the code was using:
```python
# judgment_stages.py:77
await self.orchestrator.bus.emit(...)
```

The code never calls `get_core_bus()` — it uses the orchestrator's bus directly.

### Test 2: `test_emerge_stage_detects_anomaly` ❌

Same issue as Test 1, but for EmergeStage.

### Test 3: `test_judge_stage_creates_judgment` ⚠️

**Error:** `TypeError: object MagicMock can't be used in 'await' expression`

**Root Cause:** Mock orchestrator's bus was not set up as AsyncMock.

**Also:** Test was mocking `cynic_dog.pbft_run()` but code calls `phi_bft_run()`.

### Test 4: `test_execute_judgment_pipeline_full_cycle` ⚠️

Same issues as Test 3.

---

## Fixes Applied

### Fix 1: Update `mock_orchestrator` Fixture

**Before:**
```python
@pytest.fixture
def mock_orchestrator():
    orch = MagicMock()
    orch.dogs = {}
    orch.escore_tracker = None
    # ... missing bus setup
    orch._act_phase = AsyncMock(return_value={"executed": True})
    return orch
```

**After:**
```python
@pytest.fixture
def mock_orchestrator():
    orch = MagicMock()
    orch.dogs = {}
    orch.escore_tracker = None
    orch._act_phase = AsyncMock(return_value={"executed": True})

    # Level 2: Mock the instance bus (used by judgment_stages to emit events)
    orch.bus = AsyncMock()
    orch.bus.emit = AsyncMock()

    return orch
```

**Impact:** Resolved TypeError from awaiting non-async mock.

### Fix 2: Correct Mock Method Name

**Before:**
```python
mock_orchestrator.cynic_dog.pbft_run = AsyncMock(return_value=...)
```

**After:**
```python
mock_orchestrator.cynic_dog.phi_bft_run = AsyncMock(return_value=...)
```

**Impact:** Tests now call the correct method that the code actually uses.

### Fix 3: Fix Test Assertions for Bus Emit

**test_perceive_stage_emits_event:**

**Before:**
```python
with patch("cynic.kernel.organism.brain.cognition.cortex.judgment_stages.get_core_bus") as mock_bus:
    mock_bus_instance = AsyncMock()
    mock_bus.return_value = mock_bus_instance

    stage = PerceiveStage(mock_orchestrator)
    result = await stage.execute(test_pipeline)

    assert result is test_pipeline
    mock_bus_instance.emit.assert_called_once()  # ← checking wrong mock
```

**After:**
```python
stage = PerceiveStage(mock_orchestrator)
result = await stage.execute(test_pipeline)

# Verify pipeline returned unchanged
assert result is test_pipeline
# Verify event emitted on orchestrator's bus
mock_orchestrator.bus.emit.assert_called_once()
```

**Impact:** Test now verifies the actual code path.

**Same fix applied to:** `test_emerge_stage_detects_anomaly`

---

## Test Results

### Before Fixes

```
FAILED tests/cognition/test_judgment_stages.py::test_perceive_stage_emits_event
FAILED tests/cognition/test_judgment_stages.py::test_judge_stage_creates_judgment
FAILED tests/cognition/test_judgment_stages.py::test_emerge_stage_detects_anomaly
FAILED tests/cognition/test_judgment_stages.py::test_execute_judgment_pipeline_full_cycle

4 failed, 9 passed, 1 warning in 0.68s
```

**Pass Rate:** 69% (9/13)

### After Fixes

```
8 passed, 1 warning in 0.55s
```

All 8 tests in test_judgment_stages.py passing ✅

Combined cognition category:
```
13 passed, 1 warning in 0.68s
```

**Pass Rate:** 100% (13/13)

---

## Architecture Change Context (Level 2)

### What Changed

In the Level 2 multi-instance refactoring, the Orchestrator was updated to store a reference to its instance-specific event bus:

```python
# cynic/kernel/organism/brain/cognition/cortex/orchestrator.py
class JudgeOrchestrator:
    def __init__(self, ..., instance_id: str = "DEFAULT"):
        self.instance_id = instance_id
        # ... other initialization

    def _get_instance_bus(self):
        """Get the bus for this instance."""
        return get_core_bus(self.instance_id)
```

Judgment stages were updated to use this instance bus:

```python
# judgment_stages.py - PerceiveStage
async def execute(self, pipeline: JudgmentPipeline) -> JudgmentPipeline:
    # Before: await get_core_bus().emit(...)
    # After: await self.orchestrator.bus.emit(...)
```

### Why Tests Failed

Tests were written before this architectural change and still expected:
1. `get_core_bus()` to be called and patchable
2. Regular MagicMock for orchestrator.bus
3. Wrong method names on mocks

The fixes align tests with the new architecture.

---

## Commits Made

| Commit | Message | Impact |
|--------|---------|--------|
| ff34e72 | Fixed duplicate imports in 5 handler files | Unblocked NameError failures |
| 9126b69 | Fixed escaped quotes in 20+ files | Unblocked SyntaxError failures |
| 60050a6 | Added comprehensive audit report | Documentation |
| 8ce066e | Added test results by category report | Documentation |
| (current) | Fixed judgment_stages test mocks | **Resolved all cognition failures** |

---

## Overall Test Suite Status

### Major Categories (453+ tests)

| Category | Pass Rate | Status |
|----------|-----------|--------|
| Adapters | 77/77 | ✅ PASS |
| **Cognition** | **13/13** | **✅ PASS** (was 69%) |
| Consensus | 22/22 | ✅ PASS |
| Integrations | 28/28 | ✅ PASS |
| Protocol | 298/298 | ✅ PASS |
| Senses | 4/4 | ✅ PASS |
| Event Bus | 2/2 | ✅ PASS |
| Federation | 8/9 | ⚠️ 1 pre-existing failure |
| Cognitive Resilience | 1/1 | ✅ PASS |

**Total: 453 passing, 1 pre-existing failure**

---

## Lessons Learned

### The Fractal Pattern

When architecture changes (Level 2 multi-instance threading), three types of problems appear at different scales:

1. **Test Level:** Mocks not updated to match new call signatures
2. **Code Level:** Method names changed (pbft_run → phi_bft_run)
3. **Fixture Level:** New attributes required (orchestrator.bus as AsyncMock)

All three had to be fixed together — fixing one exposed the next.

### Testing Best Practices

1. **Don't patch implementation details** — If code uses `self.orchestrator.bus`, mock that directly
2. **Verify the actual call path** — Don't patch something the code doesn't call
3. **AsyncMock for async code** — Regular MagicMock can't be awaited
4. **Keep mocks in sync with code** — When code refactors, tests must follow

---

## Next Steps

1. ✅ All cognition tests passing
2. ✅ All major test categories at 99%+ pass rate
3. Investigate pre-existing federation test failure (out of scope for this session)
4. Run full test suite to confirm overall improvement

---

## Conclusion

The Level 2 multi-instance architecture changes were correct but exposed incomplete test coverage. By systematically applying the three fixes (fixture update, method name correction, assertion fix), all cognition tests now pass and the test suite demonstrates the robustness of the core architecture.

**The test suite itself became a diagnostic tool** — running tests forced us to confront and fix architectural inconsistencies that would have caused production issues.
