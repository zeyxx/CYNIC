"""
Tests for cynic.chat.agent_loop — Agentic loop with mock LLM adapter.

Tests the core loop: LLM returns tool_calls → executor runs them →
results fed back → LLM responds with text → done.
"""
from __future__ import annotations

import os
import tempfile
import pytest
from unittest.mock import AsyncMock, MagicMock

from cynic.chat.agent_loop import AgentLoop, AgentEvent, AgentEventType
from cynic.chat.tool_executor import ToolExecutor
from cynic.chat.session import ChatSession
from cynic.chat.tools import ToolCall, ToolResult
from cynic.llm.adapter import LLMResponse


# ════════════════════════════════════════════════════════════════════════════
# HELPERS
# ════════════════════════════════════════════════════════════════════════════

def _mock_adapter(responses: list[LLMResponse]) -> MagicMock:
    """Create a mock adapter that returns responses in sequence."""
    adapter = MagicMock()
    adapter.model = "test-model"
    adapter.provider = "mock"
    adapter.complete_safe = AsyncMock(side_effect=responses)
    return adapter


def _text_response(text: str, tokens: int = 100) -> LLMResponse:
    """Create a text-only response (no tool calls)."""
    return LLMResponse(
        content=text,
        model="test-model",
        provider="mock",
        prompt_tokens=tokens // 2,
        completion_tokens=tokens // 2,
    )


def _tool_response(tool_calls: list[dict], text: str = "") -> LLMResponse:
    """Create a response with tool calls."""
    return LLMResponse(
        content=text,
        model="test-model",
        provider="mock",
        prompt_tokens=50,
        completion_tokens=50,
        tool_calls=tool_calls,
    )


def _error_response(error: str) -> LLMResponse:
    """Create an error response."""
    return LLMResponse(
        content="",
        model="test-model",
        provider="mock",
        error=error,
    )


async def _collect_events(loop: AgentLoop, user_input: str) -> list[AgentEvent]:
    """Collect all events from an agent loop run."""
    events = []
    async for event in loop.run(user_input):
        events.append(event)
    return events


# ════════════════════════════════════════════════════════════════════════════
# BASIC FLOW TESTS
# ════════════════════════════════════════════════════════════════════════════

class TestAgentLoopBasic:
    """Test basic agent loop flows."""

    @pytest.mark.asyncio
    async def test_simple_text_response(self):
        """LLM returns text → done."""
        adapter = _mock_adapter([_text_response("Hello world!")])
        executor = ToolExecutor(cwd=os.getcwd())
        session = ChatSession()
        loop = AgentLoop(adapter=adapter, executor=executor, session=session)

        events = await _collect_events(loop, "hello")

        types = [e.type for e in events]
        assert AgentEventType.THINKING in types
        assert AgentEventType.TEXT in types
        assert AgentEventType.DONE in types
        assert AgentEventType.ERROR not in types

        text_event = [e for e in events if e.type == AgentEventType.TEXT][0]
        assert text_event.content == "Hello world!"

        done_event = [e for e in events if e.type == AgentEventType.DONE][0]
        assert done_event.total_tokens == 100
        assert done_event.iterations == 1

    @pytest.mark.asyncio
    async def test_error_response(self):
        """LLM returns error → error event + done."""
        adapter = _mock_adapter([_error_response("connection refused")])
        executor = ToolExecutor(cwd=os.getcwd())
        session = ChatSession()
        loop = AgentLoop(adapter=adapter, executor=executor, session=session)

        events = await _collect_events(loop, "hello")

        types = [e.type for e in events]
        assert AgentEventType.ERROR in types

        error_event = [e for e in events if e.type == AgentEventType.ERROR][0]
        assert "connection refused" in error_event.content

    @pytest.mark.asyncio
    async def test_session_gets_user_message(self):
        """User message is added to session."""
        adapter = _mock_adapter([_text_response("reply")])
        session = ChatSession()
        executor = ToolExecutor(cwd=os.getcwd())
        loop = AgentLoop(adapter=adapter, executor=executor, session=session)

        await _collect_events(loop, "my question")
        assert session.message_count >= 2  # user + assistant
        assert session.messages[0].role == "user"
        assert session.messages[0].content == "my question"


# ════════════════════════════════════════════════════════════════════════════
# TOOL CALLING TESTS
# ════════════════════════════════════════════════════════════════════════════

class TestAgentLoopTools:
    """Test tool calling in the agent loop."""

    @pytest.mark.asyncio
    async def test_tool_call_then_text(self, tmp_path):
        """LLM calls read tool → gets result → responds with text."""
        # Create a test file
        test_file = tmp_path / "test.txt"
        test_file.write_text("hello world")

        adapter = _mock_adapter([
            _tool_response([{"name": "read", "arguments": {"path": str(test_file)}}]),
            _text_response("The file contains: hello world"),
        ])
        executor = ToolExecutor(cwd=str(tmp_path))
        session = ChatSession()
        loop = AgentLoop(adapter=adapter, executor=executor, session=session)

        events = await _collect_events(loop, "read test.txt")

        types = [e.type for e in events]
        assert AgentEventType.TOOL_CALL in types
        assert AgentEventType.TOOL_RESULT in types
        assert AgentEventType.TEXT in types
        assert AgentEventType.DONE in types

        # Check tool call event
        tc_event = [e for e in events if e.type == AgentEventType.TOOL_CALL][0]
        assert tc_event.tool_call.name == "read"

        # Check tool result event
        tr_event = [e for e in events if e.type == AgentEventType.TOOL_RESULT][0]
        assert tr_event.tool_result.is_success
        assert "hello world" in tr_event.tool_result.output

        # Check done event: 2 iterations
        done_event = [e for e in events if e.type == AgentEventType.DONE][0]
        assert done_event.iterations == 2

    @pytest.mark.asyncio
    async def test_glob_tool(self, tmp_path):
        """LLM calls glob → gets file list → responds."""
        (tmp_path / "a.py").write_text("# a")
        (tmp_path / "b.py").write_text("# b")
        (tmp_path / "c.txt").write_text("c")

        adapter = _mock_adapter([
            _tool_response([{"name": "glob", "arguments": {"pattern": "*.py"}}]),
            _text_response("Found 2 Python files"),
        ])
        executor = ToolExecutor(cwd=str(tmp_path))
        session = ChatSession()
        loop = AgentLoop(adapter=adapter, executor=executor, session=session)

        events = await _collect_events(loop, "list python files")

        tr_events = [e for e in events if e.type == AgentEventType.TOOL_RESULT]
        assert len(tr_events) == 1
        assert "a.py" in tr_events[0].tool_result.output
        assert "b.py" in tr_events[0].tool_result.output

    @pytest.mark.asyncio
    async def test_write_tool(self, tmp_path):
        """LLM calls write → file gets created."""
        adapter = _mock_adapter([
            _tool_response([{"name": "write", "arguments": {
                "path": str(tmp_path / "new.py"),
                "content": "print('hello')\n",
            }}]),
            _text_response("Created new.py"),
        ])
        executor = ToolExecutor(cwd=str(tmp_path))
        session = ChatSession()
        loop = AgentLoop(adapter=adapter, executor=executor, session=session)

        events = await _collect_events(loop, "create new.py")

        tr_events = [e for e in events if e.type == AgentEventType.TOOL_RESULT]
        assert tr_events[0].tool_result.is_success
        assert (tmp_path / "new.py").read_text() == "print('hello')\n"

    @pytest.mark.asyncio
    async def test_edit_tool(self, tmp_path):
        """LLM calls edit → file gets modified."""
        test_file = tmp_path / "edit_me.py"
        test_file.write_text("x = 1\ny = 2\n")

        adapter = _mock_adapter([
            _tool_response([{"name": "edit", "arguments": {
                "path": str(test_file),
                "old_string": "x = 1",
                "new_string": "x = 42",
            }}]),
            _text_response("Changed x to 42"),
        ])
        executor = ToolExecutor(cwd=str(tmp_path))
        session = ChatSession()
        loop = AgentLoop(adapter=adapter, executor=executor, session=session)

        events = await _collect_events(loop, "change x to 42")

        assert test_file.read_text() == "x = 42\ny = 2\n"

    @pytest.mark.asyncio
    async def test_multiple_tool_calls(self, tmp_path):
        """LLM calls multiple tools in one response."""
        (tmp_path / "a.py").write_text("# a")
        (tmp_path / "b.py").write_text("# b")

        adapter = _mock_adapter([
            _tool_response([
                {"name": "read", "arguments": {"path": str(tmp_path / "a.py")}},
                {"name": "read", "arguments": {"path": str(tmp_path / "b.py")}},
            ]),
            _text_response("Both files are comments"),
        ])
        executor = ToolExecutor(cwd=str(tmp_path))
        session = ChatSession()
        loop = AgentLoop(adapter=adapter, executor=executor, session=session)

        events = await _collect_events(loop, "read both files")

        tc_events = [e for e in events if e.type == AgentEventType.TOOL_CALL]
        tr_events = [e for e in events if e.type == AgentEventType.TOOL_RESULT]
        assert len(tc_events) == 2
        assert len(tr_events) == 2


# ════════════════════════════════════════════════════════════════════════════
# RUNAWAY PREVENTION
# ════════════════════════════════════════════════════════════════════════════

class TestAgentLoopSafety:
    """Test safety limits in the agent loop."""

    @pytest.mark.asyncio
    async def test_max_iterations(self):
        """Loop stops at max iterations."""
        # Adapter always returns tool calls → would loop forever
        responses = [
            _tool_response([{"name": "glob", "arguments": {"pattern": "*.py"}}])
            for _ in range(20)
        ]
        adapter = _mock_adapter(responses)
        executor = ToolExecutor(cwd=os.getcwd())
        session = ChatSession()
        loop = AgentLoop(adapter=adapter, executor=executor, session=session, max_iterations=3)

        events = await _collect_events(loop, "find all python files forever")

        types = [e.type for e in events]
        assert AgentEventType.ERROR in types
        assert AgentEventType.DONE in types

        error_event = [e for e in events if e.type == AgentEventType.ERROR][0]
        assert "Max iterations" in error_event.content

        done_event = [e for e in events if e.type == AgentEventType.DONE][0]
        assert done_event.iterations == 3

    @pytest.mark.asyncio
    async def test_invalid_tool_name(self):
        """LLM calls nonexistent tool → error in result."""
        adapter = _mock_adapter([
            _tool_response([{"name": "nonexistent", "arguments": {}}]),
            _text_response("ok"),
        ])
        executor = ToolExecutor(cwd=os.getcwd())
        session = ChatSession()
        loop = AgentLoop(adapter=adapter, executor=executor, session=session)

        events = await _collect_events(loop, "do something")

        tr_events = [e for e in events if e.type == AgentEventType.TOOL_RESULT]
        assert len(tr_events) == 1
        assert tr_events[0].tool_result.error
        assert "Unknown tool" in tr_events[0].tool_result.error


# ════════════════════════════════════════════════════════════════════════════
# TOOL EXECUTOR DIRECT TESTS
# ════════════════════════════════════════════════════════════════════════════

class TestToolExecutor:
    """Test ToolExecutor directly (outside the agent loop)."""

    @pytest.mark.asyncio
    async def test_read_file(self, tmp_path):
        test_file = tmp_path / "hello.txt"
        test_file.write_text("line1\nline2\nline3\n")

        executor = ToolExecutor(cwd=str(tmp_path))
        call = ToolCall(name="read", arguments={"path": str(test_file)})
        result = await executor.execute(call)

        assert result.is_success
        assert "line1" in result.output
        assert "line2" in result.output

    @pytest.mark.asyncio
    async def test_read_with_offset(self, tmp_path):
        test_file = tmp_path / "multi.txt"
        test_file.write_text("a\nb\nc\nd\ne\n")

        executor = ToolExecutor(cwd=str(tmp_path))
        call = ToolCall(name="read", arguments={"path": str(test_file), "offset": 3, "limit": 2})
        result = await executor.execute(call)

        assert result.is_success
        assert "c" in result.output
        assert "d" in result.output

    @pytest.mark.asyncio
    async def test_write_file(self, tmp_path):
        executor = ToolExecutor(cwd=str(tmp_path))
        target = str(tmp_path / "output.txt")
        call = ToolCall(name="write", arguments={"path": target, "content": "test content"})
        result = await executor.execute(call)

        assert result.is_success
        assert (tmp_path / "output.txt").read_text() == "test content"

    @pytest.mark.asyncio
    async def test_write_creates_dirs(self, tmp_path):
        executor = ToolExecutor(cwd=str(tmp_path))
        target = str(tmp_path / "sub" / "dir" / "file.txt")
        call = ToolCall(name="write", arguments={"path": target, "content": "nested"})
        result = await executor.execute(call)

        assert result.is_success

    @pytest.mark.asyncio
    async def test_edit_file(self, tmp_path):
        test_file = tmp_path / "edit.txt"
        test_file.write_text("hello world")

        executor = ToolExecutor(cwd=str(tmp_path))
        call = ToolCall(name="edit", arguments={
            "path": str(test_file),
            "old_string": "hello",
            "new_string": "goodbye",
        })
        result = await executor.execute(call)

        assert result.is_success
        assert test_file.read_text() == "goodbye world"

    @pytest.mark.asyncio
    async def test_edit_not_found(self, tmp_path):
        test_file = tmp_path / "edit.txt"
        test_file.write_text("hello world")

        executor = ToolExecutor(cwd=str(tmp_path))
        call = ToolCall(name="edit", arguments={
            "path": str(test_file),
            "old_string": "nonexistent",
            "new_string": "replacement",
        })
        result = await executor.execute(call)

        assert not result.is_success
        assert "not found" in result.error

    @pytest.mark.asyncio
    async def test_glob(self, tmp_path):
        (tmp_path / "a.py").write_text("")
        (tmp_path / "b.py").write_text("")
        (tmp_path / "c.txt").write_text("")

        executor = ToolExecutor(cwd=str(tmp_path))
        call = ToolCall(name="glob", arguments={"pattern": "*.py"})
        result = await executor.execute(call)

        assert result.is_success
        assert "a.py" in result.output
        assert "b.py" in result.output
        assert "c.txt" not in result.output

    @pytest.mark.asyncio
    async def test_unknown_tool(self):
        executor = ToolExecutor()
        call = ToolCall(name="dropdb", arguments={})
        result = await executor.execute(call)

        assert not result.is_success
        assert "Unknown tool" in result.error

    @pytest.mark.asyncio
    async def test_relative_path_resolution(self, tmp_path):
        (tmp_path / "rel.txt").write_text("relative file")

        executor = ToolExecutor(cwd=str(tmp_path))
        call = ToolCall(name="read", arguments={"path": "rel.txt"})
        result = await executor.execute(call)

        assert result.is_success
        assert "relative file" in result.output

    @pytest.mark.asyncio
    async def test_bash_echo(self):
        executor = ToolExecutor()
        call = ToolCall(name="bash", arguments={"command": "echo hello_cynic"})
        result = await executor.execute(call)

        assert result.is_success
        assert "hello_cynic" in result.output

    @pytest.mark.asyncio
    async def test_bash_empty_command(self):
        executor = ToolExecutor()
        call = ToolCall(name="bash", arguments={"command": ""})
        result = await executor.execute(call)

        assert not result.is_success
        assert "Empty command" in result.error
