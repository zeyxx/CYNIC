"""Comprehensive tests for LNSP Layer 4: Action Execution and Feedback Loop."""
from __future__ import annotations

import pytest

from cynic.protocol.lnsp.layer4 import Handler, Layer4
from cynic.protocol.lnsp.messages import create_judgment
from cynic.protocol.lnsp.types import (
    JudgmentType,
    LNSPMessage,
    ObservationType,
    VerdictType,
)

# ============================================================================
# Mock Handler for Testing
# ============================================================================


class MockHandler(Handler):
    """Mock handler for testing."""

    def __init__(self, handler_id: str, should_succeed: bool = True):
        """Initialize mock handler.

        Args:
            handler_id: Unique identifier for this handler
            should_succeed: Whether to simulate success (default True)
        """
        super().__init__(handler_id)
        self.should_succeed = should_succeed
        self.last_verdict: LNSPMessage | None = None
        self.call_count = 0

    async def handle(self, verdict: LNSPMessage) -> tuple[bool, dict]:
        """Execute a mock action.

        Args:
            verdict: Verdict message to handle

        Returns:
            Tuple of (success, result_data)
        """
        self.last_verdict = verdict
        self.call_count += 1
        return (
            self.should_succeed,
            {
                "action_executed": True,
                "target": verdict.header.target,
                "verdict_id": verdict.header.message_id,
            },
        )


# ============================================================================
# Handler Interface Tests
# ============================================================================


class TestHandlerInterface:
    """Test Handler abstract base class."""

    @pytest.mark.asyncio
    async def test_handler_creation(self):
        """Test creating a mock handler."""
        handler = MockHandler("test:handler")
        assert handler.handler_id == "test:handler"
        assert handler.call_count == 0
        assert handler.last_verdict is None

    @pytest.mark.asyncio
    async def test_handler_execution_returns_tuple(self):
        """Test handler execution returns (bool, dict)."""
        handler = MockHandler("test:handler")
        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="test:handler",
            based_on=[],
        )

        success, result = await handler.handle(verdict)

        assert isinstance(success, bool)
        assert isinstance(result, dict)
        assert success is True
        assert result["action_executed"] is True

    @pytest.mark.asyncio
    async def test_handler_id_storage(self):
        """Test handler stores its ID correctly."""
        handler_id = "blockchain:executor"
        handler = MockHandler(handler_id)
        assert handler.handler_id == handler_id


# ============================================================================
# Layer 4 Registration Tests
# ============================================================================


class TestLayer4Registration:
    """Test Layer 4 handler registration."""

    def test_register_single_handler(self):
        """Test registering a single handler."""
        layer4 = Layer4()
        handler = MockHandler("test:handler")

        layer4.register_handler(handler)

        assert "test:handler" in layer4.handlers
        assert layer4.handlers["test:handler"] is handler

    def test_register_multiple_handlers(self):
        """Test registering multiple handlers."""
        layer4 = Layer4()
        handler1 = MockHandler("system:config")
        handler2 = MockHandler("blockchain:call")
        handler3 = MockHandler("human:signal")

        layer4.register_handler(handler1)
        layer4.register_handler(handler2)
        layer4.register_handler(handler3)

        assert len(layer4.handlers) == 3
        assert layer4.handlers["system:config"] is handler1
        assert layer4.handlers["blockchain:call"] is handler2
        assert layer4.handlers["human:signal"] is handler3


# ============================================================================
# Layer 4 Execution Tests
# ============================================================================


class TestLayer4Execution:
    """Test Layer 4 verdict execution."""

    @pytest.mark.asyncio
    async def test_execute_verdict_through_handler(self):
        """Test executing a verdict through a handler."""
        layer4 = Layer4()
        handler = MockHandler("test:handler")
        layer4.register_handler(handler)

        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="test:handler",
            based_on=[],
        )

        success, feedback = await layer4.execute(verdict)

        assert success is True
        assert handler.call_count == 1

    @pytest.mark.asyncio
    async def test_handler_receives_correct_verdict(self):
        """Test handler receives the correct verdict."""
        layer4 = Layer4()
        handler = MockHandler("test:handler")
        layer4.register_handler(handler)

        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.GROWL,
            q_score=0.55,
            confidence=0.618,
            axiom_scores={"TEST": 0.6},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="test:handler",
            based_on=[],
            instance_id="test:instance",
        )

        await layer4.execute(verdict)

        assert handler.last_verdict is not None
        assert handler.last_verdict.header.message_id == verdict.header.message_id
        assert handler.last_verdict.header.source == "TEST_JUDGE"

    @pytest.mark.asyncio
    async def test_handler_return_value_captured(self):
        """Test handler return value is captured in feedback."""
        layer4 = Layer4()
        handler = MockHandler("test:handler")
        layer4.register_handler(handler)

        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="test:handler",
            based_on=[],
        )

        success, feedback = await layer4.execute(verdict)

        assert success is True
        assert feedback is not None
        assert "result" in feedback.payload
        assert feedback.payload["result"]["action_executed"] is True

    @pytest.mark.asyncio
    async def test_handler_failure_handling(self):
        """Test handling of failed handlers."""
        layer4 = Layer4()
        handler = MockHandler("test:handler", should_succeed=False)
        layer4.register_handler(handler)

        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="test:handler",
            based_on=[],
        )

        success, feedback = await layer4.execute(verdict)

        assert success is False
        assert feedback is not None
        assert feedback.payload["success"] is False

    @pytest.mark.asyncio
    async def test_non_layer3_messages_ignored(self):
        """Test that non-Layer 3 messages are ignored."""
        from cynic.protocol.lnsp.messages import create_raw_observation
        from cynic.protocol.lnsp.types import ObservationType

        layer4 = Layer4()
        handler = MockHandler("test:handler")
        layer4.register_handler(handler)

        # Create a Layer 1 observation, not a Layer 3 judgment
        observation = create_raw_observation(
            observation_type=ObservationType.METRIC_SAMPLE,
            data={"cpu": 45.0},
            source="SENSOR",
        )

        success, feedback = await layer4.execute(observation)

        assert success is False
        assert feedback is None
        assert handler.call_count == 0


# ============================================================================
# Layer 4 Feedback Tests
# ============================================================================


class TestLayer4Feedback:
    """Test Layer 4 feedback message generation and emission."""

    @pytest.mark.asyncio
    async def test_feedback_message_generation(self):
        """Test feedback message is generated after execution."""
        layer4 = Layer4()
        handler = MockHandler("test:handler")
        layer4.register_handler(handler)

        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="test:handler",
            based_on=[],
        )

        success, feedback = await layer4.execute(verdict)

        assert success is True
        assert feedback is not None
        assert feedback.header.source == "test:handler"
        assert (
            feedback.payload["observation_type"]
            == ObservationType.ACTION_RESULT.value
        )

    @pytest.mark.asyncio
    async def test_feedback_tagged_with_closes_action_id(self):
        """Test feedback is tagged with closes_action_id."""
        layer4 = Layer4()
        handler = MockHandler("test:handler")
        layer4.register_handler(handler)

        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="test:handler",
            based_on=[],
        )

        success, feedback = await layer4.execute(verdict)

        assert feedback is not None
        assert feedback.metadata.closes_action_id == verdict.header.message_id

    @pytest.mark.asyncio
    async def test_callback_called_with_feedback(self):
        """Test feedback callback is called with feedback message."""
        layer4 = Layer4()
        handler = MockHandler("test:handler")
        layer4.register_handler(handler)

        received_feedbacks: list[LNSPMessage] = []

        def capture_feedback(msg: LNSPMessage) -> None:
            received_feedbacks.append(msg)

        layer4.on_feedback(capture_feedback)

        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="test:handler",
            based_on=[],
        )

        await layer4.execute(verdict)

        assert len(received_feedbacks) == 1
        assert (
            received_feedbacks[0].payload["observation_type"]
            == ObservationType.ACTION_RESULT.value
        )

    @pytest.mark.asyncio
    async def test_multiple_callbacks_notified(self):
        """Test multiple callbacks are all notified of feedback."""
        layer4 = Layer4()
        handler = MockHandler("test:handler")
        layer4.register_handler(handler)

        received_count = [0, 0, 0]

        def callback1(msg: LNSPMessage) -> None:
            received_count[0] += 1

        def callback2(msg: LNSPMessage) -> None:
            received_count[1] += 1

        def callback3(msg: LNSPMessage) -> None:
            received_count[2] += 1

        layer4.on_feedback(callback1)
        layer4.on_feedback(callback2)
        layer4.on_feedback(callback3)

        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="test:handler",
            based_on=[],
        )

        await layer4.execute(verdict)

        assert received_count == [1, 1, 1]


# ============================================================================
# Layer 4 Target Routing Tests
# ============================================================================


class TestLayer4TargetRouting:
    """Test verdict routing to correct handlers by target."""

    @pytest.mark.asyncio
    async def test_verdict_routed_to_correct_handler(self):
        """Test verdict routed to handler with matching target."""
        layer4 = Layer4()
        handler1 = MockHandler("system:config")
        handler2 = MockHandler("blockchain:call")
        layer4.register_handler(handler1)
        layer4.register_handler(handler2)

        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="blockchain:call",
            based_on=[],
        )

        await layer4.execute(verdict)

        # Only blockchain:call handler should have been called
        assert handler2.call_count == 1
        assert handler1.call_count == 0

    @pytest.mark.asyncio
    async def test_multiple_handlers_receive_verdicts_for_their_targets(self):
        """Test multiple handlers receive verdicts for their targets."""
        layer4 = Layer4()
        handler1 = MockHandler("system:config")
        handler2 = MockHandler("blockchain:call")
        handler3 = MockHandler("human:signal")
        layer4.register_handler(handler1)
        layer4.register_handler(handler2)
        layer4.register_handler(handler3)

        verdict1 = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="system:config",
            based_on=[],
        )

        verdict2 = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.GROWL,
            q_score=0.55,
            confidence=0.618,
            axiom_scores={"TEST": 0.6},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="blockchain:call",
            based_on=[],
        )

        verdict3 = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.WAG,
            q_score=0.65,
            confidence=0.618,
            axiom_scores={"TEST": 0.7},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="human:signal",
            based_on=[],
        )

        await layer4.execute(verdict1)
        await layer4.execute(verdict2)
        await layer4.execute(verdict3)

        assert handler1.call_count == 1
        assert handler2.call_count == 1
        assert handler3.call_count == 1


# ============================================================================
# Layer 4 Stats Tests
# ============================================================================


class TestLayer4Stats:
    """Test Layer 4 statistics."""

    def test_stats_returns_correct_counts(self):
        """Test stats returns correct handler and callback counts."""
        layer4 = Layer4()
        handler1 = MockHandler("handler1")
        handler2 = MockHandler("handler2")
        handler3 = MockHandler("handler3")

        layer4.register_handler(handler1)
        layer4.register_handler(handler2)
        layer4.register_handler(handler3)

        def callback1(msg: LNSPMessage) -> None:
            pass

        def callback2(msg: LNSPMessage) -> None:
            pass

        layer4.on_feedback(callback1)
        layer4.on_feedback(callback2)

        stats = layer4.stats()

        assert stats["handler_count"] == 3
        assert stats["feedback_callback_count"] == 2


# ============================================================================
# Integration Tests
# ============================================================================


class TestLayer4Integration:
    """Integration tests for Layer 4."""

    @pytest.mark.asyncio
    async def test_handler_exception_converted_to_failure(self):
        """Test handler exception is converted to failure with error message."""

        class FailingHandler(Handler):
            async def handle(self, verdict: LNSPMessage) -> tuple[bool, dict]:
                raise ValueError("Handler error")

        layer4 = Layer4()
        handler = FailingHandler("failing:handler")
        layer4.register_handler(handler)

        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="failing:handler",
            based_on=[],
        )

        success, feedback = await layer4.execute(verdict)

        assert success is False
        assert feedback is not None
        assert feedback.payload["success"] is False
        assert "error" in feedback.payload["result"]

    @pytest.mark.asyncio
    async def test_no_matching_handler_returns_failure(self):
        """Test execution fails when no handler matches target."""
        layer4 = Layer4()
        handler = MockHandler("system:config")
        layer4.register_handler(handler)

        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="nonexistent:handler",
            based_on=[],
        )

        success, feedback = await layer4.execute(verdict)

        assert success is False
        assert feedback is None

    @pytest.mark.asyncio
    async def test_feedback_preserves_instance_id(self):
        """Test feedback preserves instance_id from verdict."""
        layer4 = Layer4()
        handler = MockHandler("test:handler")
        layer4.register_handler(handler)

        instance_id = "org:special:instance"
        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="test:handler",
            based_on=[],
            instance_id=instance_id,
        )

        success, feedback = await layer4.execute(verdict)

        assert feedback is not None
        assert feedback.metadata.instance_id == instance_id

    @pytest.mark.asyncio
    async def test_feedback_preserves_region(self):
        """Test feedback preserves region from verdict."""
        layer4 = Layer4()
        handler = MockHandler("test:handler")
        layer4.register_handler(handler)

        region = "us-west-2"
        verdict = create_judgment(
            judgment_type=JudgmentType.STATE_EVALUATION,
            verdict=VerdictType.HOWL,
            q_score=0.45,
            confidence=0.618,
            axiom_scores={"TEST": 0.5},
            data={"reason": "test"},
            source="TEST_JUDGE",
            target="test:handler",
            based_on=[],
            region=region,
        )

        success, feedback = await layer4.execute(verdict)

        assert feedback is not None
        assert feedback.metadata.region == region
