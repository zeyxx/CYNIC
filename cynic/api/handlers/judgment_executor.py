"""
Phase 3: Judgment Executor Handler

Listens to JUDGMENT_REQUESTED events and executes the full judgment cycle.
This handler bridges the event-driven API layer with the orchestrator.

Pattern:
  1. POST /judge → emit JUDGMENT_REQUESTED (return immediately)
  2. JudgmentExecutorHandler listens → orchestrator.run(cell, level)
  3. orchestrator returns Judgment → emit JUDGMENT_CREATED
  4. GET /judge/{id} → query ConsciousState (has result from JUDGMENT_CREATED)
"""

from __future__ import annotations

import logging
from typing import Optional

from cynic.api.handlers.base import HandlerGroup
from cynic.api.handlers.services import KernelServices
from cynic.core.event_bus import Event, CoreEvent, get_core_bus
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.exceptions import CynicError
from cynic.core.events_schema import JudgmentCreatedPayload
from cynic.core.judgment import Cell
from cynic.cognition.cortex.orchestrator import JudgeOrchestrator

logger = logging.getLogger(__name__)


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
        svc: KernelServices,
        *,
        orchestrator: JudgeOrchestrator,
    ):
        self._svc = svc
        self._orchestrator = orchestrator

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

        Payload structure:
        {
            "cell": {...}  # dict representation of Cell
            "cell_id": "...",
            "reality": "CODE",
            "level": "MICRO" | "REFLEX" | "MACRO" | "AUTO"
            "budget_usd": 0.5
            "source": "..."
        }
        """
        try:
            logger.debug("[HANDLER] JudgmentExecutor received JUDGMENT_REQUESTED: %s", event.event_id)
            payload = event.payload or {}

            # Try to extract cell from payload (v2 format with full cell data)
            cell_dict = payload.get("cell", {})
            cell = None
            
            if cell_dict:
                # Reconstruct Cell from dict (v2 format)
                try:
                    cell = Cell(**cell_dict)
                    logger.debug("[HANDLER] Cell reconstructed from full data: %s", cell.cell_id)
                except Exception as cell_err:
                    logger.warning(
                        "[HANDLER] Failed to reconstruct cell from full data: %s - %s",
                        cell_err,
                        cell_dict,
                    )
                    cell = None
            
            # Fallback: construct minimal Cell from individual fields
            if cell is None:
                import uuid
                cell_id = payload.get("cell_id", "")
                reality = payload.get("reality", "CODE")
                
                if not cell_id:
                    # Generate a placeholder cell_id if none provided
                    cell_id = str(uuid.uuid4())
                    logger.info(
                        "JUDGMENT_REQUESTED no cell_id: generated %s (source=%s)",
                        cell_id, payload.get("source", "unknown"),
                    )
                
                # Create Cell from available fields (robust fallback)
                try:
                    cell = Cell(
                        cell_id=cell_id,
                        reality=reality,
                        analysis=payload.get("analysis", "JUDGE"),
                        content=payload.get("content", ""),
                        context=payload.get("context", ""),
                        budget_usd=payload.get("budget_usd", 0.01),
                        time_dim=payload.get("time_dim", "PRESENT"),
                        lod=payload.get("lod", 0),
                    )
                    logger.debug("[HANDLER] Cell reconstructed from payload: %s", cell.cell_id)
                except Exception as cell_err:
                    # Last resort: create Cell with only required fields
                    logger.warning(
                        "JUDGMENT_REQUESTED partial cell data: %s (error=%s, using minimal fallback)",
                        event.event_id, cell_err,
                    )
                    cell = Cell(
                        cell_id=cell_id,
                        reality=reality,
                        analysis="JUDGE",
                        content=f"Event: {event.event_id}",
                        context=payload.get("source", "unknown"),
                        budget_usd=0.01,
                    )

            # Parse level (optional, orchestrator will auto-select if None)
            level_str = payload.get("level", "AUTO")
            level = None
            if level_str and level_str != "AUTO":
                try:
                    level = ConsciousnessLevel[level_str]
                except KeyError:
                    logger.warning("Invalid level %s, using auto-select", level_str)
                    level = None

            # Extract budget
            budget_usd = payload.get("budget_usd", cell.budget_usd)

            logger.info(
                "JudgmentExecutor: Processing %s judgment (level=%s, budget=$%.2f)",
                cell.reality,
                level_str,
                budget_usd,
            )

            # Run the orchestrator
            judgment = await self._orchestrator.run(
                cell=cell,
                level=level,
                budget_usd=budget_usd,
            )

            logger.info(
                "JudgmentExecutor: Judgment complete (verdict=%s, Q=%.1f)",
                judgment.verdict,
                judgment.q_score,
            )

            # Emit JUDGMENT_CREATED so ConsciousState picks it up
            judgment_payload = JudgmentCreatedPayload(
                judgment_id=event.event_id,  # Use request event ID as judgment ID
                verdict=judgment.verdict,
                q_score=judgment.q_score,
                confidence=judgment.confidence,
                reality=cell.reality,
                analysis=cell.analysis,
                dog_votes=judgment.dog_votes,
                axiom_scores=judgment.axiom_scores or {},
                consensus_reached=judgment.consensus_reached,
                consensus_votes=len(judgment.dog_votes),
                cost_usd=judgment.cost_usd,
                source="api:judgment_executor",
                state_key=cell.cell_id,
            )

            created_event = Event.typed(
                CoreEvent.JUDGMENT_CREATED,
                judgment_payload,
                source="judgment_executor",
            )

            await get_core_bus().emit(created_event)
            logger.debug("Emitted JUDGMENT_CREATED: %s", created_event.event_id)

        except CynicError as e:
            logger.error(
                "JudgmentExecutor failed on %s: %s",
                event.event_id,
                e,
                exc_info=True,
            )
            # Emit error event so system can track failures
            try:
                await get_core_bus().emit(
                    Event.typed(
                        CoreEvent.JUDGMENT_FAILED,
                        {"judgment_id": event.event_id, "error": str(e)},
                        source="judgment_executor",
                    )
                )
            except CynicError:
                logger.error("Failed to emit JUDGMENT_FAILED event")
