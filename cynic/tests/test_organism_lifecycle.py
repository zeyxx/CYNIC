"""
Task 8: Full Lifecycle Integration Tests — Persistence & Recovery

Tests that OrganismState can persist to disk, recover from restart,
and support full organism lifecycle (init → store → shutdown → restart → recover).
"""
import pytest
import tempfile
import json
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
        with open(persist_file) as f:
            data = json.load(f)
            assert data["level"] == "MACRO"


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
