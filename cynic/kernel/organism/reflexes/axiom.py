"""
PHASE 4: Axiom activation chain â€” axiom signaling toward TRANSCENDENCE.

Handlers: emergence_signal, decision_made_for_axiom, decision_made_for_run,
          axiom_activated, self_improvement_proposed, transcendence,
          residual_high, action_proposed, meta_cycle.
"""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING

from cynic.kernel.core.event_bus import CoreEvent, Event, EventBusError
from cynic.kernel.core.events_schema import TranscendencePayload
from cynic.kernel.core.phi import GROWL_MIN, HOWL_MIN, MAX_Q_SCORE, PHI_INV, PHI_INV_2, WAG_MIN
from cynic.kernel.organism.reflexes.base import HandlerGroup
from cynic.kernel.organism.reflexes.services import CognitionServices

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.cognition.cortex.action_proposer import ActionProposer

logger = logging.getLogger("cynic.kernel.organism.reflexes.axiom")

# A6-A9: Emergent axioms that trigger TRANSCENDENCE when all 4 active
_A6_A9 = {"EMERGENCE", "AUTONOMY", "SYMBIOSIS", "ANTIFRAGILITY"}


class AxiomHandlers(HandlerGroup):
    """Axiom signaling chain leading to TRANSCENDENCE."""

    def __init__(
        self,
        cognition: CognitionServices,
        *,
        action_proposer: ActionProposer,
        bus: Optional[EventBus] = None,
    ) -> None:
        super().__init__(bus=bus)
        self._cognition = cognition
        self._action_proposer = action_proposer

    @property
    def name(self) -> str:
        return "axiom"

    def dependencies(self) -> frozenset[str]:
        return frozenset(
            {
                "escore_tracker",
                "axiom_monitor",
                "action_proposer",
            }
        )

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.EMERGENCE_DETECTED, self._on_emergence_signal),
            (CoreEvent.DECISION_MADE, self._on_decision_made_for_axiom),
            (CoreEvent.DECISION_MADE, self._on_decision_made_for_run),
            (CoreEvent.AXIOM_ACTIVATED, self._on_axiom_activated),
            (CoreEvent.SELF_IMPROVEMENT_PROPOSED, self._on_self_improvement_proposed),
            (CoreEvent.TRANSCENDENCE, self._on_transcendence),
            (CoreEvent.RESIDUAL_HIGH, self._on_residual_high),
            (CoreEvent.ACTION_PROPOSED, self._on_action_proposed),
            (CoreEvent.META_CYCLE, self._on_meta_cycle),
        ]

    # â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    # HANDLER IMPLEMENTATIONS
    # â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    async def _on_emergence_signal(self, event: Event) -> None:
        """EMERGENCE_DETECTED â†’ signal EMERGENCE axiom."""
        try:
            await self._cognition.signal_axiom("EMERGENCE", "emergence_detector")
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_decision_made_for_axiom(self, event: Event) -> None:
        """DECISION_MADE â†’ signal AUTONOMY axiom."""
        try:
            await self._cognition.signal_axiom("AUTONOMY", "decide_agent")
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_decision_made_for_run(self, event: Event) -> None:
        """DECISION_MADE â†’ RUN EScore + EMERGENCE on confident BARK."""
        try:
            p = event.dict_payload or {}
            q_value = float(p.get("q_value", 0.0))
            verdict = p.get("recommended_action", "")

            # RUN EScore â€” decision quality as execution efficiency
            run_score = q_value * MAX_Q_SCORE
            self._cognition.escore_tracker.update_dimension("agent:cynic", "RUN", run_score)

            # EMERGENCE: confident BARK = organism sure of critical problem
            emergence_signalled = False
            if verdict == "BARK" and q_value >= PHI_INV_2:
                await self._cognition.signal_axiom(
                    "EMERGENCE",
                    "decision_made",
                    trigger="CONFIDENT_BARK",
                )
                emergence_signalled = True

            logger.debug(
                "DECISION_MADE: action=%s q=%.3f â†’ RUN EScore=%.1f%s",
                verdict,
                q_value,
                run_score,
                " EMERGENCE signalled" if emergence_signalled else "",
            )
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_axiom_activated(self, event: Event) -> None:
        """AXIOM_ACTIVATED â†’ log milestone; emit TRANSCENDENCE when all A6-A9 active."""
        try:
            axiom_name = event.dict_payload.get("axiom", "?")
            maturity = event.dict_payload.get("maturity", 0.0)
            active = self._cognition.axiom_monitor.active_axioms()

            logger.info(
                "AXIOM_ACTIVATED: %s (maturity=%.1f) â€” active: %s",
                axiom_name,
                maturity,
                active,
            )

            # Check if all A6-A9 are active
            a6_a9_active = [a for a in active if a in _A6_A9]
            if len(a6_a9_active) == 4:
                await self.bus.emit(
                    Event.typed(
                        CoreEvent.TRANSCENDENCE,
                        TranscendencePayload(
                            active_axioms=active,
                            maturity=maturity,
                            tier="TRANSCENDENT",
                            trigger=f"AXIOM_ACTIVATED:{axiom_name}",
                        ),
                        source="axiom_monitor",
                    )
                )
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_self_improvement_proposed(self, event: Event) -> None:
        """SELF_IMPROVEMENT_PROPOSED â†’ ActionProposer + CONSCIOUSNESS + JUDGE."""
        try:
            p = event.dict_payload or {}
            proposals = p.get("proposals", [])
            severity = float(p.get("severity", 0.0))

            # Route to ActionProposer
            for prop in proposals:
                self._action_proposer.propose_self_improvement(prop)

            if not proposals:
                return

            # CONSCIOUSNESS: organism aware of own state
            await self._cognition.signal_axiom(
                "CONSCIOUSNESS",
                "self_improvement",
                trigger="SELF_IMPROVEMENT_PROPOSED",
                count=len(proposals),
            )

            # JUDGE EScore: self-analysis quality
            judge_score = severity * MAX_Q_SCORE
            self._cognition.escore_tracker.update_dimension("agent:cynic", "JUDGE", judge_score)

            logger.info(
                "SELF_IMPROVEMENT_PROPOSED: count=%d severity=%.3f â†’ "
                "CONSCIOUSNESS signalled, JUDGE=%.1f",
                len(proposals),
                severity,
                judge_score,
            )
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_transcendence(self, event: Event) -> None:
        """TRANSCENDENCE â†’ EScore self-reward + milestone log."""
        try:
            active = (event.dict_payload or {}).get("active_axioms", [])
            logger.warning(
                "TRANSCENDENCE â€” all 4 emergent axioms active: %s",
                active,
            )
            self._cognition.escore_tracker.update_dimension("agent:cynic", "JUDGE", MAX_Q_SCORE)
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_residual_high(self, event: Event) -> None:
        """RESIDUAL_HIGH â†’ EMERGENCE signal + JUDGE penalty."""
        try:
            p = event.dict_payload or {}
            residual = float(p.get("residual_variance", 0.0))
            cell_id = p.get("cell_id", "")

            # THE_UNNAMEABLE = EMERGENCE by definition
            await self._cognition.signal_axiom("EMERGENCE", "residual_high")

            # EScore JUDGE penalty
            penalty_score = (1.0 - min(residual, 1.0)) * MAX_Q_SCORE
            self._cognition.escore_tracker.update_dimension("agent:cynic", "JUDGE", penalty_score)

            logger.warning(
                "RESIDUAL_HIGH: cell=%s residual=%.3f â†’ EMERGENCE signal, " "JUDGE penalty=%.1f",
                cell_id,
                residual,
                penalty_score,
            )
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_action_proposed(self, event: Event) -> None:
        """ACTION_PROPOSED â†’ EScore BUILD update."""
        try:
            p = event.dict_payload or {}
            priority = int(p.get("priority", 3))
            action_type = p.get("action_type", "")

            score = {
                1: MAX_Q_SCORE,
                2: HOWL_MIN,
                3: WAG_MIN,
            }.get(priority, GROWL_MIN)

            self._cognition.escore_tracker.update_dimension("agent:cynic", "BUILD", score)
            logger.info(
                "ACTION_PROPOSED: type=%s priority=%d â†’ BUILD EScore=%.1f",
                action_type,
                priority,
                score,
            )
        except EventBusError:
            logger.debug("handler error", exc_info=True)

    async def _on_meta_cycle(self, event: Event) -> None:
        """META_CYCLE â†’ ANTIFRAGILITY signal + CONSCIOUSNESS + JUDGE update."""
        try:
            p = event.dict_payload or {}
            evolve = p.get("evolve", {})
            pass_rate = float(evolve.get("pass_rate", 0.0))
            regression = bool(evolve.get("regression", False))

            # ANTIFRAGILITY: regression = stress signal
            if regression:
                await self._cognition.signal_axiom(
                    "ANTIFRAGILITY",
                    "meta_cycle",
                    trigger="META_CYCLE_REGRESSION",
                )

            # CONSCIOUSNESS: organism knows its state
            if pass_rate >= PHI_INV:
                await self._cognition.signal_axiom(
                    "CONSCIOUSNESS",
                    "meta_cycle",
                    trigger="META_CYCLE_HEALTH",
                    pass_rate=round(pass_rate, 3),
                )

            # EScore JUDGE: self-assessment quality
            if pass_rate >= PHI_INV:
                judge_score = pass_rate * MAX_Q_SCORE
            elif pass_rate >= PHI_INV_2:
                judge_score = WAG_MIN
            else:
                judge_score = GROWL_MIN

            self._cognition.escore_tracker.update_dimension("agent:cynic", "JUDGE", judge_score)

            logger.info(
                "META_CYCLE: pass_rate=%.1f%% regression=%s â†’ " "JUDGE EScore=%.1f%s",
                pass_rate * 100,
                regression,
                judge_score,
                " ANTIFRAGILITY signalled" if regression else "",
            )
        except EventBusError:
            logger.debug("handler error", exc_info=True)
