"""
Tests for JudgeInterface contract and BaseJudge foundation.

These tests verify:
1. JudgeInterface is abstract
2. All judges return UnifiedJudgment
3. All judges are async
4. BaseJudge provides helper methods (φ-bounding, tracking)
5. Confidence is φ-bounded to PHI_INV (0.618)
"""

import pytest
import asyncio
from abc import ABC, abstractmethod

from cynic.core.judge_interface import JudgeInterface, BaseJudge
from cynic.core.unified_state import UnifiedJudgment
from cynic.core.phi import PHI_INV, MAX_CONFIDENCE


class TestJudgeInterface:
    """Test JudgeInterface is properly abstract."""

    def test_judge_interface_is_abstract(self):
        """JudgeInterface cannot be instantiated directly."""
        with pytest.raises(TypeError):
            JudgeInterface()

    def test_judge_interface_requires_judge_method(self):
        """Any subclass must implement judge() method."""

        class IncompleteJudge(JudgeInterface):
            pass

        with pytest.raises(TypeError):
            IncompleteJudge()

    def test_judge_interface_is_abc(self):
        """JudgeInterface is an ABC."""
        assert isinstance(JudgeInterface, type)
        assert issubclass(JudgeInterface, ABC)


class TestBaseJudge:
    """Test BaseJudge foundation class."""

    def test_base_judge_is_abstract(self):
        """BaseJudge cannot be instantiated directly."""
        with pytest.raises(TypeError):
            BaseJudge(dog_id=1, dog_name="Test", axiom_focus="TEST")

    def test_base_judge_initializes_properly(self):
        """BaseJudge subclass initializes with correct attributes."""

        class TestDog(BaseJudge):
            async def judge(self, proposal_text: str, context: dict) -> UnifiedJudgment:
                return UnifiedJudgment(
                    judgment_id="test-1",
                    verdict="WAG",
                    q_score=60.0,
                    confidence=0.5,
                    axiom_scores={"FIDELITY": 60.0},
                    dog_votes={1: {"vote": "WAG", "confidence": 0.5}},
                    reasoning="Test",
                    latency_ms=10.0,
                )

        dog = TestDog(dog_id=1, dog_name="Test Dog", axiom_focus="FIDELITY")
        assert dog.dog_id == 1
        assert dog.dog_name == "Test Dog"
        assert dog.axiom_focus == "FIDELITY"
        assert dog.judgments_made == 0
        assert dog.confidence_history == []

    def test_base_judge_confidence_bounded(self):
        """BaseJudge._calculate_phi_bounded_confidence bounds to PHI_INV."""

        class TestDog(BaseJudge):
            async def judge(self, proposal_text: str, context: dict) -> UnifiedJudgment:
                pass

        dog = TestDog(dog_id=1, dog_name="Test", axiom_focus="TEST")

        # Test values above PHI_INV are clamped
        assert dog._calculate_phi_bounded_confidence(1.0) == PHI_INV
        assert dog._calculate_phi_bounded_confidence(0.9) == PHI_INV
        assert dog._calculate_phi_bounded_confidence(PHI_INV) == PHI_INV

        # Test values below PHI_INV are preserved
        assert dog._calculate_phi_bounded_confidence(0.5) == 0.5
        assert dog._calculate_phi_bounded_confidence(0.618) < PHI_INV  # 0.618 is less than PHI_INV
        assert dog._calculate_phi_bounded_confidence(0.0) == 0.0

        # Test that PHI_INV itself is returned for values >= PHI_INV
        assert abs(dog._calculate_phi_bounded_confidence(PHI_INV) - PHI_INV) < 1e-10

    def test_base_judge_tracks_judgments(self):
        """BaseJudge tracks number of judgments made."""

        class TestDog(BaseJudge):
            async def judge(self, proposal_text: str, context: dict) -> UnifiedJudgment:
                self.judgments_made += 1
                return UnifiedJudgment(
                    judgment_id="test-1",
                    verdict="WAG",
                    q_score=60.0,
                    confidence=0.5,
                    axiom_scores={"FIDELITY": 60.0},
                    dog_votes={1: {"vote": "WAG", "confidence": 0.5}},
                    reasoning="Test",
                    latency_ms=10.0,
                )

        dog = TestDog(dog_id=1, dog_name="Test", axiom_focus="TEST")
        assert dog.judgments_made == 0

        # Simulate incrementing
        dog.judgments_made += 1
        assert dog.judgments_made == 1

    def test_base_judge_tracks_confidence_history(self):
        """BaseJudge can track confidence history."""

        class TestDog(BaseJudge):
            async def judge(self, proposal_text: str, context: dict) -> UnifiedJudgment:
                return UnifiedJudgment(
                    judgment_id="test-1",
                    verdict="WAG",
                    q_score=60.0,
                    confidence=0.5,
                    axiom_scores={"FIDELITY": 60.0},
                    dog_votes={1: {"vote": "WAG", "confidence": 0.5}},
                    reasoning="Test",
                    latency_ms=10.0,
                )

        dog = TestDog(dog_id=1, dog_name="Test", axiom_focus="TEST")
        dog.confidence_history.append(0.5)
        dog.confidence_history.append(0.6)

        assert len(dog.confidence_history) == 2
        assert dog.confidence_history[0] == 0.5
        assert dog.confidence_history[1] == 0.6


class TestJudgeReturnsUnifiedJudgment:
    """Test that judges return proper UnifiedJudgment structures."""

    @pytest.mark.asyncio
    async def test_judge_returns_unified_judgment(self):
        """Judge returns UnifiedJudgment with proper structure."""

        class TestDog(BaseJudge):
            async def judge(self, proposal_text: str, context: dict) -> UnifiedJudgment:
                return UnifiedJudgment(
                    judgment_id="test-1",
                    verdict="WAG",
                    q_score=65.0,
                    confidence=0.5,
                    axiom_scores={
                        "FIDELITY": 65.0,
                        "PHI": 50.0,
                        "VERIFY": 50.0,
                        "CULTURE": 50.0,
                        "BURN": 50.0,
                    },
                    dog_votes={1: {"vote": "WAG", "confidence": 0.5}},
                    reasoning="Test judgment",
                    latency_ms=15.0,
                )

        dog = TestDog(dog_id=1, dog_name="Test", axiom_focus="FIDELITY")
        result = await dog.judge("test proposal", {})

        assert isinstance(result, UnifiedJudgment)
        assert result.verdict == "WAG"
        assert result.q_score == 65.0
        assert result.confidence == 0.5
        assert "FIDELITY" in result.axiom_scores

    @pytest.mark.asyncio
    async def test_judge_is_async(self):
        """Judge method is async and can be awaited."""

        class TestDog(BaseJudge):
            async def judge(self, proposal_text: str, context: dict) -> UnifiedJudgment:
                await asyncio.sleep(0.001)  # Simulate async work
                return UnifiedJudgment(
                    judgment_id="test-1",
                    verdict="WAG",
                    q_score=60.0,
                    confidence=0.5,
                    axiom_scores={"FIDELITY": 60.0},
                    dog_votes={1: {"vote": "WAG", "confidence": 0.5}},
                    reasoning="Test",
                    latency_ms=10.0,
                )

        dog = TestDog(dog_id=1, dog_name="Test", axiom_focus="FIDELITY")

        # Verify it's actually async
        result = dog.judge("test", {})
        assert asyncio.iscoroutine(result)

        # Await it
        judgment = await result
        assert isinstance(judgment, UnifiedJudgment)

    @pytest.mark.asyncio
    async def test_judge_validates_q_score_bounds(self):
        """Judge must return valid Q-Score in [0, 100]."""

        class TestDog(BaseJudge):
            async def judge(self, proposal_text: str, context: dict) -> UnifiedJudgment:
                return UnifiedJudgment(
                    judgment_id="test-1",
                    verdict="WAG",
                    q_score=150.0,  # INVALID
                    confidence=0.5,
                    axiom_scores={"FIDELITY": 60.0},
                    dog_votes={1: {"vote": "WAG", "confidence": 0.5}},
                    reasoning="Test",
                    latency_ms=10.0,
                )

        dog = TestDog(dog_id=1, dog_name="Test", axiom_focus="FIDELITY")

        # Should raise ValueError in UnifiedJudgment.__post_init__
        with pytest.raises(ValueError):
            await dog.judge("test", {})

    @pytest.mark.asyncio
    async def test_judge_validates_confidence_bounds(self):
        """Judge must return confidence in [0, MAX_CONFIDENCE] (0.618)."""

        class TestDog(BaseJudge):
            async def judge(self, proposal_text: str, context: dict) -> UnifiedJudgment:
                return UnifiedJudgment(
                    judgment_id="test-1",
                    verdict="WAG",
                    q_score=60.0,
                    confidence=1.0,  # INVALID (>0.618)
                    axiom_scores={"FIDELITY": 60.0},
                    dog_votes={1: {"vote": "WAG", "confidence": 0.5}},
                    reasoning="Test",
                    latency_ms=10.0,
                )

        dog = TestDog(dog_id=1, dog_name="Test", axiom_focus="FIDELITY")

        # Should raise ValueError in UnifiedJudgment.__post_init__
        with pytest.raises(ValueError):
            await dog.judge("test", {})

    @pytest.mark.asyncio
    async def test_judge_validates_verdict(self):
        """Judge must return verdict in {HOWL, WAG, GROWL, BARK}."""

        class TestDog(BaseJudge):
            async def judge(self, proposal_text: str, context: dict) -> UnifiedJudgment:
                return UnifiedJudgment(
                    judgment_id="test-1",
                    verdict="INVALID",  # INVALID
                    q_score=60.0,
                    confidence=0.5,
                    axiom_scores={"FIDELITY": 60.0},
                    dog_votes={1: {"vote": "WAG", "confidence": 0.5}},
                    reasoning="Test",
                    latency_ms=10.0,
                )

        dog = TestDog(dog_id=1, dog_name="Test", axiom_focus="FIDELITY")

        # Should raise ValueError in UnifiedJudgment.__post_init__
        with pytest.raises(ValueError):
            await dog.judge("test", {})
