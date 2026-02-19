"""CYNIC GitWatcher — CODE×PERCEIVE/REFLEX every F(5)=5s."""
from __future__ import annotations

import asyncio
import subprocess
from typing import Optional

from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.judgment import Cell
from cynic.core.phi import fibonacci
from cynic.perceive.workers.base import PerceiveWorker


class GitWatcher(PerceiveWorker):
    """
    Monitors git working tree for uncommitted changes.

    Submits CODE×PERCEIVE at REFLEX level when new changes are detected.
    Deduplicates: only submits when the change set actually differs from last check.

    interval: F(5)=5s — git status is fast (<50ms), harmless to run often.
    """

    level = ConsciousnessLevel.REFLEX
    interval_s = float(fibonacci(5))   # 5.0s
    name = "git_watcher"

    def __init__(self, cwd: str | None = None) -> None:
        self._cwd = cwd
        self._last_hash: int | None = None

    async def sense(self) -> Cell | None:
        loop = asyncio.get_running_loop()
        try:
            result = await loop.run_in_executor(
                None,
                lambda: subprocess.run(
                    ["git", "status", "--porcelain"],
                    capture_output=True, text=True, timeout=3.0,
                    cwd=self._cwd,
                ),
            )
        except (subprocess.TimeoutExpired, FileNotFoundError, OSError):
            return None

        if result.returncode != 0 or not result.stdout.strip():
            self._last_hash = None
            return None

        changes = result.stdout.strip()
        change_hash = hash(changes)
        if change_hash == self._last_hash:
            return None     # No new changes since last check

        self._last_hash = change_hash
        lines = changes.splitlines()

        return Cell(
            reality="CODE",
            analysis="PERCEIVE",
            time_dim="PRESENT",
            content={"git_status": changes[:1500], "changed_files": len(lines)},
            context=f"Git watcher: {len(lines)} changed file(s) detected",
            risk=0.0,
            complexity=min(len(lines) / 50.0, 1.0),
            budget_usd=0.001,
            metadata={"source": "git_watcher", "file_count": len(lines)},
        )
