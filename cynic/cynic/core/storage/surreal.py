"""
CYNIC SurrealDB Storage — AsyncSurreal + φ-aligned multi-model DB

Why SurrealDB over PostgreSQL:
  - SCHEMALESS records: no migrations when schema evolves
  - HNSW vector index: native cosine similarity for Scholar (no pgvector extension)
  - Document + relational + graph in one engine (replaces postgres + qdrant + json files)
  - LIVE SELECT: real-time reactions without polling guidance.json
  - Single WebSocket connection: multiplexed, lower overhead than asyncpg pool

Connection:
  Docker: ws://surrealdb:8080/rpc
  Local:  ws://localhost:8080/rpc

Auth:
  SURREAL_URL  = ws://surrealdb:8080/rpc
  SURREAL_USER = root
  SURREAL_PASS = cynic_phi_618
  SURREAL_NS   = cynic
  SURREAL_DB   = cynic

φ-Laws obeyed:
  - SCHEMALESS = BURN (don't over-engineer fixed schemas)
  - HNSW cosine = PHI (geometric similarity, not Euclidean noise)
  - Single connection = VERIFY (one truth, no pool state drift)
"""
from __future__ import annotations

import logging
import os
import time
import uuid
from typing import Any

from cynic.core.storage.interface import (
    StorageInterface,
    JudgmentRepoInterface,
    QTableRepoInterface,
    LearningRepoInterface,
    BenchmarkRepoInterface,
    ResidualRepoInterface,
    SDKSessionRepoInterface,
    ScholarRepoInterface,
    ActionProposalRepoInterface,
    DogSoulRepoInterface,
)

logger = logging.getLogger("cynic.storage.surreal")


# ════════════════════════════════════════════════════════════════════════════
# SCHEMA — SurrealQL (SCHEMALESS + indexes)
# ════════════════════════════════════════════════════════════════════════════

_SCHEMA_STATEMENTS = [
    # Tables — SCHEMALESS means fields can vary without ALTER TABLE
    "DEFINE TABLE IF NOT EXISTS judgment SCHEMALESS",
    "DEFINE TABLE IF NOT EXISTS cell SCHEMALESS",
    "DEFINE TABLE IF NOT EXISTS q_entry SCHEMALESS",
    "DEFINE TABLE IF NOT EXISTS e_score SCHEMALESS",
    "DEFINE TABLE IF NOT EXISTS scholar SCHEMALESS",
    "DEFINE TABLE IF NOT EXISTS sdk_session SCHEMALESS",
    "DEFINE TABLE IF NOT EXISTS learning_event SCHEMALESS",
    "DEFINE TABLE IF NOT EXISTS llm_benchmark SCHEMALESS",
    "DEFINE TABLE IF NOT EXISTS residual SCHEMALESS",
    "DEFINE TABLE IF NOT EXISTS consciousness_snapshot SCHEMALESS",
    "DEFINE TABLE IF NOT EXISTS action_proposal SCHEMALESS",
    "DEFINE TABLE IF NOT EXISTS dog_soul SCHEMALESS",
    # Indexes — optimize common query paths
    "DEFINE INDEX IF NOT EXISTS idx_judgment_reality ON judgment FIELDS reality",
    "DEFINE INDEX IF NOT EXISTS idx_judgment_verdict ON judgment FIELDS verdict",
    "DEFINE INDEX IF NOT EXISTS idx_judgment_created ON judgment FIELDS created_at",
    # UNIQUE index on q_entry composite key (state_key + action)
    "DEFINE INDEX IF NOT EXISTS idx_q_unique ON q_entry FIELDS state_key, action UNIQUE",
    "DEFINE INDEX IF NOT EXISTS idx_escore_agent ON e_score FIELDS agent_id UNIQUE",
    "DEFINE INDEX IF NOT EXISTS idx_sdk_created ON sdk_session FIELDS created_at",
    "DEFINE INDEX IF NOT EXISTS idx_residual_observed ON residual FIELDS observed_at",
    "DEFINE INDEX IF NOT EXISTS idx_scholar_created ON scholar FIELDS created_at",
    # HNSW vector index — cosine similarity for Scholar semantic search
    # Dimension 768 = standard BERT/sentence-transformers output
    # Falls back gracefully if no embeddings stored
    "DEFINE INDEX IF NOT EXISTS idx_scholar_vec ON scholar FIELDS embedding HNSW DIMENSION 768 DIST COSINE",
    "DEFINE INDEX IF NOT EXISTS idx_action_status ON action_proposal FIELDS status",
    "DEFINE INDEX IF NOT EXISTS idx_dog_soul_id ON dog_soul FIELDS dog_id UNIQUE",
]


def _safe_id(raw: str) -> str:
    """Make a string safe for use in SurrealDB record IDs."""
    return raw.replace(":", "_").replace("/", "_").replace(" ", "_").replace("-", "_")


def _rec(table: str, key: str) -> str:
    """Build `table:key` record ID."""
    return f"{table}:{_safe_id(key)}"


def _rows(result: Any) -> list[dict]:
    """Extract row list from a SurrealDB query result."""
    if not result:
        return []
    first = result[0] if isinstance(result, list) else result
    if isinstance(first, dict):
        return first.get("result", []) or []
    # Some SDK versions expose .result as an attribute
    if hasattr(first, "result"):
        return first.result or []
    return []


# ════════════════════════════════════════════════════════════════════════════
# REPOSITORIES
# ════════════════════════════════════════════════════════════════════════════

class JudgmentRepo(JudgmentRepoInterface):
    def __init__(self, db: Any) -> None:
        self._db = db

    async def save(self, judgment: dict[str, Any]) -> None:
        jid = judgment["judgment_id"]
        await self._db.upsert(_rec("judgment", jid), {
            **judgment,
            "created_at": judgment.get("created_at", time.time()),
        })

    async def get(self, judgment_id: str) -> Optional[dict[str, Any]]:
        result = await self._db.query(
            "SELECT * FROM judgment WHERE judgment_id = $jid LIMIT 1",
            {"jid": judgment_id},
        )
        rows = _rows(result)
        return rows[0] if rows else None

    async def recent(
        self, reality: Optional[str] = None, limit: int = 55
    ) -> list[dict[str, Any]]:
        if reality:
            result = await self._db.query(
                "SELECT * FROM judgment WHERE reality = $r ORDER BY created_at DESC LIMIT $n",
                {"r": reality, "n": limit},
            )
        else:
            result = await self._db.query(
                "SELECT * FROM judgment ORDER BY created_at DESC LIMIT $n",
                {"n": limit},
            )
        return _rows(result)

    async def stats(self) -> dict[str, Any]:
        result = await self._db.query(
            "SELECT verdict, count() AS cnt, math::mean(q_score) AS avg_q "
            "FROM judgment GROUP BY verdict"
        )
        rows = _rows(result)
        total = sum(r.get("cnt", 0) for r in rows)
        by_verdict = {r["verdict"]: r["cnt"] for r in rows if "verdict" in r}
        avg_q = (
            sum(r.get("avg_q", 0) * r.get("cnt", 0) for r in rows) / max(total, 1)
        )
        return {
            "total": total,
            "by_verdict": by_verdict,
            "avg_q_score": round(avg_q, 2),
        }


class QTableRepo(QTableRepoInterface):
    def __init__(self, db: Any) -> None:
        self._db = db

    def _rec_id(self, state_key: str, action: str) -> str:
        return _rec("q_entry", f"{state_key}__{action}")

    async def get(self, state_key: str, action: str) -> float:
        try:
            rec = await self._db.select(self._rec_id(state_key, action))
            if rec and isinstance(rec, dict):
                return float(rec.get("q_value", 0.0))
        except Exception:
            logger.debug("QTable get failed for %s/%s", state_key, action, exc_info=True)
        return 0.0

    async def update(self, state_key: str, action: str, q_value: float) -> None:
        rec_id = self._rec_id(state_key, action)
        # Fetch existing to increment visit_count
        existing: Optional[dict] = None
        try:
            existing = await self._db.select(rec_id)
        except Exception:
            logger.debug("QTable visit fetch failed for %s", rec_id, exc_info=True)
        visits = 1
        if existing and isinstance(existing, dict):
            visits = int(existing.get("visit_count", 0)) + 1
        await self._db.upsert(rec_id, {
            "state_key": state_key,
            "action": action,
            "q_value": q_value,
            "visit_count": visits,
            "last_updated": time.time(),
        })

    async def get_all_actions(self, state_key: str) -> dict[str, float]:
        result = await self._db.query(
            "SELECT action, q_value FROM q_entry WHERE state_key = $s",
            {"s": state_key},
        )
        return {r["action"]: float(r["q_value"]) for r in _rows(result)}

    async def get_all(self) -> list[dict[str, Any]]:
        """Return all Q-entries — used for warm-start."""
        result = await self._db.query(
            "SELECT state_key, action, q_value, visit_count FROM q_entry"
        )
        return _rows(result)


class LearningRepo(LearningRepoInterface):
    def __init__(self, db: Any) -> None:
        self._db = db

    async def save(self, event: dict[str, Any]) -> None:
        eid = event.get("event_id", str(uuid.uuid4()))
        await self._db.upsert(_rec("learning_event", eid), {
            **event,
            "event_id": eid,
            "created_at": time.time(),
        })

    async def recent_for_loop(self, loop_name: str, limit: int = 34) -> list[dict]:
        result = await self._db.query(
            "SELECT * FROM learning_event WHERE loop_name = $l "
            "ORDER BY created_at DESC LIMIT $n",
            {"l": loop_name, "n": limit},
        )
        return _rows(result)

    async def loop_stats(self) -> dict[str, int]:
        result = await self._db.query(
            "SELECT loop_name, count() AS cnt FROM learning_event GROUP BY loop_name"
        )
        return {r["loop_name"]: r["cnt"] for r in _rows(result)}


class BenchmarkRepo(BenchmarkRepoInterface):
    def __init__(self, db: Any) -> None:
        self._db = db

    async def save(self, result: dict[str, Any]) -> None:
        bid = result.get("benchmark_id", str(uuid.uuid4()))
        await self._db.upsert(_rec("llm_benchmark", bid), {
            **result,
            "benchmark_id": bid,
            "created_at": time.time(),
        })

    async def best_llm_for(self, dog_id: str, task_type: str) -> Optional[str]:
        since = time.time() - 7 * 86400
        result = await self._db.query(
            "SELECT llm_id, math::mean(composite_score) AS avg_score "
            "FROM llm_benchmark "
            "WHERE dog_id = $d AND task_type = $t AND created_at > $since "
            "GROUP BY llm_id ORDER BY avg_score DESC LIMIT 1",
            {"d": dog_id, "t": task_type, "since": since},
        )
        rows = _rows(result)
        return rows[0]["llm_id"] if rows else None

    async def get_all(self) -> list[dict[str, Any]]:
        result = await self._db.query(
            "SELECT dog_id, task_type, llm_id, quality_score, speed_score, cost_score "
            "FROM llm_benchmark ORDER BY created_at DESC"
        )
        return _rows(result)

    async def matrix(self) -> list[dict[str, Any]]:
        since = time.time() - 7 * 86400
        result = await self._db.query(
            "SELECT dog_id, task_type, llm_id, "
            "math::mean(composite_score) AS avg_composite, "
            "math::mean(latency_ms) AS avg_latency_ms, "
            "count() AS runs "
            "FROM llm_benchmark WHERE created_at > $since "
            "GROUP BY dog_id, task_type, llm_id "
            "ORDER BY dog_id, task_type, avg_composite DESC",
            {"since": since},
        )
        return _rows(result)


class ResidualRepo(ResidualRepoInterface):
    def __init__(self, db: Any) -> None:
        self._db = db

    async def append(self, point: dict[str, Any]) -> None:
        rid = str(uuid.uuid4())
        await self._db.upsert(_rec("residual", rid), {
            **point,
            "observed_at": time.time(),
        })

    async def recent(self, limit: int = 21) -> list[dict[str, Any]]:
        result = await self._db.query(
            "SELECT judgment_id, residual, reality, analysis, unnameable, observed_at "
            "FROM residual ORDER BY observed_at DESC LIMIT $n",
            {"n": limit},
        )
        rows = _rows(result)
        return list(reversed(rows))  # oldest-first for replay


class SDKSessionRepo(SDKSessionRepoInterface):
    def __init__(self, db: Any) -> None:
        self._db = db

    async def save(self, telemetry: dict[str, Any]) -> None:
        sid = telemetry["session_id"]
        await self._db.upsert(_rec("sdk_session", sid), {
            **telemetry,
            "task_preview": (telemetry.get("task") or "")[:200],
            "created_at": telemetry.get("created_at", time.time()),
        })

    async def recent(self, limit: int = 21) -> list[dict[str, Any]]:
        result = await self._db.query(
            "SELECT * FROM sdk_session ORDER BY created_at DESC LIMIT $n",
            {"n": limit},
        )
        return _rows(result)

    async def stats(self) -> dict[str, Any]:
        result = await self._db.query(
            "SELECT count() AS total, "
            "math::mean(total_cost_usd) AS avg_cost_usd, "
            "math::mean(reward) AS avg_reward, "
            "math::mean(output_q_score) AS avg_q_score "
            "FROM sdk_session GROUP ALL"
        )
        rows = _rows(result)
        return rows[0] if rows else {}

    async def get_last_cli_session_id(self, cwd: str = "") -> Optional[str]:
        """Return the most recent cli_session_id, optionally filtered by cwd."""
        if cwd:
            result = await self._db.query(
                "SELECT cli_session_id FROM sdk_session "
                "WHERE cwd = $cwd AND cli_session_id != '' "
                "ORDER BY created_at DESC LIMIT 1",
                {"cwd": cwd},
            )
        else:
            result = await self._db.query(
                "SELECT cli_session_id FROM sdk_session "
                "WHERE cli_session_id != '' "
                "ORDER BY created_at DESC LIMIT 1",
            )
        rows = _rows(result)
        if rows and rows[0].get("cli_session_id"):
            return rows[0]["cli_session_id"]
        return None


class ScholarRepo(ScholarRepoInterface):
    def __init__(self, db: Any) -> None:
        self._db = db

    async def append(self, entry: dict[str, Any]) -> None:
        rid = str(uuid.uuid4())
        await self._db.upsert(_rec("scholar", rid), {
            **entry,
            "cell_text": str(entry.get("cell_text", ""))[:2000],
            "created_at": time.time(),
        })

    async def recent_entries(self, limit: int = 89) -> list[dict[str, Any]]:
        result = await self._db.query(
            "SELECT cell_id, cell_text, q_score, reality, ts "
            "FROM scholar ORDER BY created_at DESC LIMIT $n",
            {"n": limit},
        )
        rows = _rows(result)
        return list(reversed(rows))  # oldest-first for buffer replay

    async def search_similar_by_embedding(
        self,
        query_embedding: list[float],
        limit: int = 10,
        min_similarity: float = 0.38,
    ) -> list[dict[str, Any]]:
        """Native HNSW cosine search — replaces Python cosine loop."""
        result = await self._db.query(
            "SELECT cell_id, cell_text, q_score, reality, ts, "
            "vector::similarity::cosine(embedding, $q) AS similarity "
            "FROM scholar "
            "WHERE embedding IS NOT NULL "
            "  AND vector::similarity::cosine(embedding, $q) >= $min_sim "
            "ORDER BY similarity DESC LIMIT $n",
            {"q": query_embedding, "min_sim": min_similarity, "n": limit},
        )
        return _rows(result)

    async def count(self) -> int:
        result = await self._db.query("SELECT count() AS n FROM scholar GROUP ALL")
        rows = _rows(result)
        return rows[0]["n"] if rows else 0


# ════════════════════════════════════════════════════════════════════════════
# ACTION PROPOSAL REPOSITORY
# ════════════════════════════════════════════════════════════════════════════

class ActionProposalRepo(ActionProposalRepoInterface):
    """Persist ProposedAction queue to SurrealDB (mirrors ~/.cynic/pending_actions.json)."""

    def __init__(self, db: Any) -> None:
        self._db = db

    async def upsert(self, action: dict[str, Any]) -> None:
        """Save or update an action proposal by action_id."""
        await self._db.upsert(_rec("action_proposal", action["action_id"]), {
            **action,
            "updated_at": time.time(),
        })

    async def all_pending(self) -> list[dict[str, Any]]:
        """Return all PENDING proposals, ordered by priority then proposed_at."""
        result = await self._db.query(
            "SELECT * FROM action_proposal "
            "WHERE status = 'PENDING' "
            "ORDER BY priority ASC, proposed_at ASC",
        )
        return _rows(result)

    async def all(self) -> list[dict[str, Any]]:
        """Return all proposals (any status), newest first."""
        result = await self._db.query(
            "SELECT * FROM action_proposal ORDER BY proposed_at DESC"
        )
        return _rows(result)

    async def update_status(self, action_id: str, status: str) -> None:
        """Update the lifecycle status of a proposal."""
        await self._db.query(
            "UPDATE $id SET status = $status, updated_at = $ts",
            {"id": _rec("action_proposal", action_id), "status": status, "ts": time.time()},
        )


# ════════════════════════════════════════════════════════════════════════════
# DOG SOUL REPOSITORY
# ════════════════════════════════════════════════════════════════════════════

class DogSoulRepo(DogSoulRepoInterface):
    """Persist DogSoul cross-session identity to SurrealDB (mirrors ~/.cynic/dogs/{id}/soul.md)."""

    def __init__(self, db: Any) -> None:
        self._db = db

    async def save(self, soul: dict[str, Any]) -> None:
        """Upsert a dog's soul record (keyed by dog_id)."""
        await self._db.upsert(_rec("dog_soul", soul["dog_id"]), {
            **soul,
            "updated_at": time.time(),
        })

    async def get(self, dog_id: str) -> Optional[dict[str, Any]]:
        """Load a dog's soul by dog_id. Returns None if not found."""
        result = await self._db.query(
            "SELECT * FROM $id",
            {"id": _rec("dog_soul", dog_id.upper())},
        )
        rows = _rows(result)
        return rows[0] if rows else None

    async def all(self) -> list[dict[str, Any]]:
        """Return all dog souls."""
        result = await self._db.query("SELECT * FROM dog_soul ORDER BY dog_id ASC")
        return _rows(result)


# ════════════════════════════════════════════════════════════════════════════
# STORAGE FACADE — one object, all repos
# ════════════════════════════════════════════════════════════════════════════

class SurrealStorage(StorageInterface):
    """
    CYNIC's unified storage facade over SurrealDB.

    One WebSocket connection = all repositories.
    Replaces asyncpg + qdrant + ~/.cynic/*.json files.

    Usage:
        storage = await SurrealStorage.create()
        await storage.judgments.save(judgment_dict)
        q = await storage.qtable.get("CODE:JUDGE:PRESENT:1", "WAG")
        await storage.close()
    """

    def __init__(
        self,
        url: str,
        user: str,
        password: str,
        namespace: str,
        database: str,
    ) -> None:
        self._url = url
        self._user = user
        self._password = password
        self._ns = namespace
        self._db_name = database
        self._conn: Any = None  # AsyncSurreal instance

    @classmethod
    def from_env(cls) -> SurrealStorage:
        return cls(
            url=os.environ.get("SURREAL_URL", "ws://localhost:8080/rpc"),
            user=os.environ.get("SURREAL_USER", "root"),
            password=os.environ.get("SURREAL_PASS", "local_dev_only"),
            namespace=os.environ.get("SURREAL_NS", "cynic"),
            database=os.environ.get("SURREAL_DB", "cynic"),
        )

    @classmethod
    async def create(
        cls,
        url: Optional[str] = None,
        user: Optional[str] = None,
        password: Optional[str] = None,
        namespace: Optional[str] = None,
        database: Optional[str] = None,
    ) -> SurrealStorage:
        """Factory: connect + create schema. Call once at startup."""
        storage = cls(
            url=url or os.environ.get("SURREAL_URL", "ws://localhost:8080/rpc"),
            user=user or os.environ.get("SURREAL_USER", "root"),
            password=password or os.environ.get("SURREAL_PASS", "local_dev_only"),
            namespace=namespace or os.environ.get("SURREAL_NS", "cynic"),
            database=database or os.environ.get("SURREAL_DB", "cynic"),
        )
        await storage.connect()
        await storage.create_schema()
        return storage

    async def connect(self) -> None:
        from surrealdb import AsyncSurreal  # type: ignore
        self._conn = AsyncSurreal(self._url)
        await self._conn.connect()
        await self._conn.signin({"username": self._user, "password": self._password})
        await self._conn.use(self._ns, self._db_name)
        logger.info(
            "*sniff* SurrealDB connected: %s → %s.%s",
            self._url, self._ns, self._db_name,
        )

    async def close(self) -> None:
        if self._conn is not None:
            await self._conn.close()
            self._conn = None
            logger.info("SurrealDB connection closed")

    async def create_schema(self) -> None:
        """Define tables and indexes. Idempotent (IF NOT EXISTS)."""
        for stmt in _SCHEMA_STATEMENTS:
            try:
                await self._conn.query(stmt)
            except Exception as exc:
                logger.debug("Schema stmt skipped (%s): %s", stmt[:40], exc)
        logger.info("*tail wag* SurrealDB schema ready (%d statements)", len(_SCHEMA_STATEMENTS))

    # ── Repository accessors ─────────────────────────────────────────────────

    @property
    def judgments(self) -> JudgmentRepo:
        return JudgmentRepo(self._conn)

    @property
    def qtable(self) -> QTableRepo:
        return QTableRepo(self._conn)

    @property
    def learning(self) -> LearningRepo:
        return LearningRepo(self._conn)

    @property
    def benchmarks(self) -> BenchmarkRepo:
        return BenchmarkRepo(self._conn)

    @property
    def residuals(self) -> ResidualRepo:
        return ResidualRepo(self._conn)

    @property
    def sdk_sessions(self) -> SDKSessionRepo:
        return SDKSessionRepo(self._conn)

    @property
    def scholar(self) -> ScholarRepo:
        return ScholarRepo(self._conn)

    @property
    def action_proposals(self) -> ActionProposalRepo:
        return ActionProposalRepo(self._conn)

    @property
    def dog_souls(self) -> DogSoulRepo:
        return DogSoulRepo(self._conn)

    async def ping(self) -> bool:
        """Return True if connection is alive."""
        try:
            await self._conn.query("SELECT 1")
            return True
        except Exception:
            return False


# ════════════════════════════════════════════════════════════════════════════
# MODULE-LEVEL SINGLETON (Phase 1B: Wrapper for server.py bootstrap)
# ════════════════════════════════════════════════════════════════════════════
#
# CRITICAL: These functions provide backward compatibility with server.py's
# lifespan bootstrap. They manage a module-level _storage singleton.
#
# FUTURE: Will be replaced with injectable pattern in Phase 2A+ once
# server.py refactored to pass storage through CynicOrganism.state.
#
# TODO (Phase 2A): Move storage lifecycle to CynicOrganism + remove these functions
# ════════════════════════════════════════════════════════════════════════════

_storage: Optional[SurrealStorage] = None


async def init_storage(
    url: Optional[str] = None,
    user: Optional[str] = None,
    password: Optional[str] = None,
    namespace: Optional[str] = None,
    database: Optional[str] = None,
) -> SurrealStorage:
    """
    Initialize module-level storage singleton.

    Call once from FastAPI lifespan startup (in server.py).
    Connects to SurrealDB and creates schema.

    Args:
        url: SurrealDB WebSocket URL (default: SURREAL_URL env var)
        user: Username (default: SURREAL_USER env var)
        password: Password (default: SURREAL_PASS env var)
        namespace: Database namespace (default: SURREAL_NS env var)
        database: Database name (default: SURREAL_DB env var)

    Returns:
        The initialized SurrealStorage instance (also stored in _storage global)

    Raises:
        ImportError: If surrealdb package not installed
        Exception: If connection fails
    """
    global _storage

    if _storage is not None:
        logger.warning("init_storage: already initialized, returning existing instance")
        return _storage

    _storage = await SurrealStorage.create(
        url=url,
        user=user,
        password=password,
        namespace=namespace,
        database=database,
    )
    return _storage


def get_storage() -> SurrealStorage:
    """
    Retrieve module-level storage singleton.

    Call anywhere in the codebase after init_storage() has been called
    (i.e., after lifespan startup).

    Returns:
        The SurrealStorage instance

    Raises:
        RuntimeError: If init_storage() hasn't been called yet
    """
    if _storage is None:
        raise RuntimeError(
            "SurrealStorage not initialized. Call init_storage() in lifespan startup first."
        )
    return _storage


async def close_storage() -> None:
    """
    Close module-level storage singleton.

    Call from FastAPI lifespan shutdown (in server.py).
    Closes the WebSocket connection to SurrealDB.
    """
    global _storage

    if _storage is not None:
        await _storage.close()
        _storage = None
        logger.info("*yawn* SurrealDB singleton closed")
    else:
        logger.debug("close_storage: no storage to close")


def reset_storage() -> None:
    """
    Reset module-level storage singleton (TEST ONLY).

    Used in test teardown to clear the singleton for the next test.
    Should NOT be called in production code.
    """
    global _storage
    _storage = None


