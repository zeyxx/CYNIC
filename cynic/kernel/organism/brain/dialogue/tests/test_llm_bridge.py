from unittest.mock import AsyncMock, patch

import pytest

from cynic.kernel.organism.brain.dialogue.llm_bridge import LLMBridge


@pytest.mark.asyncio
async def test_llm_bridge_initialization():
    """LLMBridge initializes with API key."""
    bridge = LLMBridge(api_key="test-key")
    assert bridge.api_key == "test-key"
    await bridge.close()


@pytest.mark.asyncio
async def test_generate_response_from_reasoning():
    """Generate natural language response from reasoning."""
    context = {
        "question": "Why did you choose WAG?",
        "verdict": "WAG",
        "confidence": 0.5,
        "reasoning_summary": "I chose WAG because...",
        "communication_style": "concise",
        "verbosity": "1-2 sentences max",
    }

    with patch(
        "cynic.kernel.organism.brain.dialogue.llm_bridge.AsyncAnthropic"
    ) as mock_client_class:
        mock_response = AsyncMock()
        mock_response.content = [AsyncMock()]
        mock_response.content[
            0
        ].text = "I chose WAG because it balances fairness with efficiency."

        mock_client_instance = AsyncMock()
        mock_client_instance.messages.create = AsyncMock(return_value=mock_response)
        mock_client_instance.close = AsyncMock()
        mock_client_class.return_value = mock_client_instance

        bridge = LLMBridge(api_key="test-key")
        response = await bridge.generate_response(context)

        assert "WAG" in response or "fairness" in response.lower()
        assert isinstance(response, str)

        await bridge.close()


@pytest.mark.asyncio
async def test_generate_explanation_prompt():
    """Create proper prompt for Claude."""
    bridge = LLMBridge(api_key="test-key")

    context = {
        "question": "Why WAG?",
        "verdict": "WAG",
        "axiom_scores": {"PHI": 0.85, "BURN": 0.65},
        "communication_style": "concise",
    }

    prompt = bridge._create_explanation_prompt(context)
    assert "Why WAG" in prompt
    assert "PHI" in prompt
    assert "concise" in prompt

    await bridge.close()


@pytest.mark.asyncio
async def test_handle_api_error():
    """Handle API errors gracefully."""
    context = {"question": "Why WAG?", "verdict": "WAG"}

    with patch(
        "cynic.kernel.organism.brain.dialogue.llm_bridge.AsyncAnthropic"
    ) as mock_client_class:
        mock_client_instance = AsyncMock()
        mock_client_instance.messages.create = AsyncMock(
            side_effect=Exception("API Error")
        )
        mock_client_instance.close = AsyncMock()
        mock_client_class.return_value = mock_client_instance

        bridge = LLMBridge(api_key="bad-key")
        response = await bridge.generate_response(context)
        assert "unable" in response.lower() or "error" in response.lower()

        await bridge.close()
