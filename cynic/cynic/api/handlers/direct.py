"""PHASE 1: Direct actions — ACT_REQUESTED → UniversalActuator execution."""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING

from cynic.core.event_bus import Event, CoreEvent

from cynic.api.handlers.base import HandlerGroup, KernelServices

if TYPE_CHECKING:
    from cynic.metabolism.universal import UniversalActuator
    from cynic.learning.qlearning import QTable

logger = logging.getLogger("cynic.api.handlers.direct")


class DirectActionsHandler(HandlerGroup):
    """ACT_REQUESTED → UniversalActuator.dispatch() — real execution, real feedback."""

    _EXECUTION_WINDOW = 13  # F(7) — rolling success tracking

    def __init__(
        self, svc: KernelServices, *, universal_actuator: UniversalActuator, qtable: QTable
    ) -> None:
        self._svc = svc
        self._universal_actuator = universal_actuator
        self._qtable = qtable
        self._execution_window: list[bool] = []

    @property
    def name(self) -> str:
        return "direct"

    def dependencies(self) -> frozenset[str]:
        return frozenset({
            "escore_tracker",
            "universal_actuator",
            "qtable",
        })

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.ACT_REQUESTED, self._on_act_requested),
        ]

    async def _on_act_requested(self, event: Event) -> None:
        """ACT_REQUESTED → UniversalActuator.dispatch() → real execution."""
        try:
            p = event.payload or {}
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
                action_type, success, result.duration_ms, success_rate, reward
            )

        except Exception:
            logger.debug("handler error", exc_info=True)
