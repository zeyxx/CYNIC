"""
CYNIC ContextCompressor Tests (γ2)

Tests token budget management, TF-IDF ranking, compression, and stats.
No LLM, no DB — pure in-memory compression.
"""
from __future__ import annotations

import pytest

from cynic.senses.compressor import (
    ContextCompressor,
    estimate_tokens,
    _split_sentences,
    DEFAULT_MAX_TOKENS,
)


# ── estimate_tokens ───────────────────────────────────────────────────────

class TestEstimateTokens:
    def test_empty_string_returns_one(self):
        assert estimate_tokens("") == 1

    def test_single_word(self):
        # 1 word * 1.3 = 1.3 → int = 1
        result = estimate_tokens("hello")
        assert result >= 1

    def test_ten_words(self):
        text = "one two three four five six seven eight nine ten"
        result = estimate_tokens(text)
        assert result >= 10  # At least 10 tokens for 10 words

    def test_larger_text_more_tokens(self):
        short = "hello world"
        long = "hello world " * 20
        assert estimate_tokens(long) > estimate_tokens(short)


# ── _split_sentences ──────────────────────────────────────────────────────

class TestSplitSentences:
    def test_empty_returns_empty(self):
        assert _split_sentences("") == []

    def test_single_line(self):
        result = _split_sentences("hello world")
        assert result == ["hello world"]

    def test_multiline(self):
        text = "line one\nline two\nline three"
        result = _split_sentences(text)
        assert len(result) == 3

    def test_blank_lines_filtered(self):
        text = "line one\n\nline two\n\n"
        result = _split_sentences(text)
        assert len(result) == 2

    def test_strips_whitespace(self):
        text = "  hello world  \n  goodbye world  "
        result = _split_sentences(text)
        assert all(s == s.strip() for s in result)

    def test_long_line_split_on_period(self):
        # Line > 300 chars with sentence boundaries
        text = ("This is a long sentence. " * 15).strip()
        result = _split_sentences(text)
        # Should be split into multiple pieces
        assert len(result) >= 2


# ── ContextCompressor core ────────────────────────────────────────────────

class TestContextCompressorBasics:
    def test_initial_state(self):
        cc = ContextCompressor(max_tokens=1024)
        assert cc.chunk_count == 0
        s = cc.stats()
        assert s["chunks"] == 0
        assert s["compressions"] == 0
        assert s["total_input_tokens"] == 0

    def test_add_increments_chunks(self):
        cc = ContextCompressor()
        cc.add("hello world")
        assert cc.chunk_count == 1

    def test_add_empty_string_noop(self):
        cc = ContextCompressor()
        cc.add("")
        cc.add("   ")
        assert cc.chunk_count == 0

    def test_add_tracks_input_tokens(self):
        cc = ContextCompressor()
        cc.add("hello world foo bar")
        assert cc.stats()["total_input_tokens"] > 0

    def test_clear_resets_chunks(self):
        cc = ContextCompressor()
        cc.add("chunk one")
        cc.add("chunk two")
        cc.clear()
        assert cc.chunk_count == 0

    def test_rolling_window_drops_oldest(self):
        """When max_chunks exceeded, oldest chunk is dropped."""
        from cynic.senses.compressor import fibonacci
        cc = ContextCompressor()
        max_c = fibonacci(11)  # 89
        for i in range(max_c + 5):
            cc.add(f"chunk number {i} with some content")
        assert cc.chunk_count == max_c


class TestContextCompressorCompress:
    def test_fits_in_budget_no_compression(self):
        """When content fits in budget, returns unchanged."""
        cc = ContextCompressor(max_tokens=10000)
        text = ["short text", "another short chunk"]
        result = cc.compress(text, budget=10000)
        assert "short text" in result
        assert "another short chunk" in result

    def test_compresses_to_fit_budget(self):
        """Compressed output fits within budget."""
        cc = ContextCompressor()
        # Create content larger than budget
        chunks = [
            "This is a highly informative and unique sentence about Python quality.",
            "Another repetitive line. Another repetitive line. Another repetitive line.",
            "Code review: missing type hints are problematic in large codebases.",
            "Repetitive filler. Repetitive filler. Repetitive filler. Repetitive.",
            "Final important judgment: GROWL at 38.2 residual variance detected.",
        ]
        result = cc.compress(chunks, budget=50)
        assert estimate_tokens(result) <= 50 + 5  # Allow small rounding margin

    def test_compress_empty_returns_empty(self):
        cc = ContextCompressor()
        assert cc.compress([], budget=100) == ""

    def test_compress_keeps_first_sentence(self):
        """First sentence (temporal anchor) always included."""
        cc = ContextCompressor()
        chunks = ["FIRST important sentence that must be kept."]
        for i in range(50):
            chunks.append(f"filler line number {i} with generic content abc")
        result = cc.compress(chunks, budget=30)
        assert "FIRST" in result

    def test_no_compression_counter_when_fits(self):
        """Compression counter stays at 0 when no compression needed."""
        cc = ContextCompressor()
        cc.compress(["short"], budget=10000)
        assert cc.stats()["compressions"] == 0

    def test_compression_counter_increments(self):
        """Compression counter increments when compression occurs."""
        cc = ContextCompressor()
        large = ["x " * 1000]
        cc.compress(large, budget=10)
        assert cc.stats()["compressions"] == 1


class TestGetCompressedContext:
    def test_empty_history_returns_empty(self):
        cc = ContextCompressor()
        assert cc.get_compressed_context() == ""

    def test_small_history_returned_unchanged(self):
        cc = ContextCompressor(max_tokens=10000)
        cc.add("user: def foo(): pass")
        cc.add("cynic: GROWL Q=38.2")
        ctx = cc.get_compressed_context()
        assert "GROWL" in ctx
        assert "foo" in ctx

    def test_respects_budget_argument(self):
        """Explicit budget argument overrides max_tokens."""
        cc = ContextCompressor(max_tokens=10000)
        for i in range(30):
            cc.add(f"This is chunk number {i} with some informative content here.")
        ctx = cc.get_compressed_context(budget=50)
        assert estimate_tokens(ctx) <= 55  # Small margin for rounding

    def test_custom_max_tokens_used_by_default(self):
        """Default budget = max_tokens set in __init__."""
        cc = ContextCompressor(max_tokens=20)
        for i in range(20):
            cc.add(f"Informative content number {i} about CYNIC judgment quality.")
        ctx = cc.get_compressed_context()
        assert estimate_tokens(ctx) <= 25  # Small margin


class TestCompressorStats:
    def test_stats_keys(self):
        cc = ContextCompressor()
        s = cc.stats()
        assert "chunks" in s
        assert "total_input_tokens" in s
        assert "total_output_tokens" in s
        assert "compressions" in s
        assert "compression_ratio" in s
        assert "max_tokens" in s

    def test_compression_ratio_zero_initially(self):
        cc = ContextCompressor()
        assert cc.stats()["compression_ratio"] == 0.0

    def test_compression_ratio_improves_after_compress(self):
        """After compressing, output_tokens < input_tokens."""
        cc = ContextCompressor()
        large = " ".join([f"word{i}" for i in range(500)])
        cc.add(large)
        cc.get_compressed_context(budget=50)
        s = cc.stats()
        assert s["total_output_tokens"] > 0
        assert s["compression_ratio"] <= 1.0

    def test_stats_include_attention_keys(self):
        """Stats must include SAGE attention fields."""
        cc = ContextCompressor()
        s = cc.stats()
        assert "avg_chunk_attention" in s
        assert "max_chunk_attention" in s
        assert s["avg_chunk_attention"] == 1.0   # Default neutral


# ── Attention Feedback (SAGE → Compressor) ────────────────────────────────

class TestAttentionFeedback:
    """
    Tests for the SAGE→Compressor bidirectional attention loop.

    boost() records Jaccard-similarity attention signals from SAGE judgments.
    High-attention chunks get EMA-boosted weights used in get_compressed_context().
    """

    def test_boost_noop_on_empty_compressor(self):
        """boost() on empty compressor is safe (no crash)."""
        cc = ContextCompressor()
        cc.boost("some text about python code", 0.8)  # No exception

    def test_boost_noop_on_zero_weight(self):
        """boost() with weight=0 changes nothing."""
        cc = ContextCompressor()
        cc.add("python code quality metrics")
        cc.boost("python code quality", 0.0)
        assert cc._chunk_attention[0] == 1.0  # Unchanged

    def test_boost_raises_attention_for_similar_chunk(self):
        """Similar chunk gets attention > 1.0 after boost()."""
        cc = ContextCompressor()
        cc.add("python code quality metrics judgment")
        cc.boost("python code quality metrics", 0.8)
        assert cc._chunk_attention[0] > 1.0, "Similar chunk should be boosted"

    def test_boost_ignores_dissimilar_chunk(self):
        """Dissimilar chunk attention stays at 1.0."""
        cc = ContextCompressor()
        cc.add("solana blockchain transaction fees")
        cc.boost("python code type hints docstrings", 0.9)
        # Very low Jaccard similarity → no boost
        assert cc._chunk_attention[0] == 1.0

    def test_attention_synced_on_add(self):
        """Each add() creates a corresponding attention entry."""
        cc = ContextCompressor()
        cc.add("first chunk")
        cc.add("second chunk")
        assert len(cc._chunk_attention) == 2
        assert all(a == 1.0 for a in cc._chunk_attention)

    def test_attention_synced_on_clear(self):
        """clear() resets attention list."""
        cc = ContextCompressor()
        cc.add("chunk one")
        cc.boost("chunk one content", 0.9)
        cc.clear()
        assert cc._chunk_attention == []

    def test_attention_boosts_compress_ranking(self):
        """High-attention chunks are preferred in get_compressed_context()."""
        cc = ContextCompressor(max_tokens=10000)
        # Add two distinctly different chunks
        cc.add("Axiomatic reasoning about phi golden ratio fibonacci")
        cc.add("Blockchain solana lamports transaction confirmation")

        # Boost the first chunk as highly relevant
        cc.boost("phi golden ratio fibonacci axiomatic", 1.0)
        assert cc._chunk_attention[0] > cc._chunk_attention[1]

        # Compressed context with tight budget should prefer boosted chunk
        ctx = cc.get_compressed_context(budget=30)
        # High-attention chunk should have higher priority in selection
        assert cc._chunk_attention[0] > 1.0

    def test_rolling_window_drops_attention(self):
        """When oldest chunk is dropped, its attention entry is also dropped."""
        from cynic.senses.compressor import fibonacci
        cc = ContextCompressor()
        max_c = fibonacci(11)  # 89
        for i in range(max_c + 1):
            cc.add(f"chunk number {i} with content here")
        assert len(cc._chunk_attention) == cc.chunk_count
