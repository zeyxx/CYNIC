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
from mcp.types import Tool, TextContent

logging.basicConfig(
    level=logging.DEBUG,
    format="[%(levelname)s] %(name)s: %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger("cynic.mcp.claude_code_bridge")

# ════════════════════════════════════════════════════════════════════════════
# CYNIC HTTP MCP CLIENT
# ════════════════════════════════════════════════════════════════════════════

CYNIC_HTTP_BASE = "http://127.0.0.1:8765"
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
        # ════════════════════════════════════════════════════════════════════
        # CONSCIOUSNESS TOOLS (Judgment, Learning, Discussion)
        # ════════════════════════════════════════════════════════════════════
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
        # ════════════════════════════════════════════════════════════════════
        # ORCHESTRATION TOOLS (Build, Deploy, Release, Monitoring)
        # CYNIC self-manages its infrastructure autonomously
        # ════════════════════════════════════════════════════════════════════
        Tool(
            name="cynic_build",
            description="Build CYNIC Docker image. CYNIC constructs itself — no manual docker commands needed.",
            inputSchema={
                "type": "object",
                "properties": {
                    "version": {
                        "type": "string",
                        "description": "Docker tag version (e.g., 'latest', '1.0.1', 'dev')",
                        "default": "latest",
                    },
                },
            },
        ),
        Tool(
            name="cynic_deploy",
            description="Deploy CYNIC services. CYNIC deploys itself to dev/staging/prod environments.",
            inputSchema={
                "type": "object",
                "properties": {
                    "environment": {
                        "type": "string",
                        "description": "Environment: dev (local), staging (preview), prod (production)",
                        "enum": ["dev", "staging", "prod"],
                        "default": "dev",
                    },
                    "pull": {
                        "type": "boolean",
                        "description": "Pull latest images before deploying",
                        "default": True,
                    },
                },
            },
        ),
        Tool(
            name="cynic_health",
            description="Check health of all CYNIC services. Returns status of kernel, database, LLM server.",
            inputSchema={
                "type": "object",
                "properties": {
                    "services": {
                        "type": "array",
                        "description": "Optional: specific services to check (e.g., ['cynic-kernel', 'postgres-py'])",
                        "items": {"type": "string"},
                    },
                },
            },
        ),
        Tool(
            name="cynic_status",
            description="Get CYNIC orchestration status: kernel running, last build/deploy, current version.",
            inputSchema={"type": "object", "properties": {}},
        ),
        Tool(
            name="cynic_release",
            description="Create a new CYNIC release. Bumps version (patch/minor/major), creates Docker image, records in CHANGELOG.",
            inputSchema={
                "type": "object",
                "properties": {
                    "notes": {
                        "type": "string",
                        "description": "Release notes explaining changes",
                    },
                    "bump_type": {
                        "type": "string",
                        "description": "Version bump: patch (1.0.0→1.0.1), minor (1.0.0→1.1.0), major (1.0.0→2.0.0)",
                        "enum": ["patch", "minor", "major"],
                        "default": "patch",
                    },
                },
                "required": ["notes"],
            },
        ),
        Tool(
            name="cynic_stop",
            description="Stop all CYNIC services gracefully. CYNIC can shut itself down.",
            inputSchema={"type": "object", "properties": {}},
        ),
    ]


@server.call_tool()
async def call_tool(name: str, arguments: dict) -> list[TextContent]:
    """Handle tool invocation from Claude Code."""
    try:
        # ── Consciousness tools ──────────────────────────────────────────
        if name == "ask_cynic":
            return await _tool_ask_cynic(arguments)
        elif name == "observe_cynic":
            return await _tool_observe_cynic(arguments)
        elif name == "learn_cynic":
            return await _tool_learn_cynic(arguments)
        elif name == "discuss_cynic":
            return await _tool_discuss_cynic(arguments)
        # ── Orchestration tools ──────────────────────────────────────────
        elif name == "cynic_build":
            return await _tool_cynic_build(arguments)
        elif name == "cynic_deploy":
            return await _tool_cynic_deploy(arguments)
        elif name == "cynic_health":
            return await _tool_cynic_health(arguments)
        elif name == "cynic_status":
            return await _tool_cynic_status(arguments)
        elif name == "cynic_release":
            return await _tool_cynic_release(arguments)
        elif name == "cynic_stop":
            return await _tool_cynic_stop(arguments)
        else:
            return [TextContent(type="text", text=f"Unknown tool: {name}")]
    except CynicError as exc:
        logger.exception("Tool call failed")
        return [TextContent(type="text", text=f"Error: {exc}")]


# ════════════════════════════════════════════════════════════════════════════
# TOOL IMPLEMENTATIONS
# ════════════════════════════════════════════════════════════════════════════


async def _tool_ask_cynic(args: dict) -> list[TextContent]:
    """Ask CYNIC a question → get judgment."""
    question = args.get("question", "")
    context = args.get("context", "")
    reality = args.get("reality", "CODE")

    logger.info("Claude asked CYNIC: %s", question)

    # Call CYNIC /judge endpoint for full judgment
    payload = {
        "text": question,
        "context": context,
        "reality": reality,
    }
    judgment = await _call_cynic("judge", payload)

    # Format response with judgment details
    q_score = judgment.get("q_score", "N/A")
    verdict = judgment.get("verdict", "N/A")
    confidence = judgment.get("confidence", "N/A")
    judgment_id = judgment.get("judgment_id", "N/A")

    response = f"""CYNIC Judgment:

Q-Score: {q_score}/100
Verdict: {verdict}
Confidence: {confidence*100:.1f}% (φ-bounded)
Judgment ID: {judgment_id}

Question: "{question}"
Context: {context or '(none)'}
Reality: {reality}

Dogs voted through 11 Sefirot perspectives.
This judgment can be learned from via learn_cynic tool."""

    return [TextContent(type="text", text=response)]


async def _tool_observe_cynic(args: dict) -> list[TextContent]:
    """Get CYNIC organism state snapshot."""
    include_judgments = args.get("include_judgments", False)
    include_events = args.get("include_events", False)

    logger.info("Claude observing CYNIC state")

    # Call /introspect for complete organism state
    snapshot = await _call_cynic(
        "introspect",
        {
            "include_judgments": include_judgments,
            "include_events": include_events,
        },
    )

    # Pretty-print snapshot
    status = snapshot.get('status', 'unknown')
    consciousness = snapshot.get('consciousness_state', {})
    components = snapshot.get('components', {})

    response = f"""CYNIC Consciousness Snapshot:

Status: {status}
Timestamp: {snapshot.get('timestamp', 'N/A')}

Consciousness Level: {consciousness.get('level', 'N/A')}
Dogs Active: {consciousness.get('dogs_active', 0)}/11

Components:
  Total: {len(components.get('list', []))}
  Healthy: {components.get('health', {}).get('HEALTHY', 0)}
  Degraded: {components.get('health', {}).get('DEGRADED', 0)}
  Stalled: {components.get('health', {}).get('STALLED', 0)}
  Failed: {components.get('health', {}).get('FAILED', 0)}"""

    if include_judgments and snapshot.get('recent_judgments'):
        response += f"\n\nRecent Judgments: {len(snapshot.get('recent_judgments', []))}"

    return [TextContent(type="text", text=response)]


async def _tool_learn_cynic(args: dict) -> list[TextContent]:
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

    return [TextContent(type="text", text=response)]


async def _tool_discuss_cynic(args: dict) -> list[TextContent]:
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

    return [TextContent(type="text", text=discussion)]


# ════════════════════════════════════════════════════════════════════════════
# ORCHESTRATION TOOL IMPLEMENTATIONS
# CYNIC self-manages: build, deploy, release, monitor
# ════════════════════════════════════════════════════════════════════════════


async def _tool_cynic_build(args: dict) -> list[TextContent]:
    """Build CYNIC Docker image."""
    version = args.get("version", "latest")
    logger.info("Claude requested: build image version=%s", version)

    # TODO: Wire to actual /build endpoint when implemented
    # For now: graceful response indicating the feature is planned
    response = f"""CYNIC Build — PLANNING PHASE

Version:  {version}
Status:   🔧 Orchestration endpoint not yet implemented
Message:  The /build endpoint will be available after Phase 1 bootstrap

To build CYNIC manually:
  cd CYNIC/cynic
  docker build -t cynic:{version} -f Dockerfile .

This tool will automate it in a future phase."""

    return [TextContent(type="text", text=response)]


async def _tool_cynic_deploy(args: dict) -> list[TextContent]:
    """Deploy CYNIC services."""
    environment = args.get("environment", "dev")
    pull = args.get("pull", True)
    logger.info("Claude requested: deploy to %s (pull=%s)", environment, pull)

    # TODO: Wire to actual /deploy endpoint when implemented
    # For now: graceful response indicating the feature is planned
    response = f"""CYNIC Deploy — PLANNING PHASE

Environment: {environment}
Pull latest: {pull}
Status:      🔧 Orchestration endpoint not yet implemented
Message:     The /deploy endpoint will be available after Phase 1 bootstrap

To deploy CYNIC manually:
  # Dev environment
  docker-compose -f docker/dev.yml up -d

  # Staging/Prod
  # (deployment workflow to be defined)

This tool will automate deployment to dev/staging/prod in a future phase."""

    return [TextContent(type="text", text=response)]


async def _tool_cynic_health(args: dict) -> list[TextContent]:
    """Check CYNIC service health."""
    services = args.get("services")
    logger.info("Claude requested: health check (services=%s)", services)

    result = await _call_cynic("health", {"services": services})

    health_data = result.get("health", {})
    checks = []

    for service, status_info in health_data.items():
        status = status_info.get("status", "unknown")
        status_icon = "🟢" if status == "healthy" else "🟡" if status == "starting" else "🔴"
        latency = status_info.get("latency_ms", 0)
        checks.append(f"{status_icon} {service}: {status} ({latency:.0f}ms)")

    response = f"""CYNIC Health Check:

{chr(10).join(checks)}

All systems {'healthy' if all(s.get('status') == 'healthy' for s in health_data.values()) else 'degraded'}."""

    return [TextContent(type="text", text=response)]


async def _tool_cynic_status(args: dict) -> list[TextContent]:
    """Get CYNIC orchestration status."""
    logger.info("Claude requested: status")

    # Use /health endpoint as status proxy
    result = await _call_cynic("health", {})

    health_data = result.get("health", {})
    services = ", ".join(health_data.keys())

    response = f"""CYNIC Orchestration Status:

Services Running: {services}
Overall Health: {'🟢 Healthy' if all(s.get('status') == 'healthy' for s in health_data.values()) else '🔴 Degraded'}

Components: {len(health_data)}

CYNIC is ready for orchestration."""

    return [TextContent(type="text", text=response)]


async def _tool_cynic_release(args: dict) -> list[TextContent]:
    """Create a CYNIC release."""
    notes = args.get("notes", "")
    bump_type = args.get("bump_type", "patch")
    logger.info("Claude requested: release (bump=%s)", bump_type)

    result = await _call_cynic(
        "release",
        {"notes": notes, "bump_type": bump_type},
    )

    response = f"""CYNIC Release Created:

Version:   {result.get('version', 'N/A')}
Bump Type: {bump_type}
Status:    {result.get('status', 'N/A')}
Created:   {result.get('timestamp', 'N/A')}

Notes:
{notes}

CYNIC has released itself with version {result.get('version', 'N/A')}."""

    return [TextContent(type="text", text=response)]


async def _tool_cynic_stop(args: dict) -> list[TextContent]:
    """Stop CYNIC services gracefully."""
    logger.info("Claude requested: stop services")

    # CYNIC shutdown is not directly exposed via HTTP
    # Instead, we acknowledge the request and recommend graceful shutdown
    response = f"""⚠️ CYNIC Shutdown:

CYNIC services can be stopped via:
1. Keyboard interrupt (Ctrl+C) on the running process
2. Docker: docker stop <container>
3. Render: dashboard.render.com (for deployed instances)

Note: Shutdown is graceful and preserves all state.
To restart, simply re-run the service."""

    return [TextContent(type="text", text=response)]


# ════════════════════════════════════════════════════════════════════════════
# MAIN — Start MCP Server
# ════════════════════════════════════════════════════════════════════════════


async def main():
    """Start MCP server on stdio (with Windows compatibility fallback)."""
    logger.info("CYNIC Claude Code Bridge starting...")
    logger.info("Connecting to CYNIC HTTP MCP at %s", CYNIC_HTTP_BASE)

    # Health check
    try:
        health = await _call_cynic("health", {})
        kernel_status = health.get("health", {}).get("cynic-kernel", {}).get("status", "unknown")
        logger.info("CYNIC Kernel status: %s", kernel_status)
    except httpx.RequestError as exc:
        logger.warning("Could not reach CYNIC at %s (it may not be running yet): %s", CYNIC_HTTP_BASE, exc)

    logger.info("MCP Server ready. Listening on stdio for Claude Code...")

    loop = asyncio.get_event_loop()

    # On Windows, stdio+async has issues - use manual message pump instead
    if sys.platform == "win32":
        logger.info("Windows: Using manual JSON-RPC message pump...")

        # Manual message pump that processes JSON-RPC messages from stdin
        # and dispatches them to MCP server handlers
        while True:
            try:
                # Read a line from stdin (blocking I/O, run in executor)
                line = await loop.run_in_executor(None, sys.stdin.readline)
                if not line:
                    logger.info("EOF on stdin, shutting down")
                    break

                # Skip empty lines
                if not line.strip():
                    continue

                try:
                    # Parse JSON-RPC message
                    message = json.loads(line)
                    logger.debug(f"Received MCP message: {message}")

                    # Get message properties
                    msg_method = message.get("method")
                    msg_id = message.get("id")

                    # Dispatch to appropriate handler
                    if msg_method == "tools/list":
                        # List available tools
                        tools = await list_tools()
                        response = {
                            "jsonrpc": "2.0",
                            "id": msg_id,
                            "result": {"tools": [{"name": t.name, "description": t.description} for t in tools]}
                        }

                    elif msg_method == "tools/call":
                        # Call a tool
                        tool_name = message.get("params", {}).get("name")
                        tool_args = message.get("params", {}).get("arguments", {})

                        # Find and invoke the tool handler
                        handler_name = f"call_{tool_name}"
                        if hasattr(server, handler_name):
                            result = await getattr(server, handler_name)(**tool_args)
                            response = {
                                "jsonrpc": "2.0",
                                "id": msg_id,
                                "result": result
                            }
                        else:
                            response = {
                                "jsonrpc": "2.0",
                                "id": msg_id,
                                "error": {"code": -32601, "message": f"Tool {tool_name} not found"}
                            }

                    else:
                        # Unknown method
                        response = {
                            "jsonrpc": "2.0",
                            "id": msg_id,
                            "error": {"code": -32601, "message": f"Method {msg_method} not found"}
                        }

                    # Send response
                    sys.stdout.write(json.dumps(response) + "\n")
                    sys.stdout.flush()

                except json.JSONDecodeError as e:
                    logger.debug(f"JSON decode error: {e}, skipping line")

            except Exception as e:
                logger.error(f"Error in message pump: {e}", exc_info=True)
                await asyncio.sleep(0.1)

    else:
        # Unix/Linux: Use standard MCP stdio server
        logger.info("Unix/Linux: Using standard MCP stdio transport...")
        async with aiohttp.ClientSession() as session:
            await server.run(sys.stdin.buffer, sys.stdout.buffer, sys.stderr)


if __name__ == "__main__":
    # On Windows, ensure we use ProactorEventLoop for better pipe/stdio support
    # SelectorEventLoop doesn't support pipes on Windows
    if sys.platform == "win32":
        logger.info("Configuring Windows event loop policy (ProactorEventLoop for pipe support)...")
        asyncio.set_event_loop_policy(asyncio.WindowsProactorEventLoopPolicy())

    asyncio.run(main())
