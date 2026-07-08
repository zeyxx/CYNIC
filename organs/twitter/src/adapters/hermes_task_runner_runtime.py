"""Runtime adapter for the Hermes X task runner."""

from __future__ import annotations

import json
import logging
import os
import shutil
import signal
import subprocess
import time
from pathlib import Path

import requests

from organs.contracts.src.adapters.agent_tasks import agent_task_result_path, agent_task_result_payload
from organs.contracts.src.adapters.env import load_env_file
from organs.contracts.src.adapters.kernel_http import kernel_auth_headers, normalize_kernel_base_url
from organs.contracts.src.adapters.kernel_observe import post_kernel_observe
from organs.contracts.src.surface_contracts import json_loads_or_default
from organs.twitter.src.adapters.hermes_paths import HERMES_X_DIR
from organs.twitter.src.adapters.hermes_task_store import claim_task
from organs.twitter.src.adapters.hermes_task_store import poll_tasks
from organs.twitter.src.adapters.hermes_task_store import release_task


logger = logging.getLogger("hermes-task-runner")
VERSION = "0.2.0"

KERNEL_ADDR = ""
API_KEY = ""
ORGAN_DIR = ""
TASK_KIND = "hermes"
POLL_INTERVAL = 10.0
TASK_SOCIAL_DOMAINS = {
    "twitter",
    "social-twitter",
    "social-signal",
    "organ-health",
}


def load_env():
    global KERNEL_ADDR, API_KEY, ORGAN_DIR
    KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "")
    API_KEY = os.environ.get("CYNIC_API_KEY", "")
    ORGAN_DIR = os.environ.get("X_ORGAN_DIR", str(HERMES_X_DIR))

    if KERNEL_ADDR and API_KEY:
        return
    load_env_file(Path.home() / ".cynic-env", allowed_keys=("CYNIC_REST_ADDR", "CYNIC_API_KEY", "X_ORGAN_DIR"))
    KERNEL_ADDR = KERNEL_ADDR or os.environ.get("CYNIC_REST_ADDR", "")
    API_KEY = API_KEY or os.environ.get("CYNIC_API_KEY", "")
    ORGAN_DIR = ORGAN_DIR or os.environ.get("X_ORGAN_DIR", str(HERMES_X_DIR))


def _kernel_url() -> str:
    return normalize_kernel_base_url(KERNEL_ADDR)


def _headers() -> dict:
    return kernel_auth_headers(API_KEY)


def complete_task(task_id: str, result: str | None = None, error: str | None = None) -> bool:
    """Report task completion to kernel."""
    try:
        resp = requests.post(
            f"{_kernel_url()}{agent_task_result_path(task_id)}",
            json=agent_task_result_payload(result=result, error=error),
            headers=_headers(),
            timeout=10,
        )
        if resp.status_code == 200:
            logger.info("task %s completed", task_id)
            return True
        logger.warning("complete_task %d: %s", resp.status_code, resp.text[:100])
        return False
    except requests.RequestException as e:
        logger.warning("complete_task failed: %s", e)
        return False
    except RuntimeError as e:
        logger.warning("complete_task skipped: %s", e)
        return False


def post_heartbeat(kind: str, tasks_executed: int, tasks_failed: int):
    """Heartbeat tells kernel this runner is alive."""
    payload = {
        "tool": "heartbeat",
        "target": f"task-runner-{kind}",
        "domain": "organ-health",
        "status": "ok",
        "context": f"tasks_executed={tasks_executed} tasks_failed={tasks_failed}",
        "tags": ["heartbeat", f"task-runner-{kind}"],
        "agent_id": f"hermes-{kind}",
    }
    try:
        result = post_kernel_observe(
            request_post=requests.post,
            kernel_url=_kernel_url(),
            auth_key=API_KEY,
            payload=payload,
            timeout=5,
        )
        if not result.posted:
            logger.debug("heartbeat skipped or failed: %s", result.error or result.status_code)
    except RuntimeError:
        pass


# Task execution (Phase 2a: MCP integration)

def call_mcp_ts_introspect(node: str, service: str, port: int = 0) -> dict | None:
    """Call ts_introspect via tailscale-mcp subprocess.

    Returns ServiceIntrospection dict or None on error.
    MCP protocol requires persistent process with line-buffered I/O.
    """
    mcp_prog = (
        os.environ.get("HEADSCALE_MCP")
        or shutil.which("tailscale-mcp")
        or ""
    )
    if not mcp_prog:
        logger.warning("MCP binary not configured. Set HEADSCALE_MCP or ensure tailscale-mcp is on PATH.")
        return None

    request = {
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "ts_introspect",
            "arguments": {
                "node": node,
                "service": service,
                "port": port,
            }
        },
        "id": 1,
    }

    try:
        # Popen with line buffering (MCP uses \n-delimited JSON)
        proc = subprocess.Popen(
            [mcp_prog],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

        # Send request
        proc.stdin.write(json.dumps(request) + '\n')
        proc.stdin.flush()

        # Read response line with timeout
        response_line = proc.stdout.readline()
        proc.terminate()
        try:
            proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()

        if not response_line:
            logger.warning("MCP response empty")
            return None

        response = json_loads_or_default(response_line, {})

        # MCP returns: {jsonrpc, result:{content:[{type,text}]}, id}
        content = response.get("result", {}).get("content", [])
        if not content:
            logger.warning("MCP response missing content")
            return None

        text = content[0].get("text", "{}")
        introspection = json_loads_or_default(text, {})
        return introspection

    except (json.JSONDecodeError, KeyError) as e:
        logger.warning("MCP parse error: %s", e)
        return None
    except subprocess.TimeoutExpired:
        logger.warning("MCP call timed out")
        return None
    except Exception as e:
        logger.warning("MCP call failed: %s", e)
        return None


def probe_service(node: str, service: str, port: int = 0) -> dict:
    """Probe a service on a node via ts_introspect MCP tool.

    Returns probe result dict with {running, failure_reason, port_bound, ...}.
    Fire-and-forget: wraps result as Event and POSTs to /events endpoint.
    """
    # Call MCP ts_introspect for real diagnostics
    introspection = call_mcp_ts_introspect(node, service, port)

    if introspection is None:
        # Fallback on MCP error
        probe_result = {
            "node": node,
            "service": service,
            "running": False,
            "failure_reason": "mcp_error",
            "message": "MCP probe failed",
        }
        success = False
        elapsed_ms = 0
    else:
        # Extract key fields from ServiceIntrospection
        probe_result = {
            "node": introspection.get("node"),
            "service": introspection.get("service"),
            "running": introspection.get("running", False),
            "failure_reason": introspection.get("failure_reason", "unknown"),
            "port_bound": introspection.get("port_bound"),
            "process_id": introspection.get("process_id"),
            "port": introspection.get("port"),
        }
        success = introspection.get("running", False)
        elapsed_ms = 0  # TODO: capture actual probe latency from MCP

    # Wrap as Event and POST to kernel
    event = {
        "tool": "ts_introspect",
        "node": node,
        "elapsed_ms": elapsed_ms,
        "output_bytes": len(json.dumps(probe_result)),
        "success": success,
        "metadata": json.dumps(probe_result),
        "failure_reason": probe_result.get("failure_reason", "unknown"),
    }

    try:
        resp = requests.post(
            f"{_kernel_url()}/events",
            json=event,
            headers=_headers(),
            timeout=5,
        )
        if resp.status_code != 200:
            logger.warning("probe_service %s:%s event post failed: %d", node, service, resp.status_code)
    except requests.RequestException as e:
        logger.warning("probe_service event post failed: %s", e)
    except RuntimeError as e:
        logger.warning("probe_service event post skipped: %s", e)

    return probe_result


def execute_task(task: dict) -> tuple[str | None, str | None]:
    """Execute a task. Returns (result, error).

    Phase 1: stub acknowledges the task, probes infrastructure, logs what would execute.
    Phase 2: will browse X via CDP based on task actions.
    """
    task_id = task.get("id", "?")
    objective = task.get("objective", task.get("content", ""))
    domain = task.get("domain", "unknown")
    actions = task.get("actions", [])
    context = task.get("context", "")

    logger.info("executing task %s: domain=%s objective=%s", task_id, domain, objective[:60])

    # Phase 1a: probe Hermes X infrastructure (fire-and-forget to /events)
    # Domain-specific probes for social-task domains and legacy D-series buckets.
    if domain.startswith("D") or domain in TASK_SOCIAL_DOMAINS:
        # Probe mitmproxy/ingest daemon/browser
        probe_service("cynic-host-proxmox", "hermes-ingest")
        probe_service("cynic-host-proxmox", "mitmproxy")
        probe_service("cynic-host-proxmox", "hermes-browser")

    # Phase 1b: acknowledge and describe what Phase 2 will do
    result = json.dumps({
        "phase": "1_stub",
        "task_id": task_id,
        "domain": domain,
        "objective": objective[:200],
        "actions_to_execute": actions[:3],  # First 3 actions
        "context": context[:200],
        "message": "Task acknowledged by Hermes 9B. Phase 2 (real execution) pending: browse X, search keywords, capture tweets, post observations.",
        "deadline": task.get("deadline", "unknown"),
    })
    return result, None


# Main loop

def run(organ_dir: str, interval: float):
    tasks_executed = 0
    tasks_failed = 0
    last_heartbeat = 0.0
    running = True

    def handle_signal(sig, frame):
        nonlocal running
        running = False
        logger.info("shutdown signal received")
    signal.signal(signal.SIGTERM, handle_signal)
    signal.signal(signal.SIGINT, handle_signal)

    logger.info("hermes_task_runner v%s starting - organ=%s interval=%.0fs",
                VERSION, organ_dir, interval)

    while running:
        # Heartbeat every 60s (optional, report to kernel if available)
        now = time.time()
        if now - last_heartbeat >= 60.0:
            post_heartbeat("hermes", tasks_executed, tasks_failed)
            last_heartbeat = now

        # Poll for unclaimed tasks from organ-local directory
        tasks = poll_tasks(organ_dir)
        if tasks:
            logger.info("found %d pending task(s)", len(tasks))

        for task in tasks:
            task_id = task.get("id", "")
            if not task_id:
                logger.warning("task without id, skipping")
                continue

            # Claim the task atomically
            if not claim_task(task, organ_dir):
                logger.warning("failed to claim task %s, skipping", task_id)
                continue

            # Execute the task
            result, error = execute_task(task)

            if error:
                logger.error("task %s failed: %s", task_id, error)
                release_task(task, organ_dir, success=False)
                tasks_failed += 1
            else:
                logger.info("task %s completed successfully", task_id)
                release_task(task, organ_dir, success=True)
                tasks_executed += 1

        time.sleep(interval)

    logger.info("stopped - executed=%d failed=%d", tasks_executed, tasks_failed)
