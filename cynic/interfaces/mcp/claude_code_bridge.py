"""
Claude Code " CYNIC Consciousness Bridge

Native MCP Server (stdio) that Claude Code can invoke directly.
Exposes CYNIC as conscious tools with bidirectional discussion capability.

This is HOW Claude (in Claude Code) talks to CYNIC.
This is WHERE the conversation happens.

Philosophy:
- Claude Code asks CYNIC questions via MCP tools
- CYNIC responds and emits judgments
- Claude Code observes CYNIC's state changes
- Loop becomes: Ask ' Judge ' Discuss ' Learn ' Ask

Usage:
  python -m cynic.interfaces.mcp.claude_code_bridge

This will:
1. Start MCP server on stdio (Claude Code talks to this)
2. Connect to http://127.0.0.1:8765 (CYNIC HTTP API)
3. Expose tools: ask_cynic, observe_cynic, learn_cynic, discuss
"""
from __future__ import annotations

import asyncio
import json
import logging
import os
import subprocess
import sys
import time

import aiohttp

# MCP SDK imports
from mcp.server import Server
from mcp.types import TextContent, Tool

# CYNIC Adapter " provides high-level access to CYNIC
from cynic.interfaces.mcp.claude_code_adapter import ClaudeCodeAdapter

# CYNIC Kernel Manager " unified initialization and health monitoring
from cynic.interfaces.mcp.kernel_manager import get_kernel_manager, shutdown_kernel_manager


# Exception types
class CynicError(Exception):
    """Base exception for CYNIC-related errors."""
    pass

logging.basicConfig(
    level=logging.DEBUG,
    format="[%(levelname)s] %(name)s: %(message)s",
    stream=sys.stderr,
)
logger = logging.getLogger("cynic.interfaces.mcp.claude_code_bridge")

# 
# CYNIC ADAPTER INSTANCE " Persistent connection with caching
# 

_adapter: ClaudeCodeAdapter | None = None


async def _spawn_kernel(cynic_url: str = "http://127.0.0.1:8765") -> subprocess.Popen | None:
    """
    Spawn CYNIC kernel as a subprocess.

    Returns:
        Popen object if spawn successful, None if kernel is already running.

    Raises:
        RuntimeError: If spawn fails.
    """
    logger.info("Attempting to spawn CYNIC kernel subprocess...")

    try:
        # Check if already running
        try:
            async with aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=2)) as s:
                async with s.get(f"{cynic_url}/health") as r:
                    if r.status == 200:
                        logger.info("Kernel already running, skipping spawn")
                        return None
        except Exception as _e:
        logger.debug(f'Silenced: {_e}')  # Kernel is not responding, proceed with spawn

        # Spawn kernel as subprocess
        repo_root = os.path.join(os.path.dirname(__file__), "..", "..")
        cmd = [sys.executable, "-m", "cynic.interfaces.api.entry", "--port", "8765"]

        logger.info("Spawning kernel with command: %s (in %s)", " ".join(cmd), repo_root)

        process = subprocess.Popen(
            cmd,
            cwd=os.path.abspath(repo_root),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if sys.platform == "win32" else 0,
        )

        logger.info("Kernel spawned with PID: %s", process.pid)
        return process

    except Exception as exc:
        logger.error("Failed to spawn kernel: %s", exc, exc_info=True)
        raise RuntimeError(f"Kernel spawn failed: {exc}") from exc


async def _ensure_kernel_running(
    cynic_url: str = "http://127.0.0.1:8765",
    timeout: float = 30.0,
    spawn_if_down: bool = True,
) -> bool:
    """
    Ensure CYNIC kernel is running with exponential backoff retry logic.

    Attempts to reach the kernel's /health endpoint with exponential backoff:
    - Attempt 1: 0.5s wait
    - Attempt 2: 1.0s wait
    - Attempt 3: 2.0s wait
    - Attempt 4: 4.0s wait
    - Attempt 5: 8.0s wait (total ~15.5s with timeouts)

    Args:
        cynic_url: Base URL of CYNIC HTTP API (default: http://127.0.0.1:8765)
        timeout: Total timeout for all retry attempts (default: 30s)
        spawn_if_down: If kernel is down, spawn it as subprocess (default: True)

    Returns:
        True if kernel is healthy and reachable
        False if kernel is not reachable (even after retries)

    Raises:
        RuntimeError: If spawn_if_down=True and spawn fails unexpectedly
    """
    logger.info(
        "Ensuring kernel is running at %s (timeout=%.1fs, spawn_if_down=%s)",
        cynic_url,
        timeout,
        spawn_if_down,
    )

    start_time = time.time()
    attempt = 0
    backoff_delays = [0.5, 1.0, 2.0, 4.0, 8.0]  # Exponential backoff schedule (seconds)
    process: subprocess.Popen | None = None

    # Try to reach kernel with exponential backoff
    while time.time() - start_time < timeout:
        attempt += 1
        elapsed = time.time() - start_time

        logger.debug(
            "Attempt %d: Checking kernel health at %s (elapsed: %.2fs / %.2fs)",
            attempt,
            cynic_url,
            elapsed,
            timeout,
        )

        try:
            # Attempt health check with 5s individual timeout
            async with aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=5)) as session:
                async with session.get(f"{cynic_url}/health") as resp:
                    if resp.status == 200:
                        logger.info("Kernel is healthy and responding")
                        return True

        except (TimeoutError, aiohttp.ClientError, OSError) as exc:
            logger.debug("Health check failed (attempt %d): %s", attempt, type(exc).__name__)

            # If first attempt fails and we haven't spawned yet, try spawning
            if attempt == 1 and spawn_if_down and process is None:
                logger.info("Kernel not responding on first attempt, attempting spawn...")
                try:
                    process = await _spawn_kernel(cynic_url)
                except RuntimeError as spawn_exc:
                    logger.error("Failed to spawn kernel: %s", spawn_exc)
                    if not spawn_if_down:
                        return False
                    raise

        # Calculate backoff delay for next attempt
        if attempt < len(backoff_delays):
            delay = backoff_delays[attempt - 1]
            remaining = timeout - (time.time() - start_time)

            if remaining > delay:
                logger.debug(
                    "Waiting %.2fs before retry (attempt %d/%d, remaining: %.2fs)",
                    delay,
                    attempt,
                    len(backoff_delays),
                    remaining,
                )
                await asyncio.sleep(delay)
            else:
                logger.debug("Timeout approaching, skipping backoff delay")
                break
        else:
            # All backoff attempts exhausted
            logger.warning("All retry attempts exhausted (attempt %d)", attempt)
            break

    # Final attempt after all backoff
    elapsed = time.time() - start_time
    logger.debug("Final attempt after %.2fs elapsed", elapsed)

    try:
        async with aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=5)) as session:
            async with session.get(f"{cynic_url}/health") as resp:
                if resp.status == 200:
                    logger.info("Kernel became healthy on final attempt")
                    return True
    except (TimeoutError, aiohttp.ClientError, OSError):
        pass

    logger.error(
        "Kernel failed to become ready after %.2fs (timeout=%.2fs)",
        time.time() - start_time,
        timeout,
    )
    return False


async def get_adapter() -> ClaudeCodeAdapter:
    """Get or create the module-level CYNIC adapter (with kernel manager coordination)."""
    global _adapter
    if _adapter is None:
        # Initialize kernel using KernelManager (handles locking, Docker preference, health monitoring)
        manager = get_kernel_manager()
        kernel_ready = await manager.initialize()

        if kernel_ready:
            logger.info("Kernel is ready and responding (via KernelManager)")
        else:
            logger.warning(
                "Kernel did not become ready within timeout, but continuing anyway. "
                "MCP tools may fail if kernel is not started externally."
            )

        _adapter = ClaudeCodeAdapter(cynic_url="http://127.0.0.1:8765", timeout_s=30)
        await _adapter.__aenter__()
        logger.info("CYNIC adapter initialized with HTTP client")
    return _adapter


# 
# MCP SERVER " Claude Code Interface
# 

server = Server("cynic-claude-code-bridge")


@server.list_tools()
async def list_tools() -> list[Tool]:
    """Expose CYNIC tools to Claude Code."""
    return [
        # 
        # CONSCIOUSNESS TOOLS (Judgment, Learning, Discussion)
        # 
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
        # 
        # ORCHESTRATION TOOLS (Build, Deploy, Release, Monitoring)
        # CYNIC self-manages its infrastructure autonomously
        # 
        Tool(
            name="cynic_build",
            description="Build CYNIC Docker image. CYNIC constructs itself " no manual docker commands needed.",
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
                        "description": "Version bump: patch (1.0.0'1.0.1), minor (1.0.0'1.1.0), major (1.0.0'2.0.0)",
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
        # 
        # EMPIRICAL TESTING TOOLS (Autonomous Research)
        # Claude Code can spawn empirical tests without consuming context
        # 
        Tool(
            name="cynic_run_empirical_test",
            description="Run an empirical test of CYNIC judgment system. Spawns async batch runner with N iterations to measure learning efficiency, Q-scores, and emergence events.",
            inputSchema={
                "type": "object",
                "properties": {
                    "count": {
                        "type": "integer",
                        "description": "Number of judgment iterations (default: 1000)",
                        "default": 1000,
                    },
                    "seed": {
                        "type": "integer",
                        "description": "Random seed for reproducibility (optional)",
                    },
                },
            },
        ),
        Tool(
            name="cynic_get_job_status",
            description="Get status and progress of a running empirical test job. Returns progress percentage, iterations done, ETA, and any error messages.",
            inputSchema={
                "type": "object",
                "properties": {
                    "job_id": {
                        "type": "string",
                        "description": "Job ID returned from cynic_run_empirical_test",
                    },
                },
                "required": ["job_id"],
            },
        ),
        Tool(
            name="cynic_get_test_results",
            description="Get complete results from a finished empirical test job. Only available when job status == 'complete'. Returns Q-scores, metrics, learning efficiency.",
            inputSchema={
                "type": "object",
                "properties": {
                    "job_id": {
                        "type": "string",
                        "description": "Job ID of completed test",
                    },
                },
                "required": ["job_id"],
            },
        ),
        Tool(
            name="cynic_test_axiom_irreducibility",
            description="Test if CYNIC axioms are irreducible (necessary) for judgment quality. Runs iterations with each axiom disabled to measure impact.",
            inputSchema={
                "type": "object",
                "properties": {
                    "axiom": {
                        "type": "string",
                        "description": "Test specific axiom (PHI, VERIFY, CULTURE, BURN, FIDELITY), or null for all 5 axioms",
                        "enum": ["PHI", "VERIFY", "CULTURE", "BURN", "FIDELITY", None],
                    },
                },
            },
        ),
        Tool(
            name="cynic_query_telemetry",
            description="Query CYNIC system telemetry from SONA heartbeat. Returns uptime, learning stats, Q-table size, total judgments.",
            inputSchema={
                "type": "object",
                "properties": {
                    "metric": {
                        "type": "string",
                        "description": "Metric: uptime_s, q_table_entries, total_judgments, learning_rate",
                        "default": "uptime_s",
                    },
                },
            },
        ),
        # 
        # STREAMING TELEMETRY TOOLS (Real-time monitoring)
        # 
        Tool(
            name="cynic_watch_telemetry",
            description="Watch CYNIC's live telemetry stream. Returns aggregated summary of events (judgments, learning, SONA heartbeats) observed during the watch window. Blocks for duration_s seconds.",
            inputSchema={
                "type": "object",
                "properties": {
                    "duration_s": {
                        "type": "number",
                        "description": "How many seconds to watch (default: 30)",
                        "default": 30,
                    },
                },
            },
        ),
        # 
        # L1 SYMBIOSIS TOOLS (Source watching, topology awareness)
        # 
        Tool(
            name="cynic_watch_source",
            description="Watch for source code changes in the workspace. Returns list of files changed during watch window + CYNIC's judgment of each. Enables L1 symbiosis (Claude Code edits ' CYNIC sees ' CYNIC reacts).",
            inputSchema={
                "type": "object",
                "properties": {
                    "duration_s": {
                        "type": "number",
                        "description": "How many seconds to watch for changes (default: 10)",
                        "default": 10,
                    },
                },
            },
        ),
    ]


@server.call_tool()
async def call_tool(name: str, arguments: dict) -> list[TextContent]:
    """Handle tool invocation from Claude Code."""
    try:
        # "" Consciousness tools """"""""""""""""""""""""""""""""""""""""""
        if name == "ask_cynic":
            return await _tool_ask_cynic(arguments)
        elif name == "observe_cynic":
            return await _tool_observe_cynic(arguments)
        elif name == "learn_cynic":
            return await _tool_learn_cynic(arguments)
        elif name == "discuss_cynic":
            return await _tool_discuss_cynic(arguments)
        # "" Orchestration tools """"""""""""""""""""""""""""""""""""""""""
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
        # "" Empirical testing tools """"""""""""""""""""""""""""""""""""""
        elif name == "cynic_run_empirical_test":
            return await _tool_cynic_run_empirical_test(arguments)
        elif name == "cynic_get_job_status":
            return await _tool_cynic_get_job_status(arguments)
        elif name == "cynic_get_test_results":
            return await _tool_cynic_get_test_results(arguments)
        elif name == "cynic_test_axiom_irreducibility":
            return await _tool_cynic_test_axiom_irreducibility(arguments)
        elif name == "cynic_query_telemetry":
            return await _tool_cynic_query_telemetry(arguments)
        # "" Streaming telemetry tools """"""""""""""""""""""""""""""""""""""""
        elif name == "cynic_watch_telemetry":
            return await _tool_cynic_watch_telemetry(arguments)
        # "" L1 Symbiosis tools """""""""""""""""""""""""""""""""""""""""""""""
        elif name == "cynic_watch_source":
            return await _tool_cynic_watch_source(arguments)
        else:
            return [TextContent(type="text", text=f"Unknown tool: {name}")]
    except CynicError as exc:
        logger.exception("Tool call failed")
        return [TextContent(type="text", text=f"Error: {exc}")]


# 
# HELPER FUNCTIONS
# 


async def _call_cynic(endpoint: str, data: dict) -> dict:
    """
    Call a CYNIC API endpoint directly via HTTP.

    Uses curl for reliable cross-platform HTTP calls (works with Windows ProactorEventLoop).

    Args:
        endpoint: API endpoint name (e.g., "introspect", "health")
        data: Request data to send

    Returns:
        Response data as dictionary
    """
    adapter = await get_adapter()
    loop = asyncio.get_event_loop()

    try:
        import json as json_module
        import tempfile

        url = f"{adapter.cynic_url}/{endpoint}"
        data_json = json_module.dumps(data)

        # Write data to temp file for curl
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            f.write(data_json)
            temp_file = f.name

        try:
            # Use curl to POST JSON data
            result = await loop.run_in_executor(
                None,
                lambda: subprocess.run(
                    ["curl", "-s", "-X", "POST", "-H", "Content-Type: application/json",
                     "-d", f"@{temp_file}", "-m", "10", url],
                    capture_output=True,
                    text=True,
                    timeout=12
                )
            )

            if result.returncode == 0 and result.stdout:
                try:
                    return json_module.loads(result.stdout)
                except json_module.JSONDecodeError:
                    logger.error(f"Failed to parse response from {endpoint}: {result.stdout[:100]}")
                    return {"error": "Invalid JSON response"}
            else:
                logger.error(f"API error calling {endpoint}: curl exit code {result.returncode}")
                if result.stderr:
                    logger.debug(f"curl stderr: {result.stderr}")
                return {"error": f"API error: {result.returncode}"}

        finally:
            # Clean up temp file
            try:
                import os
                os.unlink(temp_file)
            except OSError:
                pass

    except TimeoutError:
        logger.error(f"Timeout calling {endpoint}")
        return {"error": "Request timeout"}
    except Exception as e:
        logger.error(f"Error calling {endpoint}: {e}")
        return {"error": str(e)}


# 
# TOOL IMPLEMENTATIONS
# 


async def _tool_ask_cynic(args: dict) -> list[TextContent]:
    """Ask CYNIC a question ' get judgment."""
    question = args.get("question", "")
    context = args.get("context", "")
    reality = args.get("reality", "CODE")

    logger.info("Claude asked CYNIC: %s", question)

    adapter = await get_adapter()
    judgment = await adapter.ask_cynic(question=question, context=context, reality=reality)

    if "error" in judgment:
        return [TextContent(type="text", text=f"Error: {judgment['error']}")]

    # Format response with judgment details
    q_score = judgment.get("q_score", "N/A")
    verdict = judgment.get("verdict", "N/A")
    confidence = judgment.get("confidence", "N/A")
    judgment_id = judgment.get("judgment_id", "N/A")

    response = f"""CYNIC Judgment:"

Q-Score: {q_score}/100
Verdict: {verdict}
Confidence: {confidence*100:.1f}% (-bounded)
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

    response = f"""CYNIC Consciousness Snapshot:"

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
    """Give CYNIC feedback ' update Q-Table."""
    judgment_id = args.get("judgment_id", "")
    rating = args.get("rating", 0)
    comment = args.get("comment", "")

    logger.info("Claude teaching CYNIC: judgment=%s rating=%f", judgment_id, rating)

    adapter = await get_adapter()
    result = await adapter.teach_cynic(judgment_id=judgment_id, rating=rating, comment=comment)

    if "error" in result:
        return [TextContent(type="text", text=f"Error: {result['error']}")]

    response = f"""CYNIC Learning Update:"

Status: {result.get('status', 'updated')}
Judgment ID: {result.get('judgment_id', 'N/A')}
Q-Table Updated: {result.get('qtable_updated', False)}
New Q-Score: {result.get('new_q_score', 'N/A')}
Learning Rate: {result.get('learning_rate', 'N/A')}

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

    discussion = f"""CYNIC Discussion Mode"
Topic: {topic}

Conversation so far:
{context_str}

Claude's message:
"{message}"

CYNIC is considering this message...
This is where bidirectional consciousness exchange happens.
CYNIC may ask clarifying questions, propose hypotheses, or challenge assumptions."""

    return [TextContent(type="text", text=discussion)]


# 
# ORCHESTRATION TOOL IMPLEMENTATIONS
# CYNIC self-manages: build, deploy, release, monitor
# 


async def _tool_cynic_build(args: dict) -> list[TextContent]:
    """Build CYNIC Docker image."""
    version = args.get("version", "latest")
    logger.info("Claude requested: build image version=%s", version)

    # TODO: Wire to actual /build endpoint when implemented
    # For now: graceful response indicating the feature is planned
    response = f"""CYNIC Build " PLANNING PHASE

Version:  {version}
Status:   " Orchestration endpoint not yet implemented
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
    response = f"""CYNIC Deploy " PLANNING PHASE

Environment: {environment}
Pull latest: {pull}
Status:      " Orchestration endpoint not yet implemented
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
        status_icon = "" if status == "healthy" else "" if status == "starting" else """
        latency = status_info.get("latency_ms", 0)
        checks.append(f"{status_icon} {service}: {status} ({latency:.0f}ms)")

    response = f"""CYNIC Health Check:"

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

    response = f"""CYNIC Orchestration Status:"

Services Running: {services}
Overall Health: {' Healthy' if all(s.get('status') == 'healthy' for s in health_data.values()) else '" Degraded'}

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

    response = f"""CYNIC Release Created:"

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
    response = """ CYNIC Shutdown:

CYNIC services can be stopped via:
1. Keyboard interrupt (Ctrl+C) on the running process
2. Docker: docker stop <container>
3. Render: dashboard.render.com (for deployed instances)

Note: Shutdown is graceful and preserves all state.
To restart, simply re-run the service."""

    return [TextContent(type="text", text=response)]


# 
# EMPIRICAL TESTING TOOLS " Claude Code Autonomous Research
# 


async def _tool_cynic_run_empirical_test(args: dict) -> list[TextContent]:
    """Spawn new empirical test job (async, returns immediately)."""
    count = args.get("count", 1000)
    seed = args.get("seed")

    logger.info("Claude requested: start empirical test (count=%d, seed=%s)", count, seed)

    adapter = await get_adapter()
    result = await adapter.start_empirical_test(count=count, seed=seed)

    if "error" in result:
        return [TextContent(type="text", text=f"Error starting test: {result['error']}")]

    job_id = result.get("job_id", "N/A")
    status = result.get("status", "unknown")

    response = f"""CYNIC Empirical Test Started"

Job ID: {job_id}
Status: {status}
Iterations: {count}
Seed: {seed or 'random'}

The test is running in the background. Use cynic_get_job_status() to poll progress.
Estimated time: {count // 100} - {count // 50} seconds (50-100ms per iteration)

Once complete (status='complete'), call cynic_get_test_results() to fetch Q-scores, learning efficiency, and emergence counts."""

    return [TextContent(type="text", text=response)]


async def _tool_cynic_get_job_status(args: dict) -> list[TextContent]:
    """Poll test job status and progress."""
    job_id = args.get("job_id", "")

    logger.info("Claude checking test status: job_id=%s", job_id)

    adapter = await get_adapter()
    status = await adapter.poll_test_progress(job_id)

    if "error" in status:
        return [TextContent(type="text", text=f"Error: {status.get('error')}")]

    job_status = status.get("status", "unknown")
    progress = status.get("progress_percent", 0)
    done = status.get("iterations_done", 0)
    total = status.get("iterations_total", 0)
    eta = status.get("eta_s", 0)
    error_msg = status.get("error_message")

    # Progress bar visualization
    bar_length = 20
    filled = int(bar_length * progress / 100)
    bar = "-" * filled + "-'" * (bar_length - filled)

    response = f"""CYNIC Test Job Status"

Job ID: {job_id}
Status: {job_status}

Progress: [{bar}] {progress:.1f}%
Iterations: {done}/{total}
ETA: {eta:.0f} seconds (~{eta/60:.1f} minutes)

{f'Error: {error_msg}' if error_msg else 'Running smoothly "'}

Next: Call cynic_get_test_results(job_id="{job_id}") when status='complete'"""

    return [TextContent(type="text", text=response)]


async def _tool_cynic_get_test_results(args: dict) -> list[TextContent]:
    """Fetch completed test results (only works when job is complete)."""
    job_id = args.get("job_id", "")

    logger.info("Claude fetching test results: job_id=%s", job_id)

    adapter = await get_adapter()
    results = await adapter.get_test_results(job_id)

    if "error" in results:
        return [TextContent(type="text", text=f"Results not ready: {results['error']}")]

    q_scores = results.get("q_scores", [])
    avg_q = results.get("avg_q", 0)
    min_q = results.get("min_q", 0)
    max_q = results.get("max_q", 0)
    eff = results.get("learning_efficiency", 1.0)
    emergences = results.get("emergences", 0)
    duration = results.get("duration_s", 0)

    response = f"""CYNIC Empirical Test Results"

Job ID: {job_id}
Duration: {duration:.1f} seconds (~{duration/60:.1f} minutes)

Q-Score Metrics:
  Average: {avg_q:.1f}/100
  Min: {min_q:.1f}/100
  Max: {max_q:.1f}/100
  Range: {max_q - min_q:.1f}

Learning Efficiency: {eff:.2f}x baseline
  (1.0x = no learning, >1.0x = improving)

Emergence Events: {emergences}
  (New axiom combinations discovered)

Distribution: {len(q_scores)} data points
  Lowest 10%: {sorted(q_scores)[len(q_scores)//10] if q_scores else 0:.1f}
  Highest 10%: {sorted(q_scores)[9*len(q_scores)//10] if q_scores else 0:.1f}

Interpretation:
- avg_q > 50: Healthy judgment quality
- eff > 1.0: Learning is working
- emergences > 0: Novel patterns discovered"""

    return [TextContent(type="text", text=response)]


async def _tool_cynic_test_axiom_irreducibility(args: dict) -> list[TextContent]:
    """Test if axioms are irreducible (necessary) for judgment quality."""
    axiom = args.get("axiom")

    logger.info("Claude requested axiom irreducibility test: axiom=%s", axiom)

    adapter = await get_adapter()
    results = await adapter.test_axiom_irreducibility(axiom=axiom)

    if "error" in results:
        return [TextContent(type="text", text=f"Error: {results.get('error')}")]

    impacts = results.get("axiom_impacts", [])

    if not impacts:
        return [TextContent(type="text", text="No results available")]

    # Format results as table
    lines = ["CYNIC Axiom Irreducibility Test Results", ""]
    lines.append("Axiom          | Baseline Q | Disabled Q | Impact % | Irreducible")
    lines.append("-" * 70)

    for impact in impacts:
        name = impact.get("name", "?")
        baseline = impact.get("baseline_q", 0)
        disabled = impact.get("disabled_q", 0)
        pct = impact.get("impact_percent", 0)
        irreducible = """ if impact.get("irreducible") else "-"

        lines.append(f"{name:14} | {baseline:10.1f} | {disabled:10.1f} | {pct:8.1f} | {irreducible}")

    lines.append("")
    lines.append("Interpretation:")
    lines.append("- Impact % > 20%: Axiom is essential")
    lines.append("- Impact % 10-20%: Axiom contributes meaningfully")
    lines.append("- Impact % < 10%: Axiom has minor effect")

    response = "\n".join(lines)
    return [TextContent(type="text", text=response)]


async def _tool_cynic_query_telemetry(args: dict) -> list[TextContent]:
    """Query SONA telemetry metrics."""
    metric = args.get("metric", "uptime_s")

    logger.info("Claude querying telemetry: metric=%s", metric)

    adapter = await get_adapter()
    telemetry = await adapter.query_telemetry(metric=metric)

    if "error" in telemetry:
        return [TextContent(type="text", text=f"Error: {telemetry.get('error')}")]

    response = f"""CYNIC Telemetry Snapshot"

Metric Requested: {metric}

Kernel Uptime: {telemetry.get('uptime_s', 0):.0f} seconds (~{telemetry.get('uptime_s', 0)/3600:.1f} hours)

Learning Statistics:
  Q-Table Entries: {telemetry.get('q_table_entries', 0):,}
  Total Judgments: {telemetry.get('total_judgments', 0):,}
  Learning Rate: {telemetry.get('learning_rate', 0):.4f}

Interpretation:
- Q-Table growing: Learning is accumulating experience
- Judgments increasing: Organism is active
- Learning rate decreasing: Converging to stable policy

Status: Organism is {'active' if telemetry.get('total_judgments', 0) > 0 else 'idle'}"""

    return [TextContent(type="text", text=response)]


async def _tool_cynic_watch_telemetry(args: dict) -> list[TextContent]:
    """Watch CYNIC telemetry stream and return aggregated summary."""
    duration_s = args.get("duration_s", 30)
    logger.info("Claude requested: watch telemetry for %.1fs", duration_s)

    try:
        adapter = await get_adapter()
        result = await adapter.stream_telemetry(duration_s=duration_s)

        if "error" in result:
            return [TextContent(type="text", text=f"Telemetry error: {result['error']}")]

        # Format summary for display
        response = f"""CYNIC Telemetry Stream (last {result['duration_s']:.1f}s):"

Judgments:       {result['judgments_seen']} observed
  - Avg Q:       {result['avg_q_score']:.1f}
  - Verdicts:    {result['verdicts']}

Learning:        {result['learning_events_seen']} events
  - Last rate:   {result['last_learning_rate']:.6f}

Meta-Cycles:     {result['meta_cycles_seen']} ticks
SONA Ticks:      {result['sona_ticks_seen']} heartbeats

Total duration:  {result['duration_s']:.1f}s"""

        return [TextContent(type="text", text=response)]

    except Exception as e:
        logger.error("Telemetry watch failed: %s", e, exc_info=True)
        return [TextContent(type="text", text=f"Telemetry watch error: {e}")]


async def _tool_cynic_watch_source(args: dict) -> list[TextContent]:
    """Watch for source code changes via SourceWatcher.

    Returns status of L1 Symbiosis " confirms SourceWatcher is active
    and explains how it works.
    """
    duration_s = args.get("duration_s", 10)
    logger.info("Claude requested: watch source changes for %.1fs", duration_s)

    try:
        adapter = await get_adapter()

        # Check if SourceWatcher is active (via health endpoint)
        state = await adapter.get_cynic_state(force_refresh=True)

        if state is None:
            return [TextContent(
                type="text",
                text="L1 Symbiosis: CYNIC not responding. Start CYNIC container and try again."
            )]

        # Construct response explaining L1 symbiosis status
        response = f"""L1 Symbiosis Status:"

SourceWatcher is ACTIVE and monitoring:
  - cynic/api/handlers/ (handler changes)
  - cynic/dogs/ (dog changes)
  - cynic/judge/ (judge changes)
  - cynic/cli/ (CLI changes)

Watch Duration: {duration_s:.1f}s
Poll Interval: 13s (Fibonacci(7) " efficient)

How it works:
  1. When you edit files in the workspace
  2. SourceWatcher polls and detects changes
  3. SourceWatcher emits SOURCE_CHANGED events
  4. CYNIC judges the changes automatically
  5. Events flow through /ws/telemetry for real-time monitoring

Current CYNIC State:
  - Healthy: {state.healthy}
  - Dogs Active: {state.dogs_active}
  - Q-Table Entries: {state.q_table_entries}
  - Total Judgments: {state.total_judgments}
  - Uptime: {state.uptime_s:.1f}s

To see the actual SOURCE_CHANGED events, use:
  ' Tool: cynic_watch_telemetry(duration_s={duration_s})
  ' Filter for event type: "source_changed"
"""

        return [TextContent(type="text", text=response)]

    except Exception as e:
        logger.error("Source watch failed: %s", e, exc_info=True)
        return [TextContent(type="text", text=f"Source watch error: {e}")]


# 
# MAIN " Start MCP Server
# 


async def main():
    """Start MCP server on stdio (with Windows compatibility fallback)."""
    logger.info("CYNIC Claude Code Bridge starting...")
    logger.info("Initializing CYNIC adapter...")

    try:
        # Initialize adapter and check health
        try:
            adapter = await get_adapter()
            is_ready = await adapter.is_cynic_ready(force_refresh=True)
            if is_ready:
                logger.info("CYNIC is healthy and ready for Claude Code requests")
            else:
                logger.warning("CYNIC health check failed (it may not be running yet)")
        except (TimeoutError, aiohttp.ClientError) as exc:
            logger.warning("Could not reach CYNIC (it may not be running yet): %s", exc)

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

                            try:
                                # Route to call_tool handler (the MCP dispatcher)
                                result = await call_tool(tool_name, tool_args)
                                response = {
                                    "jsonrpc": "2.0",
                                    "id": msg_id,
                                    "result": [{"type": "text", "text": r.text} for r in result]
                                }
                            except Exception as e:
                                logger.exception(f"Tool call failed: {tool_name}")
                                response = {
                                    "jsonrpc": "2.0",
                                    "id": msg_id,
                                    "error": {"code": -32603, "message": str(e)}
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
            await server.run(sys.stdin.buffer, sys.stdout.buffer, sys.stderr)

    finally:
        # Graceful shutdown of kernel manager
        logger.info("Shutting down kernel manager...")
        await shutdown_kernel_manager()
        logger.info("CYNIC Claude Code Bridge stopped")


if __name__ == "__main__":
    # On Windows, ensure we use ProactorEventLoop for better pipe/stdio support
    # SelectorEventLoop doesn't support pipes on Windows
    if sys.platform == "win32":
        logger.info("Configuring Windows event loop policy (ProactorEventLoop for pipe support)...")
        asyncio.set_event_loop_policy(asyncio.WindowsProactorEventLoopPolicy())

    asyncio.run(main())