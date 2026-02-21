# Task 8: Full Lifecycle Integration Tests — Persistence & Recovery

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Verify OrganismState can persist to disk, recover from restart, and support full organism lifecycle (init → store → shutdown → restart → recover).

**Architecture:** Test persistence layer (consciousness level, dogs registry) survives shutdown/restart. Verify memory layer (judgments, residuals, actions) clears on restart. Validate full lifecycle: init → add data → persist → shutdown → restart → recover.

**Tech Stack:** Python asyncio, pytest, pytest-asyncio, OrganismState (from state_manager.py), file I/O

---

## Task 8.1: Create Lifecycle Test Suite Structure

**Files:**
- Create: `cynic/tests/test_organism_lifecycle.py`
- Modify: `cynic/organism/state_manager.py` (add persistence methods if needed)

**Step 1: Write failing test for basic persistence**

```python
# cynic/tests/test_organism_lifecycle.py
import pytest
import tempfile
import os
from pathlib import Path
from cynic.organism.state_manager import OrganismState

@pytest.mark.asyncio
async def test_consciousness_level_persists_to_disk():
    """Verify consciousness level is saved to persistent layer."""
    with tempfile.TemporaryDirectory() as tmpdir:
        state = OrganismState(storage_path=tmpdir)
        await state.initialize()

        # Set consciousness level
        await state.update_consciousness_level("MACRO")

        # Persist to disk
        await state.persist()

        # Verify file exists
        persist_file = Path(tmpdir) / "consciousness.json"
        assert persist_file.exists()

        # Verify content
        import json
        with open(persist_file) as f:
            data = json.load(f)
            assert data["level"] == "MACRO"
```

**Step 2: Run test to verify it fails**

```bash
cd cynic/
python -m pytest tests/test_organism_lifecycle.py::test_consciousness_level_persists_to_disk -v
```

Expected: `AttributeError: OrganismState has no attribute 'persist'` or `TypeError: __init__() got unexpected keyword argument 'storage_path'`

**Step 3: Add persist() method to OrganismState**

```python
# cynic/organism/state_manager.py - add to OrganismState class

async def persist(self) -> None:
    """Persist consciousness level to PERSISTENT layer."""
    async with self._lock:
        persist_file = self.storage_path / "consciousness.json"
        consciousness_data = {
            "level": self._consciousness_level,
            "timestamp": time.time(),
        }
        persist_file.write_text(json.dumps(consciousness_data, indent=2))
        logger.info(f"Persisted consciousness level: {self._consciousness_level}")
```

**Step 4: Update __init__ to accept storage_path**

```python
# cynic/organism/state_manager.py - update __init__

def __init__(self, storage_path: str | Path | None = None):
    """Initialize OrganismState with optional persistent storage path."""
    self.storage_path = Path(storage_path or Path.home() / ".cynic" / "organism_state")
    self.storage_path.mkdir(parents=True, exist_ok=True)
    # ... rest of __init__
```

**Step 5: Run test to verify it passes**

```bash
cd cynic/
python -m pytest tests/test_organism_lifecycle.py::test_consciousness_level_persists_to_disk -v
```

Expected: PASS

**Step 6: Commit**

```bash
git add cynic/tests/test_organism_lifecycle.py cynic/organism/state_manager.py
git commit -m "feat(task-8): Add consciousness level persistence to disk"
```

---

## Task 8.2: Recovery from Persistent Storage

**Files:**
- Modify: `cynic/tests/test_organism_lifecycle.py` (add new test)
- Modify: `cynic/organism/state_manager.py` (add recover() method)

**Step 1: Write failing test for recovery**

```python
# Add to test_organism_lifecycle.py

@pytest.mark.asyncio
async def test_consciousness_level_recovers_after_restart():
    """Verify consciousness level is restored from persistent layer on restart."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # First instance: set and persist
        state1 = OrganismState(storage_path=tmpdir)
        await state1.initialize()
        await state1.update_consciousness_level("MACRO")
        await state1.persist()

        # Second instance (simulates restart): recover
        state2 = OrganismState(storage_path=tmpdir)
        await state2.initialize()
        await state2.recover()

        # Verify consciousness level was restored
        assert state2.get_consciousness_level() == "MACRO"
```

**Step 2: Run test to verify it fails**

```bash
cd cynic/
python -m pytest tests/test_organism_lifecycle.py::test_consciousness_level_recovers_after_restart -v
```

Expected: `AttributeError: OrganismState has no attribute 'recover'`

**Step 3: Add recover() method to OrganismState**

```python
# cynic/organism/state_manager.py - add to OrganismState class

async def recover(self) -> None:
    """Recover consciousness level from PERSISTENT layer."""
    async with self._lock:
        persist_file = self.storage_path / "consciousness.json"
        if not persist_file.exists():
            logger.info("No persistent state to recover")
            return

        try:
            data = json.loads(persist_file.read_text())
            level = data.get("level", "REFLEX")
            self._consciousness_level = level
            logger.info(f"Recovered consciousness level: {level}")
        except Exception as e:
            logger.error(f"Failed to recover consciousness level: {e}")
```

**Step 4: Run test to verify it passes**

```bash
cd cynic/
python -m pytest tests/test_organism_lifecycle.py::test_consciousness_level_recovers_after_restart -v
```

Expected: PASS

**Step 5: Commit**

```bash
git add cynic/tests/test_organism_lifecycle.py cynic/organism/state_manager.py
git commit -m "feat(task-8): Add consciousness level recovery from persistent storage"
```

---

## Task 8.3: Memory Layer Clearing on Restart

**Files:**
- Modify: `cynic/tests/test_organism_lifecycle.py` (add new test)

**Step 1: Write test for memory layer clearing**

```python
# Add to test_organism_lifecycle.py

@pytest.mark.asyncio
async def test_memory_layer_clears_on_restart():
    """Verify MEMORY layer (judgments, residuals, actions) clears on new instance."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # First instance: add data to memory layer
        state1 = OrganismState(storage_path=tmpdir)
        await state1.initialize()
        await state1.add_judgment({"judgment_id": "j1", "q_score": 75.0})
        await state1.update_residual("r1", {"type": "gap"})
        await state1.add_action({"action_id": "a1", "type": "edit"})
        await state1.persist()

        # Second instance (simulates restart): recover
        state2 = OrganismState(storage_path=tmpdir)
        await state2.initialize()
        await state2.recover()

        # Verify MEMORY layer is empty
        assert state2.get_recent_judgments() == []
        assert state2.get_all_residuals() == {}
        assert state2.get_pending_actions() == []
```

**Step 2: Run test to verify it passes**

```bash
cd cynic/
python -m pytest tests/test_organism_lifecycle.py::test_memory_layer_clears_on_restart -v
```

Expected: PASS (memory layer should already be empty)

**Step 3: Commit**

```bash
git add cynic/tests/test_organism_lifecycle.py
git commit -m "test(task-8): Verify memory layer clears on restart"
```

---

## Task 8.4: Error Handling — Corrupt Persistent File

**Files:**
- Modify: `cynic/tests/test_organism_lifecycle.py` (add new test)

**Step 1: Write test for corrupt file handling**

```python
# Add to test_organism_lifecycle.py

@pytest.mark.asyncio
async def test_recover_handles_corrupt_persistence_file():
    """Verify recover() gracefully handles corrupt JSON."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Create corrupt persistence file
        persist_file = Path(tmpdir) / "consciousness.json"
        persist_file.write_text("{invalid json}")

        # Recovery should not crash
        state = OrganismState(storage_path=tmpdir)
        await state.initialize()
        await state.recover()  # Should not raise

        # Should fall back to default
        assert state.get_consciousness_level() == "REFLEX"
```

**Step 2: Run test to verify it fails**

```bash
cd cynic/
python -m pytest tests/test_organism_lifecycle.py::test_recover_handles_corrupt_persistence_file -v
```

Expected: May fail if recover() doesn't handle JSON errors

**Step 3: Update recover() to handle errors gracefully**

(Already implemented in Task 8.2 with try/except)

**Step 4: Run test to verify it passes**

```bash
cd cynic/
python -m pytest tests/test_organism_lifecycle.py::test_recover_handles_corrupt_persistence_file -v
```

Expected: PASS

**Step 5: Commit**

```bash
git add cynic/tests/test_organism_lifecycle.py
git commit -m "test(task-8): Add corrupt file handling test"
```

---

## Task 8.5: Full Lifecycle Cycle Test

**Files:**
- Modify: `cynic/tests/test_organism_lifecycle.py` (add new test)

**Step 1: Write comprehensive lifecycle test**

```python
# Add to test_organism_lifecycle.py

@pytest.mark.asyncio
async def test_full_lifecycle_init_persist_restart_recover():
    """End-to-end: init → add data → persist → restart → recover → verify."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # PHASE 1: Init → Add Data
        state1 = OrganismState(storage_path=tmpdir)
        await state1.initialize()
        await state1.update_consciousness_level("MACRO")
        await state1.set_dogs({
            "sage": {"model": "ollama"},
            "guardian": {"model": "claude"},
        })
        await state1.add_judgment({"judgment_id": "j1", "q_score": 75.0})

        # PHASE 2: Persist
        await state1.persist()
        persist_file = Path(tmpdir) / "consciousness.json"
        assert persist_file.exists()

        # PHASE 3: Simulated Restart
        state2 = OrganismState(storage_path=tmpdir)
        await state2.initialize()
        await state2.recover()

        # PHASE 4: Verify Recovery
        assert state2.get_consciousness_level() == "MACRO"
        assert state2.get_dogs() == {"sage": {"model": "ollama"}, "guardian": {"model": "claude"}}
        assert state2.get_recent_judgments() == []  # MEMORY cleared
```

**Step 2: Run test to verify it passes**

```bash
cd cynic/
python -m pytest tests/test_organism_lifecycle.py::test_full_lifecycle_init_persist_restart_recover -v
```

Expected: PASS

**Step 3: Commit**

```bash
git add cynic/tests/test_organism_lifecycle.py
git commit -m "test(task-8): Add comprehensive full lifecycle test"
```

---

## Task 8.6: Run Full Test Suite and Verify Coverage

**Files:**
- No new files (verification only)

**Step 1: Run all lifecycle tests**

```bash
cd cynic/
python -m pytest tests/test_organism_lifecycle.py -v
```

Expected: All tests pass (6+ tests)

**Step 2: Run with coverage report**

```bash
cd cynic/
python -m pytest tests/test_organism_lifecycle.py --cov=cynic.organism.state_manager --cov-report=term-missing -v
```

Expected: >90% coverage on persist/recover methods

**Step 3: Commit if not already done**

```bash
git status
```

(Already committed in previous steps)

---

## Task 8.7: Integration Verification

**Files:**
- No new files (verification only)

**Step 1: Verify OrganismState can be imported and used**

```bash
cd cynic/
python -c "
from cynic.organism.state_manager import OrganismState
import asyncio
import tempfile

async def test():
    with tempfile.TemporaryDirectory() as tmpdir:
        state = OrganismState(storage_path=tmpdir)
        await state.initialize()
        print('[OK] OrganismState initialized successfully')

asyncio.run(test())
"
```

Expected: `[OK] OrganismState initialized successfully`

**Step 2: Final commit summary**

```bash
cd cynic/
git log --oneline | head -10
```

Verify all Task 8 commits appear.

---

## Summary

**Deliverables:**
- ✅ `cynic/tests/test_organism_lifecycle.py` — 200+ LOC, 6+ tests
- ✅ OrganismState enhanced with persist() and recover() methods
- ✅ Full lifecycle tested: init → persist → restart → recover
- ✅ Error handling for corrupt files
- ✅ Memory layer clearing verified

**Test Results Expected:**
- 6+ tests, 100% pass rate
- >90% coverage on persistence methods
- All lifecycle scenarios validated

**Next Task:**
Task 9: Integrate OrganismState into Organism class (replaces old ConsciousState)

**Confidence:** 61.8% (φ⁻¹) — Clear requirements, straightforward implementation, well-tested foundation.

---

*sniff* Plan complete. Ready for execution via superpowers:executing-plans.
