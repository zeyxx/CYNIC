"""
DeterministicLLMAdapter — Test harness for Track C empirical learning.

Returns hardcoded responses — no Ollama required. Dogs parse the response
to extract verdict and q_score. Used for synthetic testing without external
LLM dependency.

Dogs expect response.content to be JSON with:
  {
    "verdict": "BARK" | "GROWL" | "WAG" | "HOWL",
    "q_score": <float>,
    "reasoning": "<str>"
  }
"""
from __future__ import annotations

import json
import time
from typing import Any

from cynic.llm.adapter import LLMRequest, LLMResponse
from cynic.llm.adapters.interface import LLMAdapterInterface


class DeterministicLLMAdapter(LLMAdapterInterface):
    """
    Test adapter that returns deterministic responses.

    Used by Track C to:
      - Inject synthetic high/low quality signals
      - Run convergence tests without real LLM overhead
      - Verify Dogs parse responses correctly

    Response format is JSON that Dogs can parse:
      {"verdict": "WAG", "q_score": 75.0, "reasoning": "..."}
    """

    def __init__(self, verdict: str = "WAG", q_hint: float = 55.0):
        """
        Initialize deterministic adapter.

        Args:
            verdict: One of "BARK", "GROWL", "WAG", "HOWL"
            q_hint: Q-score hint [0, 61.8] (will be clamped)
        """
        valid_verdicts = ["BARK", "GROWL", "WAG", "HOWL"]
        if verdict not in valid_verdicts:
            raise ValueError(f"verdict must be one of {valid_verdicts}, got {verdict}")

        # Clamp q_hint to φ-bounded range [0, 61.8]
        self.verdict = verdict
        self.q_hint = max(0.0, min(61.8, float(q_hint)))

    async def complete(self, request: LLMRequest) -> LLMResponse:
        """
        Return deterministic response.

        Always succeeds instantly. Dogs parse the JSON content to extract
        verdict and q_score.
        """
        # Construct response that Dogs expect
        response_data = {
            "verdict": self.verdict,
            "q_score": self.q_hint,
            "reasoning": f"Deterministic test response: {self.verdict} (q={self.q_hint:.1f})",
        }
        content = json.dumps(response_data)

        return LLMResponse(
            content=content,
            model=self.model,
            provider=self.provider,
            prompt_tokens=len(request.prompt.split()),
            completion_tokens=len(content.split()),
            cost_usd=0.0,  # Test adapter — no cost
            latency_ms=1.0,  # Near-instant
            error=None,
        )

    async def check_available(self) -> bool:
        """Test adapter always available."""
        return True

    @property
    def adapter_id(self) -> str:
        """Unique identifier for this test instance."""
        return f"test:deterministic:{self.verdict}:{self.q_hint:.1f}"

    @property
    def model(self) -> str:
        """Test model name."""
        return f"test-deterministic-{self.verdict}"

    @property
    def provider(self) -> str:
        """Test provider."""
        return "test"
