"""
CYNIC Hermes Task Runner — polls kernel for pending tasks and executes them.

The nerve connecting the brain (task dispatcher) to the organ (Hermes X).
Polls GET /agent-tasks?kind=hermes every 10s. Executes tasks, reports results.

Phase 1: stub executor (marks tasks complete with acknowledgment).
Phase 2: real execution (browse X via CDP, capture, enrich).

Usage:
    python hermes_task_runner.py
    python hermes_task_runner.py --kind hermes --interval 10

Environment:
    CYNIC_REST_ADDR  — kernel address
    CYNIC_API_KEY    — kernel auth token
"""

__version__ = "0.1.0"

import argparse
import json
import logging
import os
import signal
import sys
import time
from pathlib import Path

import requests

logger = logging.getLogger("hermes-task-runner")

# ── Config ──

KERNEL_ADDR = ""
API_KEY = ""
TASK_KIND = "hermes"
POLL_INTERVAL = 10.0


def load_env():
    global KERNEL_ADDR, API_KEY
    KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "")
    API_KEY = os.environ.get("CYNIC_API_KEY", "")
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


def _kernel_url() -> str:
    addr = KERNEL_ADDR if KERNEL_ADDR.startswith("http") else f"http://{KERNEL_ADDR}"
    return addr


def _headers() -> dict:
    return {"Content-Type": "application/json", "Authorization": f"Bearer {API_KEY}"}


# ── Task polling ──

def poll_tasks(kind: str, limit: int = 5) -> list[dict]:
    """Poll kernel for pending tasks."""
    try:
        resp = requests.get(
            f"{_kernel_url()}/agent-tasks",
            params={"kind": kind, "limit": limit},
            headers=_headers(),
            timeout=10,
        )
        if resp.status_code == 200:
            data = resp.json()
            return data.get("tasks", [])
        logger.warning("poll_tasks %d: %s", resp.status_code, resp.text[:100])
        return []
    except requests.RequestException as e:
        logger.warning("poll_tasks failed: %s", e)
        return []


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


# ── Task execution (Phase 1: stub) ──

def execute_task(task: dict) -> tuple[str | None, str | None]:
    """Execute a task. Returns (result, error).

    Phase 1: stub — acknowledges the task without real execution.
    Phase 2: will browse X via CDP based on task content.
    """
    task_id = task.get("id", "?")
    content = task.get("content", "")
    domain = task.get("domain", "")

    logger.info("executing task %s: domain=%s content=%s", task_id, domain, content[:80])

    # Phase 1: stub — report what we would do
    result = json.dumps({
        "phase": "stub",
        "acknowledged": True,
        "content_received": content[:200],
        "domain": domain,
        "message": "Task received by Hermes task runner. Real execution pending Phase 2 (CDP browse).",
    })
    return result, None


# ── Main loop ──

def run(kind: str, interval: float):
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

    logger.info("hermes_task_runner v%s starting — kind=%s interval=%.0fs kernel=%s",
                __version__, kind, interval, KERNEL_ADDR)

    while running:
        # Heartbeat every 60s
        now = time.time()
        if now - last_heartbeat >= 60.0:
            post_heartbeat(kind, tasks_executed, tasks_failed)
            last_heartbeat = now

        # Poll for tasks
        tasks = poll_tasks(kind)
        if tasks:
            logger.info("found %d pending task(s)", len(tasks))

        for task in tasks:
            task_id = task.get("id", "")
            if not task_id:
                continue

            result, error = execute_task(task)
            if complete_task(task_id, result=result, error=error):
                if error:
                    tasks_failed += 1
                else:
                    tasks_executed += 1
            else:
                tasks_failed += 1

        time.sleep(interval)

    logger.info("stopped — executed=%d failed=%d", tasks_executed, tasks_failed)


def main():
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(name)s: %(message)s",
        datefmt="%H:%M:%S",
    )
    load_env()
    if not KERNEL_ADDR:
        logger.error("CYNIC_REST_ADDR not set")
        sys.exit(1)

    parser = argparse.ArgumentParser(description="Hermes Task Runner")
    parser.add_argument("--kind", default=TASK_KIND, help="Task kind to poll")
    parser.add_argument("--interval", type=float, default=POLL_INTERVAL, help="Poll interval (seconds)")
    args = parser.parse_args()

    run(args.kind, args.interval)


if __name__ == "__main__":
    main()
