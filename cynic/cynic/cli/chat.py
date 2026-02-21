"""
CYNIC Code CLI — Interactive coding assistant REPL.

Usage:
    python -m cynic.cli chat              → interactive REPL
    python -m cynic.cli chat --model X    → use specific Ollama model
    python -m cynic.cli chat --resume ID  → resume a previous session

Discovers Ollama models via LLMRegistry, picks the best coder model,
then runs the AgentLoop with REFLEX judgment on dangerous tools.

Lightweight: no server, no DB, no full kernel needed.
"""
from __future__ import annotations

import asyncio
import os
import sys

from cynic.chat.tools import TOOLS
from cynic.chat.tool_executor import ToolExecutor
from cynic.chat.agent_loop import AgentLoop, AgentEventType
from cynic.chat.session import ChatSession
from cynic.chat.formatter import ChatFormatter
from typing import Optional

# ── Model preference order for coding ────────────────────────────────────
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


def _pick_model(available: list[str], requested: Optional[str] = None) -> Optional[str]:
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


async def _discover_and_pick(requested_model: Optional[str]) -> tuple:
    """Discover Ollama models and return (adapter, model_name)."""
    from cynic.llm.adapter import OllamaAdapter

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
    from cynic.chat.formatter import _c

    if cmd in ("/quit", "/exit", "/q"):
        path = session.save()
        print(_c("dim", f"\n*yawn* Session saved: {path}"))
        return False

    if cmd == "/help":
        print(_c("cyan", """
  /help     — Show this help
  /status   — Session stats
  /model    — Show current model
  /clear    — Clear conversation history
  /sessions — List past sessions
  /save     — Save session now
  /quit     — Exit
"""))
        return True

    if cmd == "/status":
        print(_c("cyan", f"  Session: {session.session_id}"))
        print(_c("cyan", f"  Messages: {session.message_count}"))
        print(_c("cyan", f"  Tokens: {session.total_tokens}"))
        print(_c("cyan", f"  Cost: ${session.total_cost_usd:.4f}"))
        return True

    if cmd == "/model":
        print(_c("green", f"  Model: {model}"))
        return True

    if cmd == "/clear":
        session.messages.clear()
        print(_c("dim", "  *sniff* Conversation cleared."))
        return True

    if cmd == "/sessions":
        sessions = ChatSession.list_sessions(limit=10)
        if not sessions:
            print(_c("dim", "  No saved sessions."))
        else:
            for s in sessions:
                print(_c("dim", f"  {s['session_id']}  {s['model']}  {s['message_count']} msgs"))
        return True

    if cmd == "/save":
        path = session.save()
        print(_c("dim", f"  Saved: {path}"))
        return True

    print(_c("dim", f"  Unknown command: {cmd}. Try /help"))
    return True


async def _run_repl(model_name: Optional[str] = None, resume_id: Optional[str] = None) -> None:
    """Main REPL loop."""
    from cynic.chat.formatter import _c

    # Discover model
    adapter, model = await _discover_and_pick(model_name)
    if adapter is None:
        print(_c("red", "*GROWL* No Ollama models available."))
        print(_c("dim", "  Make sure Ollama is running: ollama serve"))
        print(_c("dim", "  Install a coding model: ollama pull qwen2.5-coder:7b"))
        return

    cwd = os.getcwd()
    system_prompt = _SYSTEM_PROMPT.format(cwd=cwd)

    # Create or resume session
    if resume_id:
        try:
            session = ChatSession.load(resume_id)
            session.model = model
            print(_c("dim", f"*sniff* Resumed session {resume_id} ({session.message_count} messages)"))
        except Exception:
            print(_c("yellow", f"  Could not load session {resume_id}, starting fresh"))
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
            except Exception:
                pass


def cmd_chat() -> None:
    """Entry point for `python -m cynic.cli chat`."""
    args = sys.argv[2:]  # Skip 'cynic.cli' and 'chat'

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
