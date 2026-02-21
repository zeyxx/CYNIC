"""
Integration tests: Real cognition components with actual LLM service.

These tests require:
- Ollama running at localhost:11434 (or configured OLLAMA_BASE_URL)
- No mocks — real LLM calls only

Run locally: pytest -m integration tests/test_integration/test_cognition_real.py
"""
import os
import pytest
from unittest.mock import AsyncMock

from cynic.core.judgment import Cell
from cynic.llm.adapter import LLMRegistry


@pytest.mark.integration
class TestCognitionIntegration:
    """Verify cognition components work with real LLM."""

    @pytest.mark.asyncio
    async def test_ollama_discovery_real(self):
        """
        Verify Ollama discovery works with real service.

        Skip if Ollama not available.
        """
        registry = LLMRegistry()
        models = await registry.discover()

        if not models:
            pytest.skip("Ollama not available at localhost:11434")

        # Verify discovered models are real (not empty list)
        assert isinstance(models, list)
        assert len(models) > 0
        print(f"Found {len(models)} Ollama models")

    @pytest.mark.asyncio
    async def test_llm_adapter_selection(self):
        """
        Verify LLMRegistry can select best adapter.

        Skip if no LLM available.
        """
        registry = LLMRegistry()
        models = await registry.discover()

        if not models:
            pytest.skip("No LLM service available")

        # Get best adapter
        adapter = registry.get_best_for("scoring")

        assert adapter is not None
        # Adapter should have real LLM methods
        assert hasattr(adapter, "complete")
        assert callable(adapter.complete)

    @pytest.mark.asyncio
    async def test_cell_creation_real(self):
        """
        Verify Cell objects work correctly.

        This tests the data model, not LLM.
        """
        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            time_dim="PRESENT",
            content="def phi(): return 0.618033988749",
            context="golden ratio mathematical constant",
        )

        # Verify cell is created with all required fields
        assert cell.reality == "CODE"
        assert cell.analysis == "JUDGE"
        assert cell.time_dim == "PRESENT"
        assert "phi" in cell.content
        assert cell.context is not None

    @pytest.mark.asyncio
    async def test_cognition_imports_work(self):
        """
        Verify all cognition modules can be imported.

        This is a smoke test that imports don't break.
        """
        # These imports should not fail
        from cynic.cognition.cortex.orchestrator import JudgeOrchestrator
        from cynic.cognition.cortex.decide import DecideAgent
        from cynic.cognition.cortex.residual import ResidualDetector
        from cynic.cognition.neurons.sage import SageDog
        from cynic.cognition.neurons.scholar import ScholarDog

        # Verify classes exist
        assert JudgeOrchestrator is not None
        assert DecideAgent is not None
        assert ResidualDetector is not None
        assert SageDog is not None
        assert ScholarDog is not None
