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
