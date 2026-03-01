"""
CYNIC Code CLI â€” Interactive coding assistant REPL.

Usage:
    python -m cynic.interfaces.cli chat              â†’ interactive REPL
    python -m cynic.interfaces.cli chat --model X    â†’ use specific Ollama model
    python -m cynic.interfaces.cli chat --resume ID  â†’ resume a previous session

Discovers Ollama models via LLMRegistry, picks the best coder model,
then runs the AgentLoop with REFLEX judgment on dangerous tools.

Lightweight: no server, no DB, no full kernel needed.
"""
from __future__ import annotations

import asyncio
import os
import sys

from cynic.interfaces.chat.agent_loop import AgentLoop
from cynic.interfaces.chat.formatter import ChatFormatter
from cynic.interfaces.chat.session import ChatSession
from cynic.interfaces.chat.tool_executor import ToolExecutor

# â”€â”€ Model preference order for coding â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
_CODER_MODELS = [
    "qwen2.5-coder:7b",
    "qwen2.5-coder:7b-instruct",
    "qwen2.5-coder:7b-instruct-q4_K_M",
    "qwen2.5-coder:3b",
    "qwen3:8b",
    "qwen3:4b",
    "deepseek-coder-v2:16b",
    "codellama:7b",
    "mistral:7b",
    "gemma2:9b",
    "gemma2:2b",
    "llama3.2:3b",
    "llama3.2",
]

_SYSTEM_PROMPT = """You are CYNIC Code, a local AI coding assistant.

Tools available: bash, read, write, edit, glob, grep

Rules:
- Read files before modifying them. Use glob/grep to find files first.
- Use edit for changes to existing files, write only for new files.
- Use bash for git, npm, pip, running tests, and other commands.
- Be direct and concise. Show code, not explanations.
- If unsure about file structure, use glob to explore first.

Working directory: {cwd}
"""


def _pick_model(available: list[str], requested: str | None = None) -> str | None:
    """Pick the best coding model from available Ollama models."""
    if requested:
        # Exact match or prefix match
        for m in available:
            if m == requested or m.startswith(requested):
                return m
        return None

    # Check preference order
    for preferred in _CODER_MODELS:
        for m in available:
            if m == preferred or m.startswith(preferred.split(":")[0]):
                return m

    # Any available model
    return available[0] if available else None


async def _discover_and_pick(requested_model: str | None) -> tuple:
    """Discover Ollama models and return (adapter, model_name)."""
    from cynic.kernel.organism.brain.llm.adapter import OllamaAdapter

    probe = OllamaAdapter(model="probe")
    if not await probe.check_available():
        return None, None

    models = await probe.list_models()
    if not models:
        return None, None

    model_name = _pick_model(models, requested_model)
    if not model_name:
        return None, None

    adapter = OllamaAdapter(model=model_name)
    return adapter, model_name


def _handle_command(cmd: str, session: ChatSession, model: str) -> bool:
    """Handle /commands. Returns True if should continue REPL, False to quit."""

    if cmd in ("/quit", "/exit", "/q"):
        session.save()
        return False

    if cmd == "/help":
        return True

    if cmd == "/status":
        return True

    if cmd == "/model":
        return True

    if cmd == "/clear":
        session.messages.clear()
        return True

    if cmd == "/sessions":
        sessions = ChatSession.list_sessions(limit=10)
        if not sessions:
            pass
        else:
            for _s in sessions:
                pass
        return True

    if cmd == "/save":
        session.save()
        return True

    return True


async def _run_repl(model_name: str | None = None, resume_id: str | None = None) -> None:
    """Main REPL loop."""

    # Discover model
    adapter, model = await _discover_and_pick(model_name)
    if adapter is None:
        return

    cwd = os.getcwd()
    system_prompt = _SYSTEM_PROMPT.format(cwd=cwd)

    # Create or resume session
    if resume_id:
        try:
            session = ChatSession.load(resume_id)
            session.model = model
        except httpx.RequestError:
            session = ChatSession(system_prompt=system_prompt)
            session.model = model
    else:
        session = ChatSession(system_prompt=system_prompt)
        session.model = model

    executor = ToolExecutor(cwd=cwd)
    formatter = ChatFormatter()

    # Welcome banner
    formatter.welcome(model, cwd)

    # REPL loop
    while True:
        try:
            user_input = formatter.prompt()
        except (EOFError, KeyboardInterrupt):
            user_input = "/quit"

        if not user_input:
            continue

        if user_input.startswith("/"):
            if not _handle_command(user_input, session, model):
                break
            continue

        # Run agent loop
        loop = AgentLoop(
            adapter=adapter,
            executor=executor,
            session=session,
        )

        async for event in loop.run(user_input):
            formatter.render(event)

        # Auto-save periodically
        if session.message_count % 10 == 0:
            try:
                session.save()
            except asyncpg.Error:
                pass


def cmd_chat() -> None:
    """Entry point for `python -m cynic.interfaces.cli chat`."""
    args = sys.argv[2:]  # Skip 'cynic.interfaces.cli' and 'chat'

    model_name = None
    resume_id = None

    i = 0
    while i < len(args):
        if args[i] in ("--model", "-m") and i + 1 < len(args):
            model_name = args[i + 1]
            i += 2
        elif args[i] in ("--resume", "-r") and i + 1 < len(args):
            resume_id = args[i + 1]
            i += 2
        else:
            i += 1

    asyncio.run(_run_repl(model_name=model_name, resume_id=resume_id))
