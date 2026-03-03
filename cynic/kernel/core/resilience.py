"""
CYNIC Resilience Utilities - Retry and Breaker logic.
Implemented for 10k TPS Industrial Scale.
"""

import asyncio
import logging
import time
from functools import wraps
from typing import Any, Callable, TypeVar

logger = logging.getLogger("cynic.kernel.resilience")

T = TypeVar("T")


def async_retry(
    retries: int = 3,
    delay: float = 0.382,  # PHI_INV_2 base
    backoff: float = 1.618,  # PHI
    exceptions: tuple = (Exception,),
):
    """
    Decorator for robust async retries with exponential backoff.
    Lentille: Data Engineer / Backend
    """

    def decorator(func: Callable[..., Any]):
        @wraps(func)
        async def wrapper(*args, **kwargs):
            current_delay = delay
            last_exc = None
            for i in range(retries + 1):
                try:
                    return await func(*args, **kwargs)
                except exceptions as e:
                    last_exc = e
                    if i == retries:
                        break
                    logger.warning(
                        f"Retry {i+1}/{retries} for {func.__name__} after {current_delay:.3f}s due to {type(e).__name__}"
                    )
                    await asyncio.sleep(current_delay)
                    current_delay *= backoff
            raise last_exc

        return wrapper

    return decorator


class ResilienceStats:
    def __init__(self):
        self.failures = 0
        self.successes = 0
        self.last_failure_at = 0.0
        self.total_retries = 0

    def to_dict(self):
        return {
            "failures": self.failures,
            "successes": self.successes,
            "total_retries": self.total_retries,
            "uptime_s": time.time() - self.last_failure_at
            if self.last_failure_at > 0
            else 0,
        }
