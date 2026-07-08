"""Filesystem task store for the Hermes X task runner."""

from __future__ import annotations

import json
import logging
import os
from datetime import datetime
from pathlib import Path

from organs.contracts.src.surface_contracts import json_loads_or_default


logger = logging.getLogger("hermes-task-store")


def poll_tasks(organ_dir: str, limit: int = 5) -> list[dict]:
    """Read unclaimed tasks from an organ-local agent-tasks directory."""
    tasks_dir = Path(organ_dir) / "agent-tasks"
    if not tasks_dir.exists():
        logger.debug("agent-tasks directory not found: %s", tasks_dir)
        return []

    tasks = []
    try:
        for task_file in sorted(tasks_dir.glob("*.json"))[:limit]:
            lock_file = tasks_dir / f".{task_file.stem}.lock"
            if lock_file.exists():
                continue

            try:
                task = json_loads_or_default(task_file.read_text(), {})
                task["_file"] = str(task_file)
                task["id"] = task.get("task_id", task.get("id", ""))
                tasks.append(task)
            except json.JSONDecodeError as exc:
                logger.warning("Failed to parse task file %s: %s", task_file, exc)
                continue
    except OSError as exc:
        logger.warning("Failed to scan agent-tasks directory: %s", exc)

    return tasks


def claim_task(task: dict, organ_dir: str) -> bool:
    """Atomically claim a task by creating its lock file."""
    task_file = Path(task.get("_file", ""))
    if not task_file.exists():
        return False

    lock_file = task_file.parent / f".{task_file.stem}.lock"
    try:
        payload = {
            "pid": os.getpid(),
            "claimed_at": datetime.now().isoformat() + "Z",
        }
        with open(lock_file, "x") as handle:
            handle.write(json.dumps(payload))
        logger.info("claimed task %s (lock: %s)", task.get("id"), lock_file.name)
        return True
    except FileExistsError:
        logger.debug("task already locked: %s", task.get("id"))
        return False
    except OSError as exc:
        logger.warning("Failed to claim task %s: %s", task.get("id"), exc)
        return False


def release_task(task: dict, organ_dir: str, success: bool = True) -> None:
    """Release a task lock and optionally delete the completed task file."""
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
        elif lock_file.exists():
            lock_file.unlink()
            logger.info("released lock (task file kept for retry): %s", task_file.name)
    except OSError as exc:
        logger.warning("Failed to release task %s: %s", task.get("id"), exc)
