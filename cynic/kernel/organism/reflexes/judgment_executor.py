"""
Phase 3: Judgment Executor Handler

Listens to JUDGMENT_REQUESTED events and executes the full judgment cycle.
This handler bridges the event-driven API layer with the orchestrator.

Pattern:
  1. POST /judge â’ emit JUDGMENT_REQUESTED (return immediately)
  2. JudgmentExecutorHandler listens â’ orchestrator.run(cell, level)
  3. orchestrator returns Judgment â’ emit JUDGMENT_CREATED
  4. GET /judge/{id} â’ query ConsciousState (has result from JUDGMENT_CREATED)
"""

from __future__ import annotations

import asyncio
import logging
import uuid
from typing import Any, Optional

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.events_schema import JudgmentFailedPayload
from cynic.kernel.core.exceptions import CynicError
from cynic.kernel.core.judgment import Cell
from cynic.kernel.organism.brain.cognition.cortex.circuit_breaker import CircuitBreaker
from cynic.kernel.organism.brain.cognition.cortex.orchestrator import JudgeOrchestrator
from cynic.kernel.organism.reflexes.base import HandlerGroup
from cynic.kernel.organism.reflexes.services import CognitionServices

logger = logging.getLogger(__name__)

# Circuit breaker for orchestrator health (prevents cascade failures)
_orchestrator_breaker = CircuitBreaker()


class JudgmentExecutorHandler(HandlerGroup):
    """
    Execute judgments triggered by JUDGMENT_REQUESTED events.

    This handler runs the actual orchestrator.run() when an event-driven
    judgment request arrives from the API layer. It emits JUDGMENT_CREATED
    with the result, which ConsciousState picks up for query endpoints.

    Dependencies:
    - orchestrator: runs the judgment pipeline
    - (implicit) event bus: to emit JUDGMENT_CREATED
    """

    def __init__(
        self,
        cognition: CognitionServices,
        *,
        orchestrator: JudgeOrchestrator,
        escore_tracker: Any | None = None,
        axiom_monitor: Any | None = None,
        bus: Optional[EventBus] = None,
    ) -> None:
        super().__init__(bus=bus)
        self._cognition = cognition
        self._orchestrator = orchestrator
        self._escore_tracker = escore_tracker
        self._axiom_monitor = axiom_monitor

    @property
    def name(self) -> str:
        return "judgment_executor"

    def dependencies(self) -> frozenset[str]:
        return frozenset({"orchestrator"})

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        """Listen for JUDGMENT_REQUESTED events from API endpoints."""
        return [
            (CoreEvent.JUDGMENT_REQUESTED, self._on_judgment_requested),
        ]

    async def _on_judgment_requested(self, event: Event) -> None:
        """
        Execute judgment when JUDGMENT_REQUESTED event fires.
        """
        try:
            logger.debug(
                "[HANDLER] JudgmentExecutor received JUDGMENT_REQUESTED: %s", event.event_id
            )
            from cynic.kernel.core.events_schema import JudgmentRequestedPayload

            p = JudgmentRequestedPayload.model_validate(event.dict_payload or {})

            # ANTI-RECURSION GUARD (Lentille : SRE)
            # If the organism is recursively judging its own internal thoughts too deeply, stop.
            if p.fractal_depth > 2:
                logger.warning(f"JudgmentExecutor: RECURSION BLOCKED (depth={p.fractal_depth}) for cell {p.cell.get('cell_id') if isinstance(p.cell, dict) else 'unknown'}")
                return

            # Extract judgment_id from payload (thread it through the pipeline to match PENDING entry)
            judgment_id = p.judgment_id or event.event_id  # fallback for safety

            # Reconstruct Cell from model data
            cell_dict = p.cell
            cell = None

            if cell_dict:
                try:
                    cell = Cell(**cell_dict)
                    logger.debug("[HANDLER] Cell reconstructed from model data: %s", cell.cell_id)
                except Exception as cell_err:
                    logger.warning("[HANDLER] Failed to reconstruct cell: %s", cell_err)

            # Fallback for old payloads or errors
            if cell is None:
                cell = Cell(
                    cell_id=p.cell_id or str(uuid.uuid4()),
                    reality=p.reality or "CODE",
                    analysis="JUDGE",
                    content=f"Event: {event.event_id}",
                    budget_usd=0.01,
                )

            # Parse level
            level_str = p.level
            level = None
            if level_str and level_str != "AUTO":
                try:
                    level = ConsciousnessLevel[level_str]
                except KeyError:
                    level = None

            logger.info(
                "JudgmentExecutor: Processing %s judgment (level=%s, fractal_depth=%d)",
                cell.reality,
                level_str or "AUTO",
                p.fractal_depth,
            )

            # Check circuit breaker
            if not _orchestrator_breaker.allow():
                await self._emit_judgment_failed(
                    judgment_id, cell.cell_id, "circuit_breaker_open", "Circuit breaker OPEN"
                )
                return

            # Run the orchestrator with timeout (30s max)
            try:
                judgment = await asyncio.wait_for(
                    self._orchestrator.run(
                        cell=cell,
                        level=level,
                        budget_usd=cell.budget_usd,
                        fractal_depth=p.fractal_depth,
                    ),
                    timeout=30.0,
                )
                logger.info(
                    "JudgmentExecutor: Judgment complete (verdict=%s, Q=%.1f)",
                    judgment.verdict,
                    judgment.q_score,
                )
                # Record success for circuit breaker
                _orchestrator_breaker.record_success()

            except asyncio.TimeoutError:
                logger.error(
                    "JudgmentExecutor: Timeout on %s (exceeded 30s)",
                    cell.cell_id,
                )
                _orchestrator_breaker.record_failure()
                await self._emit_judgment_failed(
                    judgment_id=judgment_id,
                    cell_id=cell.cell_id,
                    reason="orchestrator_timeout",
                    error_message="Judgment execution exceeded 30s timeout",
                )
                return

        except CynicError as e:
            logger.error(
                "JudgmentExecutor unexpected error on %s: %s",
                event.event_id,
                e,
                exc_info=True,
            )
            _orchestrator_breaker.record_failure()
            await self._emit_judgment_failed(
                judgment_id=judgment_id,
                cell_id=event.dict_payload.get("cell_id", ""),
                reason="exception",
                error_message=f"{type(e).__name__}: {e}",
            )

    async def _emit_judgment_failed(
        self,
        judgment_id: str,
        cell_id: str,
        reason: str,
        error_message: str,
    ) -> None:
        """Emit JUDGMENT_FAILED event and update ConsciousState."""
        try:
            await self.bus.emit(
                Event.typed(
                    CoreEvent.JUDGMENT_FAILED,
                    JudgmentFailedPayload(
                        judgment_id=judgment_id,
                        cell_id=cell_id,
                        error=error_message,
                        circuit_state="",
                        failure_count=0,
                    ),
                    source="judgment_executor",
                )
            )
            logger.debug(
                "Emitted JUDGMENT_FAILED for %s (reason=%s)",
                cell_id,
                reason,
            )

            # Also update ConsciousState to reflect BARK (failure verdict)
            try:
                from cynic.kernel.organism.conscious_state import get_conscious_state

                await get_conscious_state().record_judgment_failed(judgment_id, reason)
            except Exception as e:
                logger.debug("Could not record failure in ConsciousState: %s", e)

        except Exception as e:
            logger.error("Failed to emit JUDGMENT_FAILED: %s", e, exc_info=True)
