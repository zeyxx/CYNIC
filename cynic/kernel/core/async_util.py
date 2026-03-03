"""
CYNIC Universal Async Utilities - Tier 4 Rails.

Standardizes async patterns across the organism.
Includes async-aware caching, concurrency control, and timeout management.

Lentille: Backend / Site Reliability Engineer
"""

import asyncio
import functools
import logging
from typing import Any, Callable, Coroutine, Dict, Optional, TypeVar

logger = logging.getLogger("cynic.kernel.core.async_util")

T = TypeVar("T")


def async_cached(maxsize: int = 128):
    """
    LRU Cache for coroutines.
    Correctly awaits and caches the RESULT, not the coroutine object.
    """
    cache: Dict[tuple, Any] = {}

    def decorator(func: Callable[..., Coroutine[Any, Any, T]]):
        @functools.wraps(func)
        async def wrapper(*args, **kwargs):
            key = args + tuple(sorted(kwargs.items()))
            if key in cache:
                return cache[key]
            result = await func(*args, **kwargs)
            if len(cache) >= maxsize:
                cache.pop(next(iter(cache)))
            cache[key] = result
            return result

        wrapper.cache_clear = lambda: cache.clear()
        return wrapper

    return decorator


async def gather_with_concurrency(n: int, *coros: Coroutine[Any, Any, T]) -> list[T]:
    """Limits the number of concurrent coroutines."""
    semaphore = asyncio.Semaphore(n)

    async def sem_coro(coro):
        async with semaphore:
            return await coro

    return await asyncio.gather(*(sem_coro(c) for c in coros))


async def wait_with_timeout(
    coro: Coroutine[Any, Any, T], timeout: float, name: str = "task"
) -> T:
    """Standard timeout wrapper with structured logging."""
    try:
        return await asyncio.wait_for(coro, timeout=timeout)
    except asyncio.TimeoutError:
        logger.warning(f"Async Timeout: {name} exceeded {timeout}s")
        raise


class TaskTracker:
    """Tracks background tasks to ensure they are all awaited or cancelled on shutdown."""

    def __init__(self, name: str = "tracker"):
        self.name = name
        self._tasks: set[asyncio.Task] = set()

    def create_task(
        self, coro: Coroutine[Any, Any, Any], name: Optional[str] = None
    ) -> asyncio.Task:
        task = asyncio.create_task(coro, name=name)
        self._tasks.add(task)
        task.add_done_callback(self._tasks.discard)
        return task

    async def close(self, timeout: float = 5.0):
        if not self._tasks:
            return
        for task in self._tasks:
            task.cancel()
        await asyncio.gather(*self._tasks, return_exceptions=True)
        self._tasks.clear()
