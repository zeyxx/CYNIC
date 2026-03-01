"""PHASE 1: Direct actions â€” ACT_REQUESTED â†’ UniversalActuator execution."""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBusError
from cynic.kernel.organism.reflexes.base import HandlerGroup
from cynic.kernel.organism.reflexes.services import MetabolicServices

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.learning.qlearning import QTable
    from cynic.kernel.organism.metabolism.actuator import UniversalActuator

logger = logging.getLogger("cynic.kernel.organism.reflexes.direct")


class DirectActionsHandler(HandlerGroup):
    """ACT_REQUESTED â†’ UniversalActuator.dispatch() â€” real execution, real feedback."""

    _EXECUTION_WINDOW = 13  # F(7) â€” rolling success tracking

    def __init__(
        self,
        metabolism: MetabolicServices,
        *,
        universal_actuator: UniversalActuator,
        qtable: QTable,
        bus: Optional[EventBus] = None,
    ) -> None:
        super().__init__(bus=bus)
        self._metabolism = metabolism
        self._universal_actuator = universal_actuator

        self._qtable = qtable
        self._execution_window: list[bool] = []

    @property
    def name(self) -> str:
        return "direct"

    def dependencies(self) -> frozenset[str]:
        return frozenset(
            {
                "escore_tracker",
                "universal_actuator",
                "qtable",
            }
        )

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.ACT_REQUESTED, self._on_act_requested),
        ]

    async def _on_act_requested(self, event: Event) -> None:
        """ACT_REQUESTED â†’ UniversalActuator.dispatch() â†’ real execution."""
        try:
            p = event.dict_payload or {}
            action_type = p.get("action_type", "unknown")
            payload = p.get("payload", {})

            # Execute via UniversalActuator
            result = await self._universal_actuator.dispatch(action_type, payload)

            # Track execution outcome
            success = result.success
            self._execution_window.append(success)
            if len(self._execution_window) > self._EXECUTION_WINDOW:
                self._execution_window.pop(0)

            success_rate = sum(self._execution_window) / len(self._execution_window)

            # Update QTable with execution outcome
            reward = success_rate if success else (success_rate * 0.5)
            self._qtable.update(reward=reward)

            logger.info(
                "ACT_REQUESTED dispatched: type=%s success=%s duration=%.0fms "
                "success_rate=%.2f reward=%.3f",
                action_type,
                success,
                result.duration_ms,
                success_rate,
                reward,
            )

        except EventBusError:
            logger.debug("handler error", exc_info=True)
