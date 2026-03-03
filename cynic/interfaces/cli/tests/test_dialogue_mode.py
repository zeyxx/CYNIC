"""Tests for DialogueMode interactive dialogue."""

from unittest.mock import AsyncMock, MagicMock

import pytest

from cynic.interfaces.cli.dialogue_mode import DialogueMode


@pytest.mark.asyncio
async def test_dialogue_mode_initialization():
    """DialogueMode initializes with storage and LLM."""
    dialogue_mode = DialogueMode()
    assert dialogue_mode is not None
    await dialogue_mode.close()


@pytest.mark.asyncio
async def test_dialogue_mode_process_user_message():
    """Process user message and generate response."""
    dialogue_mode = DialogueMode()

    # Mock storage and LLM
    dialogue_mode.storage = AsyncMock()
    dialogue_mode.storage.save_message = AsyncMock(return_value=1)
    dialogue_mode.llm_bridge.generate_response = AsyncMock(
        return_value="I chose WAG because..."
    )
    dialogue_mode.memory_store = AsyncMock()
    dialogue_mode.memory_store.load_memory = AsyncMock(
        return_value=MagicMock(user_style="balanced", communication_style={})
    )

    response = await dialogue_mode.process_message("Why did you choose WAG?")

    assert isinstance(response, str)
    assert len(response) > 0
    await dialogue_mode.close()


@pytest.mark.asyncio
async def test_dialogue_mode_context_for_explanation():
    """Build context for explaining judgment."""
    dialogue_mode = DialogueMode()

    judgment = {
        "verdict": "WAG",
        "q_score": 78,
        "confidence": 0.5,
        "axiom_scores": {"PHI": 0.8, "BURN": 0.6},
    }

    context = dialogue_mode._prepare_context_for_llm("Why WAG?", judgment)

    assert "question" in context
    assert "verdict" in context
    assert context["verdict"] == "WAG"


@pytest.mark.asyncio
async def test_dialogue_greeting():
    """Generate greeting message."""
    dialogue_mode = DialogueMode()

    greeting = await dialogue_mode.get_greeting()

    assert isinstance(greeting, str)
    assert "discuss" in greeting.lower() or "talk" in greeting.lower()
    await dialogue_mode.close()
