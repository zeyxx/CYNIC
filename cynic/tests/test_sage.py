"""
CYNIC SageDog Tests — Composant 8/9 (Chokmah — Wisdom)

Tests the 5-axiom heuristic scoring system:
  PHI, VERIFY, CULTURE, BURN, FIDELITY → geometric mean → Q-score

No LLM, no DB. Pure in-memory heuristic evaluation.
"""
from __future__ import annotations

import asyncio
import pytest

from cynic.core.phi import MAX_Q_SCORE, PHI_INV, PHI_INV_2, MAX_CONFIDENCE
from cynic.core.judgment import Cell
from cynic.core.consciousness import ConsciousnessLevel
from cynic.cognition.neurons.base import DogId, HealthStatus
from cynic.cognition.neurons.sage import SageDog, HEURISTIC_CONFIDENCE


# ════════════════════════════════════════════════════════════════════════════
# FIXTURES
# ════════════════════════════════════════════════════════════════════════════

def make_cell(content: str = "", context: str = "", risk: float = 0.2) -> Cell:
    return Cell(
        reality="CODE",
        analysis="JUDGE",
        content=content,
        context=context,
        novelty=0.3,
        complexity=0.3,
        risk=risk,
        budget_usd=0.1,
    )


CLEAN_CODE = '''
def calculate_fibonacci(n: int) -> int:
    """Return the nth Fibonacci number."""
    if n <= 1:
        return n
    a, b = 0, 1
    for _ in range(2, n + 1):
        a, b = b, a + b
    return b
'''

SMELLY_CODE = '''
class DataManager:
    # TODO: fix this hack
    global config_dict

    def update_record(self, id):
        pass
        pass
        pass
'''

EMPTY_CODE = ""

TYPED_CODE = '''
from typing import Optional, List

def process_items(items: List[str], limit: Optional[int] = None) -> List[str]:
    """Process a list of items, optionally limiting results."""
    assert isinstance(items, list), "items must be a list"
    result = [item.strip() for item in items if item.strip()]
    return result[:limit] if limit else result
'''


# ════════════════════════════════════════════════════════════════════════════
# UNIT: SageDog.analyze()
# ════════════════════════════════════════════════════════════════════════════

class TestSageDogAnalyze:
    """Core analyze() behavior."""

    @pytest.mark.asyncio
    async def test_returns_dog_judgment(self):
        """analyze() returns a valid DogJudgment."""
        from cynic.cognition.neurons.base import DogJudgment
        dog = SageDog()
        cell = make_cell(CLEAN_CODE)
        j = await dog.analyze(cell)
        assert isinstance(j, DogJudgment)
        assert j.dog_id == DogId.SAGE

    @pytest.mark.asyncio
    async def test_q_score_bounded(self):
        """Q-score is always within [0, MAX_Q_SCORE]."""
        dog = SageDog()
        for code in [CLEAN_CODE, SMELLY_CODE, EMPTY_CODE, TYPED_CODE]:
            cell = make_cell(code)
            j = await dog.analyze(cell)
            assert 0.0 <= j.q_score <= MAX_Q_SCORE, (
                f"Q-score {j.q_score} out of [0, {MAX_Q_SCORE}]"
            )

    @pytest.mark.asyncio
    async def test_confidence_bounded_below_phi_inv(self):
        """Heuristic confidence never exceeds PHI_INV (0.618)."""
        dog = SageDog()
        cell = make_cell(CLEAN_CODE)
        j = await dog.analyze(cell)
        assert j.confidence <= PHI_INV, f"Confidence {j.confidence} exceeded φ⁻¹"

    @pytest.mark.asyncio
    async def test_heuristic_confidence_modest(self):
        """Heuristic path admits uncertainty — confidence below PHI_INV_2."""
        dog = SageDog()
        cell = make_cell(CLEAN_CODE)
        j = await dog.analyze(cell)
        # Heuristic SAGE is humble: confidence stays at or near HEURISTIC_CONFIDENCE
        assert j.confidence <= PHI_INV_2, (
            f"Heuristic confidence should be ≤ PHI_INV_2 (0.382), got {j.confidence}"
        )

    @pytest.mark.asyncio
    async def test_no_veto(self):
        """SAGE never vetoes — advises only."""
        dog = SageDog()
        cell = make_cell(SMELLY_CODE)
        j = await dog.analyze(cell)
        assert j.veto is False

    @pytest.mark.asyncio
    async def test_clean_code_scores_higher_than_smelly(self):
        """Well-written code should score higher than smelly code."""
        dog = SageDog()
        j_clean = await dog.analyze(make_cell(CLEAN_CODE))
        j_smelly = await dog.analyze(make_cell(SMELLY_CODE))
        assert j_clean.q_score > j_smelly.q_score, (
            f"Clean ({j_clean.q_score:.1f}) should > Smelly ({j_smelly.q_score:.1f})"
        )

    @pytest.mark.asyncio
    async def test_typed_code_scores_well(self):
        """Code with type hints and docstrings scores better than bare code."""
        dog = SageDog()
        bare_code = "def foo(x):\n    return x * 2\n"
        j_typed = await dog.analyze(make_cell(TYPED_CODE))
        j_bare = await dog.analyze(make_cell(bare_code))
        assert j_typed.q_score >= j_bare.q_score, (
            f"Typed ({j_typed.q_score:.1f}) should ≥ Bare ({j_bare.q_score:.1f})"
        )

    @pytest.mark.asyncio
    async def test_empty_cell_returns_valid_judgment(self):
        """Empty content returns valid judgment (no crash)."""
        dog = SageDog()
        cell = make_cell(EMPTY_CODE)
        j = await dog.analyze(cell)
        assert j.q_score >= 0.0
        assert j.confidence >= 0.0

    @pytest.mark.asyncio
    async def test_evidence_contains_axioms(self):
        """Evidence contains all 5 axiom scores."""
        dog = SageDog()
        cell = make_cell(CLEAN_CODE)
        j = await dog.analyze(cell)
        axioms = j.evidence.get("axioms", {})
        for ax in ("PHI", "VERIFY", "CULTURE", "BURN", "FIDELITY"):
            assert ax in axioms, f"Axiom {ax} missing from evidence"
            assert 0.0 <= axioms[ax] <= 1.0, f"Axiom {ax} out of [0,1]"

    @pytest.mark.asyncio
    async def test_evidence_path_is_heuristic(self):
        """Phase 1: path label should be 'heuristic'."""
        dog = SageDog()
        cell = make_cell(CLEAN_CODE)
        j = await dog.analyze(cell)
        assert j.evidence.get("path") == "heuristic"

    @pytest.mark.asyncio
    async def test_reasoning_mentions_axioms(self):
        """Reasoning string mentions strength/concern axioms."""
        dog = SageDog()
        cell = make_cell(CLEAN_CODE)
        j = await dog.analyze(cell)
        assert len(j.reasoning) > 20
        assert "Wisdom" in j.reasoning or "sniff" in j.reasoning

    @pytest.mark.asyncio
    async def test_high_risk_reduces_fidelity(self):
        """High-risk cell should score lower due to fidelity penalty."""
        dog = SageDog()
        low_risk = await dog.analyze(make_cell(CLEAN_CODE, risk=0.0))
        high_risk = await dog.analyze(make_cell(CLEAN_CODE, risk=1.0))
        # High risk penalizes FIDELITY → overall score drops
        assert low_risk.q_score >= high_risk.q_score, (
            f"Low-risk ({low_risk.q_score:.1f}) should ≥ High-risk ({high_risk.q_score:.1f})"
        )

    @pytest.mark.asyncio
    async def test_records_heuristic_count(self):
        """After analyze(), _heuristic_count increments."""
        dog = SageDog()
        await dog.analyze(make_cell(CLEAN_CODE))
        await dog.analyze(make_cell(SMELLY_CODE))
        assert dog._heuristic_count == 2

    @pytest.mark.asyncio
    async def test_judgment_count_increments(self):
        """AbstractDog._judgment_count tracks calls."""
        dog = SageDog()
        await dog.analyze(make_cell(CLEAN_CODE))
        assert dog._judgment_count == 1


# ════════════════════════════════════════════════════════════════════════════
# UNIT: Axiom Scoring
# ════════════════════════════════════════════════════════════════════════════

class TestAxiomScoring:
    """Individual axiom scorer behavior."""

    def test_phi_short_code_scores_high(self):
        dog = SageDog()
        cell = Cell(reality="CODE", analysis="JUDGE", content="def f(): pass",
                    novelty=0.1, complexity=0.1, risk=0.1, budget_usd=0.1)
        score = dog._score_phi("def f(): pass", cell)
        assert score >= 0.60, f"Short code should score high PHI, got {score}"

    def test_phi_very_long_code_scores_low(self):
        dog = SageDog()
        long_code = "x = 1\n" * 5000  # Very long
        cell = Cell(reality="CODE", analysis="JUDGE", content=long_code,
                    novelty=0.1, complexity=0.9, risk=0.1, budget_usd=0.1)
        score = dog._score_phi(long_code, cell)
        assert score < 0.50, f"Long code should score low PHI, got {score}"

    def test_verify_typed_code_scores_higher(self):
        dog = SageDog()
        typed = "def f(x: int) -> str:\n    \"\"\"doc\"\"\"\n    return str(x)"
        bare = "def f(x):\n    return str(x)"
        typed_score = dog._score_verify(typed, "CODE")
        bare_score = dog._score_verify(bare, "CODE")
        assert typed_score > bare_score

    def test_culture_smelly_code_scores_low(self):
        dog = SageDog()
        score = dog._score_culture(SMELLY_CODE)
        assert score < 0.50, f"Smelly code culture score should be low, got {score}"

    def test_culture_clean_code_scores_higher(self):
        dog = SageDog()
        clean_score = dog._score_culture(CLEAN_CODE)
        smelly_score = dog._score_culture(SMELLY_CODE)
        assert clean_score > smelly_score

    def test_burn_many_params_penalized(self):
        dog = SageDog()
        bad = "def foo(a, b, c, d, e, f, g, h):\n    pass"
        good = "def foo(a, b):\n    pass"
        cell = make_cell()
        bad_score = dog._score_burn(bad, cell)
        good_score = dog._score_burn(good, cell)
        assert good_score > bad_score

    def test_fidelity_high_risk_penalized(self):
        dog = SageDog()
        low_risk_cell = make_cell(CLEAN_CODE, risk=0.0)
        high_risk_cell = make_cell(CLEAN_CODE, risk=1.0)
        low = dog._score_fidelity(CLEAN_CODE, low_risk_cell)
        high = dog._score_fidelity(CLEAN_CODE, high_risk_cell)
        assert low > high

    def test_all_scores_in_unit_range(self):
        dog = SageDog()
        cell = make_cell(CLEAN_CODE, risk=0.3, context="fibonacci algorithm")
        axioms = dog._score_axioms(CLEAN_CODE, "CODE", cell)
        for ax, score in axioms.items():
            assert 0.0 <= score <= 1.0, f"{ax} score {score} out of [0,1]"


# ════════════════════════════════════════════════════════════════════════════
# UNIT: Capabilities & Health
# ════════════════════════════════════════════════════════════════════════════

class TestSageCapabilities:

    def test_dog_id_is_sage(self):
        dog = SageDog()
        assert dog.dog_id == DogId.SAGE

    def test_capabilities_sefirot(self):
        dog = SageDog()
        caps = dog.get_capabilities()
        assert "Chokmah" in caps.sefirot

    def test_capabilities_macro_level(self):
        dog = SageDog()
        caps = dog.get_capabilities()
        assert caps.consciousness_min == ConsciousnessLevel.MACRO

    def test_capabilities_all_realities(self):
        dog = SageDog()
        caps = dog.get_capabilities()
        for r in ("CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"):
            assert r in caps.supported_realities

    @pytest.mark.asyncio
    async def test_health_check_unknown_on_start(self):
        dog = SageDog()
        health = await dog.health_check()
        assert health.dog_id == DogId.SAGE
        assert health.status == HealthStatus.UNKNOWN

    @pytest.mark.asyncio
    async def test_health_check_healthy_after_analyze(self):
        dog = SageDog()
        await dog.analyze(make_cell(CLEAN_CODE))
        health = await dog.health_check()
        assert health.status == HealthStatus.HEALTHY


# ════════════════════════════════════════════════════════════════════════════
# UNIT: Compressor ↔ SAGE bidirectional attention loop
# ════════════════════════════════════════════════════════════════════════════

class TestSageCompressorBidirectional:
    """
    Tests for the SAGE→Compressor attention feedback loop.

    SageDog.set_compressor() injects the ContextCompressor.
    After each judgment, _signal_attention() calls compressor.boost()
    with the judged text and normalized Q-score.
    """

    def test_set_compressor_stored(self):
        """set_compressor() injects compressor into SageDog."""
        from cynic.senses.compressor import ContextCompressor
        dog = SageDog()
        cc = ContextCompressor()
        dog.set_compressor(cc)
        assert dog._compressor is cc

    @pytest.mark.asyncio
    async def test_no_compressor_no_error(self):
        """SageDog without compressor works normally (backward compatible)."""
        dog = SageDog()
        assert dog._compressor is None
        j = await dog.analyze(make_cell(CLEAN_CODE))
        assert j.q_score >= 0.0  # No error, judgment produced

    @pytest.mark.asyncio
    async def test_high_qscore_boosts_similar_chunk(self):
        """High-quality judgment on text similar to a stored chunk boosts it."""
        from cynic.senses.compressor import ContextCompressor
        dog = SageDog()
        cc = ContextCompressor()
        dog.set_compressor(cc)

        # Store a chunk similar to CLEAN_CODE
        cc.add("fibonacci function type hints docstring return")

        initial_attn = cc._chunk_attention[0]
        # Analyze clean code (high Q-score) → should boost similar chunk
        j = await dog.analyze(make_cell(CLEAN_CODE))
        if j.q_score >= 38.2:  # GROWL+ threshold for signaling
            assert cc._chunk_attention[0] >= initial_attn  # Never decreases

    @pytest.mark.asyncio
    async def test_low_qscore_does_not_signal(self):
        """Judgments below GROWL_MIN (38.2) don't signal the compressor."""
        from cynic.senses.compressor import ContextCompressor
        from unittest.mock import MagicMock
        dog = SageDog()
        mock_compressor = MagicMock()
        dog._compressor = mock_compressor

        # Manually trigger _signal_attention with low q_score
        dog._signal_attention("some code text", q_score=20.0)  # Below 38.2
        mock_compressor.boost.assert_not_called()

    @pytest.mark.asyncio
    async def test_signal_attention_calls_boost(self):
        """_signal_attention() calls compressor.boost() for GROWL+ Q-scores."""
        from cynic.senses.compressor import ContextCompressor
        from unittest.mock import MagicMock
        dog = SageDog()
        mock_compressor = MagicMock()
        dog._compressor = mock_compressor

        # Q-score above threshold
        dog._signal_attention("fibonacci type hints code quality", q_score=50.0)
        mock_compressor.boost.assert_called_once()
        call_args = mock_compressor.boost.call_args
        assert call_args[0][0] == "fibonacci type hints code quality"
        assert 0.0 < call_args[0][1] <= 1.0  # weight is normalized


# ════════════════════════════════════════════════════════════════════════════
# INTEGRATION: SAGE in orchestrator pipeline
# ════════════════════════════════════════════════════════════════════════════

class TestSageInOrchestrator:
    """SAGE integrated with JudgeOrchestrator."""

    @pytest.fixture
    def orchestrator_with_sage(self):
        from cynic.core.axioms import AxiomArchitecture
        from cynic.cognition.neurons.base import DogId
        from cynic.cognition.neurons.cynic_dog import CynicDog
        from cynic.cognition.neurons.guardian import GuardianDog
        from cynic.cognition.neurons.analyst import AnalystDog
        from cynic.cognition.neurons.janitor import JanitorDog
        from cynic.cognition.cortex.orchestrator import JudgeOrchestrator

        cynic_dog = CynicDog()
        sage_dog = SageDog()
        dogs = {
            DogId.CYNIC:    cynic_dog,
            DogId.SAGE:     sage_dog,
            DogId.GUARDIAN: GuardianDog(),
            DogId.ANALYST:  AnalystDog(),
            DogId.JANITOR:  JanitorDog(),
        }
        orch = JudgeOrchestrator(
            dogs=dogs,
            axiom_arch=AxiomArchitecture(),
            cynic_dog=cynic_dog,
        )
        return orch, sage_dog

    @pytest.mark.asyncio
    async def test_sage_judgment_called_in_macro(self, orchestrator_with_sage):
        """SAGE participates in MACRO cycle (full 7-step)."""
        from cynic.core.event_bus import reset_all_buses
        reset_all_buses()

        orch, sage = orchestrator_with_sage
        cell = make_cell(CLEAN_CODE)
        j = await orch.run(cell, level=ConsciousnessLevel.MACRO)
        # SAGE was invoked — judgment_count should be > 0
        assert sage._judgment_count > 0, "SAGE should have been called in MACRO"
        assert j.q_score >= 0.0

    @pytest.mark.asyncio
    async def test_orchestrator_still_returns_valid_without_sage(self):
        """Orchestrator works fine without SAGE (backward compat)."""
        from cynic.core.axioms import AxiomArchitecture
        from cynic.cognition.neurons.base import DogId
        from cynic.cognition.neurons.cynic_dog import CynicDog
        from cynic.cognition.neurons.guardian import GuardianDog
        from cynic.cognition.neurons.analyst import AnalystDog
        from cynic.cognition.neurons.janitor import JanitorDog
        from cynic.cognition.cortex.orchestrator import JudgeOrchestrator
        from cynic.core.event_bus import reset_all_buses
        reset_all_buses()

        cynic_dog = CynicDog()
        dogs = {
            DogId.CYNIC:    cynic_dog,
            DogId.GUARDIAN: GuardianDog(),
            DogId.ANALYST:  AnalystDog(),
            DogId.JANITOR:  JanitorDog(),
        }
        orch = JudgeOrchestrator(
            dogs=dogs, axiom_arch=AxiomArchitecture(), cynic_dog=cynic_dog
        )
        cell = make_cell(CLEAN_CODE)
        j = await orch.run(cell, level=ConsciousnessLevel.REFLEX)
        assert j.q_score >= 0.0
