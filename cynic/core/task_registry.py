"""
CYNIC Task Registry — Track all background tasks for cleanup and monitoring.

Prevents:
- Resource leaks (forgotten asyncio.create_task calls)
- Unobserved exceptions (tasks dying silently)
- Rogue processes (autonomous tasks without supervision)

Pattern: Singleton registry that tracks all active tasks,
allowing graceful shutdown and error tracking.
"""
from __future__ import annotations

import asyncio
import logging
from typing import Set, Optional, Dict, Any
from weakref import WeakSet

logger = logging.getLogger("cynic.task_registry")


class TaskRegistry:
    """
    Singleton task registry for background task lifecycle management.

    Usage:
        # Register a task
        task = asyncio.create_task(my_coroutine())
        TaskRegistry.register(task, name="my_task")

        # Cleanup all tasks
        await TaskRegistry.cleanup_all()
    """

    _instance: Optional[TaskRegistry] = None
    _tasks: WeakSet = WeakSet()  # Weak references (auto-cleanup when GC'd)
    _task_info: Dict[int, Dict[str, Any]] = {}  # Metadata per task

    def __new__(cls) -> TaskRegistry:
        if cls._instance is None:
            cls._instance = super().__new__(cls)
            cls._instance._tasks = WeakSet()
            cls._instance._task_info = {}
        return cls._instance

    @classmethod
    def register(cls, task: asyncio.Task, name: str = "unnamed") -> None:
        """Register a task for tracking."""
        instance = cls()
        instance._tasks.add(task)

        # Store metadata
        task_id = id(task)
        instance._task_info[task_id] = {
            "name": name,
            "task": task,
            "created": asyncio.get_event_loop().time(),
        }

        # Auto-cleanup metadata when task completes
        def cleanup_metadata(t):
            instance._task_info.pop(task_id, None)

        task.add_done_callback(cleanup_metadata)
        logger.debug(f"Registered task: {name} (id={task_id})")

    @classmethod
    def get_active_count(cls) -> int:
        """Get count of active (non-completed) tasks."""
        instance = cls()
        return sum(1 for t in instance._tasks if not t.done())

    @classmethod
    def get_active_tasks(cls) -> list[asyncio.Task]:
        """Get all active tasks."""
        instance = cls()
        return [t for t in instance._tasks if not t.done()]

    @classmethod
    def get_task_info(cls) -> Dict[str, Any]:
        """Get metadata about all registered tasks."""
        instance = cls()
        return {
            "total_tracked": len(instance._task_info),
            "active_count": cls.get_active_count(),
            "tasks": [
                {
                    "name": info.get("name"),
                    "id": task_id,
                    "done": info["task"].done(),
                }
                for task_id, info in instance._task_info.items()
            ]
        }

    @classmethod
    async def cleanup_all(cls, timeout: float = 5.0) -> None:
        """
        Gracefully shutdown all registered tasks.

        1. Cancel all pending tasks
        2. Wait for cancellation with timeout
        3. Log any tasks that didn't cleanup
        """
        instance = cls()
        active = cls.get_active_tasks()

        if not active:
            logger.info("No active tasks to cleanup")
            return

        logger.info(f"Cleaning up {len(active)} active tasks...")

        # Cancel all
        for task in active:
            task.cancel()

        # Wait for cancellation with timeout
        try:
            await asyncio.wait_for(
                asyncio.gather(*active, return_exceptions=True),
                timeout=timeout
            )
            logger.info("All tasks cleaned up successfully")
        except asyncio.TimeoutError:
            logger.warning(
                f"Task cleanup timeout after {timeout}s. "
                f"{len(cls.get_active_tasks())} tasks still running."
            )

    @classmethod
    def reset(cls) -> None:
        """Reset registry (for testing only)."""
        instance = cls()
        instance._tasks = WeakSet()
        instance._task_info = {}
        logger.debug("Task registry reset")


# Global convenience functions
def register_task(task: asyncio.Task, name: str = "unnamed") -> None:
    """Register a task globally."""
    TaskRegistry.register(task, name)


def get_active_task_count() -> int:
    """Get count of active tasks."""
    return TaskRegistry.get_active_count()


async def cleanup_all_tasks(timeout: float = 5.0) -> None:
    """Cleanup all registered tasks."""
    await TaskRegistry.cleanup_all(timeout=timeout)
