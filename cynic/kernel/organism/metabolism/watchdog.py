"""
CYNIC Metabolic Watchdog - Active Health & Recovery.
Maintains the 'Body Budget' by continuously monitoring LLM backends.
"""
from __future__ import annotations
import asyncio
import logging
import httpx
from typing import Dict, Any, List

logger = logging.getLogger("cynic.organism.watchdog")

class HealthStatus:
    def __init__(self, name: str):
        self.name = name
        self.is_alive = False
        self.latency_ms = 0.0
        self.error_count = 0

class MetabolicWatchdog:
    def __init__(self, endpoints: Dict[str, str]):
        self.endpoints = endpoints
        self.status: Dict[str, HealthStatus] = {name: HealthStatus(name) for name in endpoints}
        self._monitor_task: Optional[asyncio.Task] = None
        self.is_running = False

    async def start(self):
        self.is_running = True
        self._monitor_task = asyncio.create_task(self._monitor_loop())
        logger.info("Watchdog: Metabolic monitoring started.")

    async def _monitor_loop(self):
        async with httpx.AsyncClient(timeout=2.0) as client:
            while self.is_running:
                for name, url in self.endpoints.items():
                    try:
                        start = asyncio.get_event_loop().time()
                        # Specialized health check based on name
                        path = "/api/ps" if "ollama" in name.lower() else "/health"
                        resp = await client.get(f"{url}{path}")
                        
                        s = self.status[name]
                        if resp.status_code == 200:
                            s.is_alive = True
                            s.latency_ms = (asyncio.get_event_loop().time() - start) * 1000
                            s.error_count = 0
                        else:
                            s.is_alive = False
                            s.error_count += 1
                    except Exception:
                        self.status[name].is_alive = False
                        self.status[name].error_count += 1
                
                await asyncio.sleep(10) # Fibonacci-aligned breathing

    def get_best_available_endpoint(self) -> Optional[str]:
        """Returns the healthiest, lowest-latency endpoint."""
        alive = [s for s in self.status.values() if s.is_alive]
        if not alive: return None
        sorted_status = sorted(alive, key=lambda x: x.latency_ms)
        return self.endpoints[sorted_status[0].name]

    async def stop(self):
        self.is_running = False
        if self._monitor_task:
            self._monitor_task.cancel()
