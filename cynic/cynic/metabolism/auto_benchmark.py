"""
CYNIC AutoBenchmark — Periodic LLM latency + reachability probe (T09)

Runs every F(10)×60 = 3300s (55 min). For each available LLM adapter:
  1. Calls the adapter with a fixed probe prompt
  2. Measures latency_ms and speed (tokens/s)
  3. Records in LLMRegistry via update_benchmark()

quality_score starts conservative (WAG_MIN / 2 ≈ 30.9) and will EMA-converge
toward real CYNIC Q-Scores as live judgments accumulate in the registry.

Disabled via env: CYNIC_AUTOBENCH=0
"""
from __future__ import annotations

import asyncio
import logging
import os
from typing import Any


from cynic.core.phi import fibonacci, WAG_MIN

logger = logging.getLogger("cynic.metabolism.auto_benchmark")

# F(10) × 60s = 3300s (55 min) — φ-aligned learning interval
_INTERVAL_S: int = fibonacci(10) * 60  # 3300

# Probe prompts — one per task_type we want to benchmark
# Minimal to avoid spending real LLM budget on the probe
_PROBES: list[tuple[str, str]] = [
    ("temporal_mcts", "Reply with one word: ready"),
    ("general",       "Reply with one word: ok"),
]


class AutoBenchmark:
    """
    Background LLM benchmark probe — runs every 55 min.

    Calls every available generation adapter with fixed probe prompts, measures
    latency and speed, and records the result in LLMRegistry so get_best_for()
    can make empirical routing decisions.

    Wired in server.py lifespan: after scheduler.start(), before yield.
    Not a PerceiveWorker — owns its own async loop and calls adapters directly.
    """

    def __init__(self, registry: Any) -> None:
        self._registry = registry
        self._task: asyncio.Optional[Task] = None
        self._runs = 0

    # ── Lifecycle ─────────────────────────────────────────────────────────────

    def start(self) -> None:
        """Launch background benchmark loop (no-op if CYNIC_AUTOBENCH=0)."""
        if os.environ.get("CYNIC_AUTOBENCH", "1") == "0":
            logger.info("AutoBenchmark disabled (CYNIC_AUTOBENCH=0)")
            return
        self._task = asyncio.ensure_future(self._loop())
        self._task.set_name("cynic.auto_benchmark")
        logger.info("AutoBenchmark started (interval=%ds)", _INTERVAL_S)

    async def stop(self) -> None:
        """Cancel background loop and wait for graceful exit."""
        if self._task and not self._task.done():
            self._task.cancel()
            await asyncio.gather(self._task, return_exceptions=True)

    # ── Stats ─────────────────────────────────────────────────────────────────

    def stats(self) -> dict:
        return {
            "enabled":    os.environ.get("CYNIC_AUTOBENCH", "1") != "0",
            "runs":       self._runs,
            "interval_s": _INTERVAL_S,
        }

    # ── Public trigger ────────────────────────────────────────────────────────

    async def run_once(self) -> int:
        """
        Run one probe round immediately.

        Used by POST /auto-benchmark/run and tests.
        Returns number of benchmark entries recorded.
        """
        return await self._probe_all()

    # ── Private ───────────────────────────────────────────────────────────────

    async def _loop(self) -> None:
        """Sleep interval_s, probe all adapters, repeat."""
        while True:
            try:
                await asyncio.sleep(_INTERVAL_S)
                await self._probe_all()
            except asyncio.CancelledError:
                raise
            except Exception as exc:
                logger.warning("AutoBenchmark loop error: %s", exc)

    async def _probe_all(self) -> int:
        """
        Probe every available generation LLM with each probe task.

        Returns count of benchmark entries successfully recorded.
        """
        from cynic.llm.adapter import BenchmarkResult, LLMRequest

        adapters = self._registry.get_available_for_generation()
        if not adapters:
            logger.info("AutoBenchmark: no LLMs available (round %d)", self._runs)
            return 0

        completed = 0
        for adapter in adapters:
            for task_type, prompt in _PROBES:
                try:
                    req = LLMRequest(prompt=prompt, max_tokens=64, temperature=0.0)
                    resp = await adapter.complete_safe(req)

                    # speed_score: tokens/s normalized to [0, 1], 50 tps = baseline
                    tps = resp.tokens_per_second
                    speed_score = min(1.0, tps / 50.0) if tps > 0 else 0.1

                    # quality_score: conservative until real judgments EMA-update
                    quality_score = WAG_MIN / 2.0 if resp.is_success else 0.0

                    # cost_score: 1/cost normalized; zero cost → neutral 0.5
                    cost_score = (
                        min(1.0, 0.001 / max(resp.cost_usd, 1e-9))
                        if resp.cost_usd > 0
                        else 0.5
                    )

                    self._registry.update_benchmark(
                        dog_id="AUTO",
                        task_type=task_type,
                        llm_id=adapter.adapter_id,
                        result=BenchmarkResult(
                            llm_id=adapter.adapter_id,
                            dog_id="AUTO",
                            task_type=task_type,
                            quality_score=quality_score,
                            speed_score=speed_score,
                            cost_score=cost_score,
                            error_rate=0.0 if resp.is_success else 1.0,
                        ),
                    )
                    completed += 1
                    logger.debug(
                        "AutoBenchmark %s/%s: lat=%.0fms speed=%.2f quality=%.1f",
                        adapter.adapter_id, task_type,
                        resp.latency_ms, speed_score, quality_score,
                    )
                except asyncio.CancelledError:
                    raise
                except Exception as exc:
                    logger.debug(
                        "AutoBenchmark probe error %s/%s: %s",
                        adapter.adapter_id, task_type, exc,
                    )

        self._runs += 1
        logger.info(
            "AutoBenchmark round %d: %d probes across %d LLMs",
            self._runs, completed, len(adapters),
        )
        return completed
