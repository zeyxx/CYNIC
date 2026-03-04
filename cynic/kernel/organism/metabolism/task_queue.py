"""
CYNIC Task Queue - Non-blocking Industrial Execution.
Ensures the Organism heartbeat remains active while processing heavy LLM missions.
"""
from __future__ import annotations
import asyncio
import logging
from dataclasses import dataclass, field
from typing import Any, Dict, List, Callable, Awaitable

logger = logging.getLogger("cynic.organism.task_queue")

@dataclass
class EmergenceMission:
    mission_id: str
    axiom: str
    target: str
    description: str
    payload: Dict[str, Any] = field(default_factory=dict)

class TaskOrchestrator:
    def __init__(self, concurrency_limit: int = 1):
        self.queue: asyncio.Queue[EmergenceMission] = asyncio.Queue()
        self.concurrency_limit = concurrency_limit
        self._workers: List[asyncio.Task] = []
        self.is_running = False

    async def start(self, processor_func: Callable[[EmergenceMission], Awaitable[None]]):
        """Starts background workers to process missions."""
        self.is_running = True
        for i in range(self.concurrency_limit):
            worker = asyncio.create_task(self._worker_loop(i, processor_func))
            self._workers.append(worker)
        logger.info(f"TaskOrchestrator: Started {self.concurrency_limit} workers.")

    async def _worker_loop(self, worker_id: int, processor_func: Callable[[EmergenceMission], Awaitable[None]]):
        while self.is_running:
            mission = await self.queue.get()
            try:
                logger.info(f"Worker-{worker_id}: Processing mission {mission.mission_id}")
                await processor_func(mission)
            except Exception as e:
                logger.error(f"Worker-{worker_id}: Mission {mission.mission_id} failed: {e}")
            finally:
                self.queue.task_done()

    async def add_mission(self, mission: EmergenceMission):
        await self.queue.put(mission)
        logger.info(f"TaskOrchestrator: Mission {mission.mission_id} queued.")

    async def stop(self):
        self.is_running = False
        for worker in self._workers:
            worker.cancel()
        await asyncio.gather(*self._workers, return_exceptions=True)
        logger.info("TaskOrchestrator: All workers stopped.")
