"""
PHASE 3: Intelligence cycle handlers — LOD assessment, error tracking, budget response.

Handlers: emergence, budget_warning, budget_exhausted, judgment_requested,
          judgment_for_intelligence, judgment_failed, judgment_for_compressor.
"""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING

from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.event_bus import Event, CoreEvent
from cynic.core.exceptions import CynicError
from cynic.core.phi import GROWL_MIN
from cynic.core.judgment import Cell
from cynic.senses import checkpoint as _session_checkpoint
from cynic.senses.checkpoint import CHECKPOINT_EVERY

from cynic.api.handlers.base import HandlerGroup, KernelServices

if TYPE_CHECKING:
    from cynic.cognition.cortex.orchestrator import JudgeOrchestrator
    from cynic.scheduler import ConsciousnessRhythm
    from asyncpg import Pool
    from cynic.senses.compressor import ContextCompressor

logger = logging.getLogger("cynic.api.handlers.intelligence")


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
            (CoreEvent.JUDGMENT_REQUESTED, self._on_judgment_requested),
            (CoreEvent.JUDGMENT_CREATED, self._on_judgment_for_intelligence),
            (CoreEvent.JUDGMENT_FAILED, self._on_judgment_failed),
            (CoreEvent.JUDGMENT_CREATED, self._on_judgment_for_compressor),
        ]

    # ═══════════════════════════════════════════════════════════════════════
    # HANDLER IMPLEMENTATIONS
    # ═══════════════════════════════════════════════════════════════════════

    def _update_error_rate(self) -> None:
        """Compute rolling error rate from outcome window."""
        if not self._outcome_window:
            return
        errors = sum(1 for ok in self._outcome_window if not ok)
        # Note: health_cache removed - using local state only

    async def _on_emergence(self, event: Event) -> None:
        """META cycle trigger when ResidualDetector fires."""
        try:
            cell = Cell(
                reality="CYNIC",
                analysis="EMERGE",
                time_dim="PRESENT",
                content=event.payload,
                context="Emergence detected — META cycle triggered",
                risk=0.5,
                complexity=0.6,
                budget_usd=0.1,
                metadata={
                    "source": "emergence_trigger",
                    "pattern": (
                        event.payload.get("pattern_type", "UNKNOWN")
                        if isinstance(event.payload, dict)
                        else "UNKNOWN"
                    ),
                },
            )
            self._scheduler.submit(
                cell, level=ConsciousnessLevel.META, source="emergence_trigger"
            )
        except CynicError:
            logger.debug("handler error", exc_info=True)

    async def _on_budget_warning(self, event: Event) -> None:
        """BUDGET_WARNING → orchestrator alert + HOLD EScore."""
        try:
            self._orchestrator.on_budget_warning()
            self._svc.escore_tracker.update("agent:cynic", "HOLD", GROWL_MIN)
            logger.warning("BUDGET_WARNING → HOLD EScore=%.1f (financial stress)", GROWL_MIN)
        except CynicError:
            logger.debug("handler error", exc_info=True)

    async def _on_budget_exhausted(self, event: Event) -> None:
        """BUDGET_EXHAUSTED → orchestrator shutdown + HOLD=0.0."""
        try:
            self._orchestrator.on_budget_exhausted()
            self._svc.escore_tracker.update("agent:cynic", "HOLD", 0.0)
            logger.warning(
                "BUDGET_EXHAUSTED → HOLD EScore=0.0 (financial collapse)"
            )
        except CynicError:
            logger.debug("handler error", exc_info=True)

    async def _on_judgment_requested(self, event: Event) -> None:
        """JUDGMENT_REQUESTED → update queue depth + assess LOD."""
        try:
            # Skip health_cache update - not available in KernelServices
            if hasattr(self._svc, 'assess_lod'):
                await self._svc.assess_lod()
        except CynicError:
            logger.debug("handler error", exc_info=True)

    async def _on_judgment_for_intelligence(self, event: Event) -> None:
        """JUDGMENT_CREATED → error tracking, LOD assessment, EScore dog votes."""
        try:
            p = event.payload or {}

            # 1. Update rolling outcome window
            self._outcome_window.append(True)
            if len(self._outcome_window) > self._OUTCOME_WINDOW:
                self._outcome_window.pop(0)
            self._update_error_rate()

            # 2. Assess LOD from all accumulated health signals
            if hasattr(self._svc, 'assess_lod'):
                await self._svc.assess_lod()

            # 3. Update E-Score for each Dog that voted
            dog_votes: dict = p.get("dog_votes") or {}
            for dog_id, vote_score in dog_votes.items():
                self._svc.escore_tracker.update(
                    f"agent:{dog_id}", "JUDGE", float(vote_score)
                )

            # 4. Persist E-Score to DB every 5 judgments (non-blocking)
            self._escore_persist_counter += 1
            if self._escore_persist_counter % 5 == 0 and self._db_pool is not None:
                await self._svc.escore_tracker.persist(self._db_pool)

            # 5. ANTIFRAGILITY — success after stress
            had_stress = len(self._outcome_window) > 1 and any(
                not ok for ok in self._outcome_window[:-1]
            )
            if had_stress and hasattr(self._svc, 'signal_axiom'):
                await self._svc.signal_axiom(
                    "ANTIFRAGILITY", "judgment_intelligence"
                )

            logger.debug(
                "JUDGMENT_CREATED→INTELLIGENCE: dogs=%d, LOD=%s",
                len(dog_votes),
                self._svc.lod_controller.current.name,
            )

        except CynicError:
            logger.debug("handler error", exc_info=True)

    async def _on_judgment_failed(self, event: Event) -> None:
        """JUDGMENT_FAILED → error tracking, LOD assessment, harsh EScore."""
        try:
            # 1. Update outcome window
            self._outcome_window.append(False)
            if len(self._outcome_window) > self._OUTCOME_WINDOW:
                self._outcome_window.pop(0)
            self._update_error_rate()

            # 2. Assess LOD from health
            if hasattr(self._svc, 'assess_lod'):
                await self._svc.assess_lod()

            # 3. Harsh EScore — total failure
            self._svc.escore_tracker.update("agent:cynic", "JUDGE", 0.0)
            self._svc.escore_tracker.update("agent:cynic", "HOLD", GROWL_MIN)

            logger.warning(
                "JUDGMENT_FAILED → LOD=%s → JUDGE=0.0 HOLD=%.1f",
                self._svc.lod_controller.current.name,
                GROWL_MIN,
            )
        except CynicError:
            logger.debug("handler error", exc_info=True)

    async def _on_judgment_for_compressor(self, event: Event) -> None:
        """JUDGMENT_CREATED → compressor summary + checkpoint."""
        try:
            p = event.payload or {}
            verdict = p.get("verdict", "?")
            q = p.get("q_score", 0.0)
            sk = p.get("state_key", "")
            preview = str(p.get("content_preview", ""))[:120].replace("\n", " ")

            summary = f"[{verdict} Q={q:.1f}] {sk}: {preview}"
            self._compressor.add(summary)

            # Checkpoint every CHECKPOINT_EVERY judgments
            self._checkpoint_counter += 1
            if self._checkpoint_counter % CHECKPOINT_EVERY == 0:
                _session_checkpoint.save(self._compressor)

        except CynicError:
            logger.debug("handler error", exc_info=True)