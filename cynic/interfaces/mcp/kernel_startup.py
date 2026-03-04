"""Universal kernel health check and spawn logic — modules injected by caller."""
from __future__ import annotations

import logging
import asyncio
from typing import Any

logger = logging.getLogger(__name__)

CYNIC_URL = "http://127.0.0.1:8765"
HEALTH_ENDPOINT = f"{CYNIC_URL}/api/health"


async def ensure_kernel_running(
    aiohttp_module: Any,
    spawn_fn: Any,
    timeout: float = 30.0,
    spawn_if_down: bool = False,
    url: str = HEALTH_ENDPOINT,
) -> bool:
    """Check if CYNIC kernel is healthy. Optionally spawn it if down."""
    deadline = asyncio.get_event_loop().time() + timeout
    delay = 1.0

    while asyncio.get_event_loop().time() < deadline:
        try:
            async with aiohttp_module.ClientSession() as session:
                async with session.get(url, timeout=aiohttp_module.ClientTimeout(total=5)) as resp:
                    if resp.status == 200:
                        return True
        except Exception:
            pass

        if spawn_if_down:
            spawn_fn()
            spawn_if_down = False  # only attempt once

        remaining = deadline - asyncio.get_event_loop().time()
        if remaining <= 0:
            break
        await asyncio.sleep(min(delay, remaining))
        delay = min(delay * 2, 10.0)

    logger.warning("CYNIC kernel not reachable at %s after %.1fs", url, timeout)
    return False


def do_spawn_kernel(subprocess_module: Any) -> Any:
    """Spawn the CYNIC kernel process."""
    try:
        proc = subprocess_module.Popen(
            ["python", "-m", "cynic.kernel.__main__"],
            stdout=subprocess_module.DEVNULL,
            stderr=subprocess_module.DEVNULL,
        )
        logger.info("Spawned CYNIC kernel (pid=%s)", proc.pid)
        return proc
    except Exception as exc:
        logger.error("Failed to spawn kernel: %s", exc)
        return None
