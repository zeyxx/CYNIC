"""
CYNIC Deterministic Test Adapter " For reproducible pipeline tests.

Returns fixed responses based on prompt keywords.
Avoids real LLM latency and costs during CI.
"""

from __future__ import annotations

import time

from cynic.kernel.organism.brain.llm.adapter import LLMAdapter, LLMRequest, LLMResponse


class DeterministicLLMAdapter(LLMAdapter):
    """Mocks LLM behavior with predictable outputs for testing."""

    def __init__(self, model: str = "test-deterministic"):
        super().__init__(model=model, provider="test")
        self.responses = {
            "GOOD": "VERDICT: HOWL\nSCORE: 95\nREASONING: Optimal alignment detected.",
            "BAD": "VERDICT: BARK\nSCORE: 10\nREASONING: Critical failure in logic.",
            "NEUTRAL": "VERDICT: WAG\nSCORE: 70\nREASONING: Acceptable but imperfect.",
        }

    async def complete(self, request: LLMRequest) -> LLMResponse:
        start = time.time()

        # Simple keyword matching for deterministic behavior
        content = self.responses["NEUTRAL"]
        for key, resp in self.responses.items():
            if key in request.prompt.upper():
                content = resp
                break

        return LLMResponse(
            content=content,
            model=self.model,
            provider=self.provider,
            latency_ms=(time.time() - start) * 1000,
        )

    async def check_available(self) -> bool:
        return True
