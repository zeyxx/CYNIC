"""
CYNIC Hermes Task Runner — reads organ-local tasks and executes them.

The nerve connecting the meta-agent (Gemini) to the executor (Hermes X).
Reads agent-tasks/ directory every 10s. Claims tasks atomically. Executes. Reports to kernel.

Phase 1: stub executor (acknowledges and probes infrastructure).
Phase 2: real execution (browse X via CDP, capture, enrich).

Usage:
    python hermes_task_runner.py --organ-dir ~/.cynic/organs/hermes/x
    python hermes_task_runner.py --organ-dir ~/.cynic/organs/hermes/x --interval 10

Environment:
    CYNIC_REST_ADDR  — kernel address (optional, for reporting)
    CYNIC_API_KEY    — kernel auth token (optional)
    X_ORGAN_DIR      — organ directory (fallback if --organ-dir not provided)
"""

__version__ = "0.2.0"  # Switched from kernel polling to organ-local filesystem

import argparse
import json
import logging
import os
import signal
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

import requests

logger = logging.getLogger("hermes-task-runner")

# ── Config ──

KERNEL_ADDR = ""
API_KEY = ""
ORGAN_DIR = ""
TASK_KIND = "hermes"
POLL_INTERVAL = 10.0


def load_env():
    global KERNEL_ADDR, API_KEY, ORGAN_DIR
    KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "")
    API_KEY = os.environ.get("CYNIC_API_KEY", "")
    ORGAN_DIR = os.environ.get("X_ORGAN_DIR", str(Path.home() / ".cynic" / "organs" / "hermes" / "x"))

    if KERNEL_ADDR and API_KEY:
        return
    env_file = Path.home() / ".cynic-env"
    if not env_file.exists():
        return
    for line in env_file.read_text().splitlines():
        line = line.strip()
        if line.startswith("#") or "=" not in line:
            continue
        # Handle "export KEY=VALUE" format
        if line.startswith("export "):
            line = line[7:]
        key, _, val = line.partition("=")
        val = val.strip().strip('"').strip("'")
        if key.strip() == "CYNIC_REST_ADDR" and not KERNEL_ADDR:
            KERNEL_ADDR = val
        elif key.strip() == "CYNIC_API_KEY" and not API_KEY:
            API_KEY = val
        elif key.strip() == "X_ORGAN_DIR" and not ORGAN_DIR:
            ORGAN_DIR = val


def _kernel_url() -> str:
    addr = KERNEL_ADDR if KERNEL_ADDR.startswith("http") else f"http://{KERNEL_ADDR}"
    return addr


def _headers() -> dict:
    return {"Content-Type": "application/json", "Authorization": f"Bearer {API_KEY}"}


# ── Task polling (organ-local) ──

def poll_tasks(organ_dir: str, limit: int = 5) -> list[dict]:
    """Read pending tasks from organ-local agent-tasks/ directory.

    Returns list of unclaimed tasks (where .lock file doesn't exist).
    Task files should not be locked by another hermes instance.
    """
    tasks_dir = Path(organ_dir) / "agent-tasks"
    if not tasks_dir.exists():
        logger.debug("agent-tasks directory not found: %s", tasks_dir)
        return []

    tasks = []
    try:
        for task_file in sorted(tasks_dir.glob("*.json"))[:limit]:
            lock_file = tasks_dir / f".{task_file.stem}.lock"

            # Skip if already claimed by another process
            if lock_file.exists():
                continue

            try:
                with open(task_file) as f:
                    task = json.load(f)
                    # Store filename for later claim/release
                    task["_file"] = str(task_file)
                    # Normalize ID field for compatibility with execute_task
                    task["id"] = task.get("task_id", task.get("id", ""))
                    tasks.append(task)
            except json.JSONDecodeError as e:
                logger.warning("Failed to parse task file %s: %s", task_file, e)
                continue

    except OSError as e:
        logger.warning("Failed to scan agent-tasks directory: %s", e)

    return tasks


def claim_task(task: dict, organ_dir: str) -> bool:
    """Atomically claim a task by creating a .lock file.

    Returns True if lock was created successfully, False if already locked.
    """
    task_file = Path(task.get("_file", ""))
    if not task_file.exists():
        return False

    lock_file = task_file.parent / f".{task_file.stem}.lock"

    try:
        # Write lock file with PID and timestamp
        lock_file.write_text(json.dumps({
            "pid": os.getpid(),
            "claimed_at": datetime.now().isoformat() + "Z",
        }))
        logger.info("claimed task %s (lock: %s)", task.get("id"), lock_file.name)
        return True
    except OSError as e:
        logger.warning("Failed to claim task %s: %s", task.get("id"), e)
        return False


def release_task(task: dict, organ_dir: str, success: bool = True):
    """Release a task by deleting lock and optionally the task file.

    If success=True: delete both task file and lock (task completed).
    If success=False: delete only lock file (task will be retried).
    """
    task_file = Path(task.get("_file", ""))
    if not task_file.exists():
        return

    lock_file = task_file.parent / f".{task_file.stem}.lock"

    try:
        if success:
            # Task executed successfully, delete both files
            if task_file.exists():
                task_file.unlink()
                logger.info("deleted task file: %s", task_file.name)
            if lock_file.exists():
                lock_file.unlink()
                logger.info("deleted lock file: %s", lock_file.name)
        else:
            # Task failed, release lock for retry but keep task file
            if lock_file.exists():
                lock_file.unlink()
                logger.info("released lock (task file kept for retry): %s", task_file.name)
    except OSError as e:
        logger.warning("Failed to release task %s: %s", task.get("id"), e)


def complete_task(task_id: str, result: str | None = None, error: str | None = None) -> bool:
    """Report task completion to kernel."""
    payload: dict = {}
    if result is not None:
        payload["result"] = result
    if error is not None:
        payload["error"] = error

    try:
        resp = requests.post(
            f"{_kernel_url()}/agent-tasks/{task_id}/result",
            json=payload,
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


def post_heartbeat(kind: str, tasks_executed: int, tasks_failed: int):
    """Heartbeat — tells kernel this runner is alive."""
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
        requests.post(
            f"{_kernel_url()}/observe",
            json=payload,
            headers=_headers(),
            timeout=5,
        )
    except requests.RequestException:
        pass


# ── Task execution (Phase 2a: MCP integration) ──

def call_mcp_ts_introspect(node: str, service: str, port: int = 0) -> dict | None:
    """Call ts_introspect via tailscale-mcp subprocess.

    Returns ServiceIntrospection dict or None on error.
    MCP protocol requires persistent process with line-buffered I/O.
    """
    mcp_prog = os.environ.get("TAILSCALE_MCP", "/home/user/Bureau/tailscale-mcp/tailscale-mcp")
    if not os.path.isfile(mcp_prog):
        logger.warning("MCP binary not found at %s", mcp_prog)
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

        response = json.loads(response_line)

        # MCP returns: {jsonrpc, result:{content:[{type,text}]}, id}
        content = response.get("result", {}).get("content", [])
        if not content:
            logger.warning("MCP response missing content")
            return None

        text = content[0].get("text", "{}")
        introspection = json.loads(text)
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
    Fire-and-forget: wraps result as Event and POSTs to /event endpoint.
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
            f"{_kernel_url()}/event",
            json=event,
            headers=_headers(),
            timeout=5,
        )
        if resp.status_code != 200:
            logger.warning("probe_service %s:%s event post failed: %d", node, service, resp.status_code)
    except requests.RequestException as e:
        logger.warning("probe_service event post failed: %s", e)

    return probe_result


def execute_task(task: dict) -> tuple[str | None, str | None]:
    """Execute a task. Returns (result, error).

    Phase 1: stub — acknowledges the task, probes infrastructure, logs what would execute.
    Phase 2: will browse X via CDP based on task actions.
    """
    task_id = task.get("id", "?")
    objective = task.get("objective", task.get("content", ""))
    domain = task.get("domain", "unknown")
    actions = task.get("actions", [])
    context = task.get("context", "")

    logger.info("executing task %s: domain=%s objective=%s", task_id, domain, objective[:60])

    # Phase 1a: probe Hermes X infrastructure (fire-and-forget to /event)
    # Domain-specific probes for hermes-x tasks
    if domain.startswith("D") or domain == "hermes-x":
        # Probe mitmproxy/ingest daemon/browser
        probe_service("cynic-core", "hermes-x-ingest")
        probe_service("cynic-core", "mitmproxy")
        probe_service("cynic-core", "hermes-browser")

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


# ── Main loop ──

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

    logger.info("hermes_task_runner v%s starting — organ=%s interval=%.0fs",
                __version__, organ_dir, interval)

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

    logger.info("stopped — executed=%d failed=%d", tasks_executed, tasks_failed)


def main():
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )
    load_env()

    parser = argparse.ArgumentParser(description="Hermes Task Runner — reads organ-local tasks")
    parser.add_argument("--organ-dir", default=ORGAN_DIR, help="Organ directory containing agent-tasks/")
    parser.add_argument("--interval", type=float, default=POLL_INTERVAL, help="Poll interval (seconds)")
    args = parser.parse_args()

    organ_dir = args.organ_dir or ORGAN_DIR
    if not organ_dir:
        logger.error("Organ directory not set. Use --organ-dir or set X_ORGAN_DIR")
        sys.exit(1)

    organ_dir = str(Path(organ_dir).expanduser())
    if not Path(organ_dir).exists():
        logger.error("Organ directory not found: %s", organ_dir)
        sys.exit(1)

    logger.info("Organ directory: %s", organ_dir)
    run(organ_dir, args.interval)


if __name__ == "__main__":
    main()
