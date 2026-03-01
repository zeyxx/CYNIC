"""
PHASE 3: Intelligence cycle handlers — LOD assessment, error tracking, budget response.

Handlers: emergence, budget_warning, budget_exhausted, judgment_requested,
          judgment_for_intelligence, judgment_failed, judgment_for_compressor.
"""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Optional

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.core.event_bus import Event, CoreEvent
from cynic.kernel.core.exceptions import CynicError
from cynic.kernel.core.phi import GROWL_MIN
from cynic.kernel.core.judgment import Cell
from cynic.kernel.organism.perception.senses import checkpoint as _session_checkpoint
from cynic.kernel.organism.perception.senses.checkpoint import CHECKPOINT_EVERY

from cynic.kernel.organism.handlers.base import HandlerGroup, KernelServices

if TYPE_CHECKING:
    from cynic.kernel.organism.brain.cognition.cortex.orchestrator import JudgeOrchestrator
    from cynic.kernel.organism.metabolism.scheduler import ConsciousnessRhythm
    from asyncpg import Pool
    from cynic.kernel.organism.perception.senses.compressor import ContextCompressor

logger = logging.getLogger("cynic.kernel.organism.handlers.intelligence")


class IntelligenceHandlers(HandlerGroup):
    """LOD assessment, error tracking, budget response, compressor feeding."""

    _OUTCOME_WINDOW = 21  # F(8)

    def __init__(
        self,
        svc: KernelServices,
        *,
        orchestrator: JudgeOrchestrator,
        scheduler: ConsciousnessRhythm,
        db_pool: Optional[Pool],
        compressor,  # ContextCompressor
    ) -> None:
        self._svc = svc
        self._orchestrator = orchestrator
        self._scheduler = scheduler
        self._db_pool = db_pool
        self._compressor = compressor

        # Group-local mutable state
        self._outcome_window: list[bool] = []
        self._escore_persist_counter = 0
        self._checkpoint_counter = 0

    @property
    def name(self) -> str:
        return "intelligence"

    def dependencies(self) -> frozenset[str]:
        return frozenset({
            "escore_tracker",
            "lod_controller",
            "axiom_monitor",
            "orchestrator",
            "scheduler",
            "db_pool",
            "compressor",
        })

    def subscriptions(self) -> list[tuple[CoreEvent, callable]]:
        return [
            (CoreEvent.EMERGENCE_DETECTED, self._on_emergence),
            (CoreEvent.BUDGET_WARNING, self._on_budget_warning),
            (CoreEvent.BUDGET_EXHAUSTED, self._on_budget_exhausted),
            (CoreEvent.PERCEPTION_RECEIVED, self._on_perception_received),
            (CoreEvent.JUDGMENT_REQUESTED, self._on_judgment_requested),
            (CoreEvent.JUDGMENT_CREATED, self._on_judgment_for_intelligence),
            (CoreEvent.JUDGMENT_FAILED, self._on_judgment_failed),
            (CoreEvent.JUDGMENT_CREATED, self._on_judgment_for_compressor),
        ]

    # ═══════════════════════════════════════════════════════════════════════
    # HANDLER IMPLEMENTATIONS
    # ═══════════════════════════════════════════════════════════════════════

    async def _on_perception_received(self, event: Event) -> None:
        """PERCEPTION_RECEIVED → Trigger REFLEX cycle."""
        try:
            p = event.dict_payload or {}
            content = p.get("content", "")
            reality = p.get("reality", "CYNIC")

            # Create a Reflex Cell
            cell = Cell(
                reality=reality,
                analysis="PERCEIVE",
                time_dim="PRESENT",
                content=content,
                context="Automated reflex from perception",
                risk=0.1,
                complexity=0.1,
                budget_usd=0.01,
                metadata={"source": event.source}
            )

            # Submit to scheduler
            self._scheduler.submit(
                cell,
                level=ConsciousnessLevel.REFLEX,
                source=f"perception_{event.source}"
            )
            logger.info("Intelligence: Perception received from %s → Triggered REFLEX cycle", event.source)

        except Exception as e:
            logger.debug("Intelligence: Failed to handle perception: %s", e)

    def _update_error_rate(self) -> None:
        """Compute rolling error rate from outcome window."""
        if not self._outcome_window:
            return
        errors = sum(1 for ok in self._outcome_window if not ok)
        # Note: health_cache removed - using local state only

    async def _on_emergence(self, event: Event) -> None:
        """META cycle trigger when ResidualDetector fires."""
        try:
            p = event.dict_payload or {}
            cell = Cell(
                reality="CYNIC",
                analysis="EMERGE",
                time_dim="PRESENT",
                content=str(p),
                context="Emergence detected — META cycle triggered",
                risk=0.5,
                complexity=0.6,
                budget_usd=0.1,
                metadata={
                    "source": "emergence_trigger",
                }
            )
            self._scheduler.submit(cell, level=ConsciousnessLevel.META, source="emergence")
        except Exception as e:
            logger.error("Intelligence: Failed to trigger emergence cycle: %s", e)

    async def _on_budget_warning(self, event: Event) -> None:
        """React to budget pressure."""
        logger.warning("Intelligence: Budget warning received — scaling down")

    async def _on_budget_exhausted(self, event: Event) -> None:
        """React to zero budget."""
        logger.error("Intelligence: Budget exhausted — emergency mode")

    async def _on_judgment_requested(self, event: Event) -> None:
        """API requested a judgment."""
        pass

    async def _on_judgment_for_intelligence(self, event: Event) -> None:
        """Track outcome for error rate."""
        success = event.dict_payload.get("verdict") != "BARK"
        self._outcome_window.append(success)
        if len(self._outcome_window) > self._OUTCOME_WINDOW:
            self._outcome_window.pop(0)
        self._update_error_rate()

    async def _on_judgment_failed(self, event: Event) -> None:
        """Track failure in window."""
        self._outcome_window.append(False)
        if len(self._outcome_window) > self._OUTCOME_WINDOW:
            self._outcome_window.pop(0)
        self._update_error_rate()

    async def _on_judgment_for_compressor(self, event: Event) -> None:
        """Feed high-quality judgments into the compressor."""
        verdict = event.dict_payload.get("verdict", "")
        confidence = event.dict_payload.get("confidence", 0.0)

        if verdict in ("HOWL", "WAG") and confidence > GROWL_MIN:
            try:
                # Add to compressor for future context summaries
                self._compressor.add_judgment(event.dict_payload)
            except Exception as e:
                logger.debug("Intelligence: Failed to feed compressor: %s", e)
