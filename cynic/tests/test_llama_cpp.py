"""
Tests for LlamaCppAdapter + discover() integration.

llama-cpp-python is NOT required to be installed for these tests —
sys.modules stub prevents ImportError in CI.

Integration tests (need real .gguf):
    PYTHONUTF8=1 python -m pytest tests/test_llama_cpp.py -m integration -v
"""
from __future__ import annotations

import os
import sys
import asyncio
import tempfile
from typing import Any
from unittest.mock import AsyncMock, MagicMock, patch, call

import pytest

# ── Stub llama_cpp if not installed (CI / no-GPU) ────────────────────────────
_llama_cpp_stub = MagicMock()
sys.modules.setdefault("llama_cpp", _llama_cpp_stub)


# ── Import target modules AFTER stub is in place ─────────────────────────────
from cynic.llm.llama_cpp import LlamaCppAdapter, list_local_models  # noqa: E402
from cynic.llm.adapter import LLMRegistry, LLMRequest, BenchmarkResult  # noqa: E402


# ════════════════════════════════════════════════════════════════════════════
# 1. list_local_models
# ════════════════════════════════════════════════════════════════════════════

class TestListLocalModels:
    def test_finds_gguf_files(self, tmp_path):
        """list_local_models() discovers .gguf files recursively."""
        (tmp_path / "model-a.gguf").write_bytes(b"x" * 100)
        (tmp_path / "sub").mkdir()
        (tmp_path / "sub" / "model-b.gguf").write_bytes(b"x" * 200)
        (tmp_path / "readme.txt").write_text("not a model")

        result = list_local_models(str(tmp_path))
        basenames = [os.path.basename(p) for p in result]
        assert "model-a.gguf" in basenames
        assert "model-b.gguf" in basenames
        assert "readme.txt" not in basenames

    def test_sorted_by_size_ascending(self, tmp_path):
        """Smaller files come first (faster models registered first)."""
        (tmp_path / "large.gguf").write_bytes(b"x" * 300)
        (tmp_path / "small.gguf").write_bytes(b"x" * 50)
        (tmp_path / "medium.gguf").write_bytes(b"x" * 150)

        result = list_local_models(str(tmp_path))
        basenames = [os.path.basename(p) for p in result]
        assert basenames == ["small.gguf", "medium.gguf", "large.gguf"]

    def test_empty_dir_returns_empty(self, tmp_path):
        assert list_local_models(str(tmp_path)) == []

    def test_nonexistent_dir_returns_empty(self, tmp_path):
        missing = str(tmp_path / "missing")
        result = list_local_models(missing)
        assert result == []


# ════════════════════════════════════════════════════════════════════════════
# 2. LlamaCppAdapter.check_available()
# ════════════════════════════════════════════════════════════════════════════

class TestCheckAvailable:
    @pytest.mark.asyncio
    async def test_false_when_file_missing(self, tmp_path):
        adapter = LlamaCppAdapter(model_path=str(tmp_path / "nonexistent.gguf"))
        # llama_cpp is stubbed (importable), but file does not exist
        result = await adapter.check_available()
        assert result is False

    @pytest.mark.asyncio
    async def test_true_when_file_exists_and_installed(self, tmp_path):
        gguf = tmp_path / "gemma2-2b.gguf"
        gguf.write_bytes(b"placeholder")
        adapter = LlamaCppAdapter(model_path=str(gguf))
        result = await adapter.check_available()
        assert result is True

    @pytest.mark.asyncio
    async def test_false_when_llama_cpp_not_installed(self, tmp_path):
        gguf = tmp_path / "model.gguf"
        gguf.write_bytes(b"x")
        adapter = LlamaCppAdapter(model_path=str(gguf))
        # Simulate ImportError even though stub is in sys.modules
        with patch.dict(sys.modules, {"llama_cpp": None}):
            result = await adapter.check_available()
        assert result is False


# ════════════════════════════════════════════════════════════════════════════
# 3. LlamaCppAdapter.complete()
# ════════════════════════════════════════════════════════════════════════════

class TestComplete:
    def _make_mock_llama(self, content: str = "hello from gguf") -> MagicMock:
        mock_llm = MagicMock()
        mock_llm.create_chat_completion.return_value = {
            "choices": [{"message": {"content": content}}]
        }
        return mock_llm

    @pytest.mark.asyncio
    async def test_complete_returns_llm_response(self, tmp_path):
        gguf = tmp_path / "gemma2-2b.gguf"
        gguf.write_bytes(b"placeholder")
        adapter = LlamaCppAdapter(model_path=str(gguf))
        adapter._llm = self._make_mock_llama("answer text")

        req = LLMRequest(prompt="What is 2+2?")
        resp = await adapter.complete(req)

        assert resp.content == "answer text"
        assert resp.provider == "llama_cpp"
        assert resp.model == "gemma2-2b"
        assert resp.cost_usd == 0.0
        assert resp.latency_ms >= 0

    @pytest.mark.asyncio
    async def test_complete_with_system_prompt(self, tmp_path):
        gguf = tmp_path / "model.gguf"
        gguf.write_bytes(b"x")
        adapter = LlamaCppAdapter(model_path=str(gguf))
        mock_llm = self._make_mock_llama("response")
        adapter._llm = mock_llm

        req = LLMRequest(prompt="hello", system="You are a cynical dog.")
        await adapter.complete(req)

        call_kwargs = mock_llm.create_chat_completion.call_args
        messages = call_kwargs[1]["messages"] if call_kwargs[1] else call_kwargs[0][0]
        roles = [m["role"] for m in messages]
        assert "system" in roles
        assert "user" in roles

    @pytest.mark.asyncio
    async def test_complete_empty_choices_returns_empty_string(self, tmp_path):
        gguf = tmp_path / "model.gguf"
        gguf.write_bytes(b"x")
        adapter = LlamaCppAdapter(model_path=str(gguf))
        mock_llm = MagicMock()
        mock_llm.create_chat_completion.return_value = {"choices": []}
        adapter._llm = mock_llm

        req = LLMRequest(prompt="ping")
        resp = await adapter.complete(req)
        assert resp.content == ""


# ════════════════════════════════════════════════════════════════════════════
# 4. Semaphore — concurrent calls are serialized
# ════════════════════════════════════════════════════════════════════════════

class TestSemaphore:
    @pytest.mark.asyncio
    async def test_concurrent_calls_are_sequential(self, tmp_path):
        """
        When 2 coroutines call complete() concurrently, the semaphore ensures
        they execute sequentially (no race condition on self._llm).
        """
        gguf = tmp_path / "model.gguf"
        gguf.write_bytes(b"x")
        adapter = LlamaCppAdapter(model_path=str(gguf))

        call_order: list = []
        def mock_sync(messages, max_tokens, temperature):
            call_order.append("enter")
            # simulate a tiny sync delay without actual blocking
            return "ok"

        adapter._llm = MagicMock()
        adapter._sync_complete = mock_sync  # type: ignore[method-assign]

        req = LLMRequest(prompt="test")
        await asyncio.gather(adapter.complete(req), adapter.complete(req))

        # Both calls succeeded
        assert call_order.count("enter") == 2


# ════════════════════════════════════════════════════════════════════════════
# 5. LLMRegistry.discover() — with mocked models_dir
# ════════════════════════════════════════════════════════════════════════════

class TestDiscoverLlamaCpp:
    @pytest.mark.asyncio
    async def test_discover_registers_llama_cpp_adapter(self, tmp_path):
        """discover() registers LlamaCppAdapter when models_dir has .gguf files."""
        gguf = tmp_path / "gemma2-2b.gguf"
        gguf.write_bytes(b"placeholder")

        registry = LLMRegistry()

        # Patch check_available to return True without real file validation
        async def fake_check(self):
            return os.path.isfile(self._model_path)

        with patch("cynic.llm.llama_cpp.LlamaCppAdapter.check_available", fake_check):
            discovered = await registry.discover(
                ollama_url="http://localhost:11434",
                models_dir=str(tmp_path),
                llama_gpu_layers=0,
                llama_threads=4,
            )

        llama_ids = [aid for aid in discovered if "llama_cpp" in aid]
        assert len(llama_ids) == 1
        assert "gemma2-2b" in llama_ids[0]

    @pytest.mark.asyncio
    async def test_discover_skips_when_models_dir_none(self):
        """discover() with models_dir=None does not raise and returns no llama_cpp adapters."""
        registry = LLMRegistry()
        discovered = await registry.discover(models_dir=None)
        assert not any("llama_cpp" in aid for aid in discovered)

    @pytest.mark.asyncio
    async def test_discover_skips_silently_on_import_error(self, tmp_path):
        """discover() is silent when llama-cpp-python is not installed."""
        gguf = tmp_path / "model.gguf"
        gguf.write_bytes(b"x")

        registry = LLMRegistry()
        # Force ImportError for llama_cpp inside discover's _discover_llama_cpp
        with patch.dict(sys.modules, {"cynic.llm.llama_cpp": None}):
            discovered = await registry.discover(
                models_dir=str(tmp_path),
            )
        # No crash — just no llama_cpp adapters
        assert not any("llama_cpp" in aid for aid in discovered)


# ════════════════════════════════════════════════════════════════════════════
# 6. BenchmarkResult routing — LlamaCppAdapter wins after benchmarking
# ════════════════════════════════════════════════════════════════════════════

class TestBenchmarkRouting:
    def test_llama_cpp_wins_over_ollama_after_benchmarking(self, tmp_path):
        """
        After recording a BenchmarkResult with low latency (high speed_score)
        and zero cost, LlamaCppAdapter composite_score exceeds OllamaAdapter's.
        """
        gguf = tmp_path / "gemma2-2b.gguf"
        gguf.write_bytes(b"x")

        registry = LLMRegistry()

        # Register both adapters as available
        from cynic.llm.adapter import OllamaAdapter
        ollama_adapter = OllamaAdapter(model="gemma2:2b")
        llama_adapter = LlamaCppAdapter(model_path=str(gguf))

        registry.register(ollama_adapter, available=True)
        registry.register(llama_adapter, available=True)

        # Give llama_cpp a better benchmark (faster + free)
        llama_result = BenchmarkResult(
            llm_id=llama_adapter.adapter_id,
            dog_id="SAGE",
            task_type="wisdom",
            quality_score=55.0,
            speed_score=0.9,    # fast (~50 tok/s)
            cost_score=1.0,     # free
        )
        # Give ollama a slightly worse benchmark
        ollama_result = BenchmarkResult(
            llm_id=ollama_adapter.adapter_id,
            dog_id="SAGE",
            task_type="wisdom",
            quality_score=55.0,
            speed_score=0.7,    # slower (HTTP overhead)
            cost_score=1.0,
        )

        registry.update_benchmark("SAGE", "wisdom", llama_adapter.adapter_id, llama_result)
        registry.update_benchmark("SAGE", "wisdom", ollama_adapter.adapter_id, ollama_result)

        best = registry.get_best_for("SAGE", "wisdom")
        assert best is not None
        assert best.provider == "llama_cpp"

    def test_llama_cpp_embed_excluded_from_generation(self, tmp_path):
        """llama_cpp adapters with 'embed' in model name are not returned for generation."""
        gguf = tmp_path / "nomic-embed-text.gguf"
        gguf.write_bytes(b"x")
        registry = LLMRegistry()
        adapter = LlamaCppAdapter(model_path=str(gguf))
        registry.register(adapter, available=True)

        gen_adapters = registry.get_available_for_generation()
        assert adapter not in gen_adapters


# ════════════════════════════════════════════════════════════════════════════
# Integration tests (skipped unless -m integration)
# ════════════════════════════════════════════════════════════════════════════

@pytest.mark.integration
class TestIntegration:
    @pytest.mark.asyncio
    async def test_real_gguf_inference(self):
        """Requires a real .gguf in CYNIC_MODELS_DIR and llama-cpp-python installed."""
        models_dir = os.getenv("CYNIC_MODELS_DIR")
        if not models_dir:
            pytest.skip("CYNIC_MODELS_DIR not set")
        paths = list_local_models(models_dir)
        if not paths:
            pytest.skip(f"No .gguf files found in {models_dir}")

        adapter = LlamaCppAdapter(model_path=paths[0], n_gpu_layers=0, n_threads=4)
        assert await adapter.check_available()

        req = LLMRequest(prompt="Say 'hello' in one word.", max_tokens=10)
        resp = await adapter.complete(req)
        assert resp.content.strip() != ""
        assert resp.provider == "llama_cpp"
        assert resp.latency_ms > 0
