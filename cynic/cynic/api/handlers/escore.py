"""PHASE 5: EScore dimension handlers — 7-dim reputation system."""

from __future__ import annotations

import logging

from cynic.core.event_bus import Event, CoreEvent
from cynic.core.events_schema import JudgmentCreatedPayload
from cynic.core.phi import MAX_CONFIDENCE, MAX_Q_SCORE, HOWL_MIN, WAG_MIN, GROWL_MIN, PHI_INV, PHI_INV_2

from cynic.api.handlers.base import HandlerGroup, KernelServices

logger = logging.getLogger("cynic.api.handlers.escore")


class EScoreHandlers(HandlerGroup):
    """Most independent group — updates 7 EScore dimensions."""

    def __init__(self, svc: KernelServices) -> None:
        self._svc = svc

    @property
    def name(self) -> str:
        return "escore"

    def dependencies(self) -> frozenset[str]:
        return frozenset({"escore_tracker", "axiom_monitor"})

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.JUDGMENT_CREATED, self._on_judgment_for_burn),
            (CoreEvent.LEARNING_EVENT, self._on_learning_event),
            (CoreEvent.CONSCIOUSNESS_CHANGED, self._on_consciousness_changed),
            (CoreEvent.USER_FEEDBACK, self._on_user_feedback),
            (CoreEvent.PERCEPTION_RECEIVED, self._on_perception_received),
            (CoreEvent.EWC_CHECKPOINT, self._on_ewc_checkpoint),
            (CoreEvent.Q_TABLE_UPDATED, self._on_q_table_updated),
            (CoreEvent.CONSENSUS_REACHED, self._on_consensus_reached),
            (CoreEvent.CONSENSUS_FAILED, self._on_consensus_failed),
            (CoreEvent.USER_CORRECTION, self._on_user_correction),
            (CoreEvent.ANOMALY_DETECTED, self._on_anomaly_detected),
        ]

    async def _on_judgment_for_burn(self, event: Event) -> None:
        """JUDGMENT_CREATED → BURN EScore."""
        try:
            p = JudgmentCreatedPayload.model_validate(event.payload or {})
            burn_score = min(p.confidence / MAX_CONFIDENCE, 1.0) * MAX_Q_SCORE
            self._svc.escore_tracker.update("agent:cynic", "BURN", burn_score, reality=p.reality)
            logger.debug("JUDGMENT_CREATED→BURN: verdict=%s conf=%.3f → BURN=%.1f", p.verdict, p.confidence, burn_score)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_learning_event(self, event: Event) -> None:
        """LEARNING_EVENT → JUDGE EScore + AUTONOMY signal."""
        try:
            p = event.payload or {}
            reward = float(p.get("reward", 0.0))
            judge_score = reward * MAX_Q_SCORE
            self._svc.escore_tracker.update("agent:cynic", "JUDGE", judge_score)
            await self._svc.signal_axiom("AUTONOMY", "learning_event", trigger="LEARNING_EVENT")
            logger.debug("LEARNING_EVENT: action=%s reward=%.3f → JUDGE=%.1f", p.get("action", ""), reward, judge_score)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_consciousness_changed(self, event: Event) -> None:
        """CONSCIOUSNESS_CHANGED → HOLD EScore + ANTIFRAGILITY on recovery."""
        try:
            p = event.payload or {}
            direction = p.get("direction", "DOWN")
            hold_score = HOWL_MIN if direction == "UP" else GROWL_MIN
            self._svc.escore_tracker.update("agent:cynic", "HOLD", hold_score)
            if direction == "UP":
                await self._svc.signal_axiom("ANTIFRAGILITY", "consciousness_changed", trigger="LOD_RECOVERY")
            logger.info("CONSCIOUSNESS_CHANGED: %s → HOLD=%.1f%s", direction, hold_score, " ANTIFRAGILITY signalled" if direction == "UP" else "")
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_user_feedback(self, event: Event) -> None:
        """USER_FEEDBACK → JUDGE + SOCIAL EScore."""
        try:
            p = event.payload or {}
            rating = float(p.get("rating", 3.0))
            judge_score = (rating - 1) / 4.0 * MAX_Q_SCORE
            self._svc.escore_tracker.update("agent:cynic", "JUDGE", judge_score)
            self._svc.escore_tracker.update("agent:cynic", "SOCIAL", WAG_MIN)
            logger.info("USER_FEEDBACK: rating=%d/5 → JUDGE=%.1f SOCIAL=%.1f", int(rating), judge_score, WAG_MIN)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_perception_received(self, event: Event) -> None:
        """PERCEPTION_RECEIVED → SOCIAL + HOLD EScore."""
        try:
            p = event.payload or {}
            reality = p.get("reality", "CODE")
            social_score = WAG_MIN if reality in ("SOCIAL", "HUMAN", "COSMOS") else GROWL_MIN
            hold_score = HOWL_MIN if reality == "CYNIC" else WAG_MIN
            self._svc.escore_tracker.update("agent:cynic", "SOCIAL", social_score, reality=reality)
            self._svc.escore_tracker.update("agent:cynic", "HOLD", hold_score, reality=reality)
            logger.debug("PERCEPTION_RECEIVED: reality=%s → SOCIAL=%.1f HOLD=%.1f", reality, social_score, hold_score)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_ewc_checkpoint(self, event: Event) -> None:
        """EWC_CHECKPOINT → JUDGE EScore + AUTONOMY + CONSCIOUSNESS signals."""
        try:
            p = event.payload or {}
            q_value = float(p.get("q_value", 0.5))
            judge_score = q_value * MAX_Q_SCORE
            self._svc.escore_tracker.update("agent:cynic", "JUDGE", judge_score)
            await self._svc.signal_axiom("AUTONOMY", "ewc_checkpoint", trigger="EWC_CHECKPOINT")
            await self._svc.signal_axiom("CONSCIOUSNESS", "ewc_checkpoint", trigger="EWC_CHECKPOINT", q_value=round(q_value, 3))
            logger.info("EWC_CHECKPOINT: state=%s action=%s q=%.3f → JUDGE=%.1f", p.get("state_key", ""), p.get("action", ""), q_value, judge_score)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_q_table_updated(self, event: Event) -> None:
        """Q_TABLE_UPDATED → BUILD + HOLD EScore."""
        try:
            self._svc.escore_tracker.update("agent:cynic", "BUILD", HOWL_MIN)
            self._svc.escore_tracker.update("agent:cynic", "HOLD", WAG_MIN)
            logger.info("Q_TABLE_UPDATED: flushed=%d → BUILD=%.1f HOLD=%.1f", int((event.payload or {}).get("flushed", 0)), HOWL_MIN, WAG_MIN)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_consensus_reached(self, event: Event) -> None:
        """CONSENSUS_REACHED → BUILD EScore + SYMBIOSIS + CONSCIOUSNESS signals."""
        try:
            p = event.payload or {}
            q_score = float(p.get("q_score", 0.0))
            self._svc.escore_tracker.update("agent:cynic", "BUILD", q_score)
            await self._svc.signal_axiom("SYMBIOSIS", "consensus_reached", trigger="CONSENSUS_REACHED")
            await self._svc.signal_axiom("CONSCIOUSNESS", "consensus_reached", trigger="CONSENSUS_REACHED", verdict=p.get("verdict", ""), q_score=round(q_score, 1))
            logger.debug("CONSENSUS_REACHED: votes=%d verdict=%s q=%.1f → BUILD=%.1f", int(p.get("votes", 0)), p.get("verdict", ""), q_score, q_score)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_consensus_failed(self, event: Event) -> None:
        """CONSENSUS_FAILED → EMERGENCE signal + JUDGE penalty."""
        try:
            p = event.payload or {}
            votes = int(p.get("votes", 0))
            quorum = int(p.get("quorum", 7))
            judge_score = (votes / max(quorum, 1)) * MAX_Q_SCORE if votes > 0 else GROWL_MIN
            self._svc.escore_tracker.update("agent:cynic", "JUDGE", judge_score)
            await self._svc.signal_axiom("EMERGENCE", "consensus_failed", trigger="CONSENSUS_FAILED")
            logger.warning("CONSENSUS_FAILED: votes=%d quorum=%d → JUDGE=%.1f EMERGENCE signalled", votes, quorum, judge_score)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_user_correction(self, event: Event) -> None:
        """USER_CORRECTION → JUDGE EScore (learning from human guidance)."""
        try:
            p = event.payload or {}
            correction_type = p.get("correction_type", "")
            score = HOWL_MIN if correction_type == "CRITICAL" else WAG_MIN
            self._svc.escore_tracker.update("agent:cynic", "JUDGE", score)
            logger.info("USER_CORRECTION: type=%s → JUDGE=%.1f", correction_type, score)
        except Exception:
            logger.debug("handler error", exc_info=True)

    async def _on_anomaly_detected(self, event: Event) -> None:
        """ANOMALY_DETECTED → JUDGE EScore penalty."""
        try:
            p = event.payload or {}
            severity = float(p.get("severity", 0.5))
            judge_score = (1.0 - min(severity, 1.0)) * MAX_Q_SCORE
            self._svc.escore_tracker.update("agent:cynic", "JUDGE", judge_score)
            logger.warning("ANOMALY_DETECTED: severity=%.2f → JUDGE=%.1f", severity, judge_score)
        except Exception:
            logger.debug("handler error", exc_info=True)
