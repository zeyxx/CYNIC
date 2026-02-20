"""PHASE 6: SDK/ACT execution handlers — Claude Code integration."""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING

from cynic.core.event_bus import Event, CoreEvent
from cynic.core.phi import MAX_Q_SCORE

from cynic.api.handlers.base import HandlerGroup, KernelServices

if TYPE_CHECKING:
    from cynic.judge.action_proposer import ActionProposer
    from cynic.learning.qlearning import QTable

logger = logging.getLogger("cynic.api.handlers.sdk")


class SDKHandlers(HandlerGroup):
    """SDK/ACT feedback — Claude Code integration + outcome tracking."""

    _SDK_OUTCOME_WINDOW = 13  # F(7)

    def __init__(
        self, svc: KernelServices, *, action_proposer: ActionProposer, qtable: QTable
    ) -> None:
        self._svc = svc
        self._action_proposer = action_proposer
        self._qtable = qtable
        self._sdk_outcome_window: list[bool] = []

    @property
    def name(self) -> str:
        return "sdk"

    def dependencies(self) -> frozenset[str]:
        return frozenset({
            "escore_tracker",
            "axiom_monitor",
            "action_proposer",
            "qtable",
        })

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.ACT_COMPLETED, self._on_act_completed),
            (CoreEvent.ACT_REQUESTED, self._on_act_requested_for_organism),
            (CoreEvent.SDK_TOOL_JUDGED, self._on_sdk_tool_judged),
            (CoreEvent.SDK_SESSION_STARTED, self._on_sdk_session_started),
            (CoreEvent.SDK_RESULT_RECEIVED, self._on_sdk_result_received),
        ]

    async def _on_act_completed(self, event: Event) -> None:
        """ACT_COMPLETED → emit ACT_COMPLETED with rich metadata."""
        try:
            p = event.payload or {}
            is_success = p.get("success", False)
            duration = float(p.get("duration_ms", 0.0))
            self._sdk_outcome_window.append(is_success)
            if len(self._sdk_outcome_window) > self._SDK_OUTCOME_WINDOW:
                self._sdk_outcome_window.pop(0)
            success_rate = sum(self._sdk_outcome_window) / len(self._sdk_outcome_window)
            self._svc.escore_tracker.update("agent:cynic", "RUN", success_rate * MAX_Q_SCORE)
            logger.info("ACT_COMPLETED: success=%s duration=%.0fms → RUN=%.1f", is_success, duration, success_rate * MAX_Q_SCORE)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_act_requested_for_organism(self, event: Event) -> None:
        """ACT_REQUESTED → organism execution signal."""
        try:
            p = event.payload or {}
            prompt_lines = len(str(p.get("prompt", "")).split("\n"))
            logger.debug("ACT_REQUESTED: prompt=%d lines", prompt_lines)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_sdk_tool_judged(self, event: Event) -> None:
        """SDK_TOOL_JUDGED → QTable feedback + EScore update."""
        try:
            p = event.payload or {}
            tool_name = p.get("tool_name", "")
            q_score = float(p.get("q_score", 0.5))
            self._qtable.update_tool_quality(tool_name, q_score)
            self._svc.escore_tracker.update("agent:cynic", "BUILD", q_score * MAX_Q_SCORE)
            logger.debug("SDK_TOOL_JUDGED: tool=%s q=%.3f", tool_name, q_score)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_sdk_session_started(self, event: Event) -> None:
        """SDK_SESSION_STARTED → session tracking."""
        try:
            p = event.payload or {}
            session_id = p.get("session_id", "")
            logger.info("SDK_SESSION_STARTED: session=%s", session_id)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_sdk_result_received(self, event: Event) -> None:
        """SDK_RESULT_RECEIVED → outcome tracking + QTable reward."""
        try:
            p = event.payload or {}
            is_error = p.get("is_error", False)
            reward = float(p.get("reward", 0.5))
            self._qtable.update(reward=reward)
            self._sdk_outcome_window.append(not is_error)
            if len(self._sdk_outcome_window) > self._SDK_OUTCOME_WINDOW:
                self._sdk_outcome_window.pop(0)
            logger.debug("SDK_RESULT_RECEIVED: error=%s reward=%.3f", is_error, reward)
        except Exception:
            logger.debug("handler error", exc_info=True)
