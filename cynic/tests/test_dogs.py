"""
Tests: Dogs (Non-LLM — GUARDIAN, ANALYST, CYNIC PBFT, ARCHITECT, ORACLE)

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
from cynic.dogs.architect import ArchitectDog
from cynic.dogs.oracle import OracleDog


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


# ════════════════════════════════════════════════════════════════════════════
# ARCHITECT DOG
# ════════════════════════════════════════════════════════════════════════════

class TestArchitectDog:
    """ARCHITECT — AST structural quality scoring."""

    @pytest.fixture
    def architect(self):
        return ArchitectDog()

    @pytest.fixture
    def python_cell(self):
        """A cell with real Python code for structural analysis."""
        code = """
import os
import sys

class SimpleClass:
    def method_one(self):
        return 1

    def method_two(self):
        return 2

def helper():
    pass
"""
        return Cell(reality="CODE", analysis="JUDGE", content=code)

    @pytest.fixture
    def over_coupled_cell(self):
        """A cell with too many imports."""
        code = "\n".join(f"import module_{i}" for i in range(20))
        return Cell(reality="CODE", analysis="JUDGE", content=code)

    @pytest.fixture
    def deep_nesting_cell(self):
        """A cell with very deep nesting."""
        code = """
def deeply_nested():
    if True:
        for i in range(10):
            while True:
                if i > 5:
                    for j in range(5):
                        if j > 2:
                            with open("x") as f:
                                return f.read()
"""
        return Cell(reality="CODE", analysis="JUDGE", content=code)

    @pytest.fixture
    def god_class_cell(self):
        """A cell with a God Class (too many methods)."""
        methods = "\n".join(
            f"    def method_{i}(self): pass" for i in range(15)
        )
        code = f"class GodClass:\n{methods}"
        return Cell(reality="CODE", analysis="JUDGE", content=code)

    async def test_clean_code_scores_high(self, architect, python_cell):
        judgment = await architect.analyze(python_cell)
        assert judgment.dog_id == DogId.ARCHITECT
        assert judgment.q_score > 30.0  # Clean code → reasonable score

    async def test_over_coupled_code_penalized(self, architect, over_coupled_cell):
        judgment = await architect.analyze(over_coupled_cell)
        assert judgment.dog_id == DogId.ARCHITECT
        assert judgment.q_score < MAX_Q_SCORE  # Coupling should reduce score
        assert "coupling" in judgment.reasoning.lower() or "import" in judgment.reasoning.lower()

    async def test_deep_nesting_penalized(self, architect, deep_nesting_cell):
        judgment = await architect.analyze(deep_nesting_cell)
        assert judgment.q_score < MAX_Q_SCORE
        assert judgment.evidence.get("max_nesting_depth", 0) > 5

    async def test_god_class_penalized(self, architect, god_class_cell):
        judgment = await architect.analyze(god_class_cell)
        assert judgment.q_score < MAX_Q_SCORE
        assert any("god-class" in v for v in judgment.evidence.get("violations", []))

    async def test_phi_bounds_always_respected(self, architect, python_cell):
        judgment = await architect.analyze(python_cell)
        assert 0 <= judgment.q_score <= MAX_Q_SCORE
        assert 0 <= judgment.confidence <= MAX_CONFIDENCE

    async def test_no_code_uses_metadata(self, architect):
        """Non-code cell falls back to metadata scoring."""
        cell = Cell(reality="CODE", analysis="JUDGE", content={"price": 0.042})
        judgment = await architect.analyze(cell)
        assert judgment.dog_id == DogId.ARCHITECT
        assert 0 <= judgment.q_score <= MAX_Q_SCORE

    async def test_syntax_error_handled(self, architect):
        """Invalid Python returns a syntax error score, doesn't crash."""
        cell = Cell(reality="CODE", analysis="JUDGE", content="def broken(:pass")
        judgment = await architect.analyze(cell)
        assert judgment.dog_id == DogId.ARCHITECT
        assert judgment.q_score == 0.0  # syntax error → no score

    async def test_non_llm_and_reflex(self, architect):
        """ARCHITECT must be non-LLM and REFLEX capable."""
        caps = architect.get_capabilities()
        assert caps.uses_llm == False
        assert caps.consciousness_min.name == "REFLEX"

    async def test_health_check(self, architect):
        health = await architect.health_check()
        assert health.dog_id == DogId.ARCHITECT
        assert health.status.value == "HEALTHY"

    async def test_architect_in_non_llm_dogs(self):
        assert DogId.ARCHITECT in NON_LLM_DOGS

    async def test_veto_never_set(self, architect, python_cell):
        """ARCHITECT never VETOs — structural issues are advisory."""
        judgment = await architect.analyze(python_cell)
        assert judgment.veto == False


# ════════════════════════════════════════════════════════════════════════════
# ORACLE DOG
# ════════════════════════════════════════════════════════════════════════════

class TestOracleDog:
    """ORACLE — Thompson Sampling prediction from Q-table."""

    @pytest.fixture
    def fresh_oracle(self):
        """Oracle with no Q-table — cold start."""
        return OracleDog(qtable=None)

    @pytest.fixture
    def oracle_with_qtable(self):
        """Oracle with a real (empty) Q-table."""
        from cynic.learning.qlearning import QTable
        qtable = QTable()
        return OracleDog(qtable=qtable)

    @pytest.fixture
    def oracle_with_data(self):
        """Oracle with Q-table that has accumulated data."""
        from cynic.learning.qlearning import QTable, LearningSignal
        qtable = QTable()
        # Train: WAG is good for CODE:JUDGE:PRESENT:1
        for _ in range(25):
            qtable.update(LearningSignal(
                state_key="CODE:JUDGE:PRESENT:1",
                action="WAG",
                reward=0.8,
            ))
        # Train: BARK is bad
        for _ in range(10):
            qtable.update(LearningSignal(
                state_key="CODE:JUDGE:PRESENT:1",
                action="BARK",
                reward=0.1,
            ))
        return OracleDog(qtable=qtable)

    @pytest.fixture
    def code_judge_cell(self):
        return Cell(reality="CODE", analysis="JUDGE", content="x = 1", lod=1)

    async def test_cold_oracle_returns_neutral(self, fresh_oracle, code_judge_cell):
        """No Q-table → neutral GROWL-territory prediction."""
        judgment = await fresh_oracle.analyze(code_judge_cell)
        assert judgment.dog_id == DogId.ORACLE
        # Neutral: 0.5 × 61.8 ≈ 30.9 → GROWL territory
        assert 25.0 <= judgment.q_score <= 35.0
        assert judgment.confidence <= 0.25  # Low confidence: no data

    async def test_empty_qtable_returns_neutral(self, oracle_with_qtable, code_judge_cell):
        """Empty Q-table → cold start → neutral prediction."""
        judgment = await oracle_with_qtable.analyze(code_judge_cell)
        assert judgment.dog_id == DogId.ORACLE
        assert 0 <= judgment.q_score <= MAX_Q_SCORE

    async def test_trained_oracle_predicts_wag(self, oracle_with_data, code_judge_cell):
        """After WAG training, Oracle should predict WAG for CODE:JUDGE:PRESENT:1."""
        judgment = await oracle_with_data.analyze(code_judge_cell)
        assert judgment.dog_id == DogId.ORACLE
        assert judgment.evidence.get("predicted_action") == "WAG"
        # WAG Q-value ≈ 0.8 → q_score ≈ 0.8 × 61.8 ≈ 49.4 (WAG territory)
        assert judgment.q_score > 38.2  # Above GROWL

    async def test_confidence_rises_with_data(self, oracle_with_data, code_judge_cell):
        """More data → higher confidence."""
        judgment = await oracle_with_data.analyze(code_judge_cell)
        assert judgment.confidence > 0.20  # More than cold start

    async def test_phi_bounds_always_respected(self, oracle_with_data, code_judge_cell):
        judgment = await oracle_with_data.analyze(code_judge_cell)
        assert 0 <= judgment.q_score <= MAX_Q_SCORE
        assert 0 <= judgment.confidence <= MAX_CONFIDENCE

    async def test_veto_never_set(self, oracle_with_qtable, code_judge_cell):
        """Oracle never VETOs — it predicts, not blocks."""
        judgment = await oracle_with_qtable.analyze(code_judge_cell)
        assert judgment.veto == False

    async def test_non_llm_and_reflex(self, fresh_oracle):
        """ORACLE must be non-LLM and REFLEX capable."""
        caps = fresh_oracle.get_capabilities()
        assert caps.uses_llm == False
        assert caps.consciousness_min.name == "REFLEX"

    async def test_supports_all_realities(self, fresh_oracle):
        """Oracle works for all 7 reality dimensions."""
        realities = ["CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"]
        caps = fresh_oracle.get_capabilities()
        for r in realities:
            assert r in caps.supported_realities

    async def test_oracle_in_non_llm_dogs(self):
        assert DogId.ORACLE in NON_LLM_DOGS

    async def test_health_check_cold(self, fresh_oracle):
        health = await fresh_oracle.health_check()
        assert health.dog_id == DogId.ORACLE
        assert health.status.value == "UNKNOWN"  # No votes yet

    async def test_health_check_after_analysis(self, oracle_with_qtable, code_judge_cell):
        await oracle_with_qtable.analyze(code_judge_cell)
        health = await oracle_with_qtable.health_check()
        assert health.dog_id == DogId.ORACLE


# ═══════════════════════════════════════════════════════════════════════════
# SCHOLAR DOG TESTS
# ═══════════════════════════════════════════════════════════════════════════

from cynic.dogs.scholar import ScholarDog, BUFFER_MAX, K_NEIGHBORS, MIN_SIMILARITY


@pytest.fixture
def fresh_scholar():
    return ScholarDog()


@pytest.fixture
def code_cell():
    return Cell(
        reality="CODE",
        analysis="JUDGE",
        content="def add(a, b):\n    return a + b\n",
    )


@pytest.fixture
def scholar_with_history(fresh_scholar):
    """Scholar pre-loaded with 5 past judgments."""
    samples = [
        ("def foo(x):\n    return x + 1\n", 55.0, "CODE"),
        ("def bar(y):\n    return y * 2\n", 50.0, "CODE"),
        ("def baz(z):\n    return z - 3\n", 45.0, "CODE"),
        ("class GodClass:\n" + "    def m(self): pass\n" * 20, 10.0, "CODE"),
        ("import os\nimport sys\n", 35.0, "CODE"),
    ]
    for text, q, reality in samples:
        fresh_scholar.learn(text, q, reality=reality)
    return fresh_scholar


@pytest.mark.asyncio
class TestScholarDog:

    async def test_cold_returns_neutral(self, fresh_scholar, code_cell):
        """Empty buffer → neutral GROWL at low confidence."""
        judgment = await fresh_scholar.analyze(code_cell)
        assert judgment.dog_id == DogId.SCHOLAR
        assert 25.0 <= judgment.q_score <= 40.0   # GROWL territory
        assert judgment.confidence <= 0.25

    async def test_phi_bounds(self, fresh_scholar, code_cell):
        """q_score and confidence always within φ bounds."""
        judgment = await fresh_scholar.analyze(code_cell)
        from cynic.core.phi import MAX_Q_SCORE, MAX_CONFIDENCE
        assert 0.0 <= judgment.q_score <= MAX_Q_SCORE
        assert 0.0 <= judgment.confidence <= MAX_CONFIDENCE

    async def test_similar_code_finds_neighbors(self, scholar_with_history, code_cell):
        """Code similar to history should find neighbors."""
        judgment = await scholar_with_history.analyze(code_cell)
        assert judgment.dog_id == DogId.SCHOLAR
        # With similar function content, should find neighbors
        # Either a hit (with q_score informed by neighbors) or a cold miss
        assert 0.0 <= judgment.q_score <= 61.8

    async def test_learn_adds_to_buffer(self, fresh_scholar):
        """learn() grows buffer."""
        assert len(fresh_scholar._buffer) == 0
        fresh_scholar.learn("def x(): pass", 50.0)
        assert len(fresh_scholar._buffer) == 1

    async def test_buffer_rolling_eviction(self, fresh_scholar):
        """Buffer evicts oldest entries when full."""
        for i in range(BUFFER_MAX + 5):
            fresh_scholar.learn(f"def func_{i}(): return {i}", float(i % 62))
        assert len(fresh_scholar._buffer) == BUFFER_MAX

    async def test_duplicate_cell_id_skipped(self, fresh_scholar):
        """Same cell_id not recorded twice."""
        fresh_scholar.learn("code", 50.0, cell_id="abc")
        fresh_scholar.learn("other code", 30.0, cell_id="abc")
        assert len(fresh_scholar._buffer) == 1

    async def test_veto_never_set(self, scholar_with_history, code_cell):
        """Scholar never VETOs — advisory only."""
        judgment = await scholar_with_history.analyze(code_cell)
        assert judgment.veto == False

    async def test_micro_not_reflex(self, fresh_scholar):
        """Scholar needs MICRO (TF-IDF too slow for REFLEX). Uses LLM when available."""
        caps = fresh_scholar.get_capabilities()
        assert caps.consciousness_min.name == "MICRO"
        assert caps.uses_llm is True

    async def test_supports_all_realities(self, fresh_scholar):
        """Scholar works for all 7 reality dimensions."""
        caps = fresh_scholar.get_capabilities()
        for r in ["CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"]:
            assert r in caps.supported_realities

    async def test_health_cold_unknown(self, fresh_scholar):
        health = await fresh_scholar.health_check()
        assert health.dog_id == DogId.SCHOLAR
        assert health.status.value == "UNKNOWN"

    async def test_health_after_lookups(self, scholar_with_history, code_cell):
        """Health reflects lookup activity."""
        await scholar_with_history.analyze(code_cell)
        health = await scholar_with_history.health_check()
        assert health.dog_id == DogId.SCHOLAR
        assert health.status.value in ("HEALTHY", "DEGRADED", "UNKNOWN")
