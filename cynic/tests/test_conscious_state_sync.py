"""
Phase 0: Test ConsciousState.sync_checkpoint() for data durability.
Verify that sync checkpoint flushes state to disk atomically.
"""
import pytest
import json
import tempfile
from pathlib import Path
from unittest.mock import patch
from cynic.organism.conscious_state import ConsciousState


@pytest.mark.asyncio
async def test_conscious_state_sync_checkpoint_creates_file():
    """Verify sync_checkpoint flushes state to disk."""
    with tempfile.TemporaryDirectory() as tmpdir:
        checkpoint_path = Path(tmpdir) / "conscious_state.json"

        # Create ConsciousState and add some state
        state = ConsciousState()

        # Sync checkpoint to temp location
        with patch(
            "cynic.organism.conscious_state.STATE_FILE", checkpoint_path
        ):
            await state.sync_checkpoint()

        # Verify file exists and contains valid JSON
        assert checkpoint_path.exists()
        with open(checkpoint_path, "r") as f:
            data = json.load(f)

        # Verify structure (matches ConsciousState.to_dict())
        assert "consciousness_level" in data
        assert isinstance(data.get("dogs"), dict)
        assert isinstance(data.get("recent_judgments"), list)
        assert isinstance(data.get("axioms"), dict)
        assert "stats" in data


@pytest.mark.asyncio
async def test_conscious_state_sync_checkpoint_atomic():
    """Verify sync_checkpoint writes atomically (no partial files)."""
    with tempfile.TemporaryDirectory() as tmpdir:
        checkpoint_path = Path(tmpdir) / "conscious_state.json"

        state = ConsciousState()

        with patch(
            "cynic.organism.conscious_state.STATE_FILE", checkpoint_path
        ):
            await state.sync_checkpoint()

            # Verify no temp files left behind
            temp_files = list(Path(tmpdir).glob(".conscious_state_tmp_*"))
            assert len(temp_files) == 0, f"Temp files not cleaned up: {temp_files}"

            # Verify real file is valid
            with open(checkpoint_path, "r") as f:
                data = json.load(f)
            assert data is not None
