"""
CYNIC PostgreSQL Storage — asyncpg + φ-bound DB constraints

LAW 5: Database constraints MIRROR φ-bounds in Pydantic models.
  - q_score CHECK (q_score >= 0 AND q_score <= 100)
  - confidence CHECK (confidence >= 0 AND confidence <= 0.618)
  - verdict CHECK (verdict IN ('HOWL', 'WAG', 'GROWL', 'BARK'))

Use asyncpg directly — NOT SQLAlchemy ORM. Direct SQL = transparency.

Connection pool size: F(6)=8 min, F(8)=21 max (Fibonacci-aligned).
"""
from __future__ import annotations

import logging
import os
from contextlib import asynccontextmanager
from typing import Any, Dict, List, Optional
from collections.abc import AsyncIterator

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

_pool: Pool | None = None


async def get_pool(dsn: str | None = None) -> Pool:
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

-- Residual history (rolling window persisted for warm-start)
CREATE TABLE IF NOT EXISTS residual_history (
    id              BIGSERIAL   PRIMARY KEY,
    judgment_id     TEXT        NOT NULL,
    residual        REAL        NOT NULL CHECK (residual BETWEEN 0 AND 1),
    reality         TEXT        NOT NULL,
    analysis        TEXT        NOT NULL,
    unnameable      BOOLEAN     NOT NULL DEFAULT FALSE,
    observed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- SDK Sessions (Claude Code --sdk-url sessions telemetry)
CREATE TABLE IF NOT EXISTS sdk_sessions (
    session_id      TEXT        PRIMARY KEY,
    model           TEXT        NOT NULL DEFAULT 'unknown',
    task_type       TEXT        NOT NULL DEFAULT 'general',
    complexity      TEXT        NOT NULL DEFAULT 'trivial',
    task_preview    TEXT        NOT NULL DEFAULT '',
    state_key       TEXT        NOT NULL DEFAULT '',
    tools_sequence  JSONB       NOT NULL DEFAULT '[]',
    tools_allowed   INTEGER     NOT NULL DEFAULT 0,
    tools_denied    INTEGER     NOT NULL DEFAULT 0,
    tool_allow_rate REAL        NOT NULL DEFAULT 1.0 CHECK (tool_allow_rate BETWEEN 0 AND 1),
    input_tokens    INTEGER     NOT NULL DEFAULT 0,
    output_tokens   INTEGER     NOT NULL DEFAULT 0,
    total_cost_usd  REAL        NOT NULL DEFAULT 0 CHECK (total_cost_usd >= 0),
    duration_s      REAL        NOT NULL DEFAULT 0,
    is_error        BOOLEAN     NOT NULL DEFAULT FALSE,
    output_q_score  REAL        NOT NULL DEFAULT 0,
    output_verdict  TEXT        NOT NULL DEFAULT 'GROWL',
    output_confidence REAL      NOT NULL DEFAULT 0,
    reward          REAL        NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Scholar buffer (TF-IDF memory warm-start + PGVector semantic search)
-- β1: embedding column added for pgvector cosine similarity
-- Requires: CREATE EXTENSION IF NOT EXISTS vector (run separately if pgvector installed)
CREATE TABLE IF NOT EXISTS scholar_buffer (
    id          BIGSERIAL   PRIMARY KEY,
    cell_id     TEXT        NOT NULL DEFAULT '',
    cell_text   TEXT        NOT NULL,
    q_score     REAL        NOT NULL CHECK (q_score >= 0),
    reality     TEXT        NOT NULL DEFAULT '',
    ts          REAL        NOT NULL DEFAULT 0,
    embedding   FLOAT[]     NULL,           -- Dense vector (pgvector upgradable to vector type)
    embed_model TEXT        NOT NULL DEFAULT '',  -- Model that produced the embedding
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_judgments_reality   ON judgments (reality);
CREATE INDEX IF NOT EXISTS idx_judgments_verdict   ON judgments (verdict);
CREATE INDEX IF NOT EXISTS idx_judgments_created   ON judgments (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_learning_loop       ON learning_events (loop_name);
CREATE INDEX IF NOT EXISTS idx_learning_state      ON learning_events (state_key);
CREATE INDEX IF NOT EXISTS idx_llm_bench_dog       ON llm_benchmarks (dog_id, task_type);
CREATE INDEX IF NOT EXISTS idx_residual_observed   ON residual_history (observed_at DESC);
CREATE INDEX IF NOT EXISTS idx_sdk_model           ON sdk_sessions (model);
CREATE INDEX IF NOT EXISTS idx_sdk_task_type       ON sdk_sessions (task_type);
CREATE INDEX IF NOT EXISTS idx_sdk_created         ON sdk_sessions (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_scholar_created     ON scholar_buffer (created_at DESC);
"""


async def create_schema(dsn: str | None = None) -> None:
    """Create all tables and indexes. Idempotent (IF NOT EXISTS)."""
    async with acquire() as conn:
        await conn.execute(SCHEMA_SQL)
        logger.info("CYNIC schema created/verified")


# ════════════════════════════════════════════════════════════════════════════
# JUDGMENT REPOSITORY
# ════════════════════════════════════════════════════════════════════════════

class JudgmentRepository:
    """CRUD for judgments. φ-bound enforced by DB constraints."""

    async def save(self, judgment: dict[str, Any]) -> None:
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

    async def get(self, judgment_id: str) -> dict[str, Any] | None:
        """Fetch a judgment by ID."""
        async with acquire() as conn:
            row = await conn.fetchrow(
                "SELECT * FROM judgments WHERE judgment_id = $1",
                judgment_id,
            )
            return dict(row) if row else None

    async def recent(self, reality: str | None = None, limit: int = 55) -> list[dict[str, Any]]:
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

    async def stats(self) -> dict[str, Any]:
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

    async def get_all_actions(self, state_key: str) -> dict[str, float]:
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

    async def save(self, event: dict[str, Any]) -> None:
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

    async def recent_for_loop(self, loop_name: str, limit: int = 34) -> list[dict[str, Any]]:
        async with acquire() as conn:
            rows = await conn.fetch(
                "SELECT * FROM learning_events WHERE loop_name=$1 ORDER BY created_at DESC LIMIT $2",
                loop_name, limit,
            )
            return [dict(r) for r in rows]

    async def loop_stats(self) -> dict[str, int]:
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

    async def save(self, result: dict[str, Any]) -> None:
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

    async def best_llm_for(self, dog_id: str, task_type: str) -> str | None:
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

    async def get_all(self) -> list[dict[str, Any]]:
        """Load all benchmarks for warm-start (most recent first)."""
        async with acquire() as conn:
            rows = await conn.fetch("""
                SELECT dog_id, task_type, llm_id,
                       quality_score, speed_score, cost_score
                FROM llm_benchmarks
                ORDER BY created_at DESC
            """)
            return [dict(r) for r in rows]

    async def matrix(self) -> list[dict[str, Any]]:
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
# RESIDUAL HISTORY REPOSITORY
# ════════════════════════════════════════════════════════════════════════════

class ResidualRepository:
    """Persist ResidualDetector history for warm-start across restarts."""

    async def append(self, point: dict[str, Any]) -> None:
        """Persist one residual observation."""
        async with acquire() as conn:
            await conn.execute("""
                INSERT INTO residual_history
                    (judgment_id, residual, reality, analysis, unnameable)
                VALUES ($1, $2, $3, $4, $5)
            """,
                point["judgment_id"],
                float(point["residual"]),
                point["reality"],
                point["analysis"],
                bool(point.get("unnameable", False)),
            )

    async def recent(self, limit: int = 21) -> list[dict[str, Any]]:
        """Return last `limit` observations ordered oldest-first (for replay)."""
        async with acquire() as conn:
            rows = await conn.fetch("""
                SELECT judgment_id, residual, reality, analysis, unnameable,
                       EXTRACT(EPOCH FROM observed_at) AS timestamp
                FROM residual_history
                ORDER BY observed_at DESC
                LIMIT $1
            """, limit)
            # Reverse: oldest first so they replay in chronological order
            return list(reversed([dict(r) for r in rows]))


# ════════════════════════════════════════════════════════════════════════════
# REPOSITORY FACTORY
# ════════════════════════════════════════════════════════════════════════════

_judgment_repo: JudgmentRepository | None = None
_qtable_repo: QTableRepository | None = None
_learning_repo: LearningRepository | None = None
_benchmark_repo: BenchmarkRepository | None = None
_residual_repo: ResidualRepository | None = None


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


def residuals() -> ResidualRepository:
    global _residual_repo
    if _residual_repo is None:
        _residual_repo = ResidualRepository()
    return _residual_repo


# ════════════════════════════════════════════════════════════════════════════
# SDK SESSION REPOSITORY
# ════════════════════════════════════════════════════════════════════════════

class SDKSessionRepository:
    """Persist Claude Code --sdk-url session telemetry for learning analysis."""

    async def save(self, telemetry: dict[str, Any]) -> None:
        """
        Persist one SessionTelemetry record (upsert by session_id).
        Safe to call multiple times — ON CONFLICT updates the row.
        """
        import json as _json
        async with acquire() as conn:
            await conn.execute("""
                INSERT INTO sdk_sessions (
                    session_id, model, task_type, complexity, task_preview,
                    state_key, tools_sequence, tools_allowed, tools_denied,
                    tool_allow_rate, input_tokens, output_tokens, total_cost_usd,
                    duration_s, is_error, output_q_score, output_verdict,
                    output_confidence, reward
                ) VALUES (
                    $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19
                )
                ON CONFLICT (session_id) DO UPDATE SET
                    total_cost_usd   = EXCLUDED.total_cost_usd,
                    output_q_score   = EXCLUDED.output_q_score,
                    output_verdict   = EXCLUDED.output_verdict,
                    output_confidence = EXCLUDED.output_confidence,
                    reward           = EXCLUDED.reward,
                    is_error         = EXCLUDED.is_error,
                    duration_s       = EXCLUDED.duration_s
            """,
                telemetry["session_id"],
                telemetry.get("model", "unknown"),
                telemetry.get("task_type", "general"),
                telemetry.get("complexity", "trivial"),
                (telemetry.get("task") or "")[:200],
                telemetry.get("state_key", ""),
                _json.dumps(telemetry.get("tools_sequence", [])),
                int(telemetry.get("tools_allowed", 0)),
                int(telemetry.get("tools_denied", 0)),
                float(telemetry.get("tool_allow_rate", 1.0)),
                int(telemetry.get("input_tokens", 0)),
                int(telemetry.get("output_tokens", 0)),
                float(telemetry.get("total_cost_usd", 0.0)),
                float(telemetry.get("duration_s", 0.0)),
                bool(telemetry.get("is_error", False)),
                float(telemetry.get("output_q_score", 0.0)),
                telemetry.get("output_verdict", "GROWL"),
                float(telemetry.get("output_confidence", 0.0)),
                float(telemetry.get("reward", 0.0)),
            )

    async def recent(self, limit: int = 21) -> list[dict[str, Any]]:
        """Return last N sessions ordered most-recent-first."""
        async with acquire() as conn:
            rows = await conn.fetch(
                "SELECT * FROM sdk_sessions ORDER BY created_at DESC LIMIT $1",
                limit,
            )
            return [dict(r) for r in rows]

    async def stats(self) -> dict[str, Any]:
        """Aggregate statistics — for /act/telemetry enrichment."""
        async with acquire() as conn:
            row = await conn.fetchrow("""
                SELECT
                    COUNT(*)                                    AS total,
                    AVG(total_cost_usd)                         AS avg_cost_usd,
                    AVG(reward)                                 AS avg_reward,
                    AVG(output_q_score)                         AS avg_q_score,
                    SUM(CASE WHEN is_error THEN 1 ELSE 0 END)::REAL / NULLIF(COUNT(*),0)
                                                                AS error_rate
                FROM sdk_sessions
            """)
            return dict(row) if row else {}


_sdk_session_repo: SDKSessionRepository | None = None


def sdk_sessions() -> SDKSessionRepository:
    global _sdk_session_repo
    if _sdk_session_repo is None:
        _sdk_session_repo = SDKSessionRepository()
    return _sdk_session_repo


# ════════════════════════════════════════════════════════════════════════════
# SCHOLAR BUFFER REPOSITORY
# ════════════════════════════════════════════════════════════════════════════

class ScholarRepository:
    """
    Persist Scholar Dog's buffer for warm-start across restarts.

    Each entry = one (cell_text, q_score, embedding?) pair from a completed judgment.
    On startup, Scholar loads the last BUFFER_MAX entries to recover memory.
    On learn(), new entries are appended (fire-and-forget, best-effort).

    β1 PGVector: append() now optionally stores embedding vector.
    search_similar_by_embedding() performs cosine similarity via Python
    (no pgvector extension required — upgradeable to operator <=> later).
    """

    async def append(self, entry: dict[str, Any]) -> None:
        """Persist one BufferEntry to DB (with optional embedding)."""
        embedding = entry.get("embedding")  # List[float] or None
        embed_model = entry.get("embed_model", "")
        async with acquire() as conn:
            await conn.execute("""
                INSERT INTO scholar_buffer
                    (cell_id, cell_text, q_score, reality, ts, embedding, embed_model)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
            """,
                entry.get("cell_id", ""),
                str(entry.get("cell_text", ""))[:2000],
                float(entry.get("q_score", 0.0)),
                entry.get("reality", ""),
                float(entry.get("timestamp", 0.0)),
                embedding,    # NULL if not provided
                embed_model,
            )

    async def recent_entries(self, limit: int = 89) -> list[dict[str, Any]]:
        """Return last `limit` entries oldest-first (for buffer replay)."""
        async with acquire() as conn:
            rows = await conn.fetch("""
                SELECT cell_id, cell_text, q_score, reality, ts
                FROM scholar_buffer
                ORDER BY created_at DESC
                LIMIT $1
            """, limit)
            # Reverse so oldest-first (buffer grows from left)
            return list(reversed([dict(r) for r in rows]))

    async def search_similar_by_embedding(
        self,
        query_embedding: list[float],
        limit: int = 10,
        min_similarity: float = 0.38,
    ) -> list[dict[str, Any]]:
        """
        Find semantically similar entries via cosine similarity.

        Fetches recent entries with embeddings, computes cosine similarity
        in Python. Upgradeable to pgvector <=> operator when extension available.

        Returns entries with similarity ≥ min_similarity, sorted desc by sim.
        """
        import math

        async with acquire() as conn:
            rows = await conn.fetch("""
                SELECT cell_id, cell_text, q_score, reality, ts, embedding
                FROM scholar_buffer
                WHERE embedding IS NOT NULL
                ORDER BY created_at DESC
                LIMIT 200
            """)

        if not rows:
            return []

        q_norm = math.sqrt(sum(v * v for v in query_embedding))
        if q_norm < 1e-9:
            return []

        results = []
        for row in rows:
            emb = row["embedding"]
            if not emb or len(emb) != len(query_embedding):
                continue
            dot = sum(a * b for a, b in zip(query_embedding, emb))
            e_norm = math.sqrt(sum(v * v for v in emb))
            if e_norm < 1e-9:
                continue
            sim = dot / (q_norm * e_norm)
            if sim >= min_similarity:
                results.append({
                    "cell_id": row["cell_id"],
                    "cell_text": row["cell_text"],
                    "q_score": row["q_score"],
                    "reality": row["reality"],
                    "ts": row["ts"],
                    "similarity": float(sim),
                })

        results.sort(key=lambda x: x["similarity"], reverse=True)
        return results[:limit]

    async def count(self) -> int:
        """Total entries in scholar_buffer table."""
        async with acquire() as conn:
            return await conn.fetchval("SELECT COUNT(*) FROM scholar_buffer")


_scholar_repo: ScholarRepository | None = None


def scholar() -> ScholarRepository:
    global _scholar_repo
    if _scholar_repo is None:
        _scholar_repo = ScholarRepository()
    return _scholar_repo
