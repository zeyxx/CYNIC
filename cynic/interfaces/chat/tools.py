"""
CYNIC Chat Tools — 6 coding tools in Ollama/OpenAI JSON Schema format.

These are the hands of CYNIC Code: bash, read, write, edit, glob, grep.
Matches Claude Code's tool surface but runs 100% local via Ollama.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


# ════════════════════════════════════════════════════════════════════════════
# TOOL SCHEMAS (Ollama/OpenAI function calling format)
# ════════════════════════════════════════════════════════════════════════════

TOOL_BASH = {
    "type": "function",
    "function": {
        "name": "bash",
        "description": "Execute a shell command and return stdout/stderr. Use for git, npm, pip, running tests, etc.",
        "parameters": {
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute",
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default 120)",
                },
            },
            "required": ["command"],
        },
    },
}

TOOL_READ = {
    "type": "function",
    "function": {
        "name": "read",
        "description": "Read the contents of a file. Always read before editing.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or relative file path to read",
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start from (1-based, optional)",
                },
                "limit": {
                    "type": "integer",
                    "description": "Max lines to read (optional)",
                },
            },
            "required": ["path"],
        },
    },
}

TOOL_WRITE = {
    "type": "function",
    "function": {
        "name": "write",
        "description": "Write content to a file (creates or overwrites). Use for new files only — prefer edit for existing files.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to write to",
                },
                "content": {
                    "type": "string",
                    "description": "The full content to write",
                },
            },
            "required": ["path", "content"],
        },
    },
}

TOOL_EDIT = {
    "type": "function",
    "function": {
        "name": "edit",
        "description": "Replace exact text in a file. old_string must match exactly (including whitespace). Read the file first.",
        "parameters": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to edit",
                },
                "old_string": {
                    "type": "string",
                    "description": "Exact text to find and replace",
                },
                "new_string": {
                    "type": "string",
                    "description": "Replacement text",
                },
            },
            "required": ["path", "old_string", "new_string"],
        },
    },
}

TOOL_GLOB = {
    "type": "function",
    "function": {
        "name": "glob",
        "description": "Find files matching a glob pattern. Use to discover file structure before reading.",
        "parameters": {
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g. '**/*.py', 'src/**/*.ts')",
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search in (default: cwd)",
                },
            },
            "required": ["pattern"],
        },
    },
}

TOOL_GREP = {
    "type": "function",
    "function": {
        "name": "grep",
        "description": "Search file contents for a regex pattern. Returns matching lines with file paths.",
        "parameters": {
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for",
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in (default: cwd)",
                },
                "glob": {
                    "type": "string",
                    "description": "File glob filter (e.g. '*.py')",
                },
            },
            "required": ["pattern"],
        },
    },
}


# All tools as a list (pass to LLMRequest.tools)
TOOLS: list[dict] = [TOOL_BASH, TOOL_READ, TOOL_WRITE, TOOL_EDIT, TOOL_GLOB, TOOL_GREP]

# Tool names that modify state (get REFLEX judgment before execution)
DANGEROUS_TOOLS: frozenset[str] = frozenset({"bash", "write", "edit"})

# Tool names (for validation)
TOOL_NAMES: frozenset[str] = frozenset(t["function"]["name"] for t in TOOLS)


# ════════════════════════════════════════════════════════════════════════════
# DATACLASSES
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class ToolCall:
    """A parsed tool call from the LLM."""
    name: str
    arguments: dict[str, Any] = field(default_factory=dict)
    call_id: str = ""

    @property
    def is_dangerous(self) -> bool:
        return self.name in DANGEROUS_TOOLS

    @property
    def is_valid(self) -> bool:
        return self.name in TOOL_NAMES

    def preview(self, max_len: int = 80) -> str:
        """Short preview of the call for display."""
        if self.name == "bash":
            cmd = self.arguments.get("command", "")
            return cmd[:max_len] + ("..." if len(cmd) > max_len else "")
        if self.name == "read":
            return self.arguments.get("path", "")
        if self.name == "write":
            p = self.arguments.get("path", "")
            c = self.arguments.get("content", "")
            return f"{p} ({len(c)} chars)"
        if self.name == "edit":
            return self.arguments.get("path", "")
        if self.name == "glob":
            return self.arguments.get("pattern", "")
        if self.name == "grep":
            return self.arguments.get("pattern", "")
        return str(self.arguments)[:max_len]


@dataclass
class ToolResult:
    """Result of executing a tool."""
    call: ToolCall
    output: str = ""
    error: str = ""
    blocked: bool = False          # True if CYNIC judgment blocked execution
    verdict: str = ""              # HOWL/WAG/GROWL/BARK from REFLEX judgment
    q_score: float = 0.0
    duration_ms: float = 0.0

    @property
    def is_success(self) -> bool:
        return not self.blocked and not self.error

    def to_message_content(self) -> str:
        """Format for inclusion in chat messages back to the LLM."""
        if self.blocked:
            return f"[BLOCKED by CYNIC Guardian — verdict: {self.verdict} Q={self.q_score:.1f}] {self.error}"
        if self.error:
            return f"[ERROR] {self.error}"
        return self.output
