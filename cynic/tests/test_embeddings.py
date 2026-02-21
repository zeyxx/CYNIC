"""
Tests for EmbeddingProvider + ScholarDog PGVector integration (β1).

No real DB or Ollama required — uses DummyEmbedder and mocks.
"""
import asyncio
import pytest
from unittest.mock import AsyncMock, MagicMock, patch

from cynic.core.embeddings import DummyEmbedder, OllamaEmbedder
from cynic.cognition.neurons.scholar import ScholarDog
from cynic.core.judgment import Cell


# ── DummyEmbedder ────────────────────────────────────────────────────────

class TestDummyEmbedder:
    def test_is_always_available(self):
        emb = DummyEmbedder()
        assert emb.is_available()

    def test_default_dimension(self):
        emb = DummyEmbedder()
        assert emb.dimension == 768

    def test_custom_dimension(self):
        emb = DummyEmbedder(dim=384)
        assert emb.dimension == 384

    @pytest.mark.asyncio
    async def test_embed_returns_correct_dimension(self):
        emb = DummyEmbedder(dim=384)
        vec = await emb.embed("hello world")
        assert len(vec) == 384

    @pytest.mark.asyncio
    async def test_embed_returns_unit_vector(self):
        import math
        emb = DummyEmbedder()
        vec = await emb.embed("def phi(): pass")
        norm = math.sqrt(sum(v * v for v in vec))
        assert abs(norm - 1.0) < 1e-5, f"Expected unit vector, norm={norm}"

    @pytest.mark.asyncio
    async def test_embed_is_deterministic(self):
        emb = DummyEmbedder()
        v1 = await emb.embed("same text")
        v2 = await emb.embed("same text")
        assert v1 == v2

    @pytest.mark.asyncio
    async def test_different_texts_different_vectors(self):
        emb = DummyEmbedder()
        v1 = await emb.embed("clean code with type hints")
        v2 = await emb.embed("god class wildcard imports")
        assert v1 != v2


# ── OllamaEmbedder ────────────────────────────────────────────────────────

class TestOllamaEmbedderInterface:
    def test_default_dimension(self):
        emb = OllamaEmbedder()
        assert emb.dimension == 768  # nomic-embed-text default

    def test_custom_model_dimension(self):
        emb = OllamaEmbedder(model="all-minilm")
        assert emb.dimension == 384

    def test_starts_available(self):
        emb = OllamaEmbedder()
        assert emb.is_available()

    def test_disabled_after_max_errors(self):
        emb = OllamaEmbedder()
        emb._MAX_ERRORS = 1
        emb._error_count = 1
        assert not emb.is_available()

    @pytest.mark.asyncio
    async def test_returns_zeros_when_unavailable(self):
        emb = OllamaEmbedder()
        emb._available = False
        vec = await emb.embed("test text")
        assert all(v == 0.0 for v in vec)
        assert len(vec) == emb.dimension


# ── ScholarDog β1 Integration ────────────────────────────────────────────

class TestScholarDogPGVector:
    def _make_cell(self, content="test content", reality="CODE") -> Cell:
        return Cell(
            reality=reality,
            analysis="JUDGE",
            time_dim="PRESENT",
            content=content,
            context="test context",
        )

    def test_set_embedder_stores_embedder(self):
        scholar = ScholarDog()
        emb = DummyEmbedder()
        scholar.set_embedder(emb)
        assert scholar._embedder is emb

    @pytest.mark.asyncio
    async def test_analyze_without_embedder_uses_tfidf(self):
        """Without embedder, ScholarDog still works (TF-IDF path)."""
        scholar = ScholarDog()
        cell = self._make_cell()
        judgment = await scholar.analyze(cell)
        assert judgment is not None
        assert 0.0 <= judgment.q_score <= 100.0

    @pytest.mark.asyncio
    async def test_analyze_with_dummy_embedder_but_no_db(self):
        """With embedder but no DB pool, falls back to TF-IDF."""
        scholar = ScholarDog()
        emb = DummyEmbedder()
        scholar.set_embedder(emb)  # No db_pool set
        cell = self._make_cell()
        judgment = await scholar.analyze(cell)
        assert judgment is not None
        assert 0.0 <= judgment.q_score <= 100.0

    @pytest.mark.asyncio
    async def test_vector_search_path_called_when_embedder_and_db(self):
        """With embedder + DB, _vector_search_path() is attempted."""
        scholar = ScholarDog()
        emb = DummyEmbedder()
        scholar.set_embedder(emb)

        # Mock DB pool
        mock_pool = MagicMock()
        scholar._db_pool = mock_pool

        # Mock ScholarRepository.search_similar_by_embedding to return empty
        with patch("cynic.cognition.neurons.scholar.ScholarDog._vector_search_path", new_callable=AsyncMock) as mock_vsp:
            mock_vsp.return_value = None  # Vector search returns None → fall back to TF-IDF
            cell = self._make_cell()
            scholar._buffer = []  # Empty buffer for TF-IDF fallback
            judgment = await scholar.analyze(cell)
            mock_vsp.assert_called_once()

    @pytest.mark.asyncio
    async def test_learn_with_embedder_creates_fire_and_forget(self):
        """With embedder + DB, learn() schedules embedding task."""
        scholar = ScholarDog()
        emb = DummyEmbedder()
        scholar.set_embedder(emb)

        # Mock DB pool (running event loop)
        mock_pool = MagicMock()
        scholar._db_pool = mock_pool

        # Mock the create_task call
        with patch("asyncio.get_event_loop") as mock_loop:
            mock_event_loop = MagicMock()
            mock_event_loop.is_running.return_value = True
            mock_loop.return_value = mock_event_loop

            scholar.learn("def foo(): pass", 75.0, cell_id="test-1")

        assert len(scholar._buffer) == 1
        assert scholar._buffer[0].q_score == 75.0

    @pytest.mark.asyncio
    async def test_vector_search_path_aggregates_results(self):
        """_vector_search_path aggregates similar entries correctly."""
        scholar = ScholarDog()
        emb = DummyEmbedder()
        scholar.set_embedder(emb)
        mock_pool = MagicMock()
        scholar._db_pool = mock_pool

        # Mock search to return 2 similar entries
        mock_results = [
            {"cell_id": "1", "cell_text": "foo", "q_score": 70.0, "reality": "CODE",
             "ts": 0.0, "similarity": 0.9},
            {"cell_id": "2", "cell_text": "bar", "q_score": 80.0, "reality": "CODE",
             "ts": 0.0, "similarity": 0.7},
        ]

        # Mock ScholarDog._scholar_repo directly (what the code actually uses)
        mock_repo = AsyncMock()
        mock_repo.search_similar_by_embedding = AsyncMock(return_value=mock_results)
        scholar._scholar_repo = mock_repo

        cell = self._make_cell()
        start = 0.0
        import time
        start = time.perf_counter()

        judgment = await scholar._vector_search_path(cell, "test text", start)

        assert judgment is not None
        # Weighted q: (70*0.9 + 80*0.7) / (0.9+0.7) = (63+56)/1.6 = 74.375
        assert abs(judgment.q_score - 74.375) < 1.0
        assert judgment.evidence["path"] == "pgvector"
        assert judgment.evidence["neighbors_found"] == 2
