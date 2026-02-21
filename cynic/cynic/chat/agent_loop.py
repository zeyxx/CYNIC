"""
CYNIC Agent Loop — LLM → tools → results → LLM (max F(7)=13 iterations).

The agentic core that makes CYNIC Code work. Yields AgentEvents so both
the CLI REPL and WebSocket UI can consume the same stream.

Flow:
  user_msg → LLM(messages+tools) →
    tool_calls? → judge each → execute → append results → LLM again
    text only?  → yield final response → done
    max 13 iterations (F(7))
"""
from __future__ import annotations

import logging
import sys
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, AsyncGenerator

# Python 3.9 compatibility: StrEnum added in Python 3.11
if sys.version_info >= (3, 11):
    from enum import StrEnum
else:
    class StrEnum(str, Enum):
        """Polyfill for Python <3.11."""
        pass

from cynic.chat.tools import TOOLS, ToolCall, ToolResult
from cynic.chat.tool_executor import ToolExecutor
from cynic.chat.session import ChatSession
from cynic.llm.adapter import LLMAdapter, LLMRequest, LLMResponse

logger = logging.getLogger("cynic.chat.agent")

_MAX_ITERATIONS = 13  # F(7) — phi-derived runaway prevention


# ════════════════════════════════════════════════════════════════════════════
# AGENT EVENTS (yielded to the consumer)
# ════════════════════════════════════════════════════════════════════════════

class AgentEventType(StrEnum):
    THINKING = "thinking"           # LLM is processing
    TEXT = "text"                    # Final text response (or partial)
    TOOL_CALL = "tool_call"         # About to execute a tool
    TOOL_RESULT = "tool_result"     # Tool execution complete
    JUDGMENT = "judgment"           # REFLEX judgment on a tool
    ERROR = "error"                 # Something went wrong
    DONE = "done"                   # Loop complete


@dataclass
class AgentEvent:
    """Event yielded by the agent loop."""
    type: AgentEventType
    content: str = ""               # Text content or error message
    tool_call: ToolCall | None = None
    tool_result: ToolResult | None = None
    metadata: dict[str, Any] = field(default_factory=dict)

    # Stats (populated on DONE event)
    total_tokens: int = 0
    total_cost_usd: float = 0.0
    duration_ms: float = 0.0
    iterations: int = 0


# ════════════════════════════════════════════════════════════════════════════
# AGENT LOOP
# ════════════════════════════════════════════════════════════════════════════

class AgentLoop:
    """
    The agentic loop: LLM calls tools, tools return results, LLM continues.

    Works with any LLMAdapter (Ollama, Claude, etc.) and any ToolExecutor.
    """

    def __init__(
        self,
        adapter: LLMAdapter,
        executor: ToolExecutor,
        session: ChatSession,
        max_iterations: int = _MAX_ITERATIONS,
    ) -> None:
        self.adapter = adapter
        self.executor = executor
        self.session = session
        self.max_iterations = max_iterations

    async def run(self, user_input: str) -> AsyncGenerator[AgentEvent, None]:
        """
        Process user input through the agentic loop.

        Yields AgentEvents for each step: thinking, tool calls, results, final text.
        """
        start = time.time()
        total_tokens = 0
        total_cost = 0.0

        # Add user message to session
        self.session.add_user(user_input)

        for iteration in range(self.max_iterations):
            yield AgentEvent(
                type=AgentEventType.THINKING,
                metadata={"iteration": iteration + 1},
            )

            # Call LLM with full session history + tools
            request = LLMRequest(
                prompt=user_input,
                messages=self.session.to_ollama_messages(),
                tools=TOOLS,
                max_tokens=4096,
                temperature=0.1,
            )

            try:
                response = await self.adapter.complete_safe(request)
            except asyncio.TimeoutError as exc:
                yield AgentEvent(type=AgentEventType.ERROR, content=str(exc))
                return

            if response.error:
                yield AgentEvent(type=AgentEventType.ERROR, content=response.error)
                return

            total_tokens += response.total_tokens
            total_cost += response.cost_usd

            # Check for tool calls
            if response.tool_calls:
                # Add assistant message with tool calls to session
                self.session.add_assistant(
                    content=response.content or "",
                    tool_calls=response.tool_calls,
                )

                # If there's also text, yield it
                if response.content:
                    yield AgentEvent(
                        type=AgentEventType.TEXT,
                        content=response.content,
                    )

                # Execute each tool call
                for tc_data in response.tool_calls:
                    call = ToolCall(
                        name=tc_data.get("name", ""),
                        arguments=tc_data.get("arguments", {}),
                    )

                    yield AgentEvent(
                        type=AgentEventType.TOOL_CALL,
                        tool_call=call,
                    )

                    try:
                        result = await self.executor.execute(call)
                    except httpx.RequestError as exc:
                        logger.error("Tool execution failed (%s): %s", call.name, exc, exc_info=True)
                        yield AgentEvent(
                            type=AgentEventType.ERROR,
                            content=f"Tool {call.name} failed: {str(exc)}",
                        )
                        continue

                    yield AgentEvent(
                        type=AgentEventType.TOOL_RESULT,
                        tool_result=result,
                    )

                    # Add tool result to session
                    self.session.add_tool_result(
                        name=call.name,
                        content=result.to_message_content(),
                    )

                # Continue loop — LLM needs to process tool results
                continue

            else:
                # No tool calls — this is the final response
                content = response.content or ""
                self.session.add_assistant(content=content)

                yield AgentEvent(
                    type=AgentEventType.TEXT,
                    content=content,
                )

                duration_ms = (time.time() - start) * 1000
                self.session.total_tokens += total_tokens
                self.session.total_cost_usd += total_cost

                yield AgentEvent(
                    type=AgentEventType.DONE,
                    total_tokens=total_tokens,
                    total_cost_usd=total_cost,
                    duration_ms=duration_ms,
                    iterations=iteration + 1,
                )
                return

        # Hit max iterations — force stop
        duration_ms = (time.time() - start) * 1000
        yield AgentEvent(
            type=AgentEventType.ERROR,
            content=f"Max iterations ({self.max_iterations}) reached — stopping to prevent runaway loop",
            metadata={"iterations": self.max_iterations},
        )
        yield AgentEvent(
            type=AgentEventType.DONE,
            total_tokens=total_tokens,
            total_cost_usd=total_cost,
            duration_ms=duration_ms,
            iterations=self.max_iterations,
        )
