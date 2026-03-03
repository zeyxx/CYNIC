"""
CYNIC Task Registry  Asynchronous Life-Cycle Guard.

Tracks every background task created by the organism to prevent 
zombie processes and memory leaks.

-Law: FIDELITY  Ensure every process has a clear end.
"""

from __future__ import annotations

import asyncio
import logging
from typing import Dict, Set

logger = logging.getLogger("cynic.kernel.task_registry")

class TaskRegistry:
    """
    Central registry for long-running asyncio tasks.
    Allows graceful cancellation during shutdown.
    """

    def __init__(self, instance_id: str):
        self.instance_id = instance_id
        self._tasks: Set[asyncio.Task] = set()
        self._lock = asyncio.Lock()

    def register(self, task: asyncio.Task):
        """Add a task to the registry immediately (Thread-Safe)."""
        self._tasks.add(task)
        task.add_done_callback(self._discard)

    def _discard(self, task: asyncio.Task):
        """Remove a completed task."""
        self._tasks.discard(task)

    async def close(self, timeout: float = 5.0):
        """Cancel all registered tasks and wait for them to finish."""
        async with self._lock:
            if not self._tasks:
                return
            
            count = len(self._tasks)
            logger.info(f"[{self.instance_id}] TaskRegistry: Closing {count} tasks...")
            
            for task in self._tasks:
                task.cancel()
            
            # Wait for tasks to acknowledge cancellation
            try:
                await asyncio.wait_for(
                    asyncio.gather(*list(self._tasks), return_exceptions=True),
                    timeout=timeout
                )
            except asyncio.TimeoutError:
                logger.warning(f"[{self.instance_id}] TaskRegistry: Some tasks did not stop in time.")
            finally:
                self._tasks.clear()
                logger.info(f"[{self.instance_id}] TaskRegistry: All tasks purged.")

    @property
    def active_count(self) -> int:
        return len(self._tasks)
