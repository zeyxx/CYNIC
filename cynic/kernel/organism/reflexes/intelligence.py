"""
PHASE 3: Intelligence cycle handlers â€" LOD assessment, error tracking, budget response.

This group coordinates the high-level cognitive state of the organism.
It bridges perception to judgment and manages the Level of Detail (LOD).
"""
from __future__ import annotations

import logging
import uuid
from typing import TYPE_CHECKING, Any, Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.judgment import Cell
from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.organism.reflexes.base import HandlerGroup
from cynic.kernel.organism.reflexes.services import CognitionServices
from cynic.kernel.core.events_schema import (
    PerceptionReceivedPayload, 
    JudgmentRequestedPayload,
    JudgmentCreatedPayload
)

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.cognition.cortex.orchestrator import JudgeOrchestrator
    from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm

logger = logging.getLogger("cynic.kernel.organism.reflexes.intelligence")


class IntelligenceHandlers(HandlerGroup):
    """LOD assessment, error tracking, budget response, compressor feeding."""

    _OUTCOME_WINDOW = 21  # F(8)

    def __init__(
        self,
        cognition: CognitionServices,
        *,
        orchestrator: JudgeOrchestrator,
        scheduler: ConsciousnessRhythm,
        db_pool: Any | None,
        compressor,  # ContextCompressor
        escore_tracker: Any | None = None,
        axiom_monitor: Any | None = None,
        bus: Optional[EventBus] = None,
    ) -> None:
        super().__init__(bus=bus)
        self._cognition = cognition
        self._orchestrator = orchestrator
        self._scheduler = scheduler
        self._db_pool = db_pool
        self._compressor = compressor
        self._escore_tracker = escore_tracker
        self._axiom_monitor = axiom_monitor

        # Group-local mutable state
        self._outcome_window: list[bool] = []
        self._checkpoint_counter = 0

    @property
    def name(self) -> str:
        return "intelligence"

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.PERCEPTION_RECEIVED, self._on_perception_received),
            (CoreEvent.JUDGMENT_CREATED, self._on_judgment_created),
        ]

    # â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    # HANDLER IMPLEMENTATIONS
    # â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    async def _on_perception_received(self, event: Event) -> None:
        """PERCEPTION_RECEIVED â' Trigger Full Judgment if run_judgment is True."""
        try:
            p = PerceptionReceivedPayload.model_validate(event.dict_payload or {})
            
            if p.run_judgment:
                # Transform into a judgment request
                content = p.data if p.data else ""
                
                # IMPORTANT: Use real ConsciousnessLevel and integer LOD
                level = ConsciousnessLevel.MICRO
                
                cell = Cell(
                    reality=p.reality,
                    analysis="JUDGE",
                    content=str(content),
                    context=p.context or f"Automated analysis of {p.reality} from {p.source}",
                    lod=1 # Numeric LOD is fine here
                )
                
                await self.bus.emit(Event.typed(
                    CoreEvent.JUDGMENT_REQUESTED,
                    JudgmentRequestedPayload(
                        cell_id=cell.cell_id,
                        reality=cell.reality,
                        level=level.name, # Pass the NAME of the level
                        cell=cell.model_dump(),
                        source=f"intelligence:bridge:{p.source}",
                        judgment_id=p.judgment_id or str(uuid.uuid4())
                    ),
                    source="intelligence"
                ))
                logger.info("Intelligence: Bridged perception to judgment for %s", p.source)

        except Exception as e:
            logger.error("Intelligence: Failed to bridge perception: %s", e)

    async def _on_judgment_created(self, event: Event) -> None:
        """Feed judgments to the context compressor and update health."""
        try:
            p = JudgmentCreatedPayload.model_validate(event.dict_payload or {})
            
            # 1. Update health cache for LOD controller
            self._cognition.update_health_cache(last_q_score=p.q_score)
            
            # 2. Feed compressor
            if self._compressor:
                try:
                    self._compressor.add_judgment(p.model_dump())
                except Exception as e:
                    logger.debug("Intelligence: Compressor feed failed: %s", e)

        except Exception as e:
            logger.error("Intelligence: Judgment processing failed: %s", e)
