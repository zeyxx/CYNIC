# CYNIC Test Failures — Issues to Fix

## Issue 1: Organism.start() Signature Mismatch [CRITICAL]

**File:** `/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC-clean/cynic/interfaces/api/server.py`
**Line:** 53
**Severity:** CRITICAL (57 tests blocked)

### Current Code
```python
# Line 52-53
logger.info("🧪 CYNIC Awakening (instance=%s)...", instance_id)

await organism.start(db=None)  # ❌ WRONG: 'db' parameter doesn't exist
```

### Actual Signature
```python
# In cynic/kernel/organism/organism.py
async def start(self) -> None:
    """Start the organism with background processing loops."""
    # No 'db' parameter!
```

### Error
```
TypeError: Organism.start() got an unexpected keyword argument 'db'
```

### Fix
```python
# Line 52-53
logger.info("🧪 CYNIC Awakening (instance=%s)...", instance_id)

await organism.start()  # ✅ CORRECT: No arguments
```

### Tests Unblocked
- All `tests/api/routers/test_*.py` tests (50+ tests)
- All `tests/mcp/test_*.py` integration tests (30+ tests)
- E2E tests that depend on API initialization (5+ tests)

### Verification
```bash
# After fix, this should pass:
pytest tests/api/routers/test_consciousness_ecosystem.py -v
```

---

## Issue 2: Missing get_core_bus() Function [CRITICAL]

**File:** `/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC-clean/cynic/kernel/organism/brain/cognition/cortex/residual.py`
**Line:** 35
**Severity:** CRITICAL (2 tests blocked)

### Current Code
```python
# Lines 30-40
class ResidualDetector:
    def __init__(self, bus: Optional[EventBus] = None):
        self._history: list[float] = []
        self._high_residual_count = 0
        self._threshold = PHI_INV_2 * 100
        from cynic.kernel.core.event_bus import CoreEvent, Event

        self._bus = bus or get_core_bus("DEFAULT")  # ❌ WRONG: function not defined
        # ...
```

### Error
```
NameError: name 'get_core_bus' is not defined
```

### Root Cause
The function `get_core_bus()` is either:
1. Not imported
2. Not exported from the module
3. Doesn't exist and should be replaced with a different function

### Investigation
```bash
# Search for where get_core_bus is defined
grep -r "def get_core_bus" cynic/

# Check what's available in event_bus module
grep -r "def " cynic/kernel/core/event_bus.py | head -20

# Check imports in residual.py
head -30 cynic/kernel/organism/brain/cognition/cortex/residual.py
```

### Likely Fix Options

**Option A:** Add missing import
```python
from cynic.kernel.core.event_bus import CoreEvent, Event, get_core_bus
```

**Option B:** Replace with correct function (if get_core_bus doesn't exist)
```python
# If there's a get_default_bus() or similar:
self._bus = bus or get_default_bus()
```

**Option C:** Create bus without factory function
```python
# If bus factory not available, instantiate directly:
self._bus = bus or EventBus()
```

### Tests Blocked
- `tests/test_event_bus_integration_clean.py::test_detector_integration_empirical`
- `tests/test_event_bus_integration_clean.py::test_detector_high_entropy_signal`

### Verification
```bash
# After fix, this should pass:
pytest tests/test_event_bus_integration_clean.py -v
```

---

## Issue 3: Pytest Fixture File Handle Leak [HIGH]

**Symptom:** `ValueError: I/O operation on closed file`
**Severity:** HIGH (680+ false ERROR reports)
**Location:** Test fixtures (likely conftest.py)

### Error Stack
```python
File "C:\Python313\Lib\site-packages\_pytest\capture.py", line 571, in snap
    self.tmpfile.seek(0)
ValueError: I/O operation on closed file.
```

### Root Cause
One or more pytest fixtures or tests is closing `sys.stdout` or `sys.stderr` before pytest's cleanup phase completes.

### Investigation Steps

```bash
# 1. Search for explicit file closures
grep -r "\.close()" tests/conftest.py
grep -r "\.close()" cynic/

# 2. Search for stdout/stderr manipulation
grep -r "sys\.stdout" tests/conftest.py
grep -r "sys\.stderr" tests/conftest.py

# 3. Look for context managers that modify streams
grep -rn "redirect_stdout\|redirect_stderr" tests/

# 4. Check for StringIO that might be closed
grep -rn "StringIO" tests/conftest.py

# 5. Look for capsys/capfd fixture abuse
grep -rn "capsys\|capfd" tests/conftest.py | grep -v "def test"
```

### Typical Cause Pattern
```python
# PROBLEMATIC FIXTURE
@pytest.fixture
def some_fixture(capsys):
    captured = capsys.readouterr()
    # ... work with captured

    # DON'T DO THIS:
    captured.out.close()  # ❌ This breaks pytest cleanup
```

### Fix Pattern
```python
# CORRECT FIXTURE
@pytest.fixture
def some_fixture(capsys):
    captured = capsys.readouterr()
    # ... work with captured (as is, don't close)

    yield captured  # Let pytest handle cleanup

    # Optional: explicit cleanup that doesn't close streams
    # captured = None  # Just deref, don't close
```

### Verification
```bash
# After fix, errors should change to normal PASSED/FAILED:
pytest tests/test_unified_state.py -v --tb=short

# Check if I/O error appears:
pytest tests/ -v 2>&1 | grep "ValueError: I/O operation"
# Should have 0 results
```

---

## Issue 4: Pydantic Class-Based Config [MEDIUM]

**Modules Affected:** 8 files
**Severity:** MEDIUM (non-blocking, but breaks in Pydantic v3)
**Error:** `PydanticDeprecatedSince20`

### Affected Files
1. `cynic/interfaces/api/routers/consciousness_ecosystem.py`
2. `cynic/interfaces/api/routers/governance.py`
3. `cynic/interfaces/api/routers/health_enhanced.py`
4. `cynic/interfaces/api/routers/ws_ecosystem.py`
5. `cynic/interfaces/mcp/*.py` (multiple)
6. Related test files

### Old Pattern (Pydantic v1)
```python
from pydantic import BaseModel

class SomeModel(BaseModel):
    field1: str
    field2: int

    class Config:
        frozen = True
        use_enum_values = True
```

### Error With Strict Warnings
```
PydanticDeprecatedSince20: Support for class-based `config` is deprecated,
use ConfigDict instead. Deprecated in Pydantic V2.0 to be removed in V3.0.
```

### New Pattern (Pydantic v2)
```python
from pydantic import BaseModel, ConfigDict

class SomeModel(BaseModel):
    model_config = ConfigDict(
        frozen=True,
        use_enum_values=True
    )

    field1: str
    field2: int
```

### Migration Steps
```bash
# 1. Find all class-based Config classes
grep -r "class Config:" cynic/interfaces/

# 2. For each, replace with ConfigDict pattern
# 3. Test: pytest tests/api/ -v

# 4. Check for any errors:
pytest tests/api/ -W error::DeprecationWarning --tb=short
```

### Common ConfigDict Mappings

| Old Config | New ConfigDict |
|-----------|----------------|
| `frozen = True` | `frozen=True` |
| `use_enum_values = True` | `use_enum_values=True` |
| `validate_assignment = True` | `validate_assignment=True` |
| `json_encoders = {...}` | `json_encoders={...}` |
| `arbitrary_types_allowed = True` | `arbitrary_types_allowed=True` |

### Verification
```bash
# After fix, no deprecation errors:
pytest tests/api/ -W error::DeprecationWarning -v
```

---

## Issue 5: Unknown Pytest Marker [LOW]

**Marker:** `@pytest.mark.integration`
**Count:** 9 occurrences
**Severity:** LOW (cosmetic warning only)

### Warning
```
PytestUnknownMarkWarning: Unknown pytest.mark.integration - is this a typo?
```

### Fix Location
**File:** `/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC-clean/pyproject.toml`

### Current Config
```toml
[tool.pytest.ini_options]
markers = [
    "performance: Marks tests as performance benchmarks (deselect with '-m \"not performance\"')",
    "ci_required: Marks tests that must pass in CI/CD pipeline",
    "asyncio: Marks async tests for pytest-asyncio",
]
```

### Fix
```toml
[tool.pytest.ini_options]
markers = [
    "performance: Marks tests as performance benchmarks (deselect with '-m \"not performance\"')",
    "ci_required: Marks tests that must pass in CI/CD pipeline",
    "asyncio: Marks async tests for pytest-asyncio",
    "integration: Marks tests as integration tests",
]
```

### Verification
```bash
# After fix, no warnings:
pytest tests/ -v 2>&1 | grep "PytestUnknownMarkWarning"
# Should have 0 results
```

---

## Issue 6: Asyncio Configuration Missing [LOW]

**File:** `/c/Users/zeyxm/Desktop/asdfasdfa/CYNIC-clean/pyproject.toml`
**Severity:** LOW (cosmetic warning only)

### Warning
```
PytestDeprecationWarning: The configuration option "asyncio_default_fixture_loop_scope" is unset.
The event loop scope for asynchronous fixtures will default to the fixture caching scope.
Future versions of pytest-asyncio will default the loop scope for asynchronous fixtures
to function scope.
```

### Current Config
```toml
[tool.pytest.ini_options]
asyncio_mode = "auto"
# Missing: asyncio_default_fixture_loop_scope
```

### Fix
```toml
[tool.pytest.ini_options]
asyncio_mode = "auto"
asyncio_default_fixture_loop_scope = "function"  # Add this line
```

### Verification
```bash
# After fix, no warnings:
pytest tests/ -W all -v 2>&1 | grep "asyncio_default_fixture_loop_scope"
# Should have 0 results
```

---

## Summary Table

| Issue | File | Line | Type | Time | Tests Unblocked |
|-------|------|------|------|------|-----------------|
| 1 | server.py | 53 | Signature | 1m | 57 |
| 2 | residual.py | 35 | Import | 1m | 2 |
| 3 | conftest.py | ? | Fixture | 10-20m | 680+ |
| 4 | 8 files | Various | Deprecation | 1-2h | 0 (non-blocking) |
| 5 | pyproject.toml | 28 | Config | 1m | 0 (cosmetic) |
| 6 | pyproject.toml | 28 | Config | 1m | 0 (cosmetic) |

**Total Time to Fix Critical Issues:** ~30 minutes
**Total Time to Fix All Issues:** ~2-3 hours

---

## Quick Fix Checklist

- [ ] Remove `db=None` from `organism.start()` call in server.py:53
- [ ] Add `get_core_bus` import to residual.py:35 (or find replacement)
- [ ] Identify fixture closing stdout/stderr and add guard
- [ ] Migrate 8 Pydantic models from class Config to ConfigDict
- [ ] Add "integration" marker to pyproject.toml
- [ ] Add `asyncio_default_fixture_loop_scope` to pyproject.toml
- [ ] Re-run tests: `pytest tests/ -v --tb=short`
- [ ] Expected result: 95%+ pass rate

---

## Test Verification Commands

```bash
# Run affected tests after each fix:

# After Issue 1:
pytest tests/api/routers/test_consciousness_ecosystem.py::TestEcosystemEndpoints::test_get_ecosystem_state -v

# After Issue 2:
pytest tests/test_event_bus_integration_clean.py -v

# After Issue 3:
pytest tests/test_unified_state.py -v --tb=short
# Check for "ValueError: I/O operation on closed file"

# After Issue 4:
pytest tests/api/ -W error::DeprecationWarning -v

# Final verification:
pytest tests/ -v --tb=line -q 2>&1 | tail -20
# Should show: "XXX passed" with minimal errors
```
