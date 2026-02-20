"""
Tests for LLM-upgraded Dogs — Scholar, Cartographer, Deployer temporal MCTS paths.

Each dog now extends LLMDog and routes to temporal_judgment() when an adapter
is available. These tests verify:
  1. Heuristic path still works (no LLM injected)
  2. LLM path activated when registry/adapter injected
  3. Capabilities: uses_llm=True for all three
  4. DogJudgment is valid in both paths
"""
from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock

import pytest

from cynic.core.phi import MAX_Q_SCORE, PHI_INV, PHI_INV_2
from cynic.core.judgment import Cell
from cynic.core.consciousness import ConsciousnessLevel
from cynic.cognition.neurons.base import DogId, LLMDog


# ════════════════════════════════════════════════════════════════════════════
# HELPERS
# ════════════════════════════════════════════════════════════════════════════

def make_cell(content: str = "def foo(): return 42", reality: str = "CODE") -> Cell:
    return Cell(
        reality=reality,
        analysis="JUDGE",
        content=content,
        novelty=0.3,
        complexity=0.4,
        risk=0.2,
        budget_usd=0.1,
    )


def _mock_registry(score: float = 45.0) -> MagicMock:
    """LLMRegistry mock that returns a mock adapter returning a fixed score."""
    adapter = MagicMock()
    adapter.adapter_id = "ollama:llama3.2"

    async def _complete_safe(req):
        r = MagicMock()
        r.content = f"SCORE: {score}"
        return r

    adapter.complete_safe = _complete_safe

    registry = MagicMock()
    registry.get_best_for = MagicMock(return_value=adapter)
    return registry


SAMPLE_CODE = '''
import os
import sys
import json

def load_config(path: str) -> dict:
    """Load configuration from path."""
    if not os.path.exists(path):
        return {}
    with open(path) as f:
        return json.load(f)

class App:
    def __init__(self, cfg: dict) -> None:
        self.cfg = cfg

    def run(self) -> None:
        pass  # start the application
'''


# ════════════════════════════════════════════════════════════════════════════
# ScholarDog — LLM dog
# ════════════════════════════════════════════════════════════════════════════

class TestScholarLLMUpgrade:

    def test_scholar_extends_llm_dog(self):
        from cynic.cognition.neurons.scholar import ScholarDog
        assert issubclass(ScholarDog, LLMDog)

    def test_scholar_task_type(self):
        from cynic.cognition.neurons.scholar import ScholarDog
        dog = ScholarDog()
        assert dog.task_type == "vector_rag"

    def test_scholar_uses_llm_true(self):
        from cynic.cognition.neurons.scholar import ScholarDog
        dog = ScholarDog()
        assert dog.get_capabilities().uses_llm is True

    def test_scholar_has_set_llm_registry(self):
        from cynic.cognition.neurons.scholar import ScholarDog
        dog = ScholarDog()
        assert hasattr(dog, "set_llm_registry")

    @pytest.mark.asyncio
    async def test_scholar_heuristic_path_no_llm(self):
        """Without LLM, heuristic path (TF-IDF) is used."""
        from cynic.cognition.neurons.scholar import ScholarDog
        from cynic.cognition.neurons.base import DogJudgment
        dog = ScholarDog()
        j = await dog.analyze(make_cell("def foo(): pass"))
        assert isinstance(j, DogJudgment)
        assert j.dog_id == DogId.SCHOLAR
        assert 0.0 <= j.q_score <= MAX_Q_SCORE
        assert j.llm_id is None  # Heuristic path → no LLM

    @pytest.mark.asyncio
    async def test_scholar_temporal_path_with_llm(self):
        """With LLM injected, temporal MCTS path is used."""
        from cynic.cognition.neurons.scholar import ScholarDog
        from cynic.cognition.neurons.base import DogJudgment
        dog = ScholarDog()
        dog.set_llm_registry(_mock_registry(45.0))
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert isinstance(j, DogJudgment)
        assert j.dog_id == DogId.SCHOLAR
        assert 0.0 < j.q_score <= MAX_Q_SCORE
        assert j.llm_id == "ollama:llama3.2"

    @pytest.mark.asyncio
    async def test_scholar_temporal_confidence_bounded(self):
        from cynic.cognition.neurons.scholar import ScholarDog
        dog = ScholarDog()
        dog.set_llm_registry(_mock_registry(50.0))
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert 0.0 < j.confidence <= PHI_INV

    @pytest.mark.asyncio
    async def test_scholar_temporal_evidence_has_path(self):
        from cynic.cognition.neurons.scholar import ScholarDog
        dog = ScholarDog()
        dog.set_llm_registry(_mock_registry(40.0))
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert j.evidence.get("path") == "temporal_mcts"

    @pytest.mark.asyncio
    async def test_scholar_buffer_context_included(self):
        """Buffer info is included in temporal content (dog exercises learn API)."""
        from cynic.cognition.neurons.scholar import ScholarDog
        dog = ScholarDog()
        dog.learn("def prev(): pass", 45.0)  # Prime buffer
        dog.set_llm_registry(_mock_registry(50.0))
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert j.q_score > 0


# ════════════════════════════════════════════════════════════════════════════
# CartographerDog — LLM dog
# ════════════════════════════════════════════════════════════════════════════

class TestCartographerLLMUpgrade:

    def test_cartographer_extends_llm_dog(self):
        from cynic.cognition.neurons.cartographer import CartographerDog
        assert issubclass(CartographerDog, LLMDog)

    def test_cartographer_task_type(self):
        from cynic.cognition.neurons.cartographer import CartographerDog
        dog = CartographerDog()
        assert dog.task_type == "topology"

    def test_cartographer_uses_llm_true(self):
        from cynic.cognition.neurons.cartographer import CartographerDog
        dog = CartographerDog()
        assert dog.get_capabilities().uses_llm is True

    @pytest.mark.asyncio
    async def test_cartographer_heuristic_path_no_llm(self):
        """Without LLM, NetworkX heuristic path is used."""
        from cynic.cognition.neurons.cartographer import CartographerDog
        from cynic.cognition.neurons.base import DogJudgment
        dog = CartographerDog()
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert isinstance(j, DogJudgment)
        assert j.dog_id == DogId.CARTOGRAPHER
        assert j.llm_id is None  # Heuristic path → no LLM
        assert "violations" in j.evidence

    @pytest.mark.asyncio
    async def test_cartographer_temporal_path_with_llm(self):
        """With LLM injected, temporal MCTS path is used."""
        from cynic.cognition.neurons.cartographer import CartographerDog
        from cynic.cognition.neurons.base import DogJudgment
        dog = CartographerDog()
        dog.set_llm_registry(_mock_registry(50.0))
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert isinstance(j, DogJudgment)
        assert j.dog_id == DogId.CARTOGRAPHER
        assert 0.0 < j.q_score <= MAX_Q_SCORE
        assert j.llm_id == "ollama:llama3.2"

    @pytest.mark.asyncio
    async def test_cartographer_temporal_evidence_has_path(self):
        from cynic.cognition.neurons.cartographer import CartographerDog
        dog = CartographerDog()
        dog.set_llm_registry(_mock_registry(45.0))
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert j.evidence.get("path") == "temporal_mcts"

    @pytest.mark.asyncio
    async def test_cartographer_temporal_confidence_in_range(self):
        from cynic.cognition.neurons.cartographer import CartographerDog
        dog = CartographerDog()
        dog.set_llm_registry(_mock_registry(50.0))
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert 0 < j.confidence <= PHI_INV

    @pytest.mark.asyncio
    async def test_cartographer_temporal_no_code_fallback(self):
        """Non-code cell: content derived from cell metadata."""
        from cynic.cognition.neurons.cartographer import CartographerDog
        dog = CartographerDog()
        dog.set_llm_registry(_mock_registry(40.0))
        j = await dog.analyze(make_cell("", reality="SOCIAL"))
        assert isinstance(j.q_score, float)
        assert j.llm_id == "ollama:llama3.2"

    @pytest.mark.asyncio
    async def test_cartographer_heuristic_graphs_built_counter(self):
        """Heuristic path should increment graphs_built counter."""
        from cynic.cognition.neurons.cartographer import CartographerDog
        dog = CartographerDog()  # No LLM → heuristic
        await dog.analyze(make_cell(SAMPLE_CODE))
        assert dog._graphs_built == 1

    @pytest.mark.asyncio
    async def test_cartographer_temporal_graphs_not_counted(self):
        """Temporal path skips NetworkX → graphs_built stays 0."""
        from cynic.cognition.neurons.cartographer import CartographerDog
        dog = CartographerDog()
        dog.set_llm_registry(_mock_registry(50.0))
        await dog.analyze(make_cell(SAMPLE_CODE))
        assert dog._graphs_built == 0  # Temporal path skipped AST


# ════════════════════════════════════════════════════════════════════════════
# DeployerDog — LLM dog
# ════════════════════════════════════════════════════════════════════════════

class TestDeployerLLMUpgrade:

    def test_deployer_extends_llm_dog(self):
        from cynic.cognition.neurons.deployer import DeployerDog
        assert issubclass(DeployerDog, LLMDog)

    def test_deployer_task_type(self):
        from cynic.cognition.neurons.deployer import DeployerDog
        dog = DeployerDog()
        assert dog.task_type == "deployment"

    def test_deployer_uses_llm_true(self):
        from cynic.cognition.neurons.deployer import DeployerDog
        dog = DeployerDog()
        assert dog.get_capabilities().uses_llm is True

    @pytest.mark.asyncio
    async def test_deployer_heuristic_path_no_llm(self):
        """Without LLM, regex+AST scan is used."""
        from cynic.cognition.neurons.deployer import DeployerDog
        from cynic.cognition.neurons.base import DogJudgment
        dog = DeployerDog()
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert isinstance(j, DogJudgment)
        assert j.dog_id == DogId.DEPLOYER
        assert j.llm_id is None  # Heuristic path → no LLM
        assert "violations" in j.evidence

    @pytest.mark.asyncio
    async def test_deployer_temporal_path_with_llm(self):
        """With LLM injected, temporal MCTS path is used."""
        from cynic.cognition.neurons.deployer import DeployerDog
        from cynic.cognition.neurons.base import DogJudgment
        dog = DeployerDog()
        dog.set_llm_registry(_mock_registry(52.0))
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert isinstance(j, DogJudgment)
        assert j.dog_id == DogId.DEPLOYER
        assert 0.0 < j.q_score <= MAX_Q_SCORE
        assert j.llm_id == "ollama:llama3.2"

    @pytest.mark.asyncio
    async def test_deployer_temporal_evidence_has_path(self):
        from cynic.cognition.neurons.deployer import DeployerDog
        dog = DeployerDog()
        dog.set_llm_registry(_mock_registry(50.0))
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert j.evidence.get("path") == "temporal_mcts"

    @pytest.mark.asyncio
    async def test_deployer_temporal_confidence_bounded(self):
        from cynic.cognition.neurons.deployer import DeployerDog
        dog = DeployerDog()
        dog.set_llm_registry(_mock_registry(50.0))
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert 0 < j.confidence <= PHI_INV

    @pytest.mark.asyncio
    async def test_deployer_heuristic_detects_debug_artifacts(self):
        """Heuristic path detects debug statements (no LLM needed)."""
        from cynic.cognition.neurons.deployer import DeployerDog
        dog = DeployerDog()
        code_with_prints = "x = 1\n" + "print(x)\n" * 10  # > MAX_DEBUG_ARTIFACTS=5
        j = await dog.analyze(make_cell(code_with_prints))
        assert j.evidence.get("debug_artifacts", 0) > 0

    @pytest.mark.asyncio
    async def test_deployer_temporal_no_code_fallback(self):
        """Non-code cell: content derived from cell metadata."""
        from cynic.cognition.neurons.deployer import DeployerDog
        dog = DeployerDog()
        dog.set_llm_registry(_mock_registry(45.0))
        j = await dog.analyze(make_cell("", reality="SOLANA"))
        assert isinstance(j.q_score, float)
        assert j.llm_id == "ollama:llama3.2"

    @pytest.mark.asyncio
    async def test_deployer_heuristic_secrets_counter(self):
        """Heuristic path increments secrets counter."""
        from cynic.cognition.neurons.deployer import DeployerDog
        dog = DeployerDog()  # No LLM → heuristic
        secret_code = 'password = "super_secret_password_123"\n'
        await dog.analyze(make_cell(secret_code))
        assert dog._secrets_found >= 1

    @pytest.mark.asyncio
    async def test_deployer_temporal_secrets_not_counted(self):
        """Temporal path skips static scan → secrets_found stays 0."""
        from cynic.cognition.neurons.deployer import DeployerDog
        dog = DeployerDog()
        dog.set_llm_registry(_mock_registry(50.0))
        secret_code = 'password = "super_secret_password_123"\n'
        await dog.analyze(make_cell(secret_code))
        assert dog._secrets_found == 0  # Temporal path skipped regex scan


# ════════════════════════════════════════════════════════════════════════════
# CROSS-DOG: All three have LLMDog interface
# ════════════════════════════════════════════════════════════════════════════

class TestAllThreeLLMDogInterface:
    """Verify the three dogs share the same LLMDog contract."""

    @pytest.mark.parametrize("DogClass,dog_id,task_type", [
        ("cynic.cognition.neurons.scholar.ScholarDog", DogId.SCHOLAR, "vector_rag"),
        ("cynic.cognition.neurons.cartographer.CartographerDog", DogId.CARTOGRAPHER, "topology"),
        ("cynic.cognition.neurons.deployer.DeployerDog", DogId.DEPLOYER, "deployment"),
    ])
    def test_dog_identity(self, DogClass, dog_id, task_type):
        import importlib
        module_path, class_name = DogClass.rsplit(".", 1)
        mod = importlib.import_module(module_path)
        cls = getattr(mod, class_name)
        dog = cls()
        assert dog.dog_id == dog_id
        assert dog.task_type == task_type
        assert dog.get_capabilities().uses_llm is True
        assert hasattr(dog, "set_llm_registry")
        assert hasattr(dog, "get_llm")

    @pytest.mark.parametrize("DogClass", [
        "cynic.cognition.neurons.scholar.ScholarDog",
        "cynic.cognition.neurons.cartographer.CartographerDog",
        "cynic.cognition.neurons.deployer.DeployerDog",
    ])
    @pytest.mark.asyncio
    async def test_temporal_activated_by_registry(self, DogClass):
        """Any dog with registry gets temporal path (llm_id set)."""
        import importlib
        module_path, class_name = DogClass.rsplit(".", 1)
        mod = importlib.import_module(module_path)
        cls = getattr(mod, class_name)
        dog = cls()
        dog.set_llm_registry(_mock_registry(48.0))
        j = await dog.analyze(make_cell(SAMPLE_CODE))
        assert j.llm_id == "ollama:llama3.2"
        assert 0.0 < j.q_score <= MAX_Q_SCORE
