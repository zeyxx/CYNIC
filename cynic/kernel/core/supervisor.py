"""
CYNIC Kernel Supervisor - Erlang/OTP style supervision tree.
Respects SRE & Solutions Architect Lenses.

Manages the lifecycle of critical organs (tasks). If an organ crashes,
the supervisor catches the exception, logs it to the AnomalyJournal,
and applies a restart strategy (One-For-One, etc.).
"""

from __future__ import annotations

import asyncio
import logging
from dataclasses import dataclass
from typing import Any, Callable, Coroutine, Dict, Optional

from cynic.kernel.observability.anomaly_journal import AnomalyJournal

logger = logging.getLogger("cynic.kernel.supervisor")


@dataclass
class SupervisedOrgan:
    name: str
    coro_func: Callable[[], Coroutine[Any, Any, None]]
    restart_delay: float = 1.0
    max_restarts: int = 5
    current_restarts: int = 0
    task: Optional[asyncio.Task] = None


class KernelSupervisor:
    """
    Supervises background tasks and restarts them upon failure.
    Logs failures to the AnomalyJournal for the MCTS Scientist to analyze.
    """

    def __init__(self, journal: Optional[AnomalyJournal] = None):
        self.journal = journal or AnomalyJournal()
        self._organs: Dict[str, SupervisedOrgan] = {}
        self._running = False
        self._shutdown_event = asyncio.Event()

    def register(
        self,
        name: str,
        coro_func: Callable[[], Coroutine[Any, Any, None]],
        restart_delay: float = 1.0,
    ) -> None:
        """Register a new organ to be supervised."""
        if name in self._organs:
            raise ValueError(f"Organ {name} is already registered.")
        self._organs[name] = SupervisedOrgan(
            name=name, coro_func=coro_func, restart_delay=restart_delay
        )
        logger.debug(f"Supervisor: Registered organ '{name}'")

    async def _supervise(self, organ: SupervisedOrgan):
        """The individual supervision loop for an organ."""
        while self._running and organ.current_restarts < organ.max_restarts:
            logger.info(f"Supervisor: Starting organ '{organ.name}'")
            try:
                # Execute the organ's main loop
                await organ.coro_func()
                # If it returns normally, we assume it's done or stopped intentionally
                break
            except asyncio.CancelledError:
                logger.debug(f"Supervisor: Organ '{organ.name}' cancelled.")
                break
            except Exception as e:
                organ.current_restarts += 1
                logger.error(
                    f"Supervisor: Organ '{organ.name}' crashed! ({organ.current_restarts}/{organ.max_restarts}): {e}",
                    exc_info=True,
                )

                # Log to the Anomaly Journal
                self.journal.log_heresy(
                    error=e,
                    context=f"Supervisor Organ Crash: {organ.name}",
                    metadata={"restarts": organ.current_restarts},
                )

                if organ.current_restarts < organ.max_restarts:
                    logger.warning(
                        f"Supervisor: Restarting '{organ.name}' in {organ.restart_delay}s..."
                    )
                    try:
                        await asyncio.wait_for(
                            self._shutdown_event.wait(), timeout=organ.restart_delay
                        )
                        break  # Shutdown event set during wait
                    except asyncio.TimeoutError:
                        pass  # Continue loop and restart
                else:
                    logger.critical(
                        f"Supervisor: Organ '{organ.name}' reached max restarts. FAILING."
                    )
                    # In a true Erlang model, this might escalate to a parent supervisor.
                    # For now, we emit a critical alert.

    async def start_all(self):
        """Start all registered organs under supervision."""
        if self._running:
            return
        self._running = True
        self._shutdown_event.clear()

        logger.info(f"Supervisor: Starting {len(self._organs)} organs...")
        for organ in self._organs.values():
            organ.current_restarts = 0
            organ.task = asyncio.create_task(self._supervise(organ))

    async def stop_all(self):
        """Gracefully stop all organs."""
        self._running = False
        self._shutdown_event.set()
        logger.info("Supervisor: Stopping all organs...")

        tasks_to_cancel = []
        for organ in self._organs.values():
            if organ.task and not organ.task.done():
                organ.task.cancel()
                tasks_to_cancel.append(organ.task)

        if tasks_to_cancel:
            await asyncio.gather(*tasks_to_cancel, return_exceptions=True)

        logger.info("Supervisor: All organs stopped.")
