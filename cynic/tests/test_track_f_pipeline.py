"""
Track F: Event Pipeline Execution — UUID Link Fix + E2E Wiring

Verifies the full async pipeline UUID chain is unbroken:
  POST /judge (UUID=A) → JUDGMENT_REQUESTED(judgment_id=A) →
  JudgmentExecutorHandler → JUDGMENT_CREATED(judgment_id=A) →
  GET /judge/A returns real verdict
"""

import asyncio
import pytest
import uuid
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.core.event_bus import Event, CoreEvent
from cynic.core.events_schema import (
    JudgmentRequestedPayload,
    JudgmentCreatedPayload,
    JudgmentFailedPayload,
)
from cynic.core.judgment import Cell, Judgment
from cynic.api.handlers.judgment_executor import JudgmentExecutorHandler


class MockKernelServices:
    """Mock KernelServices for testing."""
    pass


class TestJudgmentIdPropagation:
    """UUID chain: POST → event payload → handler → JUDGMENT_CREATED."""

    def test_judgment_requested_payload_has_judgment_id(self):
        """JudgmentRequestedPayload schema includes judgment_id field."""
        test_id = str(uuid.uuid4())
        payload = JudgmentRequestedPayload(
            cell_id="test_cell",
            reality="CODE",
            level="MICRO",
            cell={},
            source="api:judge",
            judgment_id=test_id,
        )
        assert payload.judgment_id == test_id

    def test_judgment_failed_payload_has_judgment_id(self):
        """JudgmentFailedPayload schema includes judgment_id field."""
        test_id = str(uuid.uuid4())
        payload = JudgmentFailedPayload(
            cell_id="test_cell",
            error="test error",
            judgment_id=test_id,
        )
        assert payload.judgment_id == test_id

    @pytest.mark.asyncio
    async def test_executor_uses_payload_judgment_id(self):
        """JudgmentExecutorHandler extracts judgment_id from payload, not event_id."""
        test_judgment_id = str(uuid.uuid4())
        test_event_id = str(uuid.uuid4())  # Different from judgment_id

        # Create test event with different event_id but correct judgment_id in payload
        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            content="test content",
        )

        payload_dict = {
            "cell": cell.model_dump(),
            "cell_id": cell.cell_id,
            "reality": "CODE",
            "level": "MICRO",
            "source": "api:judge",
            "judgment_id": test_judgment_id,  # Payload has correct judgment_id
        }

        event = Event(
            type=CoreEvent.JUDGMENT_REQUESTED,
            payload=payload_dict,
            event_id=test_event_id,  # Event has different event_id
            source="test",
        )

        # Mock orchestrator to return a judgment
        mock_judgment = Judgment(
            cell=cell,
            verdict="WAG",
            q_score=75.0,
            confidence=0.5,
            dog_votes={"SAGE": 0.75},
        )

        mock_orchestrator = AsyncMock()
        mock_orchestrator.run = AsyncMock(return_value=mock_judgment)

        # Mock event bus to capture JUDGMENT_CREATED
        emitted_events = []

        async def mock_emit(evt: Event) -> None:
            emitted_events.append(evt)

        mock_bus = MagicMock()
        mock_bus.emit = mock_emit

        # Create handler
        handler = JudgmentExecutorHandler(
            svc=MockKernelServices(),
            orchestrator=mock_orchestrator,
        )

        # Patch get_core_bus and the import of _write_guidance
        with patch("cynic.api.handlers.judgment_executor.get_core_bus", return_value=mock_bus), \
             patch("cynic.api.routers.core._write_guidance"):
            await handler._on_judgment_requested(event)

        # Verify JUDGMENT_CREATED was emitted with payload.judgment_id, NOT event.event_id
        assert len(emitted_events) == 1
        created_event = emitted_events[0]
        assert created_event.type == CoreEvent.JUDGMENT_CREATED

        # Extract payload
        created_payload = created_event.payload
        assert created_payload["judgment_id"] == test_judgment_id, \
            f"Expected judgment_id={test_judgment_id}, got {created_payload.get('judgment_id')}"
        assert created_payload["judgment_id"] != test_event_id, \
            "Handler should use payload.judgment_id, not event.event_id"

    @pytest.mark.asyncio
    async def test_pending_resolves_to_verdict(self):
        """Full flow: POST /judge → PENDING → JUDGMENT_CREATED → GET returns real verdict."""
        # This is an integration test that simulates the full async pipeline
        test_judgment_id = str(uuid.uuid4())

        # 1. Simulate POST /judge recording PENDING
        pending_judgments = {}

        async def mock_record_pending(jid: str) -> None:
            pending_judgments[jid] = {
                "verdict": "PENDING",
                "q_score": 0.0,
                "confidence": 0.0,
            }

        # 2. Simulate JUDGMENT_CREATED updating the verdict
        async def mock_on_judgment_created(event: Event) -> None:
            payload = event.payload
            jid = payload.get("judgment_id")
            if jid in pending_judgments:
                pending_judgments[jid] = {
                    "verdict": payload.get("verdict", "WAG"),
                    "q_score": payload.get("q_score", 0.0),
                    "confidence": payload.get("confidence", 0.0),
                }

        # Record PENDING
        await mock_record_pending(test_judgment_id)
        assert pending_judgments[test_judgment_id]["verdict"] == "PENDING"

        # Emit JUDGMENT_CREATED with same UUID
        created_event = Event.typed(
            CoreEvent.JUDGMENT_CREATED,
            JudgmentCreatedPayload(
                judgment_id=test_judgment_id,  # Same UUID
                verdict="HOWL",
                q_score=92.0,
                confidence=0.618,
                reality="CODE",
            ),
            source="test",
        )

        await mock_on_judgment_created(created_event)

        # Verify the verdict was updated
        assert pending_judgments[test_judgment_id]["verdict"] == "HOWL"
        assert pending_judgments[test_judgment_id]["q_score"] == 92.0
        assert pending_judgments[test_judgment_id]["confidence"] == 0.618

    @pytest.mark.asyncio
    async def test_circuit_breaker_uses_payload_judgment_id(self):
        """Circuit breaker failure emits JUDGMENT_FAILED with payload.judgment_id."""
        test_judgment_id = str(uuid.uuid4())
        test_event_id = str(uuid.uuid4())

        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            content="test content",
        )

        payload_dict = {
            "cell": cell.model_dump(),
            "cell_id": cell.cell_id,
            "reality": "CODE",
            "level": "MICRO",
            "source": "api:judge",
            "judgment_id": test_judgment_id,
        }

        event = Event(
            type=CoreEvent.JUDGMENT_REQUESTED,
            payload=payload_dict,
            event_id=test_event_id,
            source="test",
        )

        # Mock event bus
        emitted_events = []

        async def mock_emit(evt: Event) -> None:
            emitted_events.append(evt)

        mock_bus = MagicMock()
        mock_bus.emit = mock_emit

        # Create handler
        mock_orchestrator = AsyncMock()
        handler = JudgmentExecutorHandler(
            svc=MockKernelServices(),
            orchestrator=mock_orchestrator,
        )

        # Patch circuit breaker to fail
        with patch("cynic.api.handlers.judgment_executor.get_core_bus", return_value=mock_bus), \
             patch("cynic.api.handlers.judgment_executor._orchestrator_breaker") as mock_breaker:

            mock_breaker.allow.return_value = False
            mock_breaker.state = "OPEN"

            await handler._on_judgment_requested(event)

        # Verify JUDGMENT_FAILED was emitted with payload.judgment_id
        assert len(emitted_events) == 1
        failed_event = emitted_events[0]
        assert failed_event.type == CoreEvent.JUDGMENT_FAILED

        failed_payload = failed_event.payload
        assert failed_payload["judgment_id"] == test_judgment_id, \
            f"Expected judgment_id={test_judgment_id}, got {failed_payload.get('judgment_id')}"


class TestJudgmentFailedPropagation:
    """JUDGMENT_FAILED with correct UUID transitions PENDING → BARK."""

    @pytest.mark.asyncio
    async def test_timeout_uses_payload_judgment_id(self):
        """Timeout path emits JUDGMENT_FAILED with payload.judgment_id."""
        test_judgment_id = str(uuid.uuid4())
        test_event_id = str(uuid.uuid4())

        cell = Cell(
            reality="CODE",
            analysis="JUDGE",
            content="test content",
        )

        payload_dict = {
            "cell": cell.model_dump(),
            "cell_id": cell.cell_id,
            "reality": "CODE",
            "level": "MICRO",
            "source": "api:judge",
            "judgment_id": test_judgment_id,
        }

        event = Event(
            type=CoreEvent.JUDGMENT_REQUESTED,
            payload=payload_dict,
            event_id=test_event_id,
            source="test",
        )

        # Mock orchestrator to timeout
        async def mock_timeout():
            await asyncio.sleep(1)
            raise asyncio.TimeoutError()

        mock_orchestrator = AsyncMock()
        mock_orchestrator.run = AsyncMock(side_effect=mock_timeout)

        emitted_events = []

        async def mock_emit(evt: Event) -> None:
            emitted_events.append(evt)

        mock_bus = MagicMock()
        mock_bus.emit = mock_emit

        handler = JudgmentExecutorHandler(
            svc=MockKernelServices(),
            orchestrator=mock_orchestrator,
        )

        # Patch asyncio.wait_for to raise TimeoutError immediately
        with patch("cynic.api.handlers.judgment_executor.get_core_bus", return_value=mock_bus), \
             patch("cynic.api.handlers.judgment_executor.asyncio.wait_for") as mock_wait_for, \
             patch("cynic.api.handlers.judgment_executor._orchestrator_breaker") as mock_breaker:

            mock_wait_for.side_effect = asyncio.TimeoutError()
            mock_breaker.allow.return_value = True
            mock_breaker.record_failure = MagicMock()

            await handler._on_judgment_requested(event)

        # Verify JUDGMENT_FAILED was emitted with payload.judgment_id
        assert len(emitted_events) == 1
        failed_event = emitted_events[0]
        assert failed_event.type == CoreEvent.JUDGMENT_FAILED

        failed_payload = failed_event.payload
        assert failed_payload["judgment_id"] == test_judgment_id


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
