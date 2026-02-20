"""
Tests for cynic.core.storage.surreal

All tests use a mock AsyncSurreal connection — no real SurrealDB needed.
DB integration tests (pytest -m db_integration) test against real SurrealDB.
"""
from __future__ import annotations

import time
import uuid
from typing import Any, Dict, List, Optional
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from cynic.core.storage.surreal import (
    ActionProposalRepo,
    BenchmarkRepo,
    DogSoulRepo,
    JudgmentRepo,
    LearningRepo,
    QTableRepo,
    ResidualRepo,
    SDKSessionRepo,
    ScholarRepo,
    SurrealStorage,
    _rec,
    _rows,
    _safe_id,
)


# ════════════════════════════════════════════════════════════════════════════
# HELPERS
# ════════════════════════════════════════════════════════════════════════════

def _mock_db(query_result=None, select_result=None, upsert_result=None):
    """Build a mock AsyncSurreal-like connection."""
    db = MagicMock()
    db.query = AsyncMock(return_value=query_result or [{"result": []}])
    db.select = AsyncMock(return_value=select_result)
    db.upsert = AsyncMock(return_value=upsert_result or {})
    db.create = AsyncMock(return_value={})
    return db


def _query_result(rows: List[Dict]) -> List[Dict]:
    """Wrap rows in SurrealDB query result format."""
    return [{"result": rows}]


# ════════════════════════════════════════════════════════════════════════════
# UTILITY FUNCTIONS
# ════════════════════════════════════════════════════════════════════════════

class TestUtilFunctions:
    def test_safe_id_replaces_colon(self):
        assert ":" not in _safe_id("CODE:JUDGE")
        assert _safe_id("CODE:JUDGE") == "CODE_JUDGE"

    def test_safe_id_replaces_slash(self):
        assert "/" not in _safe_id("a/b")

    def test_safe_id_replaces_space(self):
        assert " " not in _safe_id("hello world")

    def test_rec_builds_record_id(self):
        rid = _rec("q_entry", "CODE_JUDGE")
        assert rid == "q_entry:CODE_JUDGE"

    def test_rec_escapes_colons(self):
        rid = _rec("judgment", "abc:def")
        assert ":" not in rid.split(":", 1)[1]  # Only table:rest, no colon in rest

    def test_rows_extracts_result_key(self):
        result = [{"result": [{"a": 1}, {"a": 2}]}]
        assert _rows(result) == [{"a": 1}, {"a": 2}]

    def test_rows_empty_list(self):
        assert _rows([]) == []

    def test_rows_none_result(self):
        assert _rows(None) == []

    def test_rows_empty_result_key(self):
        result = [{"result": []}]
        assert _rows(result) == []

    def test_rows_missing_result_key(self):
        result = [{}]
        assert _rows(result) == []


# ════════════════════════════════════════════════════════════════════════════
# JUDGMENT REPO
# ════════════════════════════════════════════════════════════════════════════

class TestJudgmentRepo:
    @pytest.mark.asyncio
    async def test_save_calls_upsert(self):
        db = _mock_db()
        repo = JudgmentRepo(db)
        judgment = {
            "judgment_id": "j1",
            "reality": "CODE",
            "verdict": "WAG",
            "q_score": 55.0,
        }
        await repo.save(judgment)
        db.upsert.assert_called_once()
        call_args = db.upsert.call_args
        assert "judgment:j1" == call_args[0][0]
        assert call_args[0][1]["judgment_id"] == "j1"

    @pytest.mark.asyncio
    async def test_save_adds_created_at(self):
        db = _mock_db()
        repo = JudgmentRepo(db)
        await repo.save({"judgment_id": "j2", "verdict": "WAG", "q_score": 55.0})
        data = db.upsert.call_args[0][1]
        assert "created_at" in data

    @pytest.mark.asyncio
    async def test_get_returns_none_when_empty(self):
        db = _mock_db(query_result=_query_result([]))
        repo = JudgmentRepo(db)
        result = await repo.get("nonexistent")
        assert result is None

    @pytest.mark.asyncio
    async def test_get_returns_row(self):
        row = {"judgment_id": "j1", "verdict": "WAG"}
        db = _mock_db(query_result=_query_result([row]))
        repo = JudgmentRepo(db)
        result = await repo.get("j1")
        assert result == row

    @pytest.mark.asyncio
    async def test_recent_no_reality_filter(self):
        rows = [{"judgment_id": "j1"}, {"judgment_id": "j2"}]
        db = _mock_db(query_result=_query_result(rows))
        repo = JudgmentRepo(db)
        result = await repo.recent()
        assert len(result) == 2

    @pytest.mark.asyncio
    async def test_recent_with_reality_filter(self):
        rows = [{"judgment_id": "j1", "reality": "CODE"}]
        db = _mock_db(query_result=_query_result(rows))
        repo = JudgmentRepo(db)
        result = await repo.recent(reality="CODE")
        assert result[0]["reality"] == "CODE"
        # Query should include reality filter
        call_args = db.query.call_args[0][0]
        assert "$r" in call_args

    @pytest.mark.asyncio
    async def test_stats_empty(self):
        db = _mock_db(query_result=_query_result([]))
        repo = JudgmentRepo(db)
        stats = await repo.stats()
        assert stats["total"] == 0
        assert stats["by_verdict"] == {}
        assert stats["avg_q_score"] == 0.0

    @pytest.mark.asyncio
    async def test_stats_aggregates(self):
        rows = [
            {"verdict": "WAG", "cnt": 10, "avg_q": 60.0},
            {"verdict": "BARK", "cnt": 5, "avg_q": 20.0},
        ]
        db = _mock_db(query_result=_query_result(rows))
        repo = JudgmentRepo(db)
        stats = await repo.stats()
        assert stats["total"] == 15
        assert stats["by_verdict"]["WAG"] == 10
        assert stats["by_verdict"]["BARK"] == 5


# ════════════════════════════════════════════════════════════════════════════
# Q-TABLE REPO
# ════════════════════════════════════════════════════════════════════════════

class TestQTableRepo:
    @pytest.mark.asyncio
    async def test_get_returns_zero_on_miss(self):
        db = _mock_db(select_result=None)
        repo = QTableRepo(db)
        val = await repo.get("CODE:JUDGE:PRESENT:1", "WAG")
        assert val == 0.0

    @pytest.mark.asyncio
    async def test_get_returns_stored_value(self):
        db = _mock_db(select_result={"q_value": 0.75, "visit_count": 5})
        repo = QTableRepo(db)
        val = await repo.get("CODE:JUDGE:PRESENT:1", "WAG")
        assert val == pytest.approx(0.75)

    @pytest.mark.asyncio
    async def test_update_upserts_record(self):
        db = _mock_db(select_result=None)
        repo = QTableRepo(db)
        await repo.update("s1", "WAG", 0.6)
        db.upsert.assert_called_once()
        data = db.upsert.call_args[0][1]
        assert data["q_value"] == pytest.approx(0.6)
        assert data["state_key"] == "s1"
        assert data["action"] == "WAG"
        assert data["visit_count"] == 1

    @pytest.mark.asyncio
    async def test_update_increments_visits(self):
        db = _mock_db(select_result={"q_value": 0.5, "visit_count": 7})
        repo = QTableRepo(db)
        await repo.update("s1", "WAG", 0.6)
        data = db.upsert.call_args[0][1]
        assert data["visit_count"] == 8

    @pytest.mark.asyncio
    async def test_get_all_actions(self):
        rows = [{"action": "WAG", "q_value": 0.6}, {"action": "BARK", "q_value": 0.2}]
        db = _mock_db(query_result=_query_result(rows))
        repo = QTableRepo(db)
        result = await repo.get_all_actions("s1")
        assert result["WAG"] == pytest.approx(0.6)
        assert result["BARK"] == pytest.approx(0.2)

    @pytest.mark.asyncio
    async def test_get_all_returns_list(self):
        rows = [
            {"state_key": "s1", "action": "WAG", "q_value": 0.6, "visit_count": 3},
        ]
        db = _mock_db(query_result=_query_result(rows))
        repo = QTableRepo(db)
        result = await repo.get_all()
        assert len(result) == 1
        assert result[0]["state_key"] == "s1"

    @pytest.mark.asyncio
    async def test_record_id_format(self):
        db = _mock_db(select_result=None)
        repo = QTableRepo(db)
        await repo.update("CODE:JUDGE", "WAG", 0.5)
        rec_id = db.upsert.call_args[0][0]
        assert rec_id.startswith("q_entry:")
        assert ":" not in rec_id.split(":", 1)[1]  # No raw colons in key


# ════════════════════════════════════════════════════════════════════════════
# LEARNING REPO
# ════════════════════════════════════════════════════════════════════════════

class TestLearningRepo:
    @pytest.mark.asyncio
    async def test_save_generates_event_id(self):
        db = _mock_db()
        repo = LearningRepo(db)
        await repo.save({"loop_name": "qlearning", "state_key": "s1", "action": "WAG", "reward": 0.6})
        data = db.upsert.call_args[0][1]
        assert "event_id" in data

    @pytest.mark.asyncio
    async def test_save_preserves_event_id(self):
        db = _mock_db()
        repo = LearningRepo(db)
        eid = str(uuid.uuid4())
        await repo.save({"event_id": eid, "loop_name": "qlearning", "state_key": "s", "action": "WAG", "reward": 0.5})
        data = db.upsert.call_args[0][1]
        assert data["event_id"] == eid

    @pytest.mark.asyncio
    async def test_recent_for_loop(self):
        rows = [{"loop_name": "qlearning", "reward": 0.5}]
        db = _mock_db(query_result=_query_result(rows))
        repo = LearningRepo(db)
        result = await repo.recent_for_loop("qlearning")
        assert len(result) == 1

    @pytest.mark.asyncio
    async def test_loop_stats(self):
        rows = [{"loop_name": "qlearning", "cnt": 42}]
        db = _mock_db(query_result=_query_result(rows))
        repo = LearningRepo(db)
        stats = await repo.loop_stats()
        assert stats["qlearning"] == 42


# ════════════════════════════════════════════════════════════════════════════
# BENCHMARK REPO
# ════════════════════════════════════════════════════════════════════════════

class TestBenchmarkRepo:
    @pytest.mark.asyncio
    async def test_save_generates_benchmark_id(self):
        db = _mock_db()
        repo = BenchmarkRepo(db)
        await repo.save({
            "dog_id": "GUARDIAN", "task_type": "code_review", "llm_id": "gemma2:2b",
            "quality_score": 0.8, "speed_score": 0.9, "cost_score": 0.7,
            "composite_score": 0.8, "latency_ms": 500,
        })
        data = db.upsert.call_args[0][1]
        assert "benchmark_id" in data

    @pytest.mark.asyncio
    async def test_best_llm_returns_none_when_empty(self):
        db = _mock_db(query_result=_query_result([]))
        repo = BenchmarkRepo(db)
        result = await repo.best_llm_for("GUARDIAN", "code_review")
        assert result is None

    @pytest.mark.asyncio
    async def test_best_llm_returns_top(self):
        rows = [{"llm_id": "gemma2:2b", "avg_score": 0.85}]
        db = _mock_db(query_result=_query_result(rows))
        repo = BenchmarkRepo(db)
        result = await repo.best_llm_for("GUARDIAN", "code_review")
        assert result == "gemma2:2b"


# ════════════════════════════════════════════════════════════════════════════
# RESIDUAL REPO
# ════════════════════════════════════════════════════════════════════════════

class TestResidualRepo:
    @pytest.mark.asyncio
    async def test_append_upserts(self):
        db = _mock_db()
        repo = ResidualRepo(db)
        await repo.append({
            "judgment_id": "j1",
            "residual": 0.75,
            "reality": "CODE",
            "analysis": "JUDGE",
            "unnameable": False,
        })
        db.upsert.assert_called_once()
        data = db.upsert.call_args[0][1]
        assert data["residual"] == pytest.approx(0.75)
        assert "observed_at" in data

    @pytest.mark.asyncio
    async def test_recent_returns_oldest_first(self):
        # recent() reverses the DESC order result → oldest first
        rows = [
            {"judgment_id": "j3", "observed_at": 3.0},
            {"judgment_id": "j2", "observed_at": 2.0},
            {"judgment_id": "j1", "observed_at": 1.0},
        ]
        db = _mock_db(query_result=_query_result(rows))
        repo = ResidualRepo(db)
        result = await repo.recent(limit=3)
        # Should be reversed (oldest first)
        assert result[0]["judgment_id"] == "j1"
        assert result[-1]["judgment_id"] == "j3"

    @pytest.mark.asyncio
    async def test_recent_empty(self):
        db = _mock_db(query_result=_query_result([]))
        repo = ResidualRepo(db)
        result = await repo.recent()
        assert result == []


# ════════════════════════════════════════════════════════════════════════════
# SDK SESSION REPO
# ════════════════════════════════════════════════════════════════════════════

class TestSDKSessionRepo:
    @pytest.mark.asyncio
    async def test_save_upserts_with_session_id(self):
        db = _mock_db()
        repo = SDKSessionRepo(db)
        await repo.save({
            "session_id": "sess1",
            "model": "claude-sonnet-4-6",
            "task": "refactor auth module",
        })
        rec_id = db.upsert.call_args[0][0]
        assert "sess1" in rec_id or "sess1" in rec_id.replace(":", "_")

    @pytest.mark.asyncio
    async def test_save_truncates_task_preview(self):
        db = _mock_db()
        repo = SDKSessionRepo(db)
        long_task = "x" * 500
        await repo.save({"session_id": "s1", "task": long_task})
        data = db.upsert.call_args[0][1]
        assert len(data["task_preview"]) <= 200

    @pytest.mark.asyncio
    async def test_stats_empty(self):
        db = _mock_db(query_result=_query_result([]))
        repo = SDKSessionRepo(db)
        stats = await repo.stats()
        assert stats == {}

    @pytest.mark.asyncio
    async def test_stats_returns_first_row(self):
        rows = [{"total": 10, "avg_cost_usd": 0.05, "avg_reward": 0.7, "avg_q_score": 55.0}]
        db = _mock_db(query_result=_query_result(rows))
        repo = SDKSessionRepo(db)
        stats = await repo.stats()
        assert stats["total"] == 10


# ════════════════════════════════════════════════════════════════════════════
# SCHOLAR REPO
# ════════════════════════════════════════════════════════════════════════════

class TestScholarRepo:
    @pytest.mark.asyncio
    async def test_append_upserts(self):
        db = _mock_db()
        repo = ScholarRepo(db)
        await repo.append({
            "cell_id": "c1",
            "cell_text": "def foo(): pass",
            "q_score": 65.0,
            "reality": "CODE",
            "ts": 1234.0,
        })
        db.upsert.assert_called_once()
        data = db.upsert.call_args[0][1]
        assert data["cell_text"] == "def foo(): pass"

    @pytest.mark.asyncio
    async def test_append_truncates_cell_text(self):
        db = _mock_db()
        repo = ScholarRepo(db)
        long_text = "x" * 5000
        await repo.append({"cell_id": "c1", "cell_text": long_text, "q_score": 50.0})
        data = db.upsert.call_args[0][1]
        assert len(data["cell_text"]) <= 2000

    @pytest.mark.asyncio
    async def test_recent_entries_oldest_first(self):
        rows = [
            {"cell_id": "c3", "created_at": 3.0},
            {"cell_id": "c2", "created_at": 2.0},
            {"cell_id": "c1", "created_at": 1.0},
        ]
        db = _mock_db(query_result=_query_result(rows))
        repo = ScholarRepo(db)
        result = await repo.recent_entries()
        assert result[0]["cell_id"] == "c1"  # oldest first

    @pytest.mark.asyncio
    async def test_count_zero(self):
        db = _mock_db(query_result=_query_result([]))
        repo = ScholarRepo(db)
        count = await repo.count()
        assert count == 0

    @pytest.mark.asyncio
    async def test_count_returns_n(self):
        db = _mock_db(query_result=_query_result([{"n": 42}]))
        repo = ScholarRepo(db)
        count = await repo.count()
        assert count == 42

    @pytest.mark.asyncio
    async def test_search_similar_passes_query_vec(self):
        db = _mock_db(query_result=_query_result([]))
        repo = ScholarRepo(db)
        q = [0.1] * 768
        await repo.search_similar_by_embedding(q)
        call_vars = db.query.call_args[0][1]
        assert call_vars["q"] == q
        assert call_vars["min_sim"] == pytest.approx(0.38)


# ════════════════════════════════════════════════════════════════════════════
# ACTION PROPOSAL REPO
# ════════════════════════════════════════════════════════════════════════════

class TestActionProposalRepo:
    def _action(self, **kw) -> dict:
        base = {
            "action_id": "abc12345",
            "judgment_id": "jid-1",
            "state_key": "CODE:JUDGE:PRESENT:1",
            "verdict": "BARK",
            "reality": "CODE",
            "action_type": "INVESTIGATE",
            "description": "Critical issue in core",
            "prompt": "Investigate the following…",
            "q_score": 30.0,
            "priority": 1,
            "proposed_at": 1_700_000_000.0,
            "status": "PENDING",
        }
        base.update(kw)
        return base

    @pytest.mark.asyncio
    async def test_upsert_calls_upsert_with_action_id(self):
        db = _mock_db()
        repo = ActionProposalRepo(db)
        action = self._action()
        await repo.upsert(action)
        db.upsert.assert_called_once()
        rec_id = db.upsert.call_args[0][0]
        assert "abc12345" in rec_id

    @pytest.mark.asyncio
    async def test_upsert_adds_updated_at(self):
        db = _mock_db()
        repo = ActionProposalRepo(db)
        await repo.upsert(self._action())
        data = db.upsert.call_args[0][1]
        assert "updated_at" in data

    @pytest.mark.asyncio
    async def test_all_pending_queries_by_status(self):
        rows = [self._action()]
        db = _mock_db(query_result=_query_result(rows))
        repo = ActionProposalRepo(db)
        result = await repo.all_pending()
        assert result[0]["action_id"] == "abc12345"
        sql = db.query.call_args[0][0]
        assert "PENDING" in sql

    @pytest.mark.asyncio
    async def test_all_returns_empty_list(self):
        db = _mock_db(query_result=_query_result([]))
        repo = ActionProposalRepo(db)
        result = await repo.all()
        assert result == []

    @pytest.mark.asyncio
    async def test_update_status_passes_status(self):
        db = _mock_db()
        repo = ActionProposalRepo(db)
        await repo.update_status("abc12345", "ACCEPTED")
        call_vars = db.query.call_args[0][1]
        assert call_vars["status"] == "ACCEPTED"


# ════════════════════════════════════════════════════════════════════════════
# DOG SOUL REPO
# ════════════════════════════════════════════════════════════════════════════

class TestDogSoulRepo:
    def _soul(self, **kw) -> dict:
        base = {
            "dog_id": "SCHOLAR",
            "total_judgments": 42,
            "avg_q_score": 65.5,
            "session_count": 3,
            "top_signals": ["CODE", "JUDGE"],
            "last_seen": "2026-02-19T12:00:00",
        }
        base.update(kw)
        return base

    @pytest.mark.asyncio
    async def test_save_upserts_with_dog_id_key(self):
        db = _mock_db()
        repo = DogSoulRepo(db)
        await repo.save(self._soul())
        db.upsert.assert_called_once()
        rec_id = db.upsert.call_args[0][0]
        assert "SCHOLAR" in rec_id.upper()

    @pytest.mark.asyncio
    async def test_save_adds_updated_at(self):
        db = _mock_db()
        repo = DogSoulRepo(db)
        await repo.save(self._soul())
        data = db.upsert.call_args[0][1]
        assert "updated_at" in data

    @pytest.mark.asyncio
    async def test_get_returns_none_when_not_found(self):
        db = _mock_db(query_result=_query_result([]))
        repo = DogSoulRepo(db)
        result = await repo.get("SCHOLAR")
        assert result is None

    @pytest.mark.asyncio
    async def test_get_returns_soul(self):
        soul = self._soul()
        db = _mock_db(query_result=_query_result([soul]))
        repo = DogSoulRepo(db)
        result = await repo.get("SCHOLAR")
        assert result["dog_id"] == "SCHOLAR"
        assert result["total_judgments"] == 42

    @pytest.mark.asyncio
    async def test_all_returns_list(self):
        souls = [self._soul(), self._soul(dog_id="GUARDIAN")]
        db = _mock_db(query_result=_query_result(souls))
        repo = DogSoulRepo(db)
        result = await repo.all()
        assert len(result) == 2


# ════════════════════════════════════════════════════════════════════════════
# SURREAL STORAGE FACADE
# ════════════════════════════════════════════════════════════════════════════

class TestSurrealStorageFacade:
    def _make_storage(self) -> SurrealStorage:
        storage = SurrealStorage(
            url="ws://localhost:8080/rpc",
            user="root",
            password="mock",  # test fixture
            namespace="cynic",
            database="cynic",
        )
        storage._conn = _mock_db()
        return storage

    def test_repo_accessors_return_correct_types(self):
        storage = self._make_storage()
        assert isinstance(storage.judgments, JudgmentRepo)
        assert isinstance(storage.qtable, QTableRepo)
        assert isinstance(storage.learning, LearningRepo)
        assert isinstance(storage.benchmarks, BenchmarkRepo)
        assert isinstance(storage.residuals, ResidualRepo)
        assert isinstance(storage.sdk_sessions, SDKSessionRepo)
        assert isinstance(storage.scholar, ScholarRepo)
        assert isinstance(storage.action_proposals, ActionProposalRepo)
        assert isinstance(storage.dog_souls, DogSoulRepo)

    @pytest.mark.asyncio
    async def test_ping_returns_true_on_success(self):
        storage = self._make_storage()
        storage._conn.query = AsyncMock(return_value=[{"result": [1]}])
        assert await storage.ping() is True

    @pytest.mark.asyncio
    async def test_ping_returns_false_on_error(self):
        storage = self._make_storage()
        storage._conn.query = AsyncMock(side_effect=Exception("Connection refused"))
        assert await storage.ping() is False

    @pytest.mark.asyncio
    async def test_create_schema_calls_query_for_each_statement(self):
        storage = self._make_storage()
        storage._conn.query = AsyncMock(return_value=[{"result": []}])
        await storage.create_schema()
        # Should be called once per statement
        from cynic.core.storage.surreal import _SCHEMA_STATEMENTS
        assert storage._conn.query.call_count == len(_SCHEMA_STATEMENTS)

    @pytest.mark.asyncio
    async def test_from_env_uses_defaults(self):
        with patch.dict("os.environ", {}, clear=True):
            storage = SurrealStorage.from_env()
            assert storage._url == "ws://localhost:8080/rpc"
            assert storage._user == "root"
            assert storage._ns == "cynic"
            assert storage._db_name == "cynic"

    @pytest.mark.asyncio
    async def test_from_env_reads_env_vars(self):
        env = {
            "SURREAL_URL": "ws://surrealdb:8080/rpc",
            "SURREAL_USER": "admin",
            "SURREAL_PASS": "secret",
            "SURREAL_NS": "prod",
            "SURREAL_DB": "organisms",
        }
        with patch.dict("os.environ", env):
            storage = SurrealStorage.from_env()
            assert storage._url == "ws://surrealdb:8080/rpc"
            assert storage._user == "admin"
            assert storage._ns == "prod"
            assert storage._db_name == "organisms"


# ════════════════════════════════════════════════════════════════════════════
# INTEGRATION — load_from_entries wiring
# ════════════════════════════════════════════════════════════════════════════

class TestLoadFromEntriesWiring:
    """Verify SurrealDB → component warm-start chain works end-to-end."""

    @pytest.mark.asyncio
    async def test_qtable_load_from_entries_uses_surreal_data(self):
        """QTable.load_from_entries() accepts SurrealDB QTableRepo.get_all() output."""
        from cynic.learning.qlearning import QTable

        surreal_rows = [
            {"state_key": "CODE:JUDGE:PRESENT:1", "action": "WAG", "q_value": 0.6, "visit_count": 10},
            {"state_key": "CODE:JUDGE:PRESENT:1", "action": "HOWL", "q_value": 0.8, "visit_count": 5},
        ]

        qtable = QTable()
        loaded = qtable.load_from_entries(surreal_rows)
        assert loaded == 2

        # Verify Q-values were loaded
        entry = qtable._get_or_create("CODE:JUDGE:PRESENT:1", "WAG")
        assert entry.q_value == pytest.approx(0.6)
        assert entry.visits == 10

    @pytest.mark.asyncio
    async def test_residual_load_from_entries_uses_surreal_data(self):
        """ResidualDetector.load_from_entries() accepts SurrealDB ResidualRepo.recent() output."""
        from cynic.cognition.cortex.residual import ResidualDetector

        surreal_rows = [
            {
                "judgment_id": "j1",
                "residual": 0.3,
                "reality": "CODE",
                "analysis": "JUDGE",
                "unnameable": False,
                "observed_at": 1000.0,
            },
            {
                "judgment_id": "j2",
                "residual": 0.7,
                "reality": "CODE",
                "analysis": "JUDGE",
                "unnameable": True,
                "observed_at": 1001.0,
            },
        ]

        detector = ResidualDetector()
        loaded = detector.load_from_entries(surreal_rows)
        assert loaded == 2
        assert len(detector._history) == 2
        assert detector._observations == 2

    @pytest.mark.asyncio
    async def test_scholar_load_from_entries_uses_surreal_data(self):
        """ScholarDog.load_from_entries() accepts SurrealDB ScholarRepo.recent_entries() output."""
        from cynic.cognition.neurons.scholar import ScholarDog

        surreal_rows = [
            {"cell_id": "c1", "cell_text": "def foo(): pass", "q_score": 65.0, "reality": "CODE", "ts": 1.0},
            {"cell_id": "c2", "cell_text": "class Bar: ...", "q_score": 45.0, "reality": "CODE", "ts": 2.0},
        ]

        scholar = ScholarDog()
        loaded = scholar.load_from_entries(surreal_rows)
        assert loaded == 2
        assert len(scholar._buffer) == 2
