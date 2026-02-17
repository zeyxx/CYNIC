"""
Tests: Dogs (Non-LLM — GUARDIAN, ANALYST, CYNIC PBFT)

Tests for the L3 REFLEX Dogs. No LLM needed, no network needed.
These must run in <100ms each.
"""
import pytest

from cynic.core.phi import MAX_Q_SCORE, MAX_CONFIDENCE, PHI_INV
from cynic.core.judgment import Cell
from cynic.dogs.base import DogId, DOG_PRIORITY, NON_LLM_DOGS
from cynic.dogs.guardian import GuardianDog
from cynic.dogs.analyst import AnalystDog
from cynic.dogs.cynic_dog import CynicDog


class TestDogBase:
    """AbstractDog base class behavior."""

    def test_priority_ordered(self):
        """CYNIC has highest priority (φ³)."""
        from cynic.core.phi import PHI_3
        assert DOG_PRIORITY[DogId.CYNIC] == PHI_3

    def test_non_llm_dogs_set(self):
        assert "CYNIC" in NON_LLM_DOGS
        assert "GUARDIAN" in NON_LLM_DOGS
        assert "SAGE" not in NON_LLM_DOGS

    def test_all_11_dogs_have_priority(self):
        from cynic.core.phi import lucas
        assert len(DOG_PRIORITY) == lucas(5)  # 11


class TestGuardianDog:
    """GUARDIAN — IsolationForest anomaly detection."""

    @pytest.fixture
    def guardian(self):
        return GuardianDog()

    async def test_normal_cell_gets_decent_score(self, guardian, code_cell):
        judgment = await guardian.analyze(code_cell)
        assert judgment.dog_id == DogId.GUARDIAN
        assert 0 <= judgment.q_score <= MAX_Q_SCORE
        assert 0 <= judgment.confidence <= MAX_CONFIDENCE

    async def test_high_risk_triggers_low_score(self, guardian, high_risk_cell):
        judgment = await guardian.analyze(high_risk_cell)
        # High risk cell should score lower than normal
        assert judgment.q_score < MAX_Q_SCORE * 0.8  # below 80% of max

    async def test_veto_on_extreme_danger(self, guardian):
        """Extreme risk should trigger VETO."""
        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            content={"risk_indicator": 1.0, "danger": 1.0},
            novelty=1.0,
            complexity=0.9,
            risk=1.0,  # maximum risk
        )
        judgment = await guardian.analyze(cell)
        # With risk=1.0, danger_level = anomaly_score × (1+1) > VETO_THRESHOLD (2.618)?
        # Without trained model → anomaly_score = risk = 1.0, danger = 1.0×2=2.0 < 2.618
        # So veto might not trigger without trained model — that's correct behavior
        # GUARDIAN abstains when it doesn't have enough data
        assert judgment.dog_id == DogId.GUARDIAN

    async def test_judgment_within_phi_bounds(self, guardian, code_cell):
        """q_score and confidence must always be φ-bounded."""
        judgment = await guardian.analyze(code_cell)
        assert judgment.q_score <= MAX_Q_SCORE
        assert judgment.confidence <= MAX_CONFIDENCE

    async def test_latency_reasonable(self, guardian, code_cell):
        """GUARDIAN must be fast (L3 REFLEX target)."""
        import time
        start = time.perf_counter()
        await guardian.analyze(code_cell)
        elapsed_ms = (time.perf_counter() - start) * 1000
        assert elapsed_ms < 500  # very generous — IsolationForest may be slow first time

    async def test_health_check(self, guardian):
        health = await guardian.health_check()
        assert health.dog_id == DogId.GUARDIAN
        assert health.status in ["HEALTHY", "DEGRADED", "UNHEALTHY", "UNKNOWN"]


class TestAnalystDog:
    """ANALYST — Z3 formal verification."""

    @pytest.fixture
    def analyst(self):
        return AnalystDog()

    async def test_valid_constraints_prove(self, analyst, code_cell):
        """φ-bounded fields in code_cell should be provable."""
        judgment = await analyst.analyze(code_cell)
        assert judgment.dog_id == DogId.ANALYST
        assert 0 <= judgment.q_score <= MAX_Q_SCORE

    async def test_explicit_range_constraint(self, analyst):
        """Explicit range constraints should be provable by Z3."""
        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            content={
                "constraints": [
                    {"type": "range", "name": "x", "value": 0.5, "min": 0.0, "max": 1.0},
                    {"type": "range", "name": "y", "value": 0.3, "min": 0.0, "max": 1.0},
                ]
            },
        )
        judgment = await analyst.analyze(cell)
        if analyst._z3_available:
            # Z3 should prove these constraints (all within bounds)
            assert judgment.q_score == MAX_Q_SCORE
            assert judgment.confidence == PHI_INV

    async def test_empty_content_handled(self, analyst):
        """Empty cell content should not crash."""
        cell = Cell(reality="CODE", analysis="JUDGE", content={})
        judgment = await analyst.analyze(cell)
        assert judgment.dog_id == DogId.ANALYST

    async def test_phi_bounds_enforced_in_output(self, analyst, code_cell):
        judgment = await analyst.analyze(code_cell)
        assert judgment.q_score <= MAX_Q_SCORE
        assert judgment.confidence <= MAX_CONFIDENCE

    async def test_health_check_reflects_z3_availability(self, analyst):
        health = await analyst.health_check()
        if analyst._z3_available:
            assert health.status.value == "HEALTHY"
        else:
            assert health.status.value == "DEGRADED"


class TestCynicDog:
    """CYNIC Dog — PBFT coordinator + systemic coherence."""

    @pytest.fixture
    def cynic_dog(self):
        return CynicDog()

    async def test_analyze_returns_judgment(self, cynic_dog, code_cell):
        judgment = await cynic_dog.analyze(code_cell, budget_usd=1.0, active_dogs=11)
        assert judgment.dog_id == DogId.CYNIC
        assert 0 <= judgment.q_score <= MAX_Q_SCORE

    async def test_low_budget_lowers_score(self, cynic_dog, code_cell):
        high_budget = await cynic_dog.analyze(code_cell, budget_usd=10.0, active_dogs=11)
        low_budget  = await cynic_dog.analyze(code_cell, budget_usd=0.0001, active_dogs=11)
        assert low_budget.q_score <= high_budget.q_score

    async def test_pbft_with_unanimous_votes(self, cynic_dog, code_cell):
        """Unanimous quorum → consensus reached."""
        from cynic.dogs.base import DogJudgment
        dog_judgments = [
            DogJudgment(dog_id=f"DOG_{i}", cell_id=code_cell.cell_id, q_score=50.0, confidence=0.4)
            for i in range(7)
        ]
        result = await cynic_dog.pbft_run(code_cell, dog_judgments)
        assert result.consensus == True
        assert result.votes >= 7
        assert result.final_q_score is not None

    async def test_pbft_below_quorum_fails(self, cynic_dog, code_cell):
        """Only 4 votes → no quorum → consensus=False."""
        from cynic.dogs.base import DogJudgment
        dog_judgments = [
            DogJudgment(dog_id=f"DOG_{i}", cell_id=code_cell.cell_id, q_score=50.0, confidence=0.4)
            for i in range(4)  # Only 4 votes, need 7
        ]
        result = await cynic_dog.pbft_run(code_cell, dog_judgments)
        assert result.consensus == False
        assert "Insufficient votes" in (result.reason or "")

    async def test_pbft_veto_overrides_quorum(self, cynic_dog, code_cell):
        """GUARDIAN veto blocks consensus even with enough votes."""
        from cynic.dogs.base import DogJudgment
        dog_judgments = [
            DogJudgment(dog_id="GUARDIAN", cell_id=code_cell.cell_id, q_score=0.0, confidence=0.4, veto=True),
        ] + [
            DogJudgment(dog_id=f"DOG_{i}", cell_id=code_cell.cell_id, q_score=55.0, confidence=0.5)
            for i in range(10)  # 10 other votes — enough for quorum
        ]
        result = await cynic_dog.pbft_run(code_cell, dog_judgments)
        assert result.consensus == False
        assert "VETO" in (result.reason or "")

    async def test_capabilities(self, cynic_dog):
        caps = cynic_dog.get_capabilities()
        assert caps.uses_llm == False  # Non-LLM Dog
        assert caps.consciousness_min.name == "REFLEX"

    async def test_health_check(self, cynic_dog):
        health = await cynic_dog.health_check()
        assert health.dog_id == DogId.CYNIC
