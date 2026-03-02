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
import time
import uuid
from typing import Any, TYPE_CHECKING
from typing_extensions import Protocol

from cynic.kernel.core.formulas import ACT_LOG_CAP
from cynic.kernel.core.storage.interface import (
    ActionProposalRepoInterface,
    AxiomFacetRepoInterface,
    BenchmarkRepoInterface,
    DogSoulRepoInterface,
    JudgmentRepoInterface,
    LearningRepoInterface,
    QTableRepoInterface,
    ResidualRepoInterface,
    ScholarRepoInterface,
    SDKSessionRepoInterface,
    StorageInterface,
)

if TYPE_CHECKING:
    from cynic.kernel.core.config import CynicConfig

logger = logging.getLogger("cynic.storage.surreal")


# ═ SCHEMA — SurrealQL (SCHEMALESS + indexes) ═════════════════════════════════

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
    "DEFINE TABLE IF NOT EXISTS axiom_facet SCHEMALESS",
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
    "DEFINE INDEX IF NOT EXISTS idx_scholar_vec ON scholar FIELDS embedding HNSW DIMENSION 768 DIST COSINE",
    "DEFINE INDEX IF NOT EXISTS idx_action_status ON action_proposal FIELDS status",
    "DEFINE INDEX IF NOT EXISTS idx_dog_soul_id ON dog_soul FIELDS dog_id UNIQUE",
    # Unique index for dynamic facets to prevent duplicates
    "DEFINE INDEX IF NOT EXISTS idx_axiom_facet_unique ON axiom_facet FIELDS axiom, reality, facet UNIQUE",
]


def _safe_id(raw: str) -> str:
    """Make a string safe for use in SurrealDB record IDs."""
    return raw.replace(":", "_").replace("/", "_").replace(" ", "_").replace("-", "_")


def _rec(table: str, key: str) -> str:
    """Build `table:key` record ID."""
    return f"{table}:{_safe_id(key)}"


# ═ SURREAL DB CLIENT PROTOCOL ═══════════════════════════════════════════════════
class SurrealDBClient(Protocol):
    """Protocol for SurrealDB async client (asyncsurreal.Surreal)."""

    async def upsert(self, record_id: str, data: dict[str, Any]) -> None:
        """Upsert a record (insert or update)."""
        ...

    async def query(self, sql: str, params: dict[str, Any] | None = None) -> Any:
        """Execute SurrealQL query with parameters."""
        ...

    async def select(self, record_id: str) -> dict[str, Any] | None:
        """Select a single record by ID."""
        ...


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


# ═ REPOSITORIES ═════════════════════════════════════════════════════════════


class JudgmentRepo(JudgmentRepoInterface):
    def __init__(self, db: SurrealDBClient) -> None:
        self._db = db

    async def save(self, judgment: dict[str, Any]) -> None:
        jid = judgment["judgment_id"]
        await self._db.upsert(
            _rec("judgment", jid),
            {
                **judgment,
                "created_at": judgment.get("created_at", time.time()),
            },
        )

    async def get(self, judgment_id: str) -> dict[str, Any] | None:
        result = await self._db.query(
            "SELECT * FROM judgment WHERE judgment_id = $jid LIMIT 1",
            {"jid": judgment_id},
        )
        rows = _rows(result)
        return rows[0] if rows else None

    async def recent(self, reality: str | None = None, limit: int = 55) -> list[dict[str, Any]]:
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
        avg_q = sum(r.get("avg_q", 0) * r.get("cnt", 0) for r in rows) / max(total, 1)
        return {
            "total": total,
            "by_verdict": by_verdict,
            "avg_q_score": round(avg_q, 2),
        }


class QTableRepo(QTableRepoInterface):
    def __init__(self, db: SurrealDBClient) -> None:
        self._db = db

    def _rec_id(self, state_key: str, action: str) -> str:
        return _rec("q_entry", f"{state_key}__{action}")

    async def get(self, state_key: str, action: str) -> float:
        try:
            rec = await self._db.select(self._rec_id(state_key, action))
            if rec and isinstance(rec, dict):
                return float(rec.get("q_value", 0.0))
        except Exception as exc:
            logger.error(f"❌ QTable persistence failure (GET) for {state_key}: {exc}")
        return 0.0

    async def update(self, state_key: str, action: str, q_value: float, visits: int = 1) -> None:
        rec_id = self._rec_id(state_key, action)
        try:
            await self._db.upsert(
                rec_id,
                {
                    "state_key": state_key,
                    "action": action,
                    "q_value": q_value,
                    "visit_count": visits,
                    "last_updated": time.time(),
                },
            )
        except Exception as exc:
            logger.error(f"❌ QTable persistence failure (UPDATE) for {rec_id}: {exc}")

    async def get_all_actions(self, state_key: str) -> dict[str, float]:
        result = await self._db.query(
            "SELECT action, q_value FROM q_entry WHERE state_key = $s",
            {"s": state_key},
        )
        return {r["action"]: float(r["q_value"]) for r in _rows(result)}

    async def get_all(self) -> list[dict[str, Any]]:
        """Return all Q-entries — used for warm-start."""
        result = await self._db.query("SELECT state_key, action, q_value, visit_count FROM q_entry")
        return _rows(result)


class LearningRepo(LearningRepoInterface):
    def __init__(self, db: SurrealDBClient) -> None:
        self._db = db

    async def save(self, event: dict[str, Any]) -> None:
        eid = event.get("event_id", str(uuid.uuid4()))
        await self._db.upsert(
            _rec("learning_event", eid),
            {
                **event,
                "event_id": eid,
                "created_at": time.time(),
            },
        )

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
    def __init__(self, db: SurrealDBClient) -> None:
        self._db = db

    async def save(self, result: dict[str, Any]) -> None:
        bid = result.get("benchmark_id", str(uuid.uuid4()))
        await self._db.upsert(
            _rec("llm_benchmark", bid),
            {
                **result,
                "benchmark_id": bid,
                "created_at": time.time(),
            },
        )

    async def best_llm_for(self, dog_id: str, task_type: str) -> str | None:
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
    def __init__(self, db: SurrealDBClient) -> None:
        self._db = db

    async def append(self, point: dict[str, Any]) -> None:
        rid = str(uuid.uuid4())
        await self._db.upsert(
            _rec("residual", rid),
            {
                **point,
                "observed_at": time.time(),
            },
        )

    async def recent(self, limit: int = 21) -> list[dict[str, Any]]:
        result = await self._db.query(
            "SELECT judgment_id, residual, reality, analysis, unnameable, observed_at "
            "FROM residual ORDER BY observed_at DESC LIMIT $n",
            {"n": limit},
        )
        rows = _rows(result)
        return list(reversed(rows))  # oldest-first for replay


class SDKSessionRepo(SDKSessionRepoInterface):
    def __init__(self, db: SurrealDBClient) -> None:
        self._db = db

    async def save(self, telemetry: dict[str, Any]) -> None:
        sid = telemetry["session_id"]
        await self._db.upsert(
            _rec("sdk_session", sid),
            {
                **telemetry,
                "task_preview": (telemetry.get("task") or "")[:200],
                "created_at": telemetry.get("created_at", time.time()),
            },
        )

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

    async def get_last_cli_session_id(self, cwd: str = "") -> str | None:
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
    def __init__(self, db: SurrealDBClient) -> None:
        self._db = db

    async def append(self, entry: dict[str, Any]) -> None:
        rid = str(uuid.uuid4())
        await self._db.upsert(
            _rec("scholar", rid),
            {
                **entry,
                "cell_text": str(entry.get("cell_text", ""))[:2000],
                "created_at": time.time(),
            },
        )

    async def recent_entries(self, limit: int = ACT_LOG_CAP) -> list[dict[str, Any]]:
        # Default: ACT_LOG_CAP (F(11)=89) — keep last 89 scholar entries
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


class ActionProposalRepo(ActionProposalRepoInterface):
    """Persist ProposedAction queue to SurrealDB (mirrors ~/.cynic/pending_actions.json)."""

    def __init__(self, db: SurrealDBClient) -> None:
        self._db = db

    async def upsert(self, action: dict[str, Any]) -> None:
        """Save or update an action proposal by action_id."""
        await self._db.upsert(
            _rec("action_proposal", action["action_id"]),
            {
                **action,
                "updated_at": time.time(),
            },
        )

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
        result = await self._db.query("SELECT * FROM action_proposal ORDER BY proposed_at DESC")
        return _rows(result)

    async def update_status(self, action_id: str, status: str) -> None:
        """Update the lifecycle status of a proposal."""
        await self._db.query(
            "UPDATE $id SET status = $status, updated_at = $ts",
            {"id": _rec("action_proposal", action_id), "status": status, "ts": time.time()},
        )


class DogSoulRepo(DogSoulRepoInterface):
    """Persist DogSoul cross-session identity to SurrealDB (mirrors ~/.cynic/dogs/{id}/soul.md)."""

    def __init__(self, db: SurrealDBClient) -> None:
        self._db = db

    async def save(self, soul: dict[str, Any]) -> None:
        """Upsert a dog's soul record (keyed by dog_id)."""
        await self._db.upsert(
            _rec("dog_soul", soul["dog_id"]),
            {
                **soul,
                "updated_at": time.time(),
            },
        )

    async def get(self, dog_id: str) -> dict[str, Any] | None:
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


class AxiomFacetRepo(AxiomFacetRepoInterface):
    """Persist dynamic axiom facets to SurrealDB."""

    def __init__(self, db: SurrealDBClient) -> None:
        self._db = db

    async def save(self, facet: dict[str, Any]) -> None:
        """Upsert a dynamic facet record."""
        # ID: axiom_facet:{axiom}_{reality}_{facet_name}
        fid = f"{facet['axiom']}_{facet['reality']}_{facet['facet']}"
        await self._db.upsert(
            _rec("axiom_facet", fid),
            {
                **facet,
                "updated_at": time.time(),
            },
        )

    async def get_all(self, axiom: str, reality: str) -> list[dict[str, Any]]:
        """Load all facets for a given axiom/reality."""
        result = await self._db.query(
            "SELECT * FROM axiom_facet WHERE axiom = $a AND reality = $r",
            {"a": axiom, "r": reality},
        )
        return _rows(result)


# ═ STORAGE FACADE — one object, all repos ═══════════════════════════════════


class SurrealStorage(StorageInterface):
    """
    CYNIC's unified storage facade over SurrealDB.

    One WebSocket connection = all repositories.
    Replaces asyncpg + qdrant + ~/.cynic/*.json files.

    Usage:
        storage = await SurrealStorage.create(config)
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
        from surrealdb import AsyncSurreal

        self._url = url
        self._user = user
        self._password = password
        self._ns = namespace
        self._db_name = database
        self._db = AsyncSurreal(self._url)
        self._conn = self._db  # for backward compatibility

        # Repos
        self._judgments = JudgmentRepo(self._db)
        self._qtable = QTableRepo(self._db)
        self._learning = LearningRepo(self._db)
        self._benchmarks = BenchmarkRepo(self._db)
        self._residuals = ResidualRepo(self._db)
        self._sdk_sessions = SDKSessionRepo(self._db)
        self._scholar = ScholarRepo(self._db)
        self._action_proposals = ActionProposalRepo(self._db)
        self._dog_souls = DogSoulRepo(self._db)
        self._axiom_facets = AxiomFacetRepo(self._db)

    @classmethod
    async def create(cls, config: CynicConfig) -> SurrealStorage:
        """
        Create and initialize a new storage instance.
        Requires an explicit configuration object (No Singletons).
        """
        if not config.surreal_url:
            raise ValueError("SurrealStorage requires surreal_url in config")

        storage = cls(
            url=config.surreal_url,
            user=config.surreal_user,
            password=config.surreal_pass,
            namespace=config.surreal_ns,
            database=config.surreal_db,
        )
        await storage.connect()
        await storage.create_schema()
        return storage

    async def connect(self) -> None:
        await self._db.connect()
        await self._db.signin({"username": self._user, "password": self._password})
        await self._db.use(self._ns, self._db_name)
        logger.info(
            "*sniff* SurrealDB connected: %s → %s.%s",
            self._url,
            self._ns,
            self._db_name,
        )

    async def close(self) -> None:
        if self._db is not None:
            await self._db.close()
            logger.info("SurrealDB connection closed")

    async def create_schema(self) -> None:
        """Define tables and indexes. Idempotent (IF NOT EXISTS)."""
        for stmt in _SCHEMA_STATEMENTS:
            try:
                await self._db.query(stmt)
            except Exception as exc:
                logger.error(f"❌ SurrealDB Schema Error in statement: {stmt[:50]}... | Error: {exc}")
        logger.info("*tail wag* SurrealDB schema ready (%d statements)", len(_SCHEMA_STATEMENTS))

    # ═ Repository accessors ═════════════════════════════════════════════════

    @property
    def judgments(self) -> JudgmentRepo:
        return self._judgments

    @property
    def qtable(self) -> QTableRepo:
        return self._qtable

    @property
    def learning(self) -> LearningRepo:
        return self._learning

    @property
    def benchmarks(self) -> BenchmarkRepo:
        return self._benchmarks

    @property
    def residuals(self) -> ResidualRepo:
        return self._residuals

    @property
    def sdk_sessions(self) -> SDKSessionRepo:
        return self._sdk_sessions

    @property
    def scholar(self) -> ScholarRepo:
        return self._scholar

    @property
    def action_proposals(self) -> ActionProposalRepo:
        return self._action_proposals

    @property
    def dog_souls(self) -> DogSoulRepo:
        return self._dog_souls

    @property
    def axiom_facets(self) -> AxiomFacetRepo:
        return self._axiom_facets

    async def ping(self) -> bool:
        """Return True if connection is alive."""
        try:
            await self._db.query("SELECT 1")
            return True
        except Exception:
            return False
