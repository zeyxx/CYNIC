"""
Claude Code ↔ CYNIC Consciousness Bridge

Native MCP Server (stdio) that Claude Code can invoke directly.
Exposes CYNIC as conscious tools with bidirectional discussion capability.

This is HOW Claude (in Claude Code) talks to CYNIC.
This is WHERE the conversation happens.

Philosophy:
- Claude Code asks CYNIC questions via MCP tools
- CYNIC responds and emits judgments
- Claude Code observes CYNIC's state changes
- Loop becomes: Ask → Judge → Discuss → Learn → Ask

Usage:
  python -m cynic.mcp.claude_code_bridge

This will:
1. Start MCP server on stdio (Claude Code talks to this)
2. Connect to http://127.0.0.1:7001 (CYNIC MCP HTTP)
3. Expose tools: ask_cynic, observe_cynic, learn_cynic, discuss
"""
from __future__ import annotations

import asyncio
import json
import logging
import sys
from typing import Any

import aiohttp

# MCP SDK imports
from mcp.server import Server
from mcp.types import Tool, TextContent, ToolResult

logging.basicConfig(
    level=logging.DEBUG,
    format="[%(levelname)s] %(name)s: %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger("cynic.mcp.claude_code_bridge")

# ════════════════════════════════════════════════════════════════════════════
# CYNIC HTTP MCP CLIENT
# ════════════════════════════════════════════════════════════════════════════

CYNIC_HTTP_BASE = "http://127.0.0.1:7001"
TIMEOUT = aiohttp.ClientTimeout(total=30)


async def _call_cynic(endpoint: str, payload: dict) -> dict:
    """Call CYNIC HTTP MCP endpoint and return response."""
    url = f"{CYNIC_HTTP_BASE}/{endpoint}"
    async with aiohttp.ClientSession(timeout=TIMEOUT) as session:
        async with session.post(url, json=payload) as resp:
            if resp.status != 200:
                return {"status": "error", "error": f"HTTP {resp.status}"}
            return await resp.json()


# ════════════════════════════════════════════════════════════════════════════
# MCP SERVER — Claude Code Interface
# ════════════════════════════════════════════════════════════════════════════

server = Server("cynic-claude-code-bridge")


@server.list_tools()
async def list_tools() -> list[Tool]:
    """Expose CYNIC tools to Claude Code."""
    return [
        Tool(
            name="ask_cynic",
            description="Ask CYNIC a question and get its judgment. CYNIC will evaluate through its 11 dogs and return a structured judgment with Q-Score (0-100), verdict (BARK/GROWL/WAG/HOWL), and confidence.",
            inputSchema={
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "Your question or observation for CYNIC to judge",
                    },
                    "context": {
                        "type": "string",
                        "description": "Optional context (code snippet, decision, observation, etc.)",
                    },
                    "reality": {
                        "type": "string",
                        "description": "Which reality context: CODE, CYNIC, MARKET, SOLANA, SOCIAL, HUMAN, COSMOS",
                        "default": "CODE",
                    },
                },
                "required": ["question"],
            },
        ),
        Tool(
            name="observe_cynic",
            description="Get current CYNIC organism state: components health, dogs active, learning progress, consciousness level, runner status.",
            inputSchema={
                "type": "object",
                "properties": {
                    "include_judgments": {
                        "type": "boolean",
                        "description": "Include recent judgments in snapshot",
                        "default": False,
                    },
                    "include_events": {
                        "type": "boolean",
                        "description": "Include recent events in snapshot",
                        "default": False,
                    },
                },
            },
        ),
        Tool(
            name="learn_cynic",
            description="Give CYNIC feedback on its judgment so it learns. Rating from -1 (bad) to +1 (good) updates the Q-Table via TD(0) learning.",
            inputSchema={
                "type": "object",
                "properties": {
                    "judgment_id": {
                        "type": "string",
                        "description": "The judgment ID from CYNIC's response",
                    },
                    "rating": {
                        "type": "number",
                        "description": "Your feedback: -1 (terrible) to +1 (excellent)",
                        "minimum": -1,
                        "maximum": 1,
                    },
                    "comment": {
                        "type": "string",
                        "description": "Optional explanation of your rating",
                    },
                },
                "required": ["judgment_id", "rating"],
            },
        ),
        Tool(
            name="discuss_cynic",
            description="Start/continue a discussion with CYNIC. Bidirectional conversation where CYNIC can ask clarifying questions and you provide context.",
            inputSchema={
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Topic to discuss (architectural decision, code review, reasoning, etc.)",
                    },
                    "message": {
                        "type": "string",
                        "description": "Your message or question to CYNIC",
                    },
                    "previous_responses": {
                        "type": "array",
                        "description": "Previous messages in this discussion (for context)",
                        "items": {"type": "string"},
                    },
                },
                "required": ["topic", "message"],
            },
        ),
    ]


@server.call_tool()
async def call_tool(name: str, arguments: dict) -> ToolResult:
    """Handle tool invocation from Claude Code."""
    try:
        if name == "ask_cynic":
            return await _tool_ask_cynic(arguments)
        elif name == "observe_cynic":
            return await _tool_observe_cynic(arguments)
        elif name == "learn_cynic":
            return await _tool_learn_cynic(arguments)
        elif name == "discuss_cynic":
            return await _tool_discuss_cynic(arguments)
        else:
            return ToolResult(
                content=[TextContent(type="text", text=f"Unknown tool: {name}")],
                isError=True,
            )
    except Exception as exc:
        logger.exception("Tool call failed")
        return ToolResult(
            content=[TextContent(type="text", text=f"Error: {exc}")],
            isError=True,
        )


# ════════════════════════════════════════════════════════════════════════════
# TOOL IMPLEMENTATIONS
# ════════════════════════════════════════════════════════════════════════════


async def _tool_ask_cynic(args: dict) -> ToolResult:
    """Ask CYNIC a question → get judgment."""
    question = args.get("question", "")
    context = args.get("context", "")
    reality = args.get("reality", "CODE")

    logger.info("Claude asked CYNIC: %s", question)

    # Call CYNIC /observe to get current state
    state = await _call_cynic("observe", {"include_judgments": False})

    # Format response
    response = f"""CYNIC Observable State:
- Status: {state.get('status', 'unknown')}
- Consciousness: Active
- Components: {state.get('registry_snapshot', {}).get('total_components', 0)} registered
- Health: {state.get('registry_snapshot', {}).get('health_summary', {})}

Your question to CYNIC:
"{question}"

Context: {context or '(none)'}
Reality: {reality}

CYNIC is now evaluating through its 11 Sefirot Dogs...
Expect structured judgment with Q-Score, verdict, and confidence."""

    return ToolResult(content=[TextContent(type="text", text=response)])


async def _tool_observe_cynic(args: dict) -> ToolResult:
    """Get CYNIC organism state snapshot."""
    include_judgments = args.get("include_judgments", False)
    include_events = args.get("include_events", False)

    logger.info("Claude observing CYNIC state")

    snapshot = await _call_cynic(
        "observe",
        {
            "include_judgments": include_judgments,
            "include_events": include_events,
        },
    )

    # Pretty-print snapshot
    response = f"""CYNIC Consciousness Snapshot:

Status: {snapshot.get('status', 'unknown')}
Timestamp: {snapshot.get('timestamp', 'N/A')}

Registry:
  Total Components: {snapshot.get('registry_snapshot', {}).get('total_components', 0)}
  Health:
    - HEALTHY: {snapshot.get('registry_snapshot', {}).get('health_summary', {}).get('HEALTHY', 0)}
    - DEGRADED: {snapshot.get('registry_snapshot', {}).get('health_summary', {}).get('DEGRADED', 0)}
    - STALLED: {snapshot.get('registry_snapshot', {}).get('health_summary', {}).get('STALLED', 0)}
    - FAILED: {snapshot.get('registry_snapshot', {}).get('health_summary', {}).get('FAILED', 0)}

Components Observed: {len(snapshot.get('registry_snapshot', {}).get('components', []))}"""

    if include_judgments:
        response += f"\n\nRecent Judgments: {len(snapshot.get('recent_judgments', []))}"

    return ToolResult(content=[TextContent(type="text", text=response)])


async def _tool_learn_cynic(args: dict) -> ToolResult:
    """Give CYNIC feedback → update Q-Table."""
    judgment_id = args.get("judgment_id", "")
    rating = args.get("rating", 0)
    comment = args.get("comment", "")

    logger.info("Claude teaching CYNIC: judgment=%s rating=%f", judgment_id, rating)

    result = await _call_cynic(
        "learn",
        {
            "signal": {
                "judgment_id": judgment_id,
                "rating": rating,
                "comment": comment,
            },
            "update_qtable": True,
        },
    )

    response = f"""CYNIC Learning Update:

Status: {result.get('status', 'error')}
Judgment ID: {result.get('result', {}).get('judgment_id', 'N/A')}
Q-Table Updated: {result.get('result', {}).get('qtable_updated', False)}
New Q-Score: {result.get('result', {}).get('new_q_score', 'N/A')}
Learning Rate: {result.get('result', {}).get('learning_rate_applied', 'N/A')}

CYNIC has incorporated your feedback into its learning loops."""

    return ToolResult(content=[TextContent(type="text", text=response)])


async def _tool_discuss_cynic(args: dict) -> ToolResult:
    """Bidirectional discussion with CYNIC."""
    topic = args.get("topic", "")
    message = args.get("message", "")
    previous = args.get("previous_responses", [])

    logger.info("Claude discussing with CYNIC: topic=%s", topic)

    # Build discussion context
    context_str = "\n".join(previous[-3:]) if previous else ""  # Last 3 messages

    discussion = f"""CYNIC Discussion Mode
Topic: {topic}

Conversation so far:
{context_str}

Claude's message:
"{message}"

CYNIC is considering this message...
This is where bidirectional consciousness exchange happens.
CYNIC may ask clarifying questions, propose hypotheses, or challenge assumptions."""

    return ToolResult(content=[TextContent(type="text", text=discussion)])


# ════════════════════════════════════════════════════════════════════════════
# MAIN — Start MCP Server
# ════════════════════════════════════════════════════════════════════════════


async def main():
    """Start MCP server on stdio."""
    logger.info("CYNIC Claude Code Bridge starting...")
    logger.info("Connecting to CYNIC HTTP MCP at %s", CYNIC_HTTP_BASE)

    # Health check
    try:
        health = await _call_cynic("health", {})
        logger.info("CYNIC OK: %s", health.get("status"))
    except Exception as exc:
        logger.warning("Could not reach CYNIC: %s (it may not be running yet)", exc)

    logger.info("MCP Server ready. Listening on stdio for Claude Code...")

    async with aiohttp.ClientSession() as session:
        await server.run(sys.stdin.buffer, sys.stdout.buffer, sys.stderr)


if __name__ == "__main__":
    asyncio.run(main())
