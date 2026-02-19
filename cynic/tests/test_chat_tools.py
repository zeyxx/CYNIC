"""
Tests for cynic.chat.tools — Tool definitions + ToolCall/ToolResult dataclasses.
"""
from __future__ import annotations

import pytest

from cynic.chat.tools import (
    TOOLS, TOOL_NAMES, DANGEROUS_TOOLS,
    TOOL_BASH, TOOL_READ, TOOL_WRITE, TOOL_EDIT, TOOL_GLOB, TOOL_GREP,
    ToolCall, ToolResult,
)


# ════════════════════════════════════════════════════════════════════════════
# TOOL SCHEMA TESTS
# ════════════════════════════════════════════════════════════════════════════

class TestToolSchemas:
    """Validate that tool schemas match Ollama/OpenAI function calling format."""

    def test_tools_list_has_6_tools(self):
        assert len(TOOLS) == 6

    def test_all_tools_have_correct_structure(self):
        for tool in TOOLS:
            assert tool["type"] == "function"
            fn = tool["function"]
            assert "name" in fn
            assert "description" in fn
            assert "parameters" in fn
            params = fn["parameters"]
            assert params["type"] == "object"
            assert "properties" in params
            assert "required" in params

    def test_tool_names(self):
        names = {t["function"]["name"] for t in TOOLS}
        assert names == {"bash", "read", "write", "edit", "glob", "grep"}
        assert names == TOOL_NAMES

    def test_dangerous_tools(self):
        assert DANGEROUS_TOOLS == {"bash", "write", "edit"}

    def test_bash_has_command_required(self):
        params = TOOL_BASH["function"]["parameters"]
        assert "command" in params["properties"]
        assert "command" in params["required"]

    def test_read_has_path_required(self):
        params = TOOL_READ["function"]["parameters"]
        assert "path" in params["properties"]
        assert "path" in params["required"]
        # offset and limit are optional
        assert "offset" in params["properties"]
        assert "limit" in params["properties"]

    def test_write_has_path_and_content_required(self):
        params = TOOL_WRITE["function"]["parameters"]
        assert set(params["required"]) == {"path", "content"}

    def test_edit_has_three_required(self):
        params = TOOL_EDIT["function"]["parameters"]
        assert set(params["required"]) == {"path", "old_string", "new_string"}

    def test_glob_has_pattern_required(self):
        params = TOOL_GLOB["function"]["parameters"]
        assert "pattern" in params["required"]

    def test_grep_has_pattern_required(self):
        params = TOOL_GREP["function"]["parameters"]
        assert "pattern" in params["required"]


# ════════════════════════════════════════════════════════════════════════════
# TOOL CALL TESTS
# ════════════════════════════════════════════════════════════════════════════

class TestToolCall:
    """Test ToolCall dataclass."""

    def test_basic_creation(self):
        call = ToolCall(name="bash", arguments={"command": "ls -la"})
        assert call.name == "bash"
        assert call.arguments["command"] == "ls -la"

    def test_is_dangerous(self):
        assert ToolCall(name="bash").is_dangerous
        assert ToolCall(name="write").is_dangerous
        assert ToolCall(name="edit").is_dangerous
        assert not ToolCall(name="read").is_dangerous
        assert not ToolCall(name="glob").is_dangerous
        assert not ToolCall(name="grep").is_dangerous

    def test_is_valid(self):
        assert ToolCall(name="bash").is_valid
        assert ToolCall(name="read").is_valid
        assert not ToolCall(name="unknown").is_valid
        assert not ToolCall(name="").is_valid

    def test_preview_bash(self):
        call = ToolCall(name="bash", arguments={"command": "git status"})
        assert "git status" in call.preview()

    def test_preview_read(self):
        call = ToolCall(name="read", arguments={"path": "/foo/bar.py"})
        assert "/foo/bar.py" in call.preview()

    def test_preview_write(self):
        call = ToolCall(name="write", arguments={"path": "test.py", "content": "x" * 100})
        assert "test.py" in call.preview()
        assert "100 chars" in call.preview()

    def test_preview_truncation(self):
        call = ToolCall(name="bash", arguments={"command": "x" * 200})
        p = call.preview(max_len=50)
        assert len(p) <= 53  # 50 + "..."
        assert p.endswith("...")


# ════════════════════════════════════════════════════════════════════════════
# TOOL RESULT TESTS
# ════════════════════════════════════════════════════════════════════════════

class TestToolResult:
    """Test ToolResult dataclass."""

    def test_success(self):
        call = ToolCall(name="read")
        result = ToolResult(call=call, output="file contents")
        assert result.is_success
        assert result.to_message_content() == "file contents"

    def test_error(self):
        call = ToolCall(name="bash")
        result = ToolResult(call=call, error="command failed")
        assert not result.is_success
        assert "[ERROR]" in result.to_message_content()

    def test_blocked(self):
        call = ToolCall(name="bash")
        result = ToolResult(
            call=call,
            blocked=True,
            verdict="BARK",
            q_score=25.0,
            error="dangerous command",
        )
        assert not result.is_success
        assert "BLOCKED" in result.to_message_content()
        assert "BARK" in result.to_message_content()

    def test_duration(self):
        call = ToolCall(name="read")
        result = ToolResult(call=call, output="ok", duration_ms=150.0)
        assert result.duration_ms == 150.0
