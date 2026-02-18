"""
CYNIC PostgreSQL Storage — asyncpg + φ-bound DB constraints

LAW 5: Database constraints MIRROR φ-bounds in Pydantic models.
  - q_score CHECK (q_score >= 0 AND q_score <= 61.8)
  - confidence CHECK (confidence >= 0 AND confidence <= 0.618)
  - verdict CHECK (verdict IN ('HOWL', 'WAG', 'GROWL', 'BARK'))

Use asyncpg directly — NOT SQLAlchemy ORM. Direct SQL = transparency.

Connection pool size: F(6)=8 min, F(8)=21 max (Fibonacci-aligned).
"""
from __future__ import annotations

import logging
import os
from contextlib import asynccontextmanager
from typing import Any, AsyncIterator, Dict, List, Optional

import asyncpg
from asyncpg import Pool, Connection

from cynic.core.phi import fibonacci, MAX_Q_SCORE, MAX_CONFIDENCE, PHI_INV

logger = logging.getLogger("cynic.storage.postgres")

# Pool size aligned with Fibonacci
POOL_MIN = fibonacci(6)   # 8 connections
POOL_MAX = fibonacci(8)   # 21 connections


# ════════════════════════════════════════════════════════════════════════════
# CONNECTION POOL SINGLETON
# ════════════════════════════════════════════════════════════════════════════

_pool: Optional[Pool] = None


async def get_pool(dsn: Optional[str] = None) -> Pool:
    """
    Get (or create) the shared asyncpg connection pool.

    DSN priority:
      1. Argument `dsn` (for tests)
      2. CYNIC_DATABASE_URL env var
      3. DATABASE_URL env var (Render compat)
    """
    global _pool
    if _pool is not None:
        return _pool

    dsn = dsn or os.environ.get("CYNIC_DATABASE_URL") or os.environ.get("DATABASE_URL")
    if not dsn:
        raise RuntimeError(
            "No database URL configured. "
            "Set CYNIC_DATABASE_URL or DATABASE_URL environment variable."
        )

    logger.info("Creating asyncpg pool (min=%d, max=%d)", POOL_MIN, POOL_MAX)
    _pool = await asyncpg.create_pool(
        dsn=dsn,
        min_size=POOL_MIN,
        max_size=POOL_MAX,
        command_timeout=30.0,
        max_inactive_connection_lifetime=300.0,  # F(7)×60=780s ÷ 2.6 ≈ 300s
    )
    return _pool


async def close_pool() -> None:
    """Close the pool gracefully. Call on shutdown."""
    global _pool
    if _pool is not None:
        await _pool.close()
        _pool = None
        logger.info("asyncpg pool closed")


@asynccontextmanager
async def acquire() -> AsyncIterator[Connection]:
    """Acquire a connection from the pool."""
    pool = await get_pool()
    async with pool.acquire() as conn:
        yield conn


# ════════════════════════════════════════════════════════════════════════════
# SCHEMA CREATION (φ-bound CHECK constraints)
# ════════════════════════════════════════════════════════════════════════════

SCHEMA_SQL = f"""
-- CYNIC Core Schema
-- φ-bound constraints enforced at DB level (mirrors Pydantic validators)

-- Judgments table
CREATE TABLE IF NOT EXISTS judgments (
    judgment_id     TEXT        PRIMARY KEY,
    cell_id         TEXT        NOT NULL,
    reality         TEXT        NOT NULL CHECK (reality IN ('CODE','SOLANA','MARKET','SOCIAL','HUMAN','CYNIC','COSMOS')),
    analysis        TEXT        NOT NULL CHECK (analysis IN ('PERCEIVE','JUDGE','DECIDE','ACT','LEARN','ACCOUNT','EMERGE')),
    time_dim        TEXT        NOT NULL DEFAULT 'PRESENT',
    lod             INTEGER     NOT NULL DEFAULT 1 CHECK (lod BETWEEN 0 AND 3),
    consciousness   INTEGER     NOT NULL DEFAULT 0 CHECK (consciousness BETWEEN 0 AND 6),
    q_score         REAL        NOT NULL CHECK (q_score >= 0 AND q_score <= {MAX_Q_SCORE}),
    verdict         TEXT        NOT NULL CHECK (verdict IN ('HOWL','WAG','GROWL','BARK')),
    confidence      REAL        NOT NULL CHECK (confidence >= 0 AND confidence <= {MAX_CONFIDENCE}),
    axiom_scores    JSONB       NOT NULL DEFAULT '{{}}',
    active_axioms   JSONB       NOT NULL DEFAULT '[]',
    dog_votes       JSONB       NOT NULL DEFAULT '{{}}',
    consensus_votes INTEGER     NOT NULL DEFAULT 0,
    consensus_quorum INTEGER    NOT NULL DEFAULT 7,
    consensus_reached BOOLEAN   NOT NULL DEFAULT FALSE,
    cost_usd        REAL        NOT NULL DEFAULT 0 CHECK (cost_usd >= 0),
    llm_calls       INTEGER     NOT NULL DEFAULT 0,
    llm_tokens      INTEGER     NOT NULL DEFAULT 0,
    residual_variance REAL      NOT NULL DEFAULT 0,
    unnameable_detected BOOLEAN NOT NULL DEFAULT FALSE,
    duration_ms     REAL        NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Cells table (what was judged)
CREATE TABLE IF NOT EXISTS cells (
    cell_id         TEXT        PRIMARY KEY,
    reality         TEXT        NOT NULL,
    analysis        TEXT        NOT NULL,
    time_dim        TEXT        NOT NULL DEFAULT 'PRESENT',
    content_hash    TEXT,                           -- sha256 of content for dedup
    context         TEXT        NOT NULL DEFAULT '',
    novelty         REAL        CHECK (novelty BETWEEN 0 AND 1),
    complexity      REAL        CHECK (complexity BETWEEN 0 AND 1),
    risk            REAL        CHECK (risk BETWEEN 0 AND 1),
    budget_usd      REAL        CHECK (budget_usd >= 0),
    llm_model       TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Learning events table (11 learning loops)
CREATE TABLE IF NOT EXISTS learning_events (
    event_id        TEXT        PRIMARY KEY,
    loop_name       TEXT        NOT NULL,
    judgment_id     TEXT        REFERENCES judgments(judgment_id),
    state_key       TEXT        NOT NULL,
    action          TEXT        NOT NULL,
    reward          REAL        NOT NULL,
    q_delta         REAL        NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Q-Table (Q-Learning state-action values)
CREATE TABLE IF NOT EXISTS q_table (
    state_key       TEXT        NOT NULL,
    action          TEXT        NOT NULL,
    q_value         REAL        NOT NULL DEFAULT 0,
    visit_count     INTEGER     NOT NULL DEFAULT 0,
    last_updated    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (state_key, action)
);

-- E-Score (Agent reputation)
CREATE TABLE IF NOT EXISTS e_scores (
    agent_id        TEXT        PRIMARY KEY,
    total           REAL        NOT NULL CHECK (total BETWEEN 0 AND 100),
    burn_score      REAL        NOT NULL DEFAULT 0,
    build_score     REAL        NOT NULL DEFAULT 0,
    judge_score     REAL        NOT NULL DEFAULT 0,
    run_score       REAL        NOT NULL DEFAULT 0,
    social_score    REAL        NOT NULL DEFAULT 0,
    graph_score     REAL        NOT NULL DEFAULT 0,
    hold_score      REAL        NOT NULL DEFAULT 0,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- LLM Benchmark results (per Dog × Task type × LLM)
CREATE TABLE IF NOT EXISTS llm_benchmarks (
    benchmark_id    TEXT        PRIMARY KEY DEFAULT gen_random_uuid()::text,
    dog_id          TEXT        NOT NULL,
    task_type       TEXT        NOT NULL,
    llm_id          TEXT        NOT NULL,
    quality_score   REAL        NOT NULL CHECK (quality_score BETWEEN 0 AND 1),
    speed_score     REAL        NOT NULL CHECK (speed_score BETWEEN 0 AND 1),
    cost_score      REAL        NOT NULL CHECK (cost_score BETWEEN 0 AND 1),
    composite_score REAL        NOT NULL CHECK (composite_score BETWEEN 0 AND 1),
    latency_ms      REAL        NOT NULL,
    cost_usd        REAL        NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Consciousness state snapshots (organism health history)
CREATE TABLE IF NOT EXISTS consciousness_snapshots (
    snapshot_id     TEXT        PRIMARY KEY,
    active_level    TEXT        NOT NULL,
    gradient        INTEGER     NOT NULL CHECK (gradient BETWEEN 0 AND 6),
    total_cycles    INTEGER     NOT NULL DEFAULT 0,
    reflex_cycles   INTEGER     NOT NULL DEFAULT 0,
    micro_cycles    INTEGER     NOT NULL DEFAULT 0,
    macro_cycles    INTEGER     NOT NULL DEFAULT 0,
    meta_cycles     INTEGER     NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_judgments_reality   ON judgments (reality);
CREATE INDEX IF NOT EXISTS idx_judgments_verdict   ON judgments (verdict);
CREATE INDEX IF NOT EXISTS idx_judgments_created   ON judgments (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_learning_loop       ON learning_events (loop_name);
CREATE INDEX IF NOT EXISTS idx_learning_state      ON learning_events (state_key);
CREATE INDEX IF NOT EXISTS idx_llm_bench_dog       ON llm_benchmarks (dog_id, task_type);
"""


async def create_schema(dsn: Optional[str] = None) -> None:
    """Create all tables and indexes. Idempotent (IF NOT EXISTS)."""
    async with acquire() as conn:
        await conn.execute(SCHEMA_SQL)
        logger.info("CYNIC schema created/verified")


# ════════════════════════════════════════════════════════════════════════════
# JUDGMENT REPOSITORY
# ════════════════════════════════════════════════════════════════════════════

class JudgmentRepository:
    """CRUD for judgments. φ-bound enforced by DB constraints."""

    async def save(self, judgment: Dict[str, Any]) -> None:
        """Persist a judgment (upsert by judgment_id)."""
        async with acquire() as conn:
            await conn.execute("""
                INSERT INTO judgments (
                    judgment_id, cell_id, reality, analysis, time_dim,
                    lod, consciousness, q_score, verdict, confidence,
                    axiom_scores, active_axioms, dog_votes,
                    consensus_votes, consensus_quorum, consensus_reached,
                    cost_usd, llm_calls, llm_tokens,
                    residual_variance, unnameable_detected, duration_ms
                ) VALUES (
                    $1, $2, $3, $4, $5,
                    $6, $7, $8, $9, $10,
                    $11::jsonb, $12::jsonb, $13::jsonb,
                    $14, $15, $16,
                    $17, $18, $19,
                    $20, $21, $22
                )
                ON CONFLICT (judgment_id) DO UPDATE SET
                    q_score = EXCLUDED.q_score,
                    verdict = EXCLUDED.verdict,
                    duration_ms = EXCLUDED.duration_ms
            """,
                judgment["judgment_id"],
                judgment.get("cell_id", ""),
                judgment.get("reality", "CODE"),
                judgment.get("analysis", "JUDGE"),
                judgment.get("time_dim", "PRESENT"),
                judgment.get("lod", 1),
                judgment.get("consciousness", 0),
                judgment["q_score"],
                judgment["verdict"],
                judgment["confidence"],
                str(judgment.get("axiom_scores", {})),
                str(judgment.get("active_axioms", [])),
                str(judgment.get("dog_votes", {})),
                judgment.get("consensus_votes", 0),
                judgment.get("consensus_quorum", 7),
                judgment.get("consensus_reached", False),
                judgment.get("cost_usd", 0.0),
                judgment.get("llm_calls", 0),
                judgment.get("llm_tokens", 0),
                judgment.get("residual_variance", 0.0),
                judgment.get("unnameable_detected", False),
                judgment.get("duration_ms", 0.0),
            )

    async def get(self, judgment_id: str) -> Optional[Dict[str, Any]]:
        """Fetch a judgment by ID."""
        async with acquire() as conn:
            row = await conn.fetchrow(
                "SELECT * FROM judgments WHERE judgment_id = $1",
                judgment_id,
            )
            return dict(row) if row else None

    async def recent(self, reality: Optional[str] = None, limit: int = 55) -> List[Dict[str, Any]]:
        """Fetch recent judgments, optionally filtered by reality."""
        async with acquire() as conn:
            if reality:
                rows = await conn.fetch(
                    "SELECT * FROM judgments WHERE reality=$1 ORDER BY created_at DESC LIMIT $2",
                    reality, limit,
                )
            else:
                rows = await conn.fetch(
                    "SELECT * FROM judgments ORDER BY created_at DESC LIMIT $1",
                    limit,
                )
            return [dict(r) for r in rows]

    async def stats(self) -> Dict[str, Any]:
        """Quick stats for health dashboard."""
        async with acquire() as conn:
            total = await conn.fetchval("SELECT COUNT(*) FROM judgments")
            by_verdict = await conn.fetch(
                "SELECT verdict, COUNT(*) as cnt FROM judgments GROUP BY verdict"
            )
            avg_q = await conn.fetchval("SELECT AVG(q_score) FROM judgments")
            return {
                "total": total,
                "by_verdict": {r["verdict"]: r["cnt"] for r in by_verdict},
                "avg_q_score": round(avg_q or 0, 2),
            }


# ════════════════════════════════════════════════════════════════════════════
# Q-TABLE REPOSITORY
# ════════════════════════════════════════════════════════════════════════════

class QTableRepository:
    """Persistent Q-Table for Q-Learning (state_key × action → q_value)."""

    async def get(self, state_key: str, action: str) -> float:
        """Get Q-value, returns 0.0 if not found."""
        async with acquire() as conn:
            val = await conn.fetchval(
                "SELECT q_value FROM q_table WHERE state_key=$1 AND action=$2",
                state_key, action,
            )
            return float(val) if val is not None else 0.0

    async def update(
        self,
        state_key: str,
        action: str,
        q_value: float,
    ) -> None:
        """Upsert Q-value."""
        async with acquire() as conn:
            await conn.execute("""
                INSERT INTO q_table (state_key, action, q_value, visit_count, last_updated)
                VALUES ($1, $2, $3, 1, NOW())
                ON CONFLICT (state_key, action) DO UPDATE SET
                    q_value = EXCLUDED.q_value,
                    visit_count = q_table.visit_count + 1,
                    last_updated = NOW()
            """, state_key, action, q_value)

    async def get_all_actions(self, state_key: str) -> Dict[str, float]:
        """Get all Q-values for a given state."""
        async with acquire() as conn:
            rows = await conn.fetch(
                "SELECT action, q_value FROM q_table WHERE state_key=$1",
                state_key,
            )
            return {r["action"]: float(r["q_value"]) for r in rows}


# ════════════════════════════════════════════════════════════════════════════
# LEARNING EVENTS REPOSITORY
# ════════════════════════════════════════════════════════════════════════════

class LearningRepository:
    """Persist learning events from all 11 learning loops."""

    async def save(self, event: Dict[str, Any]) -> None:
        async with acquire() as conn:
            await conn.execute("""
                INSERT INTO learning_events
                    (event_id, loop_name, judgment_id, state_key, action, reward, q_delta)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (event_id) DO NOTHING
            """,
                event["event_id"],
                event["loop_name"],
                event.get("judgment_id"),
                event["state_key"],
                event["action"],
                event["reward"],
                event.get("q_delta", 0.0),
            )

    async def recent_for_loop(self, loop_name: str, limit: int = 34) -> List[Dict[str, Any]]:
        async with acquire() as conn:
            rows = await conn.fetch(
                "SELECT * FROM learning_events WHERE loop_name=$1 ORDER BY created_at DESC LIMIT $2",
                loop_name, limit,
            )
            return [dict(r) for r in rows]

    async def loop_stats(self) -> Dict[str, int]:
        """Count events per learning loop."""
        async with acquire() as conn:
            rows = await conn.fetch(
                "SELECT loop_name, COUNT(*) as cnt FROM learning_events GROUP BY loop_name"
            )
            return {r["loop_name"]: r["cnt"] for r in rows}


# ════════════════════════════════════════════════════════════════════════════
# LLM BENCHMARK REPOSITORY
# ════════════════════════════════════════════════════════════════════════════

class BenchmarkRepository:
    """Persist LLM benchmark results for routing decisions."""

    async def save(self, result: Dict[str, Any]) -> None:
        async with acquire() as conn:
            await conn.execute("""
                INSERT INTO llm_benchmarks
                    (benchmark_id, dog_id, task_type, llm_id,
                     quality_score, speed_score, cost_score, composite_score,
                     latency_ms, cost_usd)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            """,
                result.get("benchmark_id", str(__import__("uuid").uuid4())),
                result["dog_id"],
                result["task_type"],
                result["llm_id"],
                result["quality_score"],
                result["speed_score"],
                result["cost_score"],
                result["composite_score"],
                result["latency_ms"],
                result.get("cost_usd", 0.0),
            )

    async def best_llm_for(self, dog_id: str, task_type: str) -> Optional[str]:
        """Return the LLM with the highest composite score (EMA of recent runs)."""
        async with acquire() as conn:
            row = await conn.fetchrow("""
                SELECT llm_id, AVG(composite_score) as avg_score
                FROM llm_benchmarks
                WHERE dog_id=$1 AND task_type=$2
                  AND created_at > NOW() - INTERVAL '7 days'
                GROUP BY llm_id
                ORDER BY avg_score DESC
                LIMIT 1
            """, dog_id, task_type)
            return row["llm_id"] if row else None

    async def get_all(self) -> List[Dict[str, Any]]:
        """Load all benchmarks for warm-start (most recent first)."""
        async with acquire() as conn:
            rows = await conn.fetch("""
                SELECT dog_id, task_type, llm_id,
                       quality_score, speed_score, cost_score
                FROM llm_benchmarks
                ORDER BY created_at DESC
            """)
            return [dict(r) for r in rows]

    async def matrix(self) -> List[Dict[str, Any]]:
        """Full benchmark matrix (dog × task × llm → score)."""
        async with acquire() as conn:
            rows = await conn.fetch("""
                SELECT dog_id, task_type, llm_id,
                       AVG(composite_score) as avg_composite,
                       AVG(latency_ms) as avg_latency_ms,
                       COUNT(*) as runs
                FROM llm_benchmarks
                WHERE created_at > NOW() - INTERVAL '7 days'
                GROUP BY dog_id, task_type, llm_id
                ORDER BY dog_id, task_type, avg_composite DESC
            """)
            return [dict(r) for r in rows]


# ════════════════════════════════════════════════════════════════════════════
# REPOSITORY FACTORY
# ════════════════════════════════════════════════════════════════════════════

_judgment_repo: Optional[JudgmentRepository] = None
_qtable_repo: Optional[QTableRepository] = None
_learning_repo: Optional[LearningRepository] = None
_benchmark_repo: Optional[BenchmarkRepository] = None


def judgments() -> JudgmentRepository:
    global _judgment_repo
    if _judgment_repo is None:
        _judgment_repo = JudgmentRepository()
    return _judgment_repo


def qtable() -> QTableRepository:
    global _qtable_repo
    if _qtable_repo is None:
        _qtable_repo = QTableRepository()
    return _qtable_repo


def learning() -> LearningRepository:
    global _learning_repo
    if _learning_repo is None:
        _learning_repo = LearningRepository()
    return _learning_repo


def benchmarks() -> BenchmarkRepository:
    global _benchmark_repo
    if _benchmark_repo is None:
        _benchmark_repo = BenchmarkRepository()
    return _benchmark_repo
