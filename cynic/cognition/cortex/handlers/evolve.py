"""
EvolveHandler — L4 META cycle (self-benchmark via probe cells).

Extracted from JudgeOrchestrator.evolve().

Responsibility:
- Run 5 canonical probe cells at REFLEX level (<200ms total)
- Compare q_scores against expected ranges
- Detect regression (>20% pass_rate drop)
- Keep rolling history F(8)=21
- Persist results to benchmark_registry
- Emit META_CYCLE event
"""
from __future__ import annotations

import logging
import time
from typing import Any, Optional

from cynic.cognition.cortex.handlers.base import BaseHandler, HandlerResult
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.events_schema import MetaCyclePayload

logger = logging.getLogger("cynic.cognition.cortex.handlers.evolve")


class EvolveHandler(BaseHandler):
    """
    L4 META cycle handler (self-benchmark).

    Injects:
    - orchestrator: JudgeOrchestrator (to call run() on probe cells)
    - benchmark_registry: BenchmarkRegistry (optional, for persistence)
    """

    handler_id = "evolve"
    version = "1.0"
    description = "L4 META cycle: self-benchmark via probe cells, regression detection"

    def __init__(
        self,
        orchestrator: Optional[Any] = None,
        benchmark_registry: Optional[Any] = None,
        evolve_history: Optional[list[dict]] = None,
    ) -> None:
        self.orchestrator = orchestrator
        self.benchmark_registry = benchmark_registry
        self._evolve_history = evolve_history or []

    async def execute(self, **kwargs: Any) -> HandlerResult:
        """
        Execute L4 META cycle (self-benchmark).

        Returns:
            HandlerResult with summary dict in output (pass_rate, regression, results)
        """
        t0 = time.perf_counter()
        try:
            summary = await self._evolve_cycle()
            duration_ms = (time.perf_counter() - t0) * 1000

            return HandlerResult(
                success=True,
                handler_id=self.handler_id,
                output=summary,
                duration_ms=duration_ms,
                metadata={
                    "pass_rate": summary["pass_rate"],
                    "regression_detected": summary["regression"],
                    "probes_passed": f"{summary['pass_count']}/{summary['total']}",
                },
            )
        except EventBusError as e:
            duration_ms = (time.perf_counter() - t0) * 1000
            self._log_error("execute_evolve", e)
            return HandlerResult(
                success=False,
                handler_id=self.handler_id,
                error=str(e),
                duration_ms=duration_ms,
            )

    async def _evolve_cycle(self) -> dict[str, Any]:
        """
        L4 META: Self-benchmark via 5 canonical probe cells.

        Runs each probe at REFLEX level (no LLM, <200ms total), compares
        q_scores against expected ranges, detects regression vs previous call.

        Returns summary dict:
          pass_rate, pass_count, total, regression, results[]
        """
        from cynic.cognition.cortex.probes import PROBE_CELLS, ProbeResult

        if not self.orchestrator:
            raise ValueError("EvolveHandler requires orchestrator instance")

        results: list[ProbeResult] = []

        for probe in PROBE_CELLS:
            t0 = time.time()
            try:
                judgment = await self.orchestrator.run(
                    probe["cell"],
                    level=ConsciousnessLevel.REFLEX,
                )
                elapsed = (time.time() - t0) * 1000
                passed = probe["min_q"] <= judgment.q_score <= probe["max_q"]
                results.append(
                    ProbeResult(
                        name=probe["name"],
                        q_score=judgment.q_score,
                        verdict=judgment.verdict,
                        expected_min=probe["min_q"],
                        expected_max=probe["max_q"],
                        passed=passed,
                        duration_ms=elapsed,
                    )
                )
            except CynicError as exc:
                elapsed = (time.time() - t0) * 1000
                logger.warning("evolve() probe %s failed: %s", probe["name"], exc)
                results.append(
                    ProbeResult(
                        name=probe["name"],
                        q_score=0.0,
                        verdict="BARK",
                        expected_min=probe["min_q"],
                        expected_max=probe["max_q"],
                        passed=False,
                        duration_ms=elapsed,
                        error=str(exc),
                    )
                )

        pass_count = sum(1 for r in results if r.passed)
        pass_rate = pass_count / len(results) if results else 0.0

        # Regression: pass_rate dropped >20% vs previous evolve()
        regression = False
        if self._evolve_history:
            prev_rate = self._evolve_history[-1]["pass_rate"]
            regression = pass_rate < prev_rate - 0.20

        summary: dict[str, Any] = {
            "timestamp": time.time(),
            "pass_rate": round(pass_rate, 3),
            "pass_count": pass_count,
            "total": len(results),
            "regression": regression,
            "results": [r.to_dict() for r in results],
        }

        # Keep last F(8)=21 evolve() snapshots
        self._evolve_history.append(summary)
        if len(self._evolve_history) > 21:
            self._evolve_history.pop(0)

        # Persist probe runs to DB (no-op if benchmark_registry not wired)
        if self.benchmark_registry is not None:
            try:
                await self.benchmark_registry.record_evolve(results)
            except CynicError as exc:
                logger.warning("BenchmarkRegistry.record_evolve() failed: %s", exc)

        await get_core_bus().emit(
            Event.typed(
                CoreEvent.META_CYCLE,
                MetaCyclePayload(evolve=summary),
            )
        )

        log_line = "evolve() %d/%d probes passed (%.0f%%)%s"
        logger.info(
            log_line,
            pass_count,
            len(results),
            pass_rate * 100,
            " -- REGRESSION DETECTED" if regression else "",
        )

        return summary
