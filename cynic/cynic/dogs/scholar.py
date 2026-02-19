"""
CYNIC Scholar Dog — Chesed (Kindness)

SCHOLAR-lite: TF-IDF cosine similarity against in-session judgment history.
No Qdrant, no LLM — pure sklearn, zero external dependencies beyond numpy.

Full SCHOLAR (Phase 2): Qdrant vector database + LLM embeddings.

Responsibilities:
  - Maintain rolling buffer of past (cell_text, q_score) pairs
  - Find K=F(4)=3 nearest neighbors by TF-IDF cosine similarity
  - Return weighted-average q_score of similar past cells
  - High similarity + consistent past → high confidence
  - Cold buffer → neutral GROWL (cautious default)

Why Scholar?
  Chesed = Loving-Kindness. Scholar remembers what it saw before
  and treats new code with the same judgment it applied to similar code.
  "I've seen this pattern 8 times — it always ends in BARK."

φ-integration:
  Buffer: F(11)=89 cells max (rolling window)
  K neighbors: F(4)=3
  Min similarity: PHI_INV_2 = 0.382
  Cold confidence: 0.200
  Warm confidence: scales with similarity × consistency, max PHI_INV

  VETO: impossible — Scholar advises, never blocks.

Scholar ↔ QTable (recursive meta-learning):
  When QTable is injected (set_qtable), Scholar blends its TF-IDF prediction
  with QTable's historical Q-value for the current state:
    blended_q = tfidf_q × (1 - w) + qtable_q × w
  where w = min(qtable_visits / F(8), 0.618) × PHI_INV_2
  Effect: "I've seen this 8 times in TF-IDF AND QTable agrees → higher confidence"
  This is read-only QTable access (no mutation, no side effects).

Design contract:
  - learn(cell_text, q_score) — called after consensus to add to buffer
  - analyze(cell) — TF-IDF lookup against current buffer
  - Buffer is instance-level (reset on restart; Phase 2 = PostgreSQL warm-load)
  - May read QTable for confidence weighting (read-only, inject via set_qtable)
"""
from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import Any


import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity

from cynic.core.phi import PHI_INV, PHI_INV_2, MAX_Q_SCORE, MAX_CONFIDENCE, phi_bound_score, fibonacci
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.core.embeddings import EmbeddingProvider
from cynic.dogs.base import (
    AbstractDog, LLMDog, DogCapabilities, DogHealth, DogJudgment,
    DogId, HealthStatus,
)

logger = logging.getLogger("cynic.dogs.scholar")

# Buffer capacity: F(11) = 89
BUFFER_MAX: int = fibonacci(11)  # 89

# K nearest neighbors: F(4) = 3
K_NEIGHBORS: int = fibonacci(4)  # 3

# Minimum cosine similarity to count as "similar" (PHI_INV_2)
MIN_SIMILARITY: float = PHI_INV_2  # 0.382

# Confidence when buffer is cold
COLD_CONFIDENCE: float = 0.200

# Neutral q_score when no similar history
NEUTRAL_Q: float = 0.5 * MAX_Q_SCORE  # 30.9 → GROWL


@dataclass
class BufferEntry:
    """One recorded judgment in Scholar's memory."""
    cell_text: str
    q_score: float      # Final consensus q_score at time of recording
    cell_id: str = ""
    reality: str = ""
    timestamp: float = field(default_factory=time.time)


class ScholarDog(LLMDog):
    """
    Scholar (Chesed) — TF-IDF similarity search over past judgment history.

    Heuristic path: TF-IDF cosine similarity (K nearest neighbors).
    Temporal MCTS path (when LLM available): 7-perspective temporal judgment.

    Three modes (heuristic):
      1. Cold (empty buffer): neutral GROWL at low confidence
      2. Warm (1-20 entries): similarity lookup, moderate confidence
      3. Rich (21+ entries, F(8)): reliable predictions, rising confidence

    Learning: call scholar.learn(cell_text, q_score) after consensus
    to grow the buffer. Buffer auto-evicts oldest entries at BUFFER_MAX.
    """

    def __init__(self) -> None:
        super().__init__(DogId.SCHOLAR, task_type="vector_rag")
        self._buffer: list[BufferEntry] = []
        self._vectorizer: TfidfVectorizer | None = None
        self._matrix: np.ndarray | None = None
        self._matrix_dirty: bool = True  # Rebuild matrix on next analyze()
        self._lookups: int = 0
        self._hits: int = 0     # Lookups that found ≥1 neighbor above MIN_SIMILARITY
        self._cold_lookups: int = 0
        self._db_pool: Any | None = None  # asyncpg pool (None = no persistence)
        self._embedder: EmbeddingProvider | None = None  # β1: vector embedder
        self._qtable: Any | None = None  # QTable (read-only, injected via set_qtable)

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.SCHOLAR,
            sefirot="Chesed — Loving-Kindness",
            consciousness_min=ConsciousnessLevel.MICRO,  # TF-IDF too slow for REFLEX
            uses_llm=True,
            supported_realities={"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"},
            supported_analyses={"PERCEIVE", "JUDGE", "DECIDE", "ACT", "LEARN", "ACCOUNT", "EMERGE"},
            technology="TF-IDF cosine similarity (sklearn, in-memory buffer)",
            max_concurrent=8,   # Read-heavy, mostly safe to parallelize
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Route to temporal MCTS path (LLM available) or TF-IDF heuristic path.
        """
        start = time.perf_counter()
        self._lookups += 1
        cell_text = self._extract_text(cell)
        adapter = await self.get_llm()
        if adapter is not None:
            return await self._temporal_path(cell, cell_text, adapter, start)
        return await self._heuristic_path(cell, cell_text, start)

    async def _temporal_path(
        self,
        cell: Cell,
        cell_text: str,
        adapter: Any,
        start: float,
    ) -> DogJudgment:
        """7-perspective temporal MCTS judgment via Ollama."""
        from cynic.llm.temporal import temporal_judgment

        buffer_ctx = (
            f"[Scholar memory: {len(self._buffer)} past judgments | "
            f"Hit ratio: {self._hits / max(self._lookups, 1):.0%}]"
        )
        content = f"{cell_text[:1800]}\n\n{buffer_ctx}"
        tj = await temporal_judgment(adapter, content)

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=tj.phi_aggregate,
            confidence=tj.confidence,
            reasoning=(
                f"*sniff* Scholar temporal MCTS: Q={tj.phi_aggregate:.1f} "
                f"from 7 perspectives (buf={len(self._buffer)})"
            ),
            evidence=tj.to_dict(),
            latency_ms=latency,
            llm_id=tj.llm_id,
            veto=False,
        )
        self.record_judgment(judgment)
        return judgment

    async def _heuristic_path(
        self,
        cell: Cell,
        cell_text: str,
        start: float,
    ) -> DogJudgment:
        """
        β1: Try PGVector semantic search first, fall back to TF-IDF in-memory.

        Vector path: embedder + DB available → cosine similarity on stored embeddings
        TF-IDF path: fallback when no embedder or DB
        """
        # β1: PGVector path — embedder + DB + non-empty DB
        if self._embedder is not None and self._embedder.is_available() and self._db_pool is not None:
            try:
                judgment = await self._vector_search_path(cell, cell_text, start)
                if judgment is not None:
                    return judgment
            except Exception as e:
                logger.debug("ScholarDog: vector search failed, falling back: %s", e)

        # TF-IDF fallback path
        if len(self._buffer) == 0:
            self._cold_lookups += 1
            return self._neutral_judgment(cell, start, reason="cold-buffer")

        # Rebuild TF-IDF matrix if dirty
        if self._matrix_dirty:
            self._rebuild_matrix()

        if self._vectorizer is None or self._matrix is None:
            self._cold_lookups += 1
            return self._neutral_judgment(cell, start, reason="no-vectorizer")

        # Vectorize query
        try:
            query_vec = self._vectorizer.transform([cell_text])
        except Exception as e:
            logger.debug("Scholar transform error: %s", e)
            return self._neutral_judgment(cell, start, reason=f"transform-error")

        # Cosine similarity against buffer matrix
        sims = cosine_similarity(query_vec, self._matrix)[0]

        # Find K best neighbors above MIN_SIMILARITY threshold
        neighbors = self._top_k_neighbors(sims)

        if not neighbors:
            self._cold_lookups += 1
            return self._neutral_judgment(cell, start, reason="no-similar-cells")

        self._hits += 1
        q_score, confidence, evidence = self._aggregate(neighbors, sims, state_key=cell.state_key())

        reasoning = (
            f"*sniff* Found {len(neighbors)} similar past cells "
            f"(best sim={evidence['best_similarity']:.0%}, mean Q={evidence['mean_q']:.1f}) "
            f"→ predicts Q={q_score:.1f}"
        )

        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=phi_bound_score(q_score),
            confidence=confidence,
            reasoning=reasoning,
            evidence=evidence,
            latency_ms=latency,
            veto=False,  # Scholar advises, never blocks
        )
        self.record_judgment(judgment)
        return judgment

    # ── Learning API ────────────────────────────────────────────────────────

    def set_db_pool(self, pool: Any) -> None:
        """
        Inject asyncpg pool for DB persistence.
        When set, learn() will fire-and-forget append to scholar_buffer table.
        Call this before any learn() calls for full persistence coverage.
        """
        self._db_pool = pool
        logger.info("ScholarDog: DB persistence enabled (pool injected)")

    def set_embedder(self, embedder: EmbeddingProvider) -> None:
        """
        β1: Inject embedding provider for PGVector semantic search.

        When set:
          - learn() generates and persists embeddings alongside text
          - analyze() uses vector similarity search (DB must be available)
          - Falls back to TF-IDF if embedder unavailable or DB not set

        Call this at kernel startup, after set_db_pool().
        """
        self._embedder = embedder
        logger.info(
            "ScholarDog: PGVector enabled (model=%s, dim=%d)",
            getattr(embedder, "_model", "unknown"),
            embedder.dimension,
        )

    def set_qtable(self, qtable: Any) -> None:
        """
        Inject QTable for recursive meta-learning (read-only access).

        When set, Scholar blends its TF-IDF prediction with QTable's historical
        Q-value for the current cell's state — the Scholar↔QTable feedback loop.
        """
        self._qtable = qtable
        logger.info("ScholarDog: QTable injected — recursive meta-learning enabled")

    async def load_from_db(self, pool: Any) -> int:
        """
        Warm-start buffer from DB (call once at kernel startup).
        Returns number of entries loaded.
        """
        from cynic.core.storage.postgres import ScholarRepository
        try:
            repo = ScholarRepository()
            entries = await repo.recent_entries(limit=BUFFER_MAX)
            for e in entries:
                self._buffer.append(BufferEntry(
                    cell_text=e.get("cell_text", ""),
                    q_score=e.get("q_score", NEUTRAL_Q),
                    cell_id=e.get("cell_id", ""),
                    reality=e.get("reality", ""),
                    timestamp=e.get("ts", time.time()),
                ))
            if entries:
                self._matrix_dirty = True
                logger.info("ScholarDog: warm-start %d entries from DB", len(entries))
            return len(entries)
        except Exception as exc:
            logger.warning("ScholarDog: DB warm-start failed: %s", exc)
            return 0

    def load_from_entries(self, entries: list) -> int:
        """
        Warm-start buffer from a list of dicts (source-agnostic).

        Used by SurrealDB path: server.py fetches scholar rows from SurrealDB,
        passes them here (oldest-first). Same logic as load_from_db() without asyncpg.

        entries: [{"cell_text", "q_score", "cell_id", "reality", "ts"}, ...]
        Returns: count of entries loaded.
        """
        for e in entries:
            self._buffer.append(BufferEntry(
                cell_text=e.get("cell_text", ""),
                q_score=e.get("q_score", NEUTRAL_Q),
                cell_id=e.get("cell_id", ""),
                reality=e.get("reality", ""),
                timestamp=e.get("ts", time.time()),
            ))
        if entries:
            self._matrix_dirty = True
            logger.info("ScholarDog: warm-start %d entries (load_from_entries)", len(entries))
        return len(entries)

    def learn(self, cell_text: str, q_score: float, cell_id: str = "", reality: str = "") -> None:
        """
        Record a completed judgment into Scholar's memory buffer.

        Call this AFTER consensus is reached, with the final consensus q_score.
        Not called during analyze() to avoid feedback loop contamination.
        """
        # Skip if identical cell_id already in buffer
        if cell_id and any(e.cell_id == cell_id for e in self._buffer):
            return

        # Ensure string (cell.content may be dict/None)
        safe_text = cell_text if isinstance(cell_text, str) else str(cell_text)
        entry = BufferEntry(
            cell_text=safe_text[:2000],  # Cap at 2k chars
            q_score=phi_bound_score(q_score),
            cell_id=cell_id,
            reality=reality,
        )
        self._buffer.append(entry)

        # Rolling eviction: keep newest BUFFER_MAX entries
        if len(self._buffer) > BUFFER_MAX:
            self._buffer.pop(0)

        self._matrix_dirty = True  # Force rebuild on next analyze

        # Persist to DB (fire-and-forget — never block learn())
        if self._db_pool is not None:
            import asyncio
            from cynic.core.storage.postgres import ScholarRepository
            embedder = self._embedder  # capture before async

            async def _persist(e=entry):
                try:
                    embedding = None
                    embed_model = ""
                    if embedder is not None and embedder.is_available():
                        embedding = await embedder.embed(e.cell_text)
                        embed_model = getattr(embedder, "_model", "unknown")
                    await ScholarRepository().append({
                        "cell_id": e.cell_id,
                        "cell_text": e.cell_text,
                        "q_score": e.q_score,
                        "reality": e.reality,
                        "timestamp": e.timestamp,
                        "embedding": embedding,
                        "embed_model": embed_model,
                    })
                except Exception:
                    pass  # Never propagate DB errors from learn()
            try:
                asyncio.get_running_loop().create_task(_persist())
            except Exception:
                pass

    # ── β1: PGVector Search ───────────────────────────────────────────────────

    async def _vector_search_path(
        self,
        cell: Cell,
        cell_text: str,
        start: float,
    ) -> DogJudgment | None:
        """
        β1: Dense vector similarity search against scholar_buffer table.

        1. Embed query text via EmbeddingProvider
        2. Cosine similarity against stored embeddings (Python, upgradeable to pgvector)
        3. Aggregate top-K results → q_score + confidence

        Returns None if no results found (caller falls back to TF-IDF).
        """
        from cynic.core.storage.postgres import ScholarRepository

        query_vec = await self._embedder.embed(cell_text)  # type: ignore[union-attr]
        if not query_vec or all(v == 0.0 for v in query_vec):
            return None

        repo = ScholarRepository()
        similar = await repo.search_similar_by_embedding(
            query_embedding=query_vec,
            limit=K_NEIGHBORS,
            min_similarity=MIN_SIMILARITY,
        )

        if not similar:
            return None

        self._hits += 1

        # Similarity-weighted q_score
        total_sim = sum(r["similarity"] for r in similar)
        weighted_q = sum(r["q_score"] * r["similarity"] for r in similar) / total_sim
        best_sim = max(r["similarity"] for r in similar)

        richness = min(1.0, len(similar) / K_NEIGHBORS)
        raw_conf = best_sim * richness
        confidence = min(max(raw_conf, COLD_CONFIDENCE), MAX_CONFIDENCE)

        latency = (time.perf_counter() - start) * 1000
        reasoning = (
            f"*sniff* Vector search: {len(similar)} similar embeddings "
            f"(best sim={best_sim:.0%}) → Q={weighted_q:.1f}"
        )
        evidence = {
            "path": "pgvector",
            "neighbors_found": len(similar),
            "best_similarity": round(best_sim, 3),
            "weighted_q": round(weighted_q, 1),
            "embed_dim": len(query_vec),
        }
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=phi_bound_score(weighted_q),
            confidence=confidence,
            reasoning=reasoning,
            evidence=evidence,
            latency_ms=latency,
            veto=False,
        )
        self.record_judgment(judgment)
        return judgment

    # ── Internals ────────────────────────────────────────────────────────────

    def _extract_text(self, cell: Cell) -> str:
        """Extract searchable text from a Cell."""
        parts = []
        if cell.content:
            raw = cell.content if isinstance(cell.content, str) else str(cell.content)
            parts.append(raw[:2000])
        # Augment with structured metadata for better matching
        parts.append(f"reality:{cell.reality}")
        parts.append(f"analysis:{cell.analysis}")
        return " ".join(parts) if parts else cell.state_key()

    def _rebuild_matrix(self) -> None:
        """Rebuild TF-IDF vectorizer and matrix from current buffer."""
        if not self._buffer:
            self._vectorizer = None
            self._matrix = None
            self._matrix_dirty = False
            return

        texts = [e.cell_text for e in self._buffer]
        try:
            vect = TfidfVectorizer(
                max_features=512,     # Cap vocabulary (speed)
                sublinear_tf=True,    # Log normalization
                min_df=1,
                analyzer="word",
                token_pattern=r"[a-zA-Z_][a-zA-Z0-9_]{1,}",  # code tokens
            )
            matrix = vect.fit_transform(texts)
            self._vectorizer = vect
            self._matrix = matrix
            self._matrix_dirty = False
            logger.debug("Scholar rebuilt TF-IDF matrix: %d docs", len(self._buffer))
        except Exception as e:
            logger.warning("Scholar TF-IDF rebuild failed: %s", e)
            self._vectorizer = None
            self._matrix = None
            self._matrix_dirty = False

    def _top_k_neighbors(self, sims: np.ndarray) -> list[int]:
        """Return indices of top-K neighbors above MIN_SIMILARITY, sorted desc."""
        above_threshold = [
            (i, sims[i])
            for i in range(len(sims))
            if sims[i] >= MIN_SIMILARITY
        ]
        above_threshold.sort(key=lambda x: x[1], reverse=True)
        return [idx for idx, _ in above_threshold[:K_NEIGHBORS]]

    def _aggregate(
        self,
        neighbor_indices: list[int],
        sims: np.ndarray,
        state_key: str = "",
    ) -> tuple[float, float, dict[str, Any]]:
        """
        Compute q_score and confidence from K neighbors.

        q_score = similarity-weighted average of neighbor q_scores
              [+ Q-Table blend if qtable injected]
        confidence = f(best_similarity, consistency, buffer_richness, qtable_bonus)
        """
        neighbor_q = [self._buffer[i].q_score for i in neighbor_indices]
        neighbor_sims = [float(sims[i]) for i in neighbor_indices]

        # Similarity-weighted mean q_score
        total_weight = sum(neighbor_sims)
        weighted_q = sum(q * s for q, s in zip(neighbor_q, neighbor_sims)) / total_weight

        # Consistency: 1 - (std_dev / MAX_Q_SCORE) — high consistency → tighter prediction
        if len(neighbor_q) > 1:
            consistency = 1.0 - (float(np.std(neighbor_q)) / MAX_Q_SCORE)
        else:
            consistency = 0.7  # Single neighbor: moderate consistency

        best_sim = max(neighbor_sims)
        mean_q = float(np.mean(neighbor_q))

        # Buffer richness bonus (rises toward PHI_INV as buffer fills)
        richness = min(1.0, len(self._buffer) / fibonacci(8))  # saturates at F(8)=21

        # ── Scholar ↔ QTable recursive meta-learning ─────────────────────────
        # If QTable has been injected and knows this state, blend its prediction.
        # Max QTable influence: PHI_INV_2 = 38.2% (never dominates TF-IDF)
        qtable_blend_applied = False
        qtable_q = weighted_q  # default: no blend
        qtable_confidence_bonus = 0.0

        if self._qtable is not None and state_key:
            _best_act  = self._qtable.exploit(state_key)  # best known action, not fixed WAG pivot
            qtable_raw = self._qtable.predict_q(state_key, _best_act) * MAX_Q_SCORE
            qtable_visits = sum(
                e.visits
                for e in self._qtable._table.get(state_key, {}).values()
            )
            # Blend weight: grows with QTable visits, capped at PHI_INV_2 = 0.382
            blend_weight = min(qtable_visits / fibonacci(8), PHI_INV_2)
            if blend_weight > 0:
                # Blend: TF-IDF stays dominant, QTable provides calibration
                qtable_q = weighted_q * (1.0 - blend_weight) + qtable_raw * blend_weight
                # Confidence bonus: QTable agreement with TF-IDF → higher trust
                agreement = 1.0 - abs(qtable_raw - weighted_q) / MAX_Q_SCORE
                qtable_confidence_bonus = blend_weight * agreement * PHI_INV_2
                qtable_blend_applied = True

        # Final confidence: best_sim × consistency × richness + qtable_bonus
        raw_confidence = best_sim * consistency * richness + qtable_confidence_bonus
        confidence = min(max(raw_confidence, COLD_CONFIDENCE), MAX_CONFIDENCE)

        evidence: dict[str, Any] = {
            "neighbors_found": len(neighbor_indices),
            "best_similarity": round(best_sim, 3),
            "mean_q": round(mean_q, 1),
            "weighted_q": round(weighted_q, 1),
            "consistency": round(consistency, 3),
            "buffer_size": len(self._buffer),
            "neighbor_realities": [self._buffer[i].reality for i in neighbor_indices],
        }
        if qtable_blend_applied:
            evidence["qtable_blend"] = round(qtable_q, 1)

        return qtable_q, confidence, evidence

    def _neutral_judgment(
        self,
        cell: Cell,
        start: float,
        reason: str = "no-data",
    ) -> DogJudgment:
        """Neutral GROWL when Scholar has no relevant history."""
        latency = (time.perf_counter() - start) * 1000
        judgment = DogJudgment(
            dog_id=self.dog_id,
            cell_id=cell.cell_id,
            q_score=phi_bound_score(NEUTRAL_Q),
            confidence=COLD_CONFIDENCE,
            reasoning=f"*head tilt* No similar past cells ({reason}) — defaulting to neutral GROWL",
            evidence={"reason": reason, "buffer_size": len(self._buffer)},
            latency_ms=latency,
            veto=False,
        )
        self.record_judgment(judgment)
        return judgment

    async def health_check(self) -> DogHealth:
        total = self._lookups
        hit_ratio = self._hits / max(total, 1)
        status = (
            HealthStatus.HEALTHY  if hit_ratio >= 0.5 and len(self._buffer) >= fibonacci(4) else
            HealthStatus.DEGRADED if total > 0 else
            HealthStatus.UNKNOWN
        )
        return DogHealth(
            dog_id=self.dog_id,
            status=status,
            latency_p50_ms=self.avg_latency_ms,
            details=(
                f"Buffer: {len(self._buffer)}/{BUFFER_MAX}, "
                f"Lookups: {total}, Hits: {self._hits} ({hit_ratio:.0%}), "
                f"Cold: {self._cold_lookups}"
            ),
        )
