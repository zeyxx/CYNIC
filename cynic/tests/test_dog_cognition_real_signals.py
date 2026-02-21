"""
DogCognition with REAL Signals — Proves entropy > 0

This test demonstrates the paradigm shift:
- Phase 4 benchmark had entropy < 0 (adding noise)
- With REAL signal analysis, entropy > 0 (creating knowledge)

The key change: DogCognition._judge_domain() now analyzes by signal TYPE,
not just COUNT. This makes efficiency positive and confidence honest.
"""
import pytest
from unittest.mock import MagicMock
from cynic.cognition.neurons.dog_state import (
    DogState, DogCognitionState, DogMetabolismState, DogSensoryState, DogMemoryState
)
from cynic.cognition.cortex.dog_cognition import DogCognition, DogCognitionConfig
from cynic.cognition.cortex.entropy_tracker import EntropyTracker
from cynic.core.judgment import Cell


class TestDogCognitionRealSignals:
    """Test DogCognition with REAL signals proving entropy > 0."""

    @pytest.mark.asyncio
    async def test_dog_creates_knowledge_with_real_signals(self):
        """Prove: Real signal analysis creates efficiency > 0."""
        dog_cognition = DogCognition("ANALYST", DogCognitionConfig())
        entropy_tracker = EntropyTracker()

        # Create dog state with REAL signals (not empty)
        dog_state = DogState(
            dog_id="ANALYST",
            cognition=DogCognitionState(
                local_qtable={},
                confidence_history=[],
                last_verdict="",
                last_q_score=50.0,
            ),
            metabolism=DogMetabolismState(pending_actions=[]),
            senses=DogSensoryState(
                observed_signals=[
                    {"type": "security_issue", "severity": "high"},
                    {"type": "security_issue", "severity": "medium"},
                    {"type": "performance_gap", "magnitude": "10%"},
                    {"type": "documentation", "missing": True},
                ],
                compressed_context="",
            ),
            memory=DogMemoryState(),
        )

        # Cell to judge
        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            content="# code with issues",
        )

        # Judge the cell
        judgment = await dog_cognition.judge_cell(cell, dog_state)

        # Track entropy
        metrics = await entropy_tracker.track_judgment(
            dog_id="ANALYST",
            cell_id="test_cell",
            signals=dog_state.senses.observed_signals,
            verdict=judgment.verdict,
            confidence=judgment.confidence,
        )

        # PROOF: Entropy > 0 with real signals
        # H(input) = entropy of 4 signals (2 security, 1 perf, 1 doc)
        # H(output) = confidence entropy (security concerns → lower score → lower confidence)
        # efficiency = H(input) - H(output) > 0

        assert metrics.h_input > 1.0, f"Expected diverse signals (H > 1), got {metrics.h_input}"
        assert metrics.efficiency > 0.0, \
            f"Paradigm shift FAILED: efficiency={metrics.efficiency} should be > 0"
        assert judgment.q_score < 40.0, \
            f"Security issues should lower score, got {judgment.q_score}"

        print(f"\n✅ PARADIGM SHIFT VALIDATED:")
        print(f"   H(input)={metrics.h_input:.2f} (diverse signals)")
        print(f"   H(output)={metrics.h_output:.3f} (confident judgment)")
        print(f"   efficiency={metrics.efficiency:.3f} > 0 ✓ (creating knowledge)")
        print(f"   q_score={judgment.q_score:.1f} (security concerns)")
        print(f"   confidence={judgment.confidence:.3f} (honest, not based on count)")

    @pytest.mark.asyncio
    async def test_dog_specialization_security_vs_style(self):
        """Prove: Dogs analyze by signal TYPE, not COUNT."""
        dog_cognition = DogCognition("GUARDIAN", DogCognitionConfig())

        dog_state = DogState(
            dog_id="GUARDIAN",
            cognition=DogCognitionState(
                local_qtable={},
                confidence_history=[],
                last_verdict="",
                last_q_score=50.0,
            ),
            metabolism=DogMetabolismState(pending_actions=[]),
            senses=DogSensoryState(
                observed_signals=[],  # Empty initially
                compressed_context="",
            ),
            memory=DogMemoryState(),
        )

        cell = Cell(reality="CODE", analysis="JUDGE", content="x")

        # Scenario 1: Security issues (critical)
        dog_state.senses.observed_signals = [
            {"type": "security_issue"},
            {"type": "security_issue"},
            {"type": "security_issue"},
        ]
        judgment_security = await dog_cognition.judge_cell(cell, dog_state)

        # Scenario 2: Style violations (minor)
        dog_state.senses.observed_signals = [
            {"type": "style_violation"},
            {"type": "style_violation"},
            {"type": "style_violation"},
        ]
        judgment_style = await dog_cognition.judge_cell(cell, dog_state)

        # PROOF: Type matters, not count
        assert judgment_security.q_score < judgment_style.q_score, \
            f"Security (Q={judgment_security.q_score}) should rank worse than style (Q={judgment_style.q_score})"

        print(f"\n✅ SIGNAL TYPE ANALYSIS:")
        print(f"   3 security issues → Q={judgment_security.q_score:.1f} (critical)")
        print(f"   3 style violations → Q={judgment_style.q_score:.1f} (minor)")
        print(f"   Type matters more than count ✓")

    @pytest.mark.asyncio
    async def test_confidence_based_on_clarity_not_frequency(self):
        """Prove: Confidence is about signal clarity, not judgment count."""
        dog_cognition = DogCognition("ANALYST", DogCognitionConfig())

        dog_state_clear = DogState(
            dog_id="ANALYST",
            cognition=DogCognitionState(
                local_qtable={},
                confidence_history=[],
                last_verdict="",
                last_q_score=50.0,
            ),
            metabolism=DogMetabolismState(pending_actions=[]),
            senses=DogSensoryState(
                # CLEAR signals: diverse types
                observed_signals=[
                    {"type": "security_issue"},
                    {"type": "performance_gap"},
                    {"type": "documentation"},
                ],
                compressed_context="",
            ),
            memory=DogMemoryState(),
        )

        dog_state_unclear = DogState(
            dog_id="ANALYST",
            cognition=DogCognitionState(
                local_qtable={},
                confidence_history=[],
                last_verdict="",
                last_q_score=50.0,
            ),
            metabolism=DogMetabolismState(pending_actions=[]),
            senses=DogSensoryState(
                # UNCLEAR signals: same type repeated
                observed_signals=[
                    {"type": "unknown"},
                    {"type": "unknown"},
                    {"type": "unknown"},
                ],
                compressed_context="",
            ),
            memory=DogMemoryState(),
        )

        cell = Cell(reality="CODE", analysis="JUDGE", content="y")

        # Both have same judgment count (3 signals)
        judgment_clear = await dog_cognition.judge_cell(cell, dog_state_clear)
        judgment_unclear = await dog_cognition.judge_cell(cell, dog_state_unclear)

        # But clarity (diversity) should differ
        assert judgment_clear.confidence > judgment_unclear.confidence, \
            f"Clear signals (Q={judgment_clear.confidence:.3f}) should have higher confidence than unclear (Q={judgment_unclear.confidence:.3f})"

        print(f"\n✅ CLARITY OVER COUNT:")
        print(f"   Diverse signals → confidence={judgment_clear.confidence:.3f}")
        print(f"   Same type repeated → confidence={judgment_unclear.confidence:.3f}")
        print(f"   Clarity wins ✓")
