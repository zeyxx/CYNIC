"""
CYNIC DecideAgent — JUDGMENT_CREATED -> DECISION_MADE

Subscribes to JUDGMENT_CREATED events. For BARK/GROWL verdicts with
sufficient confidence (>= phi^-2 = 0.382), consults the Q-Table and
emits DECISION_MADE on the core bus.

Fire-and-forget: never blocks. All logic is async + bus.emit().
"""
from __future__ import annotations

import logging
from typing import Any, Dict

from cynic.core.event_bus import CoreEvent, Event, EventBus, get_core_bus

logger = logging.getLogger("cynic.judge.decide")

# phi^-2 = 0.382 — minimum confidence to trigger auto-decide
_PHI_INV_2 = 0.382

# Verdicts that warrant a policy consultation
_ALERT_VERDICTS = {"BARK", "GROWL"}

# Realities that produce actionable prompts for the runner
_ACT_REALITIES = frozenset({"CODE", "CYNIC"})


def _build_action_prompt(
    reality: str,
    analysis: str,
    verdict: str,
    content_preview: str,
    context: str,
) -> str:
    """
    Convert judgment context → actionable Claude prompt.
    Rule-based templates — no LLM needed for routing.
    Called only for BARK/GROWL verdicts.
    """
    body = (content_preview or context or "(no detail available)").strip()[:400]

    if reality == "CODE":
        if verdict == "BARK":
            return (
                f"[CYNIC AUTO-ACT] BARK detected on code analysis.\n"
                f"Please review and fix the following critical issue:\n\n{body}"
            )
        return (
            f"[CYNIC AUTO-ACT] GROWL detected on code analysis.\n"
            f"Please review the following code quality concern:\n\n{body}"
        )
    if reality == "CYNIC":
        if verdict == "BARK":
            return (
                f"[CYNIC AUTO-ACT] BARK on self-assessment ({analysis}).\n"
                f"Critical organism issue — please investigate:\n\n{body}"
            )
        return (
            f"[CYNIC AUTO-ACT] GROWL on self-assessment ({analysis}).\n"
            f"Organism quality degradation — please review:\n\n{body}"
        )
    # Generic fallback for other realities
    return (
        f"[CYNIC AUTO-ACT] {verdict} on {reality}\u00b7{analysis}.\n"
        f"Please investigate:\n\n{body}"
    )


class DecideAgent:
    """
    Autonomous decision layer — sits between Judge and Act.

    Listens for judgments, consults the Q-Table for BARK/GROWL results,
    and emits DECISION_MADE so downstream actors can react without
    manual intervention.
    """

    def __init__(self, qtable: Any) -> None:
        """
        qtable: cynic.learning.qlearning.QTable — consulted for best action.
        """
        self._qtable = qtable
        self._decisions_made: int = 0
        self._skipped: int = 0
        self._handler = self._on_judgment

    # ---- Lifecycle --------------------------------------------------------

    def start(self, bus: EventBus) -> None:
        """Subscribe to JUDGMENT_CREATED. Must be called with a running event loop."""
        bus.on(CoreEvent.JUDGMENT_CREATED, self._handler)
        logger.info("DecideAgent started — subscribed to JUDGMENT_CREATED")

    def stop(self, bus: EventBus) -> None:
        """Unsubscribe from JUDGMENT_CREATED."""
        bus.off(CoreEvent.JUDGMENT_CREATED, self._handler)
        logger.info("DecideAgent stopped")

    # ---- Handler ----------------------------------------------------------

    async def _on_judgment(self, event: Event) -> None:
        payload = event.payload or {}
        verdict = payload.get("verdict", "")
        confidence = float(payload.get("confidence", 0.0))
        state_key = payload.get("state_key", "")
        judgment_id = payload.get("judgment_id", "")
        q_score = payload.get("q_score", 0.0)
        # Enriched fields from orchestrator.py (JUDGMENT_CREATED now includes these)
        reality = payload.get("reality", "")
        analysis = payload.get("analysis", "")
        content_preview = payload.get("content_preview", "")
        context = payload.get("context", "")

        if verdict not in _ALERT_VERDICTS or confidence < _PHI_INV_2:
            self._skipped += 1
            logger.debug(
                "DecideAgent skip: verdict=%s confidence=%.3f",
                verdict, confidence,
            )
            return

        # Consult Q-Table for best action
        recommended_action = self._qtable.exploit(state_key)
        q_entry = self._qtable._table.get(state_key, {}).get(recommended_action, None)
        q_value = q_entry.q_value if q_entry is not None else 0.0

        # Build action prompt (non-empty only for ACT_REALITIES — others get generic)
        action_prompt = _build_action_prompt(
            reality, analysis, verdict, content_preview, context
        )

        decision_payload: Dict[str, Any] = {
            "judgment_id": judgment_id,
            "state_key": state_key,
            "recommended_action": recommended_action,
            "q_value": q_value,
            "confidence": confidence,
            "trigger": "auto_decide",
            "reality": reality,
            "content_preview": content_preview,
            "action_prompt": action_prompt,
        }

        bus = get_core_bus()
        await bus.emit(Event(
            type=CoreEvent.DECISION_MADE,
            payload=decision_payload,
            source="decide_agent",
        ))

        self._decisions_made += 1
        logger.info(
            "DecideAgent decision: verdict=%s reality=%s state=%s action=%s q=%.3f",
            verdict, reality, state_key, recommended_action, q_value,
        )

    # ---- Stats ------------------------------------------------------------

    def stats(self) -> Dict[str, Any]:
        return {
            "decisions_made": self._decisions_made,
            "skipped": self._skipped,
        }
