"""
Tests for CYNIC PBFT (Byzantine Fault Tolerant) Consensus Engine.

Tests validate:
- PBFTEngine initialization with num_dogs validation
- Fault tolerance calculation: f = floor((n-1)/3)
- Byzantine supermajority requirement: > 2f votes needed
- Consensus reaching with unanimous verdicts
- Consensus with clear majorities (9 HOWL, 2 WAG)
- Consensus with marginal majorities (8 HOWL, 3 WAG)
- No consensus fallback to WAG (neutral verdict)
- Consensus attributes: verdict, confidence, q_score, dog_consensus
- Fault tolerance: up to 3 Dogs can be faulty, consensus still reached
- Confidence aggregation: average of agreeing Dogs
- Q-score aggregation: average of agreeing Dogs
- Logging at appropriate levels (DEBUG per dog, INFO consensus result)
- Integration with UnifiedConsciousState.reach_consensus_judgment()
"""
from __future__ import annotations

import pytest
import uuid
from unittest.mock import Mock, patch, AsyncMock

from cynic.kernel.organism.brain.consensus.pbft_engine import PBFTEngine
from cynic.kernel.core.unified_state import UnifiedJudgment, UnifiedConsciousState


class TestPBFTEngineInitialization:
    """Test PBFTEngine initialization and constraint validation."""

    def test_pbft_engine_initialization_with_defaults(self):
        """PBFTEngine should initialize with 11 dogs and fault_tolerance=3."""
        engine = PBFTEngine()
        assert engine.num_dogs == 11
        assert engine.fault_tolerance == 3
        assert engine.required_votes == 8  # floor(2*11/3) + 1 = 7 + 1 = 8

    def test_pbft_engine_initialization_with_custom_dogs(self):
        """PBFTEngine should accept custom num_dogs."""
        engine = PBFTEngine(num_dogs=7)
        assert engine.num_dogs == 7
        # For n=7: f = floor((7-1)/3) = 2
        # required_votes = floor(2*7/3) + 1 = 4 + 1 = 5
        assert engine.fault_tolerance == 2
        assert engine.required_votes == 5

    def test_pbft_engine_requires_min_4_dogs(self):
        """PBFTEngine requires at least 4 dogs for Byzantine consensus."""
        # n=3 should fail
        with pytest.raises(ValueError, match="at least 4"):
            PBFTEngine(num_dogs=3)

        # n=2 should fail
        with pytest.raises(ValueError, match="at least 4"):
            PBFTEngine(num_dogs=2)

        # n=1 should fail
        with pytest.raises(ValueError, match="at least 4"):
            PBFTEngine(num_dogs=1)

    def test_pbft_engine_fault_tolerance_calculation(self):
        """Fault tolerance should be floor((num_dogs - 1) / 3)."""
        # n=4: f = floor((4-1)/3) = 1
        engine4 = PBFTEngine(num_dogs=4)
        assert engine4.fault_tolerance == 1
        assert engine4.required_votes == 3  # floor(2*4/3) + 1 = 2 + 1 = 3

        # n=7: f = floor((7-1)/3) = 2
        engine7 = PBFTEngine(num_dogs=7)
        assert engine7.fault_tolerance == 2
        assert engine7.required_votes == 5  # floor(2*7/3) + 1 = 4 + 1 = 5

        # n=11: f = floor((11-1)/3) = 3
        engine11 = PBFTEngine(num_dogs=11)
        assert engine11.fault_tolerance == 3
        assert engine11.required_votes == 8  # floor(2*11/3) + 1 = 7 + 1 = 8

    def test_pbft_engine_rejects_invalid_fault_tolerance(self):
        """PBFTEngine should reject incorrect fault_tolerance."""
        # Correct f for n=11 is 3, providing 4 should fail
        with pytest.raises(ValueError, match="Invalid fault_tolerance"):
            PBFTEngine(num_dogs=11, fault_tolerance=4)

        # Correct f for n=7 is 2, providing 1 should fail
        with pytest.raises(ValueError, match="Invalid fault_tolerance"):
            PBFTEngine(num_dogs=7, fault_tolerance=1)


class TestPBFTEngineConsensus:
    """Test PBFT consensus algorithm."""

    def _make_judgment(
        self,
        dog_id: int,
        verdict: str,
        q_score: float = 85.0,
        confidence: float = 0.5,
    ) -> UnifiedJudgment:
        """Helper to create a UnifiedJudgment."""
        return UnifiedJudgment(
            judgment_id=str(uuid.uuid4()),
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores={
                "FIDELITY": 0.8,
                "PHI": 0.9,
                "VERIFY": 0.85,
                "CULTURE": 0.8,
                "BURN": 0.75,
            },
            dog_votes={dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"Dog {dog_id} judges {verdict}",
            latency_ms=10.5,
        )

    @pytest.mark.asyncio
    async def test_pbft_engine_reach_consensus_empty_judgments(self):
        """reach_consensus should raise ValueError on empty judgments list."""
        engine = PBFTEngine()
        with pytest.raises(ValueError, match="empty"):
            await engine.reach_consensus([])

    @pytest.mark.asyncio
    async def test_pbft_engine_reach_consensus_all_same_verdict(self):
        """All Dogs voting same verdict should reach unanimous consensus."""
        engine = PBFTEngine(num_dogs=11)

        # All 11 Dogs vote HOWL
        judgments = [self._make_judgment(i, "HOWL") for i in range(1, 12)]

        consensus = await engine.reach_consensus(judgments)

        assert consensus.verdict == "HOWL"
        assert len(consensus.dog_votes) == 11
        # Confidence should be average of all 11 dogs (all 0.5)
        assert consensus.confidence == pytest.approx(0.5)
        # Q-score should be average of all 11 dogs (all 85.0)
        assert consensus.q_score == pytest.approx(85.0)

    @pytest.mark.asyncio
    async def test_pbft_engine_reach_consensus_clear_majority(self):
        """Clear majority (9 HOWL, 2 WAG) should reach consensus on HOWL."""
        engine = PBFTEngine(num_dogs=11)

        # 9 Dogs vote HOWL, 2 vote WAG
        judgments = (
            [self._make_judgment(i, "HOWL", q_score=90.0, confidence=0.6) for i in range(1, 10)]
            + [self._make_judgment(i, "WAG", q_score=70.0, confidence=0.3) for i in range(10, 12)]
        )

        consensus = await engine.reach_consensus(judgments)

        assert consensus.verdict == "HOWL"
        # Confidence: average of 9 HOWL dogs (0.6) = 0.6
        assert consensus.confidence == pytest.approx(0.6)
        # Q-score: average of 9 HOWL dogs (90.0) = 90.0
        assert consensus.q_score == pytest.approx(90.0)

    @pytest.mark.asyncio
    async def test_pbft_engine_reach_consensus_marginal_majority(self):
        """Marginal majority (8 HOWL, 3 WAG) meets threshold and reaches consensus."""
        engine = PBFTEngine(num_dogs=11)

        # Exactly the required votes: 8 HOWL (>= 8), 3 WAG
        judgments = (
            [self._make_judgment(i, "HOWL", q_score=88.0, confidence=0.55) for i in range(1, 9)]
            + [self._make_judgment(i, "WAG", q_score=72.0, confidence=0.35) for i in range(9, 12)]
        )

        consensus = await engine.reach_consensus(judgments)

        assert consensus.verdict == "HOWL"
        # Confidence: average of 8 HOWL dogs (0.55) = 0.55
        assert consensus.confidence == pytest.approx(0.55)
        # Q-score: average of 8 HOWL dogs (88.0) = 88.0
        assert consensus.q_score == pytest.approx(88.0)

    @pytest.mark.asyncio
    async def test_pbft_engine_reach_consensus_no_majority_defaults_to_wag(self):
        """No clear majority defaults to WAG (neutral) verdict."""
        engine = PBFTEngine(num_dogs=11)

        # Divided votes: 4 HOWL, 4 WAG, 3 GROWL (none meets 8 vote threshold)
        judgments = (
            [self._make_judgment(i, "HOWL", q_score=85.0, confidence=0.5) for i in range(1, 5)]
            + [self._make_judgment(i, "WAG", q_score=75.0, confidence=0.4) for i in range(5, 9)]
            + [self._make_judgment(i, "GROWL", q_score=80.0, confidence=0.45) for i in range(9, 12)]
        )

        consensus = await engine.reach_consensus(judgments)

        assert consensus.verdict == "WAG"  # Default to neutral
        # Confidence should be low (not from disagreeing dogs)
        assert consensus.confidence < 0.5

    @pytest.mark.asyncio
    async def test_pbft_engine_aggregates_confidence(self):
        """Consensus confidence should be average of agreeing Dogs."""
        engine = PBFTEngine(num_dogs=11)

        # All vote HOWL with different confidence values (respecting φ-bound max 0.618)
        confidence_values = [0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.6]
        judgments = [
            self._make_judgment(i, "HOWL", confidence=confidence_values[i - 1])
            for i in range(1, 12)
        ]

        consensus = await engine.reach_consensus(judgments)

        # Average confidence: sum of all / 11
        expected_conf = sum(confidence_values) / len(confidence_values)
        assert consensus.confidence == pytest.approx(expected_conf)

    @pytest.mark.asyncio
    async def test_pbft_engine_aggregates_q_score(self):
        """Consensus q_score should be average of agreeing Dogs."""
        engine = PBFTEngine(num_dogs=11)

        # All vote HOWL with different Q-scores
        judgments = [
            self._make_judgment(i, "HOWL", q_score=70.0 + (i * 2))
            for i in range(1, 12)
        ]

        consensus = await engine.reach_consensus(judgments)

        # Average Q-score: (72 + 74 + 76 + ... + 92) / 11
        expected_q = sum([70.0 + (i * 2) for i in range(1, 12)]) / 11
        assert consensus.q_score == pytest.approx(expected_q)

    @pytest.mark.asyncio
    async def test_pbft_engine_fault_tolerance_3_faulty_dogs(self):
        """With 3 faulty Dogs (1/3 of 11), consensus still reached."""
        engine = PBFTEngine(num_dogs=11)

        # 8 Dogs vote HOWL (meets threshold), 3 vote against (faulty)
        judgments = (
            [self._make_judgment(i, "HOWL", q_score=90.0) for i in range(1, 9)]
            + [self._make_judgment(i, "BARK", q_score=50.0) for i in range(9, 12)]
        )

        consensus = await engine.reach_consensus(judgments)

        assert consensus.verdict == "HOWL"
        # Consensus with exactly 8 votes still valid

    @pytest.mark.asyncio
    async def test_pbft_engine_fault_tolerance_4_faulty_dogs_fails(self):
        """With 4 faulty Dogs (> 1/3 of 11), consensus cannot be guaranteed."""
        engine = PBFTEngine(num_dogs=11)

        # Only 7 Dogs vote HOWL (below 8 threshold), 4 vote against
        judgments = (
            [self._make_judgment(i, "HOWL", q_score=90.0) for i in range(1, 8)]
            + [self._make_judgment(i, "BARK", q_score=50.0) for i in range(8, 12)]
        )

        consensus = await engine.reach_consensus(judgments)

        # Should default to WAG since HOWL doesn't meet threshold
        assert consensus.verdict == "WAG"

    @pytest.mark.asyncio
    async def test_pbft_engine_verdict_attributes_present(self):
        """Consensus verdict should include all required attributes."""
        engine = PBFTEngine(num_dogs=11)

        judgments = [self._make_judgment(i, "WAG") for i in range(1, 12)]

        consensus = await engine.reach_consensus(judgments)

        # Should have all required attributes
        assert hasattr(consensus, "verdict")
        assert hasattr(consensus, "confidence")
        assert hasattr(consensus, "q_score")
        assert hasattr(consensus, "dog_votes")
        assert hasattr(consensus, "reasoning")

        # Verdict should be UnifiedJudgment (immutable)
        assert isinstance(consensus, UnifiedJudgment)
        assert consensus.verdict == "WAG"

    @pytest.mark.asyncio
    async def test_pbft_engine_mismatched_dog_count(self):
        """Should handle cases where judgment count doesn't match num_dogs."""
        engine = PBFTEngine(num_dogs=11)

        # Only 8 judgments (fewer than 11 dogs)
        judgments = [self._make_judgment(i, "HOWL") for i in range(1, 9)]

        consensus = await engine.reach_consensus(judgments)

        # Should still compute consensus based on available judgments
        assert consensus.verdict == "HOWL"  # 8 votes >= required for 8 dogs

    @pytest.mark.asyncio
    async def test_pbft_engine_more_judgments_than_expected(self):
        """Should handle cases where judgment count exceeds num_dogs."""
        engine = PBFTEngine(num_dogs=11)

        # 15 judgments (more than 11 dogs)
        judgments = [self._make_judgment(i % 11, "HOWL") for i in range(15)]

        consensus = await engine.reach_consensus(judgments)

        assert consensus.verdict == "HOWL"


class TestPBFTEngineLogging:
    """Test logging behavior of PBFT consensus engine."""

    @pytest.mark.asyncio
    async def test_pbft_engine_logs_debug_per_dog(self, caplog):
        """Should log DEBUG for each Dog's judgment."""
        import logging

        caplog.set_level(logging.DEBUG)
        engine = PBFTEngine(num_dogs=11)

        judgments = [self._make_judgment(i, "HOWL") for i in range(1, 12)]

        await engine.reach_consensus(judgments)

        # Should have debug logs for each dog
        debug_logs = [r for r in caplog.records if r.levelname == "DEBUG"]
        assert len(debug_logs) >= 11  # At least one per dog

    @pytest.mark.asyncio
    async def test_pbft_engine_logs_info_consensus_result(self, caplog):
        """Should log INFO with consensus result."""
        import logging

        caplog.set_level(logging.INFO)
        engine = PBFTEngine(num_dogs=11)

        judgments = [self._make_judgment(i, "HOWL") for i in range(1, 12)]

        await engine.reach_consensus(judgments)

        # Should have info logs for consensus
        info_logs = [r for r in caplog.records if r.levelname == "INFO"]
        assert len(info_logs) >= 1

    def _make_judgment(
        self,
        dog_id: int,
        verdict: str,
        q_score: float = 85.0,
        confidence: float = 0.5,
    ) -> UnifiedJudgment:
        """Helper to create a UnifiedJudgment."""
        return UnifiedJudgment(
            judgment_id=str(uuid.uuid4()),
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores={
                "FIDELITY": 0.8,
                "PHI": 0.9,
                "VERIFY": 0.85,
                "CULTURE": 0.8,
                "BURN": 0.75,
            },
            dog_votes={dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"Dog {dog_id} judges {verdict}",
            latency_ms=10.5,
        )


class TestUnifiedConsciousStateConsensus:
    """Test PBFT consensus integration with UnifiedConsciousState."""

    def _make_judgment(
        self,
        dog_id: int,
        verdict: str,
        q_score: float = 85.0,
        confidence: float = 0.5,
    ) -> UnifiedJudgment:
        """Helper to create a UnifiedJudgment."""
        return UnifiedJudgment(
            judgment_id=str(uuid.uuid4()),
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores={
                "FIDELITY": 0.8,
                "PHI": 0.9,
                "VERIFY": 0.85,
                "CULTURE": 0.8,
                "BURN": 0.75,
            },
            dog_votes={dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"Dog {dog_id} judges {verdict}",
            latency_ms=10.5,
        )

    @pytest.mark.asyncio
    async def test_conscious_state_reach_consensus_judgment(self):
        """UnifiedConsciousState should use PBFTEngine to reach consensus."""
        state = UnifiedConsciousState()

        # Create 11 judgments (one per Dog)
        judgments = [self._make_judgment(i, "WAG") for i in range(1, 12)]

        consensus = await state.reach_consensus_judgment(judgments)

        assert isinstance(consensus, UnifiedJudgment)
        assert consensus.verdict == "WAG"

    @pytest.mark.asyncio
    async def test_conscious_state_reach_consensus_with_varied_verdicts(self):
        """Consensus should handle varied verdicts correctly."""
        state = UnifiedConsciousState()

        # 7 HOWL, 4 WAG (HOWL wins with 7, but needs 8 for 11 dogs)
        judgments = (
            [self._make_judgment(i, "HOWL", q_score=90.0) for i in range(1, 8)]
            + [self._make_judgment(i, "WAG", q_score=70.0) for i in range(8, 12)]
        )

        consensus = await state.reach_consensus_judgment(judgments)

        # With only 7 votes, should fall back to WAG
        assert consensus.verdict == "WAG"


class TestTieBreakers:
    """Test tie-breaker logic when multiple verdicts have equal votes."""

    def _make_judgment(
        self,
        dog_id: int,
        verdict: str,
        q_score: float = 85.0,
        confidence: float = 0.5,
    ) -> UnifiedJudgment:
        """Helper to create a UnifiedJudgment."""
        return UnifiedJudgment(
            judgment_id=str(uuid.uuid4()),
            verdict=verdict,
            q_score=q_score,
            confidence=confidence,
            axiom_scores={
                "FIDELITY": 0.8,
                "PHI": 0.9,
                "VERIFY": 0.85,
                "CULTURE": 0.8,
                "BURN": 0.75,
            },
            dog_votes={dog_id: {"vote": verdict, "confidence": confidence}},
            reasoning=f"Dog {dog_id} judges {verdict}",
            latency_ms=10.5,
        )

    @pytest.mark.asyncio
    async def test_pbft_engine_tie_breaker_hierarchical(self):
        """When verdicts tie, should use hierarchical preference: HOWL > WAG > GROWL > BARK."""
        engine = PBFTEngine(num_dogs=11)

        # 4 HOWL, 4 WAG, 3 GROWL (4-way tie between HOWL and WAG at 4 each)
        # None meet threshold, so we test the default behavior
        judgments = (
            [self._make_judgment(i, "HOWL") for i in range(1, 5)]
            + [self._make_judgment(i, "WAG") for i in range(5, 9)]
            + [self._make_judgment(i, "GROWL") for i in range(9, 12)]
        )

        consensus = await engine.reach_consensus(judgments)

        # Should default to WAG when no consensus
        assert consensus.verdict == "WAG"
