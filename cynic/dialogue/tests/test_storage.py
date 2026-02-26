"""Tests for dialogue storage backend."""

import pytest
import tempfile
import asyncio
import time
from pathlib import Path
from cynic.dialogue.storage import DialogueStore
from cynic.dialogue.models import UserMessage, CynicMessage


@pytest.fixture
async def dialogue_store():
    """Create temporary dialogue store for testing."""
    tmpdir = tempfile.mkdtemp()
    try:
        store = DialogueStore(Path(tmpdir) / "test.db")
        await store.initialize()
        yield store
        await store.close()
        # Give sqlite time to release the lock
        time.sleep(0.1)
    finally:
        import shutil
        shutil.rmtree(tmpdir, ignore_errors=True)


@pytest.mark.asyncio
async def test_store_initialization(dialogue_store):
    """Store initializes SQLite table."""
    assert dialogue_store.db_path.exists()


@pytest.mark.asyncio
async def test_save_user_message(dialogue_store):
    """Save and retrieve user message."""
    msg = UserMessage("question", "Why WAG?", 0.5, "j123")
    msg_id = await dialogue_store.save_message(msg)
    assert msg_id is not None


@pytest.mark.asyncio
async def test_save_cynic_message(dialogue_store):
    """Save and retrieve CYNIC message."""
    msg = CynicMessage("reasoning", "Because...", 0.5, {"PHI": 0.8}, "j123")
    msg_id = await dialogue_store.save_message(msg)
    assert msg_id is not None


@pytest.mark.asyncio
async def test_get_last_n_messages(dialogue_store):
    """Retrieve last N messages from history."""
    # Add messages
    for i in range(5):
        msg = UserMessage("question", f"Question {i}", 0.5, None)
        await dialogue_store.save_message(msg)

    # Get last 3
    messages = await dialogue_store.get_last_n_messages(3)
    assert len(messages) == 3


@pytest.mark.asyncio
async def test_get_conversation_context(dialogue_store):
    """Get conversation context around judgment."""
    # Add some messages
    msg1 = UserMessage("question", "What do you think?", 0.5, "j123")
    msg2 = CynicMessage("reasoning", "I think...", 0.5, {}, "j123")

    id1 = await dialogue_store.save_message(msg1)
    id2 = await dialogue_store.save_message(msg2)

    context = await dialogue_store.get_conversation_context("j123")
    assert len(context) >= 2
