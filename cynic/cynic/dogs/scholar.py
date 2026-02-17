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

Design contract:
  - learn(cell_text, q_score) — called after consensus to add to buffer
  - analyze(cell) — TF-IDF lookup against current buffer
  - Buffer is instance-level (reset on restart; Phase 2 = PostgreSQL warm-load)
  - Never touches QTable, never creates side effects in other systems
"""
from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity

from cynic.core.phi import PHI_INV, PHI_INV_2, MAX_Q_SCORE, MAX_CONFIDENCE, phi_bound_score, fibonacci
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.dogs.base import (
    AbstractDog, DogCapabilities, DogHealth, DogJudgment,
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


class ScholarDog(AbstractDog):
    """
    Scholar (Chesed) — TF-IDF similarity search over past judgment history.

    Analyzes new cells by asking: "Have I seen something like this before?
    What was the verdict?" Returns a prediction based on K nearest neighbors.

    Three modes:
      1. Cold (empty buffer): neutral GROWL at low confidence
      2. Warm (1-20 entries): similarity lookup, moderate confidence
      3. Rich (21+ entries, F(8)): reliable predictions, rising confidence

    Learning: call scholar.learn(cell_text, q_score) after consensus
    to grow the buffer. Buffer auto-evicts oldest entries at BUFFER_MAX.
    """

    def __init__(self) -> None:
        super().__init__(DogId.SCHOLAR)
        self._buffer: List[BufferEntry] = []
        self._vectorizer: Optional[TfidfVectorizer] = None
        self._matrix: Optional[np.ndarray] = None
        self._matrix_dirty: bool = True  # Rebuild matrix on next analyze()
        self._lookups: int = 0
        self._hits: int = 0     # Lookups that found ≥1 neighbor above MIN_SIMILARITY
        self._cold_lookups: int = 0

    def get_capabilities(self) -> DogCapabilities:
        return DogCapabilities(
            dog_id=DogId.SCHOLAR,
            sefirot="Chesed — Loving-Kindness",
            consciousness_min=ConsciousnessLevel.MICRO,  # TF-IDF too slow for REFLEX
            uses_llm=False,
            supported_realities={"CODE", "SOLANA", "MARKET", "SOCIAL", "HUMAN", "CYNIC", "COSMOS"},
            supported_analyses={"PERCEIVE", "JUDGE", "DECIDE", "ACT", "LEARN", "ACCOUNT", "EMERGE"},
            technology="TF-IDF cosine similarity (sklearn, in-memory buffer)",
            max_concurrent=8,   # Read-heavy, mostly safe to parallelize
        )

    async def analyze(self, cell: Cell, **kwargs: Any) -> DogJudgment:
        """
        Find similar past cells, return weighted-average q_score.

        If buffer is cold or no similar cells found → neutral GROWL.
        """
        start = time.perf_counter()
        self._lookups += 1

        cell_text = self._extract_text(cell)

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
        q_score, confidence, evidence = self._aggregate(neighbors, sims)

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

    def _top_k_neighbors(self, sims: np.ndarray) -> List[int]:
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
        neighbor_indices: List[int],
        sims: np.ndarray,
    ) -> Tuple[float, float, Dict[str, Any]]:
        """
        Compute q_score and confidence from K neighbors.

        q_score = similarity-weighted average of neighbor q_scores
        confidence = f(best_similarity, consistency, buffer_richness)
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

        # Final confidence: best_sim × consistency × richness, bounded to MAX_CONFIDENCE
        raw_confidence = best_sim * consistency * richness
        confidence = min(max(raw_confidence, COLD_CONFIDENCE), MAX_CONFIDENCE)

        evidence = {
            "neighbors_found": len(neighbor_indices),
            "best_similarity": round(best_sim, 3),
            "mean_q": round(mean_q, 1),
            "weighted_q": round(weighted_q, 1),
            "consistency": round(consistency, 3),
            "buffer_size": len(self._buffer),
            "neighbor_realities": [self._buffer[i].reality for i in neighbor_indices],
        }

        return weighted_q, confidence, evidence

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
