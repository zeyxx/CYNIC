"""
Perception Handler — Bridges Sensory Input to Cognitive Judgment.

This reflex listens for PERCEPTION_RECEIVED events from the Somatic Gateway,
translates them into Cells, and requests a Judgment cycle.

Lentilles: AI Infra, Backend, Robotics.
"""

from __future__ import annotations

import logging
import uuid
from typing import Optional

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBus
from cynic.kernel.core.events_schema import JudgmentRequestedPayload
from cynic.kernel.core.judgment import Cell
from cynic.kernel.organism.reflexes.base import HandlerGroup
from cynic.kernel.organism.reflexes.services import SensoryServices

logger = logging.getLogger(__name__)

class PerceptionHandler(HandlerGroup):
    """
    Reflex that reacts to new sensory data.
    """

    def __init__(self, sensory: SensoryServices, bus: Optional[EventBus] = None):
        super().__init__(bus=bus)
        self._sensory = sensory

    @property
    def name(self) -> str:
        return "perception_handler"

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.PERCEPTION_RECEIVED, self._on_perception),
        ]

    async def _on_perception(self, event: Event) -> None:
        """
        React to new perception.
        """
        try:
            data = event.dict_payload
            reality = data.get("reality", "INTERNAL")
            
            logger.info(f"PerceptionHandler: Reacting to {reality} from {event.source}")
            
            # 1. Create a Cell from perception
            cell = Cell(
                cell_id=str(uuid.uuid4()),
                reality=reality,
                analysis="PERCEPTION",
                content=data.get("data", {}),
                budget_usd=0.01
            )
            
            # 2. Request Judgment
            await self.bus.emit(Event.typed(
                CoreEvent.JUDGMENT_REQUESTED,
                JudgmentRequestedPayload(
                    judgment_id=str(uuid.uuid4()),
                    cell=cell.model_dump(),
                    level="AUTO", # Let the selector decide
                    reality=reality
                ),
                source="perception_handler"
            ))
            
        except Exception as e:
            logger.error(f"PerceptionHandler crash: {e}", exc_info=True)
