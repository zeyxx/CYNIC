"""
Tests for cynic.chat.session — ChatMessage and ChatSession.
"""
from __future__ import annotations

import json
import os
import tempfile
import pytest

from cynic.chat.session import ChatMessage, ChatSession, _MAX_MESSAGES


# ════════════════════════════════════════════════════════════════════════════
# CHAT MESSAGE TESTS
# ════════════════════════════════════════════════════════════════════════════

class TestChatMessage:
    """Test ChatMessage dataclass."""

    def test_basic_creation(self):
        msg = ChatMessage(role="user", content="hello")
        assert msg.role == "user"
        assert msg.content == "hello"
        assert msg.timestamp > 0

    def test_to_ollama_simple(self):
        msg = ChatMessage(role="user", content="hello")
        d = msg.to_ollama()
        assert d == {"role": "user", "content": "hello"}

    def test_to_ollama_with_tool_calls(self):
        msg = ChatMessage(
            role="assistant",
            content="",
            tool_calls=[{"name": "bash", "arguments": {"command": "ls"}}],
        )
        d = msg.to_ollama()
        assert d["role"] == "assistant"
        assert len(d["tool_calls"]) == 1
        assert d["tool_calls"][0]["function"]["name"] == "bash"

    def test_to_ollama_tool_result(self):
        msg = ChatMessage(role="tool", content="file list...", name="bash")
        d = msg.to_ollama()
        assert d["role"] == "tool"
        assert d["name"] == "bash"

    def test_roundtrip_dict(self):
        msg = ChatMessage(
            role="assistant",
            content="hello",
            tool_calls=[{"name": "read", "arguments": {"path": "x.py"}}],
            metadata={"key": "val"},
        )
        d = msg.to_dict()
        restored = ChatMessage.from_dict(d)
        assert restored.role == msg.role
        assert restored.content == msg.content
        assert restored.tool_calls == msg.tool_calls
        assert restored.metadata == msg.metadata

    def test_from_dict_defaults(self):
        msg = ChatMessage.from_dict({})
        assert msg.role == "user"
        assert msg.content == ""


# ════════════════════════════════════════════════════════════════════════════
# CHAT SESSION TESTS
# ════════════════════════════════════════════════════════════════════════════

class TestChatSession:
    """Test ChatSession management and persistence."""

    def test_create_session(self):
        session = ChatSession(system_prompt="test")
        assert session.session_id
        assert session.system_prompt == "test"
        assert session.message_count == 0

    def test_add_messages(self):
        session = ChatSession()
        session.add_user("hello")
        session.add_assistant("hi there")
        session.add_tool_result("bash", "output")
        assert session.message_count == 3

    def test_last_user_message(self):
        session = ChatSession()
        session.add_user("first")
        session.add_assistant("reply")
        session.add_user("second")
        assert session.last_user_message == "second"

    def test_last_user_message_empty(self):
        session = ChatSession()
        assert session.last_user_message == ""

    def test_rolling_cap(self):
        session = ChatSession()
        for i in range(_MAX_MESSAGES + 20):
            session.add_user(f"msg {i}")
        assert session.message_count == _MAX_MESSAGES

    def test_to_ollama_messages(self):
        session = ChatSession(system_prompt="be helpful")
        session.add_user("hello")
        session.add_assistant("hi")

        msgs = session.to_ollama_messages()
        assert msgs[0] == {"role": "system", "content": "be helpful"}
        assert msgs[1] == {"role": "user", "content": "hello"}
        assert msgs[2] == {"role": "assistant", "content": "hi"}

    def test_to_ollama_no_system(self):
        session = ChatSession()
        session.add_user("hello")
        msgs = session.to_ollama_messages()
        assert len(msgs) == 1
        assert msgs[0]["role"] == "user"

    def test_save_and_load(self, tmp_path, monkeypatch):
        """Test session persistence roundtrip."""
        # Override the chats directory
        monkeypatch.setattr("cynic.chat.session._CHATS_DIR", str(tmp_path))

        session = ChatSession(session_id="test123", system_prompt="system")
        session.model = "qwen2.5-coder:7b"
        session.total_tokens = 500
        session.add_user("hello")
        session.add_assistant("world")

        path = session.save()
        assert os.path.exists(path)

        # Load it back
        loaded = ChatSession.load("test123")
        assert loaded.session_id == "test123"
        assert loaded.system_prompt == "system"
        assert loaded.model == "qwen2.5-coder:7b"
        assert loaded.total_tokens == 500
        assert loaded.message_count == 2
        assert loaded.messages[0].content == "hello"
        assert loaded.messages[1].content == "world"

    def test_list_sessions(self, tmp_path, monkeypatch):
        monkeypatch.setattr("cynic.chat.session._CHATS_DIR", str(tmp_path))

        # Create a couple sessions
        s1 = ChatSession(session_id="aaa", system_prompt="")
        s1.model = "model1"
        s1.add_user("hi")
        s1.save()

        s2 = ChatSession(session_id="bbb", system_prompt="")
        s2.model = "model2"
        s2.add_user("hello")
        s2.add_assistant("world")
        s2.save()

        sessions = ChatSession.list_sessions()
        assert len(sessions) == 2
        # Should be sorted by mtime (newest first)
        ids = [s["session_id"] for s in sessions]
        assert "aaa" in ids
        assert "bbb" in ids

    def test_list_sessions_empty(self, tmp_path, monkeypatch):
        monkeypatch.setattr("cynic.chat.session._CHATS_DIR", str(tmp_path / "nonexistent"))
        sessions = ChatSession.list_sessions()
        assert sessions == []

    def test_tool_calls_in_session(self):
        session = ChatSession()
        session.add_user("read foo.py")
        session.add_assistant("", tool_calls=[{"name": "read", "arguments": {"path": "foo.py"}}])
        session.add_tool_result("read", "file contents here")
        session.add_assistant("The file contains...")

        msgs = session.to_ollama_messages()
        assert len(msgs) == 4
        assert msgs[1]["tool_calls"][0]["function"]["name"] == "read"
        assert msgs[2]["role"] == "tool"
        assert msgs[2]["name"] == "read"
