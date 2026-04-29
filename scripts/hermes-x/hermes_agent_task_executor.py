#!/usr/bin/env python3
"""
Hermes Agent Task Executor — Phase 2 real execution.

Unlike hermes_task_runner.py (Phase 1 stub), this spawns a Hermes Agent instance
to actually execute tasks: browse X, analyze signals, post observations.

Workflow:
  1. Claim task from agent-tasks/ (lock file)
  2. Spawn `hermes chat` with task prompt + SKILL.md context
  3. Agent executes actions via its tools
  4. Capture observations posted to /observe
  5. Release task when done
  6. Update SKILL.md with any new learned skills

Usage:
    python3 hermes_agent_task_executor.py --organ-dir ~/.cynic/organs/hermes/x --interval 30

Environment:
    X_ORGAN_DIR  — organ directory (fallback)
"""

__version__ = "0.3.0"  # Real execution variant

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

logger = logging.getLogger("hermes-task-executor")

ORGAN_DIR = ""
POLL_INTERVAL = 30.0


def load_env():
    global ORGAN_DIR
    ORGAN_DIR = os.environ.get("X_ORGAN_DIR", str(Path.home() / ".cynic" / "organs" / "hermes" / "x"))


def poll_tasks(organ_dir: str, limit: int = 1) -> list[dict]:
    """Read unclaimed tasks from organ-local agent-tasks/ directory."""
    tasks_dir = Path(organ_dir) / "agent-tasks"
    if not tasks_dir.exists():
        logger.debug("agent-tasks directory not found: %s", tasks_dir)
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
                    task["id"] = task.get("task_id", task.get("id", ""))
                    tasks.append(task)
            except json.JSONDecodeError as e:
                logger.warning("Failed to parse task file %s: %s", task_file, e)
                continue

    except OSError as e:
        logger.warning("Failed to scan agent-tasks directory: %s", e)

    return tasks


def claim_task(task: dict, organ_dir: str) -> bool:
    """Claim task by creating lock file."""
    task_file = Path(task.get("_file", ""))
    if not task_file.exists():
        return False

    lock_file = task_file.parent / f".{task_file.stem}.lock"

    try:
        lock_file.write_text(json.dumps({
            "pid": os.getpid(),
            "claimed_at": datetime.now().isoformat() + "Z",
        }))
        logger.info("claimed task %s", task.get("id"))
        return True
    except OSError as e:
        logger.warning("Failed to claim task: %s", e)
        return False


def release_task(task: dict, organ_dir: str, success: bool = True):
    """Release task by deleting lock and optionally task file."""
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
    skill_path = Path(organ_dir) / "agent" / "SKILL.md"
    if not skill_path.exists():
        return ""
    try:
        return skill_path.read_text()
    except IOError:
        return ""


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
    prompt = f"""
TASK: {objective}

Domain: {domain}
Task ID: {task_id}

Actions to execute:
{action_str}

Success criteria:
- Complete the actions listed above
- Post observations to /observe endpoint
- Include narratives and signal_score in observations

Current skills are loaded — use them if relevant.
Use the browser and code execution tools to complete this task.
"""

    # Inject SKILL.md as context
    skill_context = load_skill(organ_dir)
    if skill_context:
        prompt = f"SKILL CONTEXT:\n{skill_context}\n\n{prompt}"

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
            timeout=300,  # 5 min timeout per task
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
