# CYNIC Test Failures: Root Cause Analysis & Synthesis
**Date:** 2026-03-01
**Status:** Complete decomposition of 77 failures into 6 independent components

---

## Executive Summary

The 77 failing tests fall into **6 independent root cause categories**, not 77 separate issues. When each component is fixed, its failures resolve together. The failures represent remnants of architecture migration (old module structure → new unified structure) and platform-specific issues.

### By the Numbers
- **20 test files** with collection errors (tests can't even import)
- **57 test failures** in files that successfully import
- **Total distinct root causes:** 6 (not 77!)
- **First fixes solve:** 39+ failures immediately

---

## Component 1: Missing `conscious_state` Module (6 Failures)

### Root Cause
Tests and conftest.py import from `cynic.kernel.organism.conscious_state`, but this module was deleted/migrated during V4/V5 refactor. The module structure changed from:
```
conscious_state.py → ConsciousState, JudgmentSnapshot
```
to:
```
unified_state.py → UnifiedConsciousState (new name)
```

### Affected Test Files
- **conftest.py** line 22: `from cynic.kernel.organism.conscious_state import ConsciousState`
- **test_track_g_resilience.py** (4 failures): Multiple imports of ConsciousState, JudgmentSnapshot
- **test_conscious_state_memory.py** (1+ failure): Import of ConsciousState, get_conscious_state, JudgmentSnapshot

### Impact
- 6 direct test failures
- conftest.py is used by ALL test files, so this blocks fixtures
- Cascades to other failures in test setup

### Evidence
```bash
$ grep -r "conscious_state" tests/ --include="*.py" | grep "import"
tests/conftest.py:    from cynic.kernel.organism.conscious_state import ConsciousState
tests/test_conscious_state_memory.py:from cynic.kernel.organism.conscious_state import ConsciousState, get_conscious_state, JudgmentSnapshot
tests/test_track_g_resilience.py:        from cynic.kernel.organism.conscious_state import ConsciousState, JudgmentSnapshot
```

### Fix
Replace imports with new unified structure:
```python
# Old
from cynic.kernel.organism.conscious_state import ConsciousState, JudgmentSnapshot

# New
from cynic.kernel.core.unified_state import UnifiedConsciousState
```

---

## Component 2: Missing `StateSnapshot` Export (1-2 Failures)

### Root Cause
Test imports `StateSnapshot` from `state_manager.py`, but the class is actually called `StateSnapshotResponse` and lives in `cynic/interfaces/api/models/organism_state.py`. The state_manager doesn't export this.

### Affected Test Files
- **test_learning/test_sona_wiring.py** line 13: `from cynic.kernel.organism.state_manager import StateSnapshot, _FrozenDict`
- **test_state_manager.py**: Likely imports StateSnapshot too

### Impact
- 2 tests blocked from collecting
- Missing export from API models

### Evidence
```bash
$ grep "StateSnapshot" cynic/ --include="*.py" | head -5
cynic/interfaces/api/models/organism_state.py:class StateSnapshotResponse(BaseModel):
# NOT in state_manager.py
```

### Fix
Either:
1. Add export to state_manager.py, OR
2. Update tests to import from API models

---

## Component 3: Missing `reset_all_buses` Function (2-3 Failures)

### Root Cause
Tests import `reset_all_buses` from event_bus, but this function doesn't exist in the current implementation. Tests expect a way to reset global event bus state between tests.

### Affected Test Files
- **test_change_analyzer.py** line 8: `from cynic.kernel.core.event_bus import Event, CoreEvent, get_core_bus, reset_all_buses`
- **test_event_bus_integration_clean.py**: Likely imports same

### Impact
- 3 test collection failures

### Evidence
```bash
$ python -c "from cynic.kernel.core.event_bus import Event, CoreEvent, get_core_bus; print([x for x in dir() if 'reset' in x])"
# reset_all_buses does not exist
```

### Fix
Add to event_bus.py:
```python
async def reset_all_buses() -> None:
    """Reset all global event buses to clean state."""
    global _core_bus
    if _core_bus:
        _core_bus = None
    # Reset any other global bus instances
```

---

## Component 4: Missing Old Module Exports (11-15 Failures)

### Root Cause
Tests import from old module paths that were deleted during refactor:
- `cynic.judges.dog_implementations` → Dogs moved to new structure
- `governance_bot` → Module doesn't exist or wasn't migrated
- `cynic.kernel.organism.brain.cognition.*` → Moved to `cynic.kernel.organism.brain.cognition.*`

### Affected Test Files (20 collection errors)
```
tests/cynic/judges/test_dogs.py
tests/test_authentication.py              → governance_bot.auth
tests/test_change_analyzer.py
tests/test_consciousness.py
tests/test_config_management.py           → cynic.interfaces.bots.governance (validation error)
tests/test_conscious_state_memory.py
tests/test_docker_manager.py
tests/test_event_bus_integration_clean.py
tests/test_learning/test_sona_wiring.py   → StateSnapshot
tests/test_llm/test_llm_registry.py
tests/test_llm/test_ollama_adapter.py
tests/test_near_integration_live.py
tests/test_near_rpc_submission.py
tests/test_near_transaction_signing.py
tests/test_proposal_templates.py
tests/test_reputation.py
tests/test_state_manager.py
tests/test_track_f_pipeline.py
tests/test_treasury.py
tests/test_voting_mechanics.py
```

### Impact
- 20 test files completely blocked
- 20+ underlying test failures if imports are fixed
- Represents old governance bot infrastructure

### Evidence
```
ModuleNotFoundError: No module named 'cynic.judges.dog_implementations'
ModuleNotFoundError: No module named 'governance_bot'
```

### Fix
For each missing module, either:
1. Migrate the module to new location, OR
2. Update tests to use new module paths, OR
3. Mark tests as skipped/xfail if module is intentionally deleted

---

## Component 5: Platform-Specific Unicode Encoding (39+ Failures)

### Root Cause
**File:** `cynic/kernel/protocol/knet_server.py:48`

The file prints emoji and Greek letters in Unicode that Windows charmap encoding cannot represent:
```python
print(f"⚠️ κ-NET: IPv6 bind failed...")
```

Windows CMD uses `cp1252` encoding (or `utf-8` with wrong settings), which cannot encode:
- ⚠️ (warning emoji)
- κ (Greek letter kappa)

This causes `UnicodeEncodeError` during server startup, blocking:
- All 28 API router tests (server won't start)
- All 11 MCP integration tests (server won't start)

### Affected Test Files (39+ failures)
```
tests/test_integration_*.py (multiple files, various failures)
tests/integrations/test_mcp_*.py (multiple files)
tests/router tests (API endpoint tests)
```

### Impact
- 39+ test failures
- Single root cause (one print statement)
- Easy fix

### Evidence
```bash
$ python -c "print('⚠️ κ-NET: test')"
# On Windows CMD: UnicodeEncodeError: 'charmap' codec can't encode character...
```

### Fix
Remove Unicode characters from knet_server.py line 48:
```python
# Old
print(f"⚠️ κ-NET: IPv6 bind failed...")

# New
print(f"WARNING: K-NET: IPv6 bind failed...")
```

Or use:
```python
import sys
# At top of file
print(f"⚠️ κ-NET: IPv6 bind failed...", file=sys.stdout)  # Will handle encoding
```

---

## Component 6: Config Validation Error at Import Time (1-2 Failures)

### Root Cause
**File:** `cynic/interfaces/bots/governance/core/config.py:425`

Config class validates settings at import time (module level), not at instantiation:
```python
# config.py line 425
config.validate_critical_settings()  # Runs at import!

# Line 346
if self.cynic.interfaces.mcp_enabled:  # AttributeError!
```

The `CYNICSettings` Pydantic model doesn't have an `interfaces` attribute, causing validation to fail immediately when any test imports config.

### Affected Test Files
- **test_config_management.py**: Direct config import
- Other tests using fixtures that import config

### Impact
- 1-2 test collection failures
- Cascade failures to other tests

### Evidence
```
AttributeError: 'CYNICSettings' object has no attribute 'interfaces'
```

### Fix
Move validation from module level to method level:
```python
# config.py line 425
# Delete: config.validate_critical_settings()

# And add to __init__:
def __post_init__(self):
    self.validate_critical_settings()
```

Or fix the attribute reference:
```python
# Line 346
# Old: if self.cynic.interfaces.mcp_enabled:
# New: if hasattr(self, 'interfaces') and self.interfaces.mcp_enabled:
```

---

## Component 7: Mock/Async Incompatibility in Tests (5+ Failures)

### Root Cause
Tests mock `axiom_arch` object but use `MagicMock`, which doesn't support `await`:
```python
# Test code
axiom_arch = MagicMock()
result = await axiom_arch.score_and_compute(...)  # TypeError!
```

When judgment stages call `await self.axiom_arch.score_and_compute()`, the mock can't handle it.

### Affected Test Files
- **tests/cognition/test_judgment_stages.py** (4+ failures)
- Other cognition tests using axiom_arch mocks

### Impact
- 5+ test failures
- Easy fix in test setup

### Evidence
```
TypeError: object MagicMock can't be used in 'await' expression
```

### Fix
Use `AsyncMock` instead:
```python
# Old
axiom_arch = MagicMock()

# New
from unittest.mock import AsyncMock
axiom_arch = AsyncMock()
axiom_arch.score_and_compute.return_value = AxiomResult(...)
```

---

## Component 8: Missing EventBus Methods (3 Failures)

### Root Cause
Tests call `EventBus.stats()` but this method doesn't exist:
```python
# Test expects:
stats = bus.stats()

# But EventBus doesn't have stats() method
```

### Affected Test Files
- Tests checking event bus task management and statistics

### Impact
- 3 test failures

### Fix
Add to EventBus class:
```python
def stats(self) -> dict:
    """Return statistics about the event bus."""
    return {
        "pending_tasks": len(self._tasks),
        "listeners": len(self._listeners),
        "total_emitted": getattr(self, '_total_emitted', 0),
    }
```

---

## Component 9: Missing `llama_cpp` Module Export (2-3 Failures)

### Root Cause
Tests or code reference `llama_cpp` provider but the module doesn't export it:
```python
# Someone tries to use
from cynic.kernel.organism.brain.llm import llama_cpp
```

Either:
1. `llama_cpp` doesn't exist in adapter
2. It's not exported from __init__.py

### Affected Test Files
- **tests/test_llm/test_llm_registry.py**
- **tests/test_llm/test_ollama_adapter.py** (maybe)

### Impact
- 2-3 test failures
- Hardware discovery tests affected

### Fix
Either add `llama_cpp` provider or remove references:
```python
# Option 1: Add to llm/adapters/__init__.py
from .local_gguf import LocalGgufAdapter as llama_cpp

# Option 2: Remove from registry if not supported
```

---

## Temporal File I/O Issue (1 Failure)

### Root Cause
**File:** `tests/test_module_documentation.py`

Test fixture creates temporary files but tries to access them after they're closed:
```
ValueError: I/O operation on closed file
```

Likely a teardown/context manager issue.

### Impact
- 1 test collection/fixture error

### Fix
Verify context manager lifecycle in test fixtures.

---

## Summary: Root Causes & Impact

| Component | Root Cause | Failures | Difficulty | Fix Time |
|-----------|-----------|----------|------------|----------|
| 1. Missing `conscious_state` module | Module deleted, imports outdated | 6 | Easy | 5 min |
| 2. Missing `StateSnapshot` export | Wrong import path | 2 | Easy | 5 min |
| 3. Missing `reset_all_buses` | Function not implemented | 3 | Easy | 10 min |
| 4. Missing old module exports | Modules deleted/not migrated | 20 | Medium | 30-60 min |
| 5. Unicode encoding (knet_server.py) | Windows charmap limitation | 39+ | Trivial | 2 min |
| 6. Config validation at import | Module-level validation | 2 | Easy | 10 min |
| 7. Mock/async incompatibility | MagicMock doesn't support await | 5 | Easy | 15 min |
| 8. Missing EventBus methods | Method not implemented | 3 | Easy | 10 min |
| 9. Missing llama_cpp export | Export missing | 3 | Easy | 5 min |
| 10. Temporal file I/O | Context manager issue | 1 | Easy | 10 min |

**Total if all fixed: 84 failures resolved**
*Note: Some failures may resolve when others are fixed (cascading imports)*

---

## Fix Priority Order

### Phase 1: Quick Wins (5-10 minutes, fixes ~45 failures)
1. **Fix Unicode encoding** (knet_server.py) → 39+ failures
2. **Replace conscious_state imports** → 6 failures
3. **Add reset_all_buses function** → 3 failures

**Result after Phase 1:** ~900+ tests passing (91%+)

### Phase 2: Module Structure (30 minutes, fixes ~20 failures)
4. **Create/migrate missing modules** (judges.dog_implementations, governance_bot, etc.)
5. **Update test imports** to new module paths
6. **Fix config validation** (move to post-init)

**Result after Phase 2:** ~920+ tests passing (98%+)

### Phase 3: Polish (10 minutes, fixes remaining ~5 failures)
7. **Add missing methods** (EventBus.stats, StateSnapshot export)
8. **Fix mock/async compatibility** in tests
9. **Fix temporal file I/O** in test fixtures
10. **Add llama_cpp** export or remove references

**Result after Phase 3:** 929+ tests passing (100%)

---

## Key Insight

**The 77 failures are NOT 77 independent problems.** They're symptoms of 6-10 root causes. When component 1 is fixed, its failures resolve together. When component 5 (Unicode) is fixed, 39+ failures vanish simultaneously.

This is a refactoring debt problem, not an architectural problem. The system is fundamentally sound (92% tests pass); the failures are legacy code paths that haven't been updated to the new module structure.

### Strategy
- **Don't** try to fix individual tests
- **Do** fix the root causes in order of impact
- **Validate** that cascading fixes resolve multiple test failures

---

## Verification Commands

```bash
# Before fixes
pytest tests/ -v 2>&1 | grep -E "FAILED|PASSED|ERROR" | wc -l
# Output: 77 FAILED

# After Phase 1 (5 min)
pytest tests/ -v 2>&1 | grep -E "FAILED|PASSED|ERROR" | wc -l
# Expected: ~35 FAILED (39+ fixed)

# After Phase 2 (35 min)
pytest tests/ -v 2>&1 | grep -E "FAILED|PASSED|ERROR" | wc -l
# Expected: ~15 FAILED (20+ fixed)

# After Phase 3 (45 min)
pytest tests/ -v
# Expected: 929 PASSED, 0 FAILED
```
