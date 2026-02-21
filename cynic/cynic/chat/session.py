"""
CYNIC Chat Session — Multi-turn conversation with persistence.

Sessions are stored as JSON in ~/.cynic/chats/{session_id}.json.
Rolling cap: F(11)=89 messages — older messages compressed via ContextCompressor.
"""
from __future__ import annotations

import json
import os
import time
import uuid
from dataclasses import dataclass, field
from typing import Any


# ════════════════════════════════════════════════════════════════════════════
# CONSTANTS
# ════════════════════════════════════════════════════════════════════════════

_CHATS_DIR = os.path.join(os.path.expanduser("~"), ".cynic", "chats")
_MAX_MESSAGES = 89  # F(11) — rolling cap


# ════════════════════════════════════════════════════════════════════════════
# CHAT MESSAGE
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class ChatMessage:
    """One message in a chat session."""
    role: str                           # user / assistant / system / tool
    content: str = ""
    tool_calls: list[dict] | None = None  # From assistant
    tool_call_id: str = ""              # For tool result messages
    name: str = ""                      # Tool name (for role=tool)
    timestamp: float = field(default_factory=time.time)
    metadata: dict[str, Any] = field(default_factory=dict)

    def to_ollama(self) -> dict:
        """Convert to Ollama chat message format."""
        msg: dict[str, Any] = {"role": self.role, "content": self.content}
        if self.tool_calls:
            msg["tool_calls"] = [
                {"function": {"name": tc["name"], "arguments": tc["arguments"]}}
                for tc in self.tool_calls
            ]
        if self.name:
            msg["name"] = self.name
        return msg

    def to_dict(self) -> dict:
        """Serialize for JSON persistence."""
        d: dict[str, Any] = {
            "role": self.role,
            "content": self.content,
            "timestamp": self.timestamp,
        }
        if self.tool_calls:
            d["tool_calls"] = self.tool_calls
        if self.tool_call_id:
            d["tool_call_id"] = self.tool_call_id
        if self.name:
            d["name"] = self.name
        if self.metadata:
            d["metadata"] = self.metadata
        return d

    @classmethod
    def from_dict(cls, d: dict) -> ChatMessage:
        return cls(
            role=d.get("role", "user"),
            content=d.get("content", ""),
            tool_calls=d.get("tool_calls"),
            tool_call_id=d.get("tool_call_id", ""),
            name=d.get("name", ""),
            timestamp=d.get("timestamp", 0.0),
            metadata=d.get("metadata", {}),
        )


# ════════════════════════════════════════════════════════════════════════════
# CHAT SESSION
# ════════════════════════════════════════════════════════════════════════════

class ChatSession:
    """
    Multi-turn chat session with persistence.

    Messages roll at F(11)=89 — oldest messages are dropped (future:
    compressed via ContextCompressor before drop).
    """

    def __init__(
        self,
        session_id: str | None = None,
        system_prompt: str = "",
    ) -> None:
        self.session_id = session_id or uuid.uuid4().hex[:12]
        self.system_prompt = system_prompt
        self.messages: list[ChatMessage] = []
        self.created_at = time.time()
        self.model: str = ""
        self.total_tokens: int = 0
        self.total_cost_usd: float = 0.0

    def add_user(self, content: str) -> ChatMessage:
        """Add a user message."""
        msg = ChatMessage(role="user", content=content)
        self._append(msg)
        return msg

    def add_assistant(self, content: str, tool_calls: list[dict] | None = None) -> ChatMessage:
        """Add an assistant message (text and/or tool calls)."""
        msg = ChatMessage(role="assistant", content=content, tool_calls=tool_calls)
        self._append(msg)
        return msg

    def add_tool_result(self, name: str, content: str, call_id: str = "") -> ChatMessage:
        """Add a tool result message."""
        msg = ChatMessage(role="tool", content=content, name=name, tool_call_id=call_id)
        self._append(msg)
        return msg

    def _append(self, msg: ChatMessage) -> None:
        """Append message with rolling cap."""
        self.messages.append(msg)
        if len(self.messages) > _MAX_MESSAGES:
            # Keep system context + trim oldest non-system messages
            self.messages = self.messages[-_MAX_MESSAGES:]

    def to_ollama_messages(self) -> list[dict]:
        """Convert full session to Ollama message format."""
        msgs: list[dict] = []
        if self.system_prompt:
            msgs.append({"role": "system", "content": self.system_prompt})
        for m in self.messages:
            msgs.append(m.to_ollama())
        return msgs

    @property
    def message_count(self) -> int:
        return len(self.messages)

    @property
    def last_user_message(self) -> str:
        for m in reversed(self.messages):
            if m.role == "user":
                return m.content
        return ""

    # ── Persistence ───────────────────────────────────────────────────────

    def save(self) -> str:
        """Save session to ~/.cynic/chats/{id}.json. Returns path."""
        os.makedirs(_CHATS_DIR, exist_ok=True)
        path = os.path.join(_CHATS_DIR, f"{self.session_id}.json")
        data = {
            "session_id": self.session_id,
            "created_at": self.created_at,
            "model": self.model,
            "system_prompt": self.system_prompt,
            "total_tokens": self.total_tokens,
            "total_cost_usd": self.total_cost_usd,
            "messages": [m.to_dict() for m in self.messages],
        }
        with open(path, "w", encoding="utf-8") as fh:
            json.dump(data, fh, indent=2)
        return path

    @classmethod
    def load(cls, session_id: str) -> ChatSession:
        """Load session from ~/.cynic/chats/{id}.json."""
        path = os.path.join(_CHATS_DIR, f"{session_id}.json")
        with open(path, encoding="utf-8") as fh:
            data = json.load(fh)

        session = cls(
            session_id=data.get("session_id", session_id),
            system_prompt=data.get("system_prompt", ""),
        )
        session.created_at = data.get("created_at", 0.0)
        session.model = data.get("model", "")
        session.total_tokens = data.get("total_tokens", 0)
        session.total_cost_usd = data.get("total_cost_usd", 0.0)
        session.messages = [ChatMessage.from_dict(m) for m in data.get("messages", [])]
        return session

    @staticmethod
    def list_sessions(limit: int = 20) -> list[dict]:
        """List recent sessions (newest first)."""
        if not os.path.isdir(_CHATS_DIR):
            return []
        files = []
        for name in os.listdir(_CHATS_DIR):
            if name.endswith(".json"):
                path = os.path.join(_CHATS_DIR, name)
                files.append((os.path.getmtime(path), name, path))
        files.sort(reverse=True)

        sessions = []
        for _, name, path in files[:limit]:
            try:
                with open(path, encoding="utf-8") as fh:
                    data = json.load(fh)
                sessions.append({
                    "session_id": data.get("session_id", name.replace(".json", "")),
                    "created_at": data.get("created_at", 0.0),
                    "model": data.get("model", ""),
                    "message_count": len(data.get("messages", [])),
                })
            except json.JSONDecodeError:
                continue
        return sessions
