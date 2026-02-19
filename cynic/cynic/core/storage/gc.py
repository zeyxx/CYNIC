"""
CYNIC StorageGarbageCollector — Disk pressure response (BURN axiom)

Pure SQL. No abstractions. Triggered by DISK_PRESSURE event.
Prunes low-value data first: BARK verdicts, oldest records, oversized buffers.

Prune order (lowest Q-Score value first — BURN axiom):
  1. judgments        — BARK verdicts older than 7 days  (batch=1000)
  2. scholar_buffer   — keep F(11)=89 newest entries
  3. residual_history — keep F(11)=89 newest entries
  4. llm_benchmarks   — records older than 30 days
  5. consciousness_snapshots — keep F(10)=55 newest

φ-derived constants:
  _KEEP_SCHOLAR   = F(11) = 89
  _KEEP_RESIDUAL  = F(11) = 89
  _KEEP_SNAPSHOTS = F(10) = 55
  _PRUNE_BATCH    = 1000  (max rows per table per run — protect DB)
"""
from __future__ import annotations

import logging
import time
from typing import Any, Dict, Optional

from cynic.core.phi import fibonacci

logger = logging.getLogger("cynic.storage.gc")

# Retention window sizes (Fibonacci)
_KEEP_SCHOLAR   = fibonacci(11)  # 89
_KEEP_RESIDUAL  = fibonacci(11)  # 89
_KEEP_SNAPSHOTS = fibonacci(10)  # 55

# Max rows deleted per table per GC run (prevents long-running transactions)
_PRUNE_BATCH = 1_000

# Time-based retention (days)
_BARK_RETENTION_DAYS = 7
_BENCH_RETENTION_DAYS = 30


class StorageGarbageCollector:
    """
    SQL-pure garbage collector for CYNIC's PostgreSQL storage.

    One collect() call runs up to 5 targeted DELETE statements.
    Each DELETE is bounded (_PRUNE_BATCH rows max) to avoid locking the DB.

    Triggered by DISK_PRESSURE event in state.py.
    Also callable directly for testing / manual cleanup.
    """

    def __init__(self) -> None:
        self._runs: int = 0
        self._total_deleted: int = 0
        self._last_run: float = 0.0

    async def collect(self, pool) -> dict[str, Any]:
        """
        Run one GC pass across all 5 tables.

        Returns a summary dict:
          {"judgments": N, "scholar_buffer": N, "residual_history": N,
           "llm_benchmarks": N, "consciousness_snapshots": N, "total": N,
           "duration_ms": N}
        """
        if pool is None:
            logger.debug("StorageGC: no DB pool — skipping")
            return {"total": 0}

        t0 = time.perf_counter()
        self._runs += 1
        results: dict[str, int] = {}

        async with pool.acquire() as conn:

            # ── 1. judgments — BARK verdicts older than N days ─────────────
            # Lowest Q-Score first (burn the worst first)
            r = await conn.execute(f"""
                DELETE FROM judgments
                WHERE judgment_id IN (
                    SELECT judgment_id FROM judgments
                    WHERE verdict = 'BARK'
                      AND created_at < NOW() - INTERVAL '{_BARK_RETENTION_DAYS} days'
                    ORDER BY q_score ASC, created_at ASC
                    LIMIT {_PRUNE_BATCH}
                )
            """)
            results["judgments"] = int(r.split()[-1])

            # ── 2. scholar_buffer — keep F(11)=89 newest ──────────────────
            r = await conn.execute(f"""
                DELETE FROM scholar_buffer
                WHERE id NOT IN (
                    SELECT id FROM scholar_buffer
                    ORDER BY created_at DESC
                    LIMIT {_KEEP_SCHOLAR}
                )
                AND id IN (
                    SELECT id FROM scholar_buffer
                    ORDER BY created_at ASC
                    LIMIT {_PRUNE_BATCH}
                )
            """)
            results["scholar_buffer"] = int(r.split()[-1])

            # ── 3. residual_history — keep F(11)=89 newest ────────────────
            r = await conn.execute(f"""
                DELETE FROM residual_history
                WHERE id NOT IN (
                    SELECT id FROM residual_history
                    ORDER BY observed_at DESC
                    LIMIT {_KEEP_RESIDUAL}
                )
                AND id IN (
                    SELECT id FROM residual_history
                    ORDER BY observed_at ASC
                    LIMIT {_PRUNE_BATCH}
                )
            """)
            results["residual_history"] = int(r.split()[-1])

            # ── 4. llm_benchmarks — older than 30 days ────────────────────
            r = await conn.execute(f"""
                DELETE FROM llm_benchmarks
                WHERE benchmark_id IN (
                    SELECT benchmark_id FROM llm_benchmarks
                    WHERE created_at < NOW() - INTERVAL '{_BENCH_RETENTION_DAYS} days'
                    ORDER BY created_at ASC
                    LIMIT {_PRUNE_BATCH}
                )
            """)
            results["llm_benchmarks"] = int(r.split()[-1])

            # ── 5. consciousness_snapshots — keep F(10)=55 newest ─────────
            r = await conn.execute(f"""
                DELETE FROM consciousness_snapshots
                WHERE snapshot_id NOT IN (
                    SELECT snapshot_id FROM consciousness_snapshots
                    ORDER BY created_at DESC
                    LIMIT {_KEEP_SNAPSHOTS}
                )
                AND snapshot_id IN (
                    SELECT snapshot_id FROM consciousness_snapshots
                    ORDER BY created_at ASC
                    LIMIT {_PRUNE_BATCH}
                )
            """)
            results["consciousness_snapshots"] = int(r.split()[-1])

        total = sum(results.values())
        self._total_deleted += total
        self._last_run = time.time()
        duration_ms = (time.perf_counter() - t0) * 1000

        results["total"] = total
        results["duration_ms"] = round(duration_ms, 1)

        logger.info(
            "StorageGC run #%d: deleted %d rows total in %.1fms — %s",
            self._runs, total, duration_ms,
            {k: v for k, v in results.items() if k not in ("total", "duration_ms") and v > 0},
        )
        return results

    def stats(self) -> dict[str, Any]:
        return {
            "runs": self._runs,
            "total_deleted": self._total_deleted,
            "last_run": self._last_run,
        }
