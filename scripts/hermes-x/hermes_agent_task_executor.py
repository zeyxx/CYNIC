#!/usr/bin/env python3
"""
Hermes Agent Task Executor — Phase 2 real execution (kernel-integrated).

Unlike hermes_task_runner.py (Phase 1 stub), this spawns a Hermes Agent instance
to actually execute tasks: browse X, analyze signals, post observations.

V0.4.0 (2026-05-02): Integrated with kernel /agent-tasks API.
  Before: polled ORGAN_DIR/agent-tasks/ (FS local)
  Now: polls kernel /agent-tasks via REST (K15 seam 3 complete)

Workflow:
  1. Poll kernel /agent-tasks?status=pending
  2. Claim task (local lock file, kernel tracks via API)
  3. Spawn `hermes chat` with task prompt + SKILL.md context
  4. Agent executes actions via its tools
  5. Capture observations posted to /observe
  6. Release task when done (delete local lock, kernel marks done)

Usage:
    python3 hermes_agent_task_executor.py --organ-dir ~/.cynic/organs/hermes/x --interval 30

Environment:
    CYNIC_REST_ADDR — kernel endpoint (default: env or <TAILSCALE_CORE>:3030)
    CYNIC_API_KEY   — kernel auth token
    X_ORGAN_DIR     — organ directory (fallback)
"""

__version__ = "0.4.0"  # Kernel-integrated variant

import argparse
import json
import logging
import os
import signal
import subprocess
import sys
import time
import requests
from datetime import datetime
from pathlib import Path

logger = logging.getLogger("hermes-task-executor")

ORGAN_DIR = ""
POLL_INTERVAL = 30.0
KERNEL_ADDR = ""
KERNEL_API_KEY = ""


def load_env():
    global ORGAN_DIR, KERNEL_ADDR, KERNEL_API_KEY
    ORGAN_DIR = os.environ.get("X_ORGAN_DIR", str(Path.home() / ".cynic" / "organs" / "hermes" / "x"))
    KERNEL_ADDR = os.environ.get("CYNIC_REST_ADDR", "http://<TAILSCALE_CORE>:3030")
    KERNEL_API_KEY = os.environ.get("CYNIC_API_KEY", "")


def poll_tasks(organ_dir: str, limit: int = 1) -> list[dict]:
    """Fetch pending tasks from kernel /agent-tasks API (K15 seam 3).

    Falls back to local FS if kernel unavailable (FS is for backward compat).
    """
    # Try kernel API first
    if KERNEL_ADDR and KERNEL_API_KEY:
        try:
            headers = {"Authorization": f"Bearer {KERNEL_API_KEY}"}
            url = f"{KERNEL_ADDR}/agent-tasks?status=pending&limit={limit}"
            response = requests.get(url, headers=headers, timeout=5)

            if response.status_code == 200:
                kernel_tasks = response.json()
                # Convert kernel format to executor format
                tasks = []
                if isinstance(kernel_tasks, dict) and "tasks" in kernel_tasks:
                    kernel_tasks = kernel_tasks["tasks"]
                elif isinstance(kernel_tasks, int):
                    # API returned count, not tasks
                    logger.debug("kernel /agent-tasks returned count=%d, polling fallback", kernel_tasks)
                    kernel_tasks = []

                for t in kernel_tasks:
                    if isinstance(t, dict):
                        t["_source"] = "kernel"  # Mark source for later
                        t["id"] = t.get("agent_id", t.get("id", ""))
                        tasks.append(t)

                if tasks:
                    logger.info("polled kernel: found %d pending task(s)", len(tasks))
                    return tasks
        except requests.RequestException as e:
            logger.warning("kernel /agent-tasks fetch failed: %s, falling back to local FS", e)

    # Fallback: read from local FS (for backward compat during transition)
    tasks_dir = Path(organ_dir) / "agent-tasks"
    if not tasks_dir.exists():
        logger.debug("agent-tasks directory not found: %s (no local or kernel tasks)", tasks_dir)
        return []

    tasks = []
    try:
        for task_file in sorted(tasks_dir.glob("*.json"))[:limit]:
            lock_file = tasks_dir / f".{task_file.stem}.lock"

            # Skip if already claimed
            if lock_file.exists():
                continue

            try:
                with open(task_file) as f:
                    task = json.load(f)
                    task["_file"] = str(task_file)
                    task["_source"] = "local"
                    task["id"] = task.get("task_id", task.get("id", ""))
                    tasks.append(task)
            except json.JSONDecodeError as e:
                logger.warning("Failed to parse task file %s: %s", task_file, e)
                continue

    except OSError as e:
        logger.warning("Failed to scan agent-tasks directory: %s", e)

    return tasks


def claim_task(task: dict, organ_dir: str) -> bool:
    """Claim task by creating lock file (works for kernel and local sources)."""
    task_id = task.get("id", "")
    source = task.get("_source", "local")

    # For kernel tasks, claim is implicit (just track local lock for coordination)
    # For local tasks, create lock file as before
    if source == "kernel":
        # Create local lock file for coordination even though kernel tracks it
        lock_file = Path(organ_dir) / "agent-tasks" / f".{task_id}.lock"
        try:
            lock_file.parent.mkdir(parents=True, exist_ok=True)
            lock_file.write_text(json.dumps({
                "pid": os.getpid(),
                "claimed_at": datetime.now().isoformat() + "Z",
                "source": "kernel",
            }))
            logger.info("claimed kernel task %s (local coordination lock)", task_id)
            return True
        except OSError as e:
            logger.warning("Failed to create coordination lock for task %s: %s", task_id, e)
            # Don't fail here; we can still execute
            return True
    else:
        # Local FS task
        task_file = Path(task.get("_file", ""))
        if not task_file.exists():
            logger.warning("task file not found: %s", task_file)
            return False

        lock_file = task_file.parent / f".{task_file.stem}.lock"
        try:
            lock_file.write_text(json.dumps({
                "pid": os.getpid(),
                "claimed_at": datetime.now().isoformat() + "Z",
            }))
            logger.info("claimed local task %s", task_id)
            return True
        except OSError as e:
            logger.warning("Failed to claim task %s: %s", task_id, e)
            return False


def release_task(task: dict, organ_dir: str, success: bool = True):
    """Release task by deleting lock and optionally task file.

    For kernel tasks: delete coordination lock, kernel tracks completion via API.
    For local tasks: delete lock and optionally task file (as before).
    """
    task_id = task.get("id", "")
    source = task.get("_source", "local")

    if source == "kernel":
        # For kernel tasks, just clean up coordination lock
        lock_file = Path(organ_dir) / "agent-tasks" / f".{task_id}.lock"
        try:
            if lock_file.exists():
                lock_file.unlink()
                logger.info("released coordination lock for kernel task: %s", task_id)
        except OSError as e:
            logger.warning("Failed to release coordination lock: %s", e)
        # Kernel will handle task cleanup on next cycle
    else:
        # Local FS task (backward compat)
        task_file = Path(task.get("_file", ""))
        if not task_file.exists():
            return

        lock_file = task_file.parent / f".{task_file.stem}.lock"

        try:
            if success:
                if task_file.exists():
                    task_file.unlink()
                    logger.info("deleted task file: %s", task_file.name)
                if lock_file.exists():
                    lock_file.unlink()
                    logger.info("deleted lock file: %s", lock_file.name)
            else:
                if lock_file.exists():
                    lock_file.unlink()
                    logger.info("released lock for retry: %s", task_file.name)
        except OSError as e:
            logger.warning("Failed to release task: %s", e)


def load_skill(organ_dir: str) -> str:
    """Load SKILL.md to inject into prompt."""
    # Try root SKILL.md first (updated by organ cycle), fall back to agent/SKILL.md
    skill_path = Path(organ_dir) / "SKILL.md"
    if not skill_path.exists():
        skill_path = Path(organ_dir) / "agent" / "SKILL.md"
    if not skill_path.exists():
        return ""
    try:
        return skill_path.read_text()
    except IOError:
        return ""


def extract_domain_guidance(skill_content: str) -> str:
    """Extract domain confidence scores and weights from SKILL.md.

    Looks for patterns like:
      - **D1 Domain:** 530 verdicts, avg confidence 0.273

    Returns formatted guidance for agent.
    """
    if not skill_content:
        return ""

    domain_weights = []

    # Parse lines containing domain metrics
    for line in skill_content.split('\n'):
        if "avg confidence" in line and "Domain:" in line:
            try:
                # Extract domain name and confidence
                parts = line.split("Domain:")
                if len(parts) > 1:
                    domain_part = parts[0].split("**")[-1].strip()  # e.g., "D1"
                    confidence_parts = parts[1].split("avg confidence ")
                    if len(confidence_parts) > 1:
                        conf_str = confidence_parts[1].split(',')[0].strip()
                        confidence = float(conf_str)
                        domain_weights.append((domain_part, confidence))
            except (ValueError, IndexError):
                pass

    if not domain_weights:
        return ""

    # Sort by confidence descending
    domain_weights.sort(key=lambda x: x[1], reverse=True)

    # Format as guidance
    guidance = "\nDOMAIN EXPLORATION WEIGHTS (based on learned confidence):\n"
    for domain, conf in domain_weights:
        weight = conf / max(w[1] for w in domain_weights) if domain_weights else 0
        guidance += f"  {domain}: confidence={conf:.3f}, weight={weight:.2%}\n"

    guidance += "\nPrioritize domains with higher confidence — they show consistent patterns.\n"
    return guidance


def execute_task(task: dict, organ_dir: str) -> tuple[str | None, str | None]:
    """Execute task via `hermes chat` subprocess.

    Returns (result, error).
    """
    task_id = task.get("id", "?")
    objective = task.get("objective", "")
    actions = task.get("actions", [])
    domain = task.get("domain", "unknown")

    logger.info("executing task %s: domain=%s objective=%s", task_id, domain, objective[:60])

    # Build prompt for Hermes Agent
    action_str = "\n".join(f"  - {a}" for a in actions)

    # Load SKILL.md to inject domain insights and guidance
    skill_context = load_skill(organ_dir)

    prompt = f"""
TASK: {objective}

Domain focus: {domain}
Task ID: {task_id}

Actions to execute:
{action_str}

Success criteria:
- Complete the actions listed above
- Post observations to /observe endpoint
- Include narratives and signal_score in observations

GUIDANCE:
- Focus your exploration on the domain specified above
- Use learned domain patterns from SKILL.md (below) to prioritize signal detection
- Extract high-confidence signals (prefer domains with avg_confidence > 0.30)
- Track domain patterns: consistency matters more than novelty for emerging trends
"""

    # Add domain weight guidance
    if skill_context:
        domain_guidance = extract_domain_guidance(skill_context)
        prompt += domain_guidance
        prompt += "\nCURRENT ORGANISM KNOWLEDGE:\n"
        prompt += f"\n{skill_context}\n"
    else:
        prompt += "\n(No learned patterns yet — establish baseline observations)\n"

    prompt += f"""
Your domain ({domain}) and the patterns above should guide your exploration.
Use the browser and code execution tools to complete this task.
"""

    logger.debug("prompt: %s", prompt[:200])

    # Spawn hermes agent
    try:
        # Run hermes with the task prompt
        # -q: single query (non-interactive)
        # --quiet: suppress banners
        # Note: hermes will need proper credentials configured
        result = subprocess.run(
            ["hermes", "chat", "-q", prompt, "--quiet"],
            capture_output=True,
            text=True,
            timeout=600,  # 10 min timeout per task (hermes setup + git can be slow)
        )

        if result.returncode == 0:
            logger.info("task %s completed", task_id)
            # Capture output for logging
            output = result.stdout[:500] if result.stdout else "(no output)"
            return output, None
        else:
            error_msg = result.stderr[:500] if result.stderr else "(no error message)"
            logger.error("task %s failed: %s", task_id, error_msg)
            return None, error_msg

    except FileNotFoundError:
        error = "hermes CLI not found. Install Hermes Agent first."
        logger.error(error)
        return None, error
    except subprocess.TimeoutExpired:
        error = "task execution timed out (5 min)"
        logger.error(error)
        return None, error
    except Exception as e:
        error = f"task execution failed: {e}"
        logger.error(error)
        return None, error


def run(organ_dir: str, interval: float):
    """Main loop."""
    tasks_executed = 0
    tasks_failed = 0
    running = True

    def handle_signal(sig, frame):
        nonlocal running
        running = False
        logger.info("shutdown signal received")

    signal.signal(signal.SIGTERM, handle_signal)
    signal.signal(signal.SIGINT, handle_signal)

    logger.info("hermes_agent_task_executor v%s starting — organ=%s interval=%.0fs",
                __version__, organ_dir, interval)

    while running:
        # Poll for tasks
        tasks = poll_tasks(organ_dir)
        if tasks:
            logger.info("found %d pending task(s)", len(tasks))

        for task in tasks:
            task_id = task.get("id", "")
            if not task_id:
                logger.warning("task without id, skipping")
                continue

            # Claim task
            if not claim_task(task, organ_dir):
                logger.warning("failed to claim task %s, skipping", task_id)
                continue

            # Execute task
            result, error = execute_task(task, organ_dir)

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

    parser = argparse.ArgumentParser(
        description="Hermes Agent Task Executor — Phase 2 real execution"
    )
    parser.add_argument("--organ-dir", default=ORGAN_DIR, help="Organ directory")
    parser.add_argument("--interval", type=float, default=POLL_INTERVAL, help="Poll interval (seconds)")
    args = parser.parse_args()

    organ_dir = args.organ_dir or ORGAN_DIR
    if not organ_dir:
        logger.error("Organ directory not set")
        sys.exit(1)

    organ_dir = str(Path(organ_dir).expanduser())
    if not Path(organ_dir).exists():
        logger.error("Organ directory not found: %s", organ_dir)
        sys.exit(1)

    logger.info("Organ directory: %s", organ_dir)
    run(organ_dir, args.interval)


if __name__ == "__main__":
    main()
