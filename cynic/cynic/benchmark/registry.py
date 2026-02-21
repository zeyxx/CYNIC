"""
CYNIC BenchmarkRegistry — Probe Run Persistence & Drift Detection

Records each evolve() probe run to PostgreSQL for:
  1. Historical pass_rate tracking (drift detection)
  2. Experiment #0 baseline (alpha,gamma grid search)
  3. Long-term quality trending

Schema (plain PostgreSQL — TimescaleDB when rows > 1M):
  probe_runs:          one row per probe per evolve() call
  benchmark_snapshots: rolling aggregate per probe (rolling window)

phi constraints at DB level:
  q_score CHECK (q_score >= 0 AND q_score <= 100)
  verdict CHECK (verdict IN ('HOWL','WAG','GROWL','BARK'))
"""
from __future__ import annotations

import logging
import statistics
from typing import Any, TYPE_CHECKING


if TYPE_CHECKING:
    from cynic.cognition.cortex.probes import ProbeResult

logger = logging.getLogger("cynic.benchmark")

# ── DDL ──────────────────────────────────────────────────────────────────────

_CREATE_TABLES_SQL = """
CREATE TABLE IF NOT EXISTS probe_runs (
    id          BIGSERIAL PRIMARY KEY,
    run_ts      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    probe_id    VARCHAR(10)  NOT NULL,
    probe_name  VARCHAR(100) NOT NULL,
    q_score     DOUBLE PRECISION NOT NULL
                    CHECK (q_score >= 0 AND q_score <= 100),
    verdict     VARCHAR(10)  NOT NULL
                    CHECK (verdict IN ('HOWL', 'WAG', 'GROWL', 'BARK')),
    passed      BOOLEAN NOT NULL,
    duration_ms DOUBLE PRECISION NOT NULL,
    source      VARCHAR(50)  NOT NULL DEFAULT 'evolve',
    error       TEXT         NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS benchmark_snapshots (
    id           BIGSERIAL PRIMARY KEY,
    snapshot_ts  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    probe_id     VARCHAR(10)  NOT NULL,
    probe_name   VARCHAR(100) NOT NULL,
    pass_rate    DOUBLE PRECISION NOT NULL,
    mean_q       DOUBLE PRECISION NOT NULL,
    std_q        DOUBLE PRECISION NOT NULL,
    run_count    INTEGER      NOT NULL,
    source       VARCHAR(50)  NOT NULL DEFAULT 'evolve'
);

CREATE INDEX IF NOT EXISTS idx_probe_runs_ts
    ON probe_runs (run_ts DESC);
CREATE INDEX IF NOT EXISTS idx_probe_runs_probe
    ON probe_runs (probe_id, run_ts DESC);
CREATE INDEX IF NOT EXISTS idx_snapshots_probe
    ON benchmark_snapshots (probe_id, snapshot_ts DESC);
"""

_INSERT_PROBE_RUN = """
INSERT INTO probe_runs
    (probe_id, probe_name, q_score, verdict, passed, duration_ms, source, error)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
"""

_INSERT_SNAPSHOT = """
INSERT INTO benchmark_snapshots
    (probe_id, probe_name, pass_rate, mean_q, std_q, run_count, source)
VALUES ($1, $2, $3, $4, $5, $6, $7)
"""

_LAST_N_RUNS_SQL = """
WITH ranked AS (
    SELECT
        probe_id, probe_name, q_score, passed,
        ROW_NUMBER() OVER (PARTITION BY probe_id ORDER BY run_ts DESC) AS rn
    FROM probe_runs
    WHERE source = $1
)
SELECT probe_id, probe_name, q_score, passed
FROM ranked
WHERE rn <= $2
ORDER BY probe_id, rn
"""

_LAST_2_SNAPSHOTS_SQL = """
WITH ranked AS (
    SELECT
        probe_id, probe_name, pass_rate, mean_q, snapshot_ts,
        ROW_NUMBER() OVER (PARTITION BY probe_id ORDER BY snapshot_ts DESC) AS rn
    FROM benchmark_snapshots
)
SELECT probe_id, probe_name, pass_rate, mean_q, snapshot_ts, rn
FROM ranked
WHERE rn <= 2
ORDER BY probe_id, rn
"""


# ── BenchmarkRegistry ─────────────────────────────────────────────────────────

class BenchmarkRegistry:
    """
    Persists probe run results and computes drift alerts.

    Usage:
        await BenchmarkRegistry.create_tables(pool)   # once, at startup
        reg = BenchmarkRegistry(pool)

        # after each evolve():
        await reg.record_evolve(probe_results)

        # periodic drift check:
        result = await reg.snapshot(window_runs=10)
        alerts = await reg.drift_alerts(threshold=0.15)
    """

    def __init__(self, pool) -> None:
        self._pool = pool

    # ── DDL (class method — callable before instance exists) ─────────────────

    @staticmethod
    async def create_tables(pool) -> None:
        """Create probe_runs and benchmark_snapshots tables (idempotent)."""
        async with pool.acquire() as conn:
            await conn.execute(_CREATE_TABLES_SQL)
        logger.info("BenchmarkRegistry tables ready")

    # ── Write ─────────────────────────────────────────────────────────────────

    async def record_evolve(
        self,
        results: list[ProbeResult],
        source: str = "evolve",
    ) -> None:
        """
        Batch-insert all probe results from one evolve() call.

        Silently returns if pool is None (no-DB / heuristic mode).
        """
        if self._pool is None:
            return

        rows = [
            (
                r.name.split(":")[0],   # 'P1' from 'P1:clean_code'
                r.name,
                r.q_score,
                r.verdict,
                r.passed,
                r.duration_ms,
                source,
                r.error,
            )
            for r in results
        ]
        async with self._pool.acquire() as conn:
            await conn.executemany(_INSERT_PROBE_RUN, rows)
        logger.debug("BenchmarkRegistry: %d probe runs recorded", len(rows))

    # ── Read / Aggregate ──────────────────────────────────────────────────────

    async def snapshot(
        self,
        window_runs: int = 10,
        source: str = "evolve",
    ) -> dict[str, Any]:
        """
        Aggregate last N runs per probe, save to benchmark_snapshots.

        Returns: {probe_id: {probe_name, pass_rate, mean_q, std_q, run_count}}
        Returns {} if pool is None or no runs exist yet.
        """
        if self._pool is None:
            return {}

        async with self._pool.acquire() as conn:
            rows = await conn.fetch(_LAST_N_RUNS_SQL, source, window_runs)

            # Group by probe_id in Python
            by_probe: dict[str, dict] = {}
            for row in rows:
                pid = row["probe_id"]
                if pid not in by_probe:
                    by_probe[pid] = {
                        "probe_name": row["probe_name"],
                        "q_scores": [],
                        "passed": [],
                    }
                by_probe[pid]["q_scores"].append(row["q_score"])
                by_probe[pid]["passed"].append(row["passed"])

            result: dict[str, Any] = {}
            snapshot_rows = []

            for probe_id, data in by_probe.items():
                q_scores = data["q_scores"]
                passed = data["passed"]
                n = len(q_scores)
                pass_rate = sum(1 for p in passed if p) / n
                mean_q = statistics.mean(q_scores)
                std_q = statistics.stdev(q_scores) if n > 1 else 0.0

                result[probe_id] = {
                    "probe_name": data["probe_name"],
                    "pass_rate": pass_rate,
                    "mean_q": mean_q,
                    "std_q": std_q,
                    "run_count": n,
                }
                snapshot_rows.append(
                    (probe_id, data["probe_name"], pass_rate, mean_q, std_q, n, source)
                )

            if snapshot_rows:
                await conn.executemany(_INSERT_SNAPSHOT, snapshot_rows)

        logger.info(
            "BenchmarkRegistry: snapshot for %d probes (window=%d)", len(result), window_runs
        )
        return result

    async def drift_alerts(
        self,
        threshold: float = 0.15,
    ) -> list[dict[str, Any]]:
        """
        Compare the two most recent snapshots per probe.

        Returns probes where pass_rate dropped >= threshold vs previous snapshot.
        severity: CRITICAL if delta >= 0.30, WARNING otherwise.

        Returns [] if pool is None or insufficient snapshot history.
        """
        if self._pool is None:
            return []

        async with self._pool.acquire() as conn:
            rows = await conn.fetch(_LAST_2_SNAPSHOTS_SQL)

        # Group by probe_id (rows ordered probe_id ASC, rn ASC)
        by_probe: dict[str, list[dict]] = {}
        for row in rows:
            pid = row["probe_id"]
            by_probe.setdefault(pid, []).append(dict(row))

        alerts = []
        for probe_id, snapshots in by_probe.items():
            if len(snapshots) < 2:
                continue
            current = snapshots[0]    # rn=1 (most recent)
            previous = snapshots[1]   # rn=2 (previous)
            delta = previous["pass_rate"] - current["pass_rate"]
            if delta >= threshold:
                alerts.append({
                    "probe_id": probe_id,
                    "probe_name": current["probe_name"],
                    "current_pass_rate": current["pass_rate"],
                    "previous_pass_rate": previous["pass_rate"],
                    "delta": delta,
                    "severity": "CRITICAL" if delta >= 0.30 else "WARNING",
                })

        if alerts:
            logger.warning(
                "BenchmarkRegistry: %d drift alert(s) (threshold=%.0f%%)",
                len(alerts), threshold * 100,
            )
        return alerts
