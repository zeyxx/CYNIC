"""
Integration tests: SDK bridge with real Claude API.

These tests require:
- ANTHROPIC_API_KEY environment variable set
- Real Claude API calls (no mocks)

Skip gracefully if API key not configured.
Run locally: pytest -m integration tests/test_integration/test_sdk_real.py
"""
import os
import pytest

from cynic.metabolism.runner import ClaudeCodeRunner
from cynic.llm.adapter import LLMRegistry, ClaudeAdapter


@pytest.mark.integration
class TestSDKBridgeRealClaude:
    """Real Claude API calls via SDK bridge."""

    @pytest.mark.asyncio
    async def test_real_claude_api_available(self):
        """
        Check if Claude API is available.

        Skip if ANTHROPIC_API_KEY not set.
        """
        api_key = os.getenv("ANTHROPIC_API_KEY")

        if not api_key:
            pytest.skip("ANTHROPIC_API_KEY not configured")

        # Verify adapter can connect
        registry = LLMRegistry()
        adapter = registry.get_best_for("scoring")

        assert adapter is not None
        assert isinstance(adapter, ClaudeAdapter)

    @pytest.mark.asyncio
    async def test_real_claude_completion(self):
        """
        Real completion call to Claude.

        Skip if API key not available.
        """
        api_key = os.getenv("ANTHROPIC_API_KEY")

        if not api_key:
            pytest.skip("ANTHROPIC_API_KEY not configured")

        registry = LLMRegistry()
        adapter = registry.get_best_for("scoring")

        if not isinstance(adapter, ClaudeAdapter):
            pytest.skip("Claude API not configured")

        # Make real API call
        messages = [
            {
                "role": "user",
                "content": "What is the golden ratio φ (phi)? Answer in one sentence.",
            }
        ]

        from cynic.llm.adapter import LLMRequest

        request = LLMRequest(
            model="claude-haiku-4-5-20251001",
            messages=messages,
            max_tokens=100,
            temperature=0.0,
        )

        try:
            response = await adapter.complete(request)

            # Verify real response (not a mock)
            assert response.raw_message is not None
            assert len(response.raw_message) > 0

            # Verify response mentions golden ratio or 0.618
            assert (
                "golden" in response.raw_message.lower()
                or "0.618" in response.raw_message
            )

        except ValidationError as e:
            pytest.skip(f"Claude API call failed: {e}")

    def test_sdk_imports_work(self):
        """
        Verify SDK modules can be imported.

        Smoke test for import integrity.
        """
        from cynic.metabolism.runner import ClaudeCodeRunner
        from cynic.metabolism.llm_router import LLMRouter

        assert ClaudeCodeRunner is not None
        assert LLMRouter is not None
