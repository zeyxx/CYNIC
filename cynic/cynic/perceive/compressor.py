"""
CYNIC ContextCompressor — Token Budget Management (γ2)

Manages the context window budget for LLM calls. When accumulated session
history exceeds the token budget, the compressor ranks and selects the most
informative content to keep.

Strategy: TF-IDF sentence ranking — sentences with rare, discriminating
terms score higher. The compressor keeps the top-scored sentences that fit
within the budget (preserving original order in output).

φ-derived constants:
  PHI_INV_2 = 0.382 — minimum essential ratio always preserved
  PHI_INV   = 0.618 — default target compression ratio

Usage:
    compressor = ContextCompressor(max_tokens=4096)
    compressor.add("user: def foo(): pass  # What's the quality?")
    compressor.add("cynic: GROWL Q=38.2 — missing type hints")
    context = compressor.get_compressed_context(budget=1024)

Token estimation: ~1.3 tokens/word (conservative approximation).
"""
from __future__ import annotations

import logging
import re
from typing import Any, Dict, List, Optional, Tuple

from cynic.core.phi import PHI_INV, PHI_INV_2, fibonacci

logger = logging.getLogger("cynic.perceive.compressor")

# Default max tokens (F(12) = 144 × 32 ≈ 4096)
DEFAULT_MAX_TOKENS: int = 4096

# Tokens per word (conservative approximation for average code+prose)
_TOKENS_PER_WORD: float = 1.3

# Minimum chunk tokens to consider non-trivial
_MIN_CHUNK_TOKENS: int = 5

# Attention feedback (SAGE → Compressor bidirectional loop)
_ATTENTION_ALPHA: float = PHI_INV_2          # EMA α = 0.382 — conservative; past dominates
_ATTENTION_THRESHOLD: float = 0.05           # Min Jaccard similarity to boost a chunk


# ── Token utilities ────────────────────────────────────────────────────────

def estimate_tokens(text: str) -> int:
    """
    Approximate token count for text.
    Uses word count × 1.3 (conservative; code tends to be token-dense).
    """
    words = len(text.split())
    return max(1, int(words * _TOKENS_PER_WORD))


# ── TF-IDF scorer ─────────────────────────────────────────────────────────

def _tfidf_score_sentences(sentences: list[str]) -> list[tuple[str, float]]:
    """
    Score sentences using TF-IDF.
    Falls back to term-frequency scoring if scikit-learn unavailable.
    Returns list of (sentence, score) sorted by score descending.
    """
    if not sentences:
        return []

    try:
        from sklearn.feature_extraction.text import TfidfVectorizer
        import numpy as np

        # Fit TF-IDF on all sentences
        vec = TfidfVectorizer(
            max_features=500,
            strip_accents="unicode",
            analyzer="word",
            token_pattern=r"(?u)\b\w+\b",
            ngram_range=(1, 2),
            sublinear_tf=True,
        )
        try:
            tfidf_matrix = vec.fit_transform(sentences)
            # Score = mean TF-IDF weight per sentence (row sum / row length)
            scores = np.asarray(tfidf_matrix.sum(axis=1)).flatten()
            # Normalize [0, 1]
            max_s = scores.max()
            if max_s > 0:
                scores = scores / max_s
        except ValueError:
            # All sentences identical or too sparse
            scores = [1.0 / len(sentences)] * len(sentences)

        return sorted(
            zip(sentences, scores.tolist()),
            key=lambda x: x[1],
            reverse=True,
        )

    except ImportError:
        # Fallback: term-frequency scoring
        return _tf_score_sentences(sentences)


def _tf_score_sentences(sentences: list[str]) -> list[tuple[str, float]]:
    """Fallback term-frequency scorer (no sklearn needed)."""
    # Count term frequencies across all sentences
    all_words: dict[str, int] = {}
    for sent in sentences:
        for word in re.findall(r"\b\w+\b", sent.lower()):
            all_words[word] = all_words.get(word, 0) + 1

    # Compute IDF-like inverse frequency weight
    n = len(sentences)
    idf: dict[str, float] = {
        word: 1.0 / (count + 1) for word, count in all_words.items()
    }

    def score(sentence: str) -> float:
        words = re.findall(r"\b\w+\b", sentence.lower())
        if not words:
            return 0.0
        return sum(idf.get(w, 0.0) for w in words) / len(words)

    scored = [(s, score(s)) for s in sentences]
    max_s = max(s for _, s in scored) if scored else 1.0
    if max_s > 0:
        scored = [(s, v / max_s) for s, v in scored]

    return sorted(scored, key=lambda x: x[1], reverse=True)


# ── ContextCompressor ──────────────────────────────────────────────────────

class ContextCompressor:
    """
    Intelligent context window manager.

    Accumulates session chunks (conversation turns, code snippets, judgments).
    When get_compressed_context() is called with a budget, returns the most
    informative content that fits within the token budget.

    Preserves original temporal order in output (not sorted by score).
    Uses TF-IDF to prefer informative over repetitive content.
    """

    def __init__(self, max_tokens: int = DEFAULT_MAX_TOKENS) -> None:
        self._max_tokens = max_tokens
        self._chunks: list[str] = []
        self._chunk_attention: list[float] = []  # Per-chunk attention weight (default 1.0)
        self._total_input_tokens: int = 0
        self._total_output_tokens: int = 0
        self._compressions: int = 0
        # Rolling max chunks (F(11) = 89 — generous history window)
        self._max_chunks: int = fibonacci(11)

    # ── Public API ────────────────────────────────────────────────────────

    def add(self, text: str) -> None:
        """
        Add a chunk to the session history.

        Chunks are conversation turns, code snippets, judgment summaries, etc.
        Trims oldest chunks when over max_chunks (rolling window).
        """
        if not text or not text.strip():
            return

        self._chunks.append(text)
        self._chunk_attention.append(1.0)  # New chunks start with neutral attention
        self._total_input_tokens += estimate_tokens(text)

        # Rolling window — drop oldest when full
        if len(self._chunks) > self._max_chunks:
            dropped = self._chunks.pop(0)
            if self._chunk_attention:
                self._chunk_attention.pop(0)
            logger.debug(
                "ContextCompressor: dropped oldest chunk (%d tokens)",
                estimate_tokens(dropped),
            )

    def boost(self, query: str, weight: float) -> None:
        """
        Signal attention: query text was relevant to a judgment of quality=weight.

        Updates per-chunk attention via EMA.  Chunks with high Jaccard similarity
        to query get attention boosted; future compressions will prioritize them.

        Called by SageDog after each judgment to close the feedback loop:
          Compressor → (context) → SAGE → (attention signal) → Compressor

        Args:
            query:  Text that SAGE just judged (cell content + context).
            weight: Judgment quality, normalized [0, 1] (q_score / MAX_Q_SCORE).
        """
        if not self._chunks or weight <= 0.0 or not query:
            return

        # Sync attention list length with chunks
        while len(self._chunk_attention) < len(self._chunks):
            self._chunk_attention.append(1.0)

        query_words = frozenset(re.findall(r"\b\w+\b", query.lower()))
        if not query_words:
            return

        for i, chunk in enumerate(self._chunks):
            chunk_words = frozenset(re.findall(r"\b\w+\b", chunk.lower()))
            if not chunk_words:
                continue

            # Jaccard similarity: |A ∩ B| / |A ∪ B|
            inter = len(query_words & chunk_words)
            union = len(query_words | chunk_words)
            sim = inter / union if union > 0 else 0.0

            if sim < _ATTENTION_THRESHOLD:
                continue  # Below noise floor — skip

            # Attention boost: neutral=1.0, max = 1 + weight×sim×φ⁻¹
            boost_val = 1.0 + weight * sim * PHI_INV

            # EMA update (α = PHI_INV_2 = 0.382 — conservative; past dominates)
            self._chunk_attention[i] = (
                (1.0 - _ATTENTION_ALPHA) * self._chunk_attention[i]
                + _ATTENTION_ALPHA * boost_val
            )

    def compress(
        self,
        chunks: list[str],
        budget: int,
        chunk_attentions: list[float] | None = None,
    ) -> str:
        """
        Compress chunks to fit within token budget.

        If total tokens ≤ budget, returns full content unchanged.
        Otherwise, ranks sentences by TF-IDF and greedily selects the
        highest-scored ones that fit, preserving original order in output.

        Args:
            chunks: List of text chunks to compress.
            budget:  Max tokens in the output.

        Returns:
            Compressed string (within budget).
        """
        if not chunks:
            return ""

        full_text = "\n".join(c for c in chunks if c.strip())
        total_tokens = estimate_tokens(full_text)

        if total_tokens <= budget:
            return full_text  # Already fits — no compression needed

        self._compressions += 1
        logger.debug(
            "ContextCompressor: compressing %d→%d tokens (ratio=%.2f)",
            total_tokens, budget, budget / total_tokens,
        )

        # Split into sentences for ranking
        sentences = _split_sentences(full_text)

        if not sentences:
            # Fallback: hard truncate by words
            words = full_text.split()
            approx_words = int(budget / _TOKENS_PER_WORD)
            return " ".join(words[:approx_words])

        # Rank sentences by TF-IDF
        scored = _tfidf_score_sentences(sentences)

        # Apply per-chunk attention weights: SAGE feedback boosts similar past chunks
        if chunk_attentions:
            # Map each sentence back to its source chunk's attention weight
            sentence_attn: dict[str, float] = {}
            for i, chunk in enumerate(chunks):
                attn = chunk_attentions[i] if i < len(chunk_attentions) else 1.0
                for sent in _split_sentences(chunk):
                    sentence_attn[sent] = attn  # Last chunk wins for duplicates
            if sentence_attn:
                scored = [
                    (s, score * sentence_attn.get(s, 1.0))
                    for s, score in scored
                ]
                scored.sort(key=lambda x: x[1], reverse=True)

        # Greedy selection: take highest-scored until budget exhausted
        # Always keep at least the first sentence (recency anchor)
        selected_indices: list[int] = [0]
        used_tokens = estimate_tokens(sentences[0])
        sentence_index = {s: i for i, s in enumerate(sentences)}

        for sentence, _score in scored:
            idx = sentence_index.get(sentence, -1)
            if idx == 0:
                continue  # Already included (first sentence)
            tok = estimate_tokens(sentence)
            if tok < _MIN_CHUNK_TOKENS:
                continue  # Skip trivial sentences
            if used_tokens + tok <= budget:
                selected_indices.append(idx)
                used_tokens += tok

        # Sort selected back to original temporal order
        selected_indices.sort()
        result = "\n".join(sentences[i] for i in selected_indices)
        self._total_output_tokens += used_tokens

        return result

    def get_compressed_context(self, budget: int | None = None) -> str:
        """
        Return compressed context from accumulated session history.

        Passes per-chunk attention weights to compress() so SAGE-boosted
        chunks are prioritized over equal-TF-IDF alternatives.

        Args:
            budget: Max tokens (defaults to self._max_tokens).

        Returns:
            Compressed string ready for LLM context injection.
        """
        effective_budget = budget if budget is not None else self._max_tokens
        # Sync attention list with current chunks
        attn = list(self._chunk_attention[:len(self._chunks)])
        while len(attn) < len(self._chunks):
            attn.append(1.0)
        return self.compress(self._chunks, effective_budget, chunk_attentions=attn)

    def to_dict(self) -> dict[str, Any]:
        """
        Serialize compressor state for checkpointing.

        Returns a JSON-serializable dict with chunks, attention weights,
        and metadata needed to restore the session across restarts.
        """
        attn = list(self._chunk_attention[:len(self._chunks)])
        while len(attn) < len(self._chunks):
            attn.append(1.0)
        return {
            "chunks": list(self._chunks),
            "chunk_attention": attn,
            "max_tokens": self._max_tokens,
            "compressions": self._compressions,
            "total_input_tokens": self._total_input_tokens,
        }

    def restore_from_dict(self, data: dict[str, Any]) -> int:
        """
        Restore compressor state from a checkpoint dict.

        Replaces current state entirely. Trims to _max_chunks if needed.
        Returns number of chunks restored.
        """
        chunks = data.get("chunks", [])
        attns = data.get("chunk_attention", [])

        # Direct restore — skip add() to preserve attention weights
        self._chunks = list(chunks)
        self._chunk_attention = [
            attns[i] if i < len(attns) else 1.0
            for i in range(len(chunks))
        ]
        self._compressions = data.get("compressions", 0)
        self._total_input_tokens = data.get("total_input_tokens", 0)

        # Enforce rolling window
        while len(self._chunks) > self._max_chunks:
            self._chunks.pop(0)
            if self._chunk_attention:
                self._chunk_attention.pop(0)

        return len(self._chunks)

    def clear(self) -> None:
        """Clear session history (start of new session)."""
        self._chunks = []
        self._chunk_attention = []

    def stats(self) -> dict[str, Any]:
        """Return compression statistics."""
        attn = self._chunk_attention[:len(self._chunks)]
        return {
            "chunks": len(self._chunks),
            "total_input_tokens": self._total_input_tokens,
            "total_output_tokens": self._total_output_tokens,
            "compressions": self._compressions,
            "compression_ratio": round(
                self._total_output_tokens / max(self._total_input_tokens, 1), 3
            ),
            "max_tokens": self._max_tokens,
            "avg_chunk_attention": round(sum(attn) / len(attn), 3) if attn else 1.0,
            "max_chunk_attention": round(max(attn), 3) if attn else 1.0,
        }

    @property
    def chunk_count(self) -> int:
        return len(self._chunks)


# ── Sentence splitter ─────────────────────────────────────────────────────

def _split_sentences(text: str) -> list[str]:
    """
    Split text into sentence-like units for scoring.

    Splits on newlines and sentence-ending punctuation.
    Filters out empty strings and whitespace-only lines.
    """
    # Primary split on newlines
    lines = text.splitlines()
    sentences: list[str] = []
    for line in lines:
        line = line.strip()
        if not line:
            continue
        # Secondary split on sentence boundaries within long lines
        if len(line) > 300:
            parts = re.split(r"(?<=[.!?])\s+", line)
            sentences.extend(p.strip() for p in parts if p.strip())
        else:
            sentences.append(line)
    return [s for s in sentences if s]
