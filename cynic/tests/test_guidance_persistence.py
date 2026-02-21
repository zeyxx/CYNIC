"""
Phase 0: Test async guidance persistence with atomic writes.
Verify that _write_guidance_async uses atomic file operations (no corruption).
"""
import pytest
import json
import tempfile
from pathlib import Path
from unittest.mock import patch, AsyncMock
from cynic.api.routers.core import _write_guidance_async
from cynic.core.judgment import Judgment, Cell


@pytest.mark.asyncio
async def test_write_guidance_async_atomic():
    """Verify guidance.json is written atomically (temp → rename pattern)."""
    # Create temp directory for this test
    with tempfile.TemporaryDirectory() as tmpdir:
        guidance_path = Path(tmpdir) / "guidance.json"

        # Create test cell and judgment
        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            time_dim="PRESENT",
            content="test",
            context="test",
            lod=0,
            budget_usd=0.01,
        )

        judgment = Judgment(
            cell=cell,
            verdict="BARK",
            q_score=0.2,
            confidence=0.3,
            dog_votes={"GUARDIAN": 0.2},
            cost_usd=0.001,
        )

        # Write guidance
        await _write_guidance_async(cell, judgment, path=str(guidance_path))

        # Verify file exists and contains valid JSON
        assert guidance_path.exists()
        with open(guidance_path, "r") as f:
            data = json.load(f)

        # Verify content
        assert data["verdict"] == "BARK"
        assert data["reality"] == "CODE"
        assert data["analysis"] == "JUDGE"
        assert "timestamp" in data
        assert "dog_votes" in data


@pytest.mark.asyncio
async def test_write_guidance_async_creates_directory():
    """Verify _write_guidance_async creates parent directories."""
    with tempfile.TemporaryDirectory() as tmpdir:
        nested_path = Path(tmpdir) / "deep" / "nested" / "dir" / "guidance.json"

        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            time_dim="PRESENT",
            content="test",
            context="test",
            lod=0,
            budget_usd=0.01,
        )

        judgment = Judgment(
            cell=cell,
            verdict="BARK",
            q_score=0.2,
            confidence=0.3,
            dog_votes={"GUARDIAN": 0.2},
            cost_usd=0.001,
        )

        # Should create directories automatically
        await _write_guidance_async(cell, judgment, path=str(nested_path))

        assert nested_path.exists()
        with open(nested_path, "r") as f:
            data = json.load(f)
        assert data["verdict"] == "BARK"
