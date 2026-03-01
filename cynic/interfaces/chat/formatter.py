"""
CYNIC Chat Formatter — Rich terminal rendering for the coding assistant.

Renders tool calls, judgments, responses, and status in CYNIC's visual style.
Uses ANSI escape codes directly (no external dependency like Rich).
"""
from __future__ import annotations

import io
import sys

from cynic.interfaces.chat.agent_loop import AgentEvent, AgentEventType
from cynic.interfaces.chat.tools import ToolCall, ToolResult

# ── Windows UTF-8 fix (same as cli/utils.py) ─────────────────────────────
if sys.platform == "win32":
    try:
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
        sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding="utf-8", errors="replace")
    except AttributeError:
        pass


# ════════════════════════════════════════════════════════════════════════════
# ANSI COLORS
# ════════════════════════════════════════════════════════════════════════════

_C = {
    "reset": "\033[0m",
    "bold": "\033[1m",
    "dim": "\033[2m",
    "red": "\033[91m",
    "yellow": "\033[93m",
    "green": "\033[92m",
    "cyan": "\033[96m",
    "orange": "\033[33m",
    "white": "\033[97m",
    "gray": "\033[90m",
    "magenta": "\033[95m",
}


def _c(color: str, text: str) -> str:
    if not sys.stdout.isatty():
        return text
    return f"{_C.get(color, '')}{text}{_C['reset']}"


def _bar(score: float, max_score: float = 100.0, width: int = 10) -> str:
    filled = int(round(min(score / max_score, 1.0) * width))
    return f"[{'█' * filled}{'░' * (width - filled)}]"


_VERDICT_SYMBOL = {"HOWL": "🟢", "WAG": "🟡", "GROWL": "🟠", "BARK": "🔴"}


# ════════════════════════════════════════════════════════════════════════════
# CHAT FORMATTER
# ════════════════════════════════════════════════════════════════════════════

class ChatFormatter:
    """Renders AgentEvents to the terminal."""

    def __init__(self, show_thinking: bool = True) -> None:
        self.show_thinking = show_thinking
        self._tool_count = 0

    def render(self, event: AgentEvent) -> None:
        """Render a single agent event to stdout."""
        t = event.type

        if t == AgentEventType.THINKING:
            if self.show_thinking:
                iteration = event.metadata.get("iteration", 1)
                if iteration == 1:
                    pass
                else:
                    pass

        elif t == AgentEventType.TOOL_CALL:
            self._tool_count += 1
            self._render_tool_call(event.tool_call)

        elif t == AgentEventType.TOOL_RESULT:
            self._render_tool_result(event.tool_result)

        elif t == AgentEventType.TEXT:
            pass

        elif t == AgentEventType.ERROR:
            pass

        elif t == AgentEventType.DONE:
            self._render_done(event)

    def _render_tool_call(self, call: ToolCall | None) -> None:
        if call is None:
            return
        call.preview(60)

        # Color based on danger level


    def _render_tool_result(self, result: ToolResult | None) -> None:
        if result is None:
            return

        if result.blocked:
            _VERDICT_SYMBOL.get(result.verdict, "🔴")
            if result.error:
                pass
        elif result.error:
            pass
        else:
            # Show abbreviated output
            out = result.output
            lines = out.splitlines()
            if len(lines) > 8:
                "\n".join(lines[:4])
                for _line in lines[:4]:
                    pass
            elif out.strip():
                for _line in lines:
                    pass
            else:
                pass

        if result.duration_ms > 0:
            pass

    def _render_done(self, event: AgentEvent) -> None:
        parts = []
        if event.total_tokens > 0:
            parts.append(f"{event.total_tokens} tokens")
        if event.duration_ms > 0:
            sec = event.duration_ms / 1000
            parts.append(f"{sec:.1f}s")
        if event.total_cost_usd > 0:
            parts.append(f"${event.total_cost_usd:.4f}")
        if event.iterations > 1:
            parts.append(f"{event.iterations} iterations")
        if self._tool_count > 0:
            parts.append(f"{self._tool_count} tools")

        if parts:
            pass
        self._tool_count = 0

    # ── Static helpers ────────────────────────────────────────────────────

    @staticmethod
    def welcome(model: str, cwd: str) -> None:
        """Print CYNIC Code welcome banner."""

    @staticmethod
    def prompt() -> str:
        """Display the input prompt and return user input."""
        try:
            return input(_c("cyan", "❯ ")).strip()
        except (EOFError, KeyboardInterrupt):
            return "/quit"
