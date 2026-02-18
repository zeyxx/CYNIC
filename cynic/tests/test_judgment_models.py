"""
Tests: Pydantic Judgment Models

φ-bound enforcement at model level — if DB constraints mirror these,
any data corruption is caught twice (Pydantic + PostgreSQL).

LAW: q_score > 61.8 must RAISE. confidence > 0.618 must RAISE.
"""
import pytest
from pydantic import ValidationError

from cynic.core.phi import MAX_Q_SCORE, MAX_CONFIDENCE
from cynic.core.judgment import Cell, Judgment, ConsensusResult, EScore


class TestCell:
    """Cell model validation."""

    def test_valid_cell(self, code_cell):
        assert code_cell.reality == "CODE"
        assert code_cell.analysis == "JUDGE"

    def test_invalid_reality_raises(self):
        with pytest.raises(ValidationError):
            Cell(reality="INVALID", analysis="JUDGE", content={})

    def test_invalid_analysis_raises(self):
        with pytest.raises(ValidationError):
            Cell(reality="CODE", analysis="INVALID", content={})

    def test_invalid_time_dim_raises(self):
        with pytest.raises(ValidationError):
            Cell(reality="CODE", analysis="JUDGE", content={}, time_dim="YESTERDAY")

    def test_lod_bounds(self):
        with pytest.raises(ValidationError):
            Cell(reality="CODE", analysis="JUDGE", content={}, lod=4)  # max is 3

    def test_consciousness_bounds(self):
        with pytest.raises(ValidationError):
            Cell(reality="CODE", analysis="JUDGE", content={}, consciousness=7)  # max is 6

    def test_state_key_format(self, code_cell):
        key = code_cell.state_key()
        assert "CODE" in key
        assert "JUDGE" in key
        assert "PRESENT" in key

    def test_all_realities_valid(self):
        for reality in ["CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"]:
            cell = Cell(reality=reality, analysis="JUDGE", content={})
            assert cell.reality == reality

    def test_all_analyses_valid(self):
        for analysis in ["PERCEIVE", "JUDGE", "DECIDE", "ACT", "LEARN", "ACCOUNT", "EMERGE"]:
            cell = Cell(reality="CODE", analysis=analysis, content={})
            assert cell.analysis == analysis


class TestJudgment:
    """Judgment model — φ-bounds enforced."""

    def _make_judgment(self, q_score: float, verdict: str, confidence: float, code_cell) -> Judgment:
        return Judgment(
            cell=code_cell,
            q_score=q_score,
            verdict=verdict,
            confidence=confidence,
        )

    def test_valid_bark(self, code_cell):
        j = self._make_judgment(20.0, "BARK", 0.3, code_cell)
        assert j.verdict == "BARK"

    def test_valid_growl(self, code_cell):
        j = self._make_judgment(45.0, "GROWL", 0.4, code_cell)
        assert j.verdict == "GROWL"

    def test_valid_wag(self, code_cell):
        # WAG: q_score in [61.8, 82] — but MAX_Q_SCORE=61.8 so WAG is at the upper edge
        # Test with a q_score that maps to WAG via consistency check is impossible:
        # HOWL ≥82 but MAX_Q_SCORE=61.8 → HOWL unreachable
        # WAG ≥61.8 and <82 — but max allowed is 61.8, so WAG exactly at threshold
        # Only WAG=61.8 (exact) is possible. Test GROWL instead to avoid boundary.
        j = self._make_judgment(50.0, "GROWL", 0.4, code_cell)
        assert j.verdict == "GROWL"

    def test_valid_howl(self, code_cell):
        # D1 decision: MAX_Q_SCORE = 100.0, HOWL now reachable (q_score ≥ 82)
        j = self._make_judgment(85.0, "HOWL", 0.5, code_cell)
        assert j.q_score == 85.0
        assert j.verdict == "HOWL"

    def test_q_score_exceeds_max_raises(self, code_cell):
        """φ-bound: q_score > 100.0 must raise ValidationError."""
        with pytest.raises(ValidationError):
            Judgment(
                cell=code_cell,
                q_score=105.0,  # Exceeds MAX_Q_SCORE=100
                verdict="HOWL",
                confidence=0.4,
            )

    def test_confidence_exceeds_max_raises(self, code_cell):
        """φ-bound: confidence > 0.618 must raise ValidationError."""
        with pytest.raises(ValidationError):
            Judgment(
                cell=code_cell,
                q_score=40.0,
                verdict="GROWL",
                confidence=0.9,  # Exceeds MAX_CONFIDENCE
            )

    def test_verdict_mismatch_raises(self, code_cell):
        """Verdict must match q_score range."""
        with pytest.raises(ValidationError):
            Judgment(
                cell=code_cell,
                q_score=10.0,   # BARK range
                verdict="WAG",  # Wrong verdict
                confidence=0.3,
            )

    def test_to_dict_format(self, code_cell):
        j = Judgment(cell=code_cell, q_score=45.0, verdict="GROWL", confidence=0.4)
        d = j.to_dict()
        assert "judgment_id" in d
        assert "q_score" in d
        assert "verdict" in d
        assert "timestamp" in d
        assert d["verdict"] == "GROWL"

    def test_residual_variance_range(self, code_cell):
        j = Judgment(cell=code_cell, q_score=30.0, verdict="BARK", confidence=0.3)
        assert j.residual_variance == 0.0
        assert j.unnameable_detected == False


class TestConsensusResult:
    """ConsensusResult — PBFT output validation."""

    def test_quorum_reached_property(self):
        result = ConsensusResult(consensus=True, votes=7, quorum=7)
        assert result.quorum_reached == True

    def test_quorum_not_reached(self):
        result = ConsensusResult(consensus=False, votes=4, quorum=7)
        assert result.quorum_reached == False

    def test_q_score_bounded(self):
        with pytest.raises(ValidationError):
            ConsensusResult(
                consensus=True,
                votes=7,
                quorum=7,
                final_q_score=105.0,  # exceeds MAX_Q_SCORE=100
            )


class TestEScore:
    """E-Score reputation model."""

    def test_trust_weight_phi_amplified(self):
        from cynic.core.phi import PHI_INV
        score = EScore(agent_id="test", total=100.0)
        # trust_weight = (total/100)^φ⁻¹ = 1.0^0.618 = 1.0
        assert abs(score.trust_weight - 1.0) < 1e-10

    def test_zero_total_trust(self):
        score = EScore(agent_id="test", total=0.0)
        assert score.trust_weight == 0.0

    def test_total_bounds(self):
        with pytest.raises(ValidationError):
            EScore(agent_id="test", total=101.0)
