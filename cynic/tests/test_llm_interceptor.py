"""
CYNIC LLMCallInterceptor Tests (T20)

Verifies that complete_safe() logs every LLM call to ~/.cynic/llm_calls.jsonl
with a rolling cap of F(13)=233.
No real LLM calls — pure mock adapter.
"""
from __future__ import annotations

import asyncio
import json
import os
import pytest

import cynic.llm.adapter as _adapter_mod
from cynic.llm.adapter import (
    LLMAdapter,
    LLMRequest,
    LLMResponse,
    _LLM_LOG_CAP,
)


# ── helpers ─────────────────────────────────────────────────────────────────

class _MockAdapter(LLMAdapter):
    """Minimal adapter returning a fixed LLMResponse."""

    def __init__(self, response: LLMResponse) -> None:
        super().__init__(model=response.model, provider=response.provider)
        self._resp = response

    async def complete(self, request: LLMRequest) -> LLMResponse:
        return self._resp

    async def check_available(self) -> bool:
        return True


def _make_adapter(
    model: str = "gemma2:2b",
    provider: str = "ollama",
    latency_ms: float = 200.0,
    cost_usd: float = 0.0,
    error: str | None = None,
) -> _MockAdapter:
    resp = LLMResponse(
        content="" if error else "result",
        model=model,
        provider=provider,
        prompt_tokens=10,
        completion_tokens=5,
        cost_usd=cost_usd,
        latency_ms=latency_ms,
        error=error,
    )
    return _MockAdapter(resp)


def _run(coro):
    return asyncio.get_event_loop().run_until_complete(coro)


# ── tests ────────────────────────────────────────────────────────────────────

class TestLLMInterceptor:

    def test_record_written_after_call(self, tmp_path, monkeypatch):
        """After complete_safe(), log file exists with exactly one line."""
        log_path = str(tmp_path / "llm_calls.jsonl")
        monkeypatch.setattr(_adapter_mod, "_LLM_LOG_PATH", log_path)

        adapter = _make_adapter()
        _run(adapter.complete_safe(LLMRequest(prompt="hello")))

        assert os.path.exists(log_path)
        with open(log_path, encoding="utf-8") as fh:
            lines = fh.readlines()
        assert len(lines) == 1

    def test_record_has_required_fields(self, tmp_path, monkeypatch):
        """Log record contains all expected fields."""
        log_path = str(tmp_path / "llm_calls.jsonl")
        monkeypatch.setattr(_adapter_mod, "_LLM_LOG_PATH", log_path)

        _run(_make_adapter(model="haiku", provider="claude", cost_usd=0.001)
             .complete_safe(LLMRequest(prompt="review this code")))

        with open(log_path, encoding="utf-8") as fh:
            record = json.loads(fh.readline())

        for key in ("ts", "provider", "model", "latency_ms", "prompt_tokens",
                    "completion_tokens", "cost_usd", "success", "error", "prompt_preview"):
            assert key in record, f"missing field: {key}"

        assert record["model"] == "haiku"
        assert record["provider"] == "claude"
        assert record["cost_usd"] == pytest.approx(0.001)
        assert record["success"] is True

    def test_rolling_cap_enforced(self, tmp_path, monkeypatch):
        """After 250 calls the log file holds exactly F(13)=233 lines."""
        log_path = str(tmp_path / "llm_calls.jsonl")
        monkeypatch.setattr(_adapter_mod, "_LLM_LOG_PATH", log_path)

        adapter = _make_adapter()
        for _ in range(250):
            _run(adapter.complete_safe(LLMRequest(prompt="x")))

        with open(log_path, encoding="utf-8") as fh:
            lines = fh.readlines()
        assert len(lines) == _LLM_LOG_CAP  # 233

    def test_error_response_logged_as_failure(self, tmp_path, monkeypatch):
        """An LLM error still gets logged with success=False."""
        log_path = str(tmp_path / "llm_calls.jsonl")
        monkeypatch.setattr(_adapter_mod, "_LLM_LOG_PATH", log_path)

        _run(_make_adapter(error="model not found")
             .complete_safe(LLMRequest(prompt="anything")))

        with open(log_path, encoding="utf-8") as fh:
            record = json.loads(fh.readline())

        assert record["success"] is False
        assert record["error"] == "model not found"

    def test_prompt_preview_truncated_at_80(self, tmp_path, monkeypatch):
        """prompt_preview is at most 80 characters even for long prompts."""
        log_path = str(tmp_path / "llm_calls.jsonl")
        monkeypatch.setattr(_adapter_mod, "_LLM_LOG_PATH", log_path)

        long_prompt = "A" * 300
        _run(_make_adapter().complete_safe(LLMRequest(prompt=long_prompt)))

        with open(log_path, encoding="utf-8") as fh:
            record = json.loads(fh.readline())

        assert len(record["prompt_preview"]) <= 80

    def test_log_error_does_not_crash_caller(self, monkeypatch):
        """Even if the log path is unwritable the caller still gets a response."""
        # Point to an obviously unwritable path
        monkeypatch.setattr(
            _adapter_mod,
            "_LLM_LOG_PATH",
            "/nonexistent_root_dir/subdir/llm_calls.jsonl",
        )

        adapter = _make_adapter()
        response = _run(adapter.complete_safe(LLMRequest(prompt="safe?")))

        assert response is not None
        assert response.error is None
        assert response.content == "result"
