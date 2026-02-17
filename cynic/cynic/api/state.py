"""
CYNIC API State — Kernel singleton wired at startup.

One AppState per process. Initialized via FastAPI lifespan.
All routes get this via Depends(get_app_state).
"""
from __future__ import annotations

import time
import logging
from dataclasses import dataclass, field
from typing import Dict, List, Optional

from cynic.core.axioms import AxiomArchitecture
from cynic.core.consciousness import get_consciousness
from cynic.core.event_bus import get_core_bus
from cynic.dogs.base import DogId
from cynic.dogs.cynic_dog import CynicDog
from cynic.dogs.guardian import GuardianDog
from cynic.dogs.analyst import AnalystDog
from cynic.dogs.janitor import JanitorDog
from cynic.judge.orchestrator import JudgeOrchestrator
from cynic.learning.qlearning import QTable, LearningLoop

logger = logging.getLogger("cynic.api.state")


@dataclass
class AppState:
    """
    The kernel wired and ready.

    Built once at startup, lives for the process lifetime.
    """
    orchestrator: JudgeOrchestrator
    qtable: QTable
    learning_loop: LearningLoop
    started_at: float = field(default_factory=time.time)
    _pool: Optional[object] = None  # asyncpg pool (None if no DB)

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at

    @property
    def dogs(self) -> List[str]:
        return list(self.orchestrator.dogs.keys())


def build_kernel(db_pool=None) -> AppState:
    """
    Wire the full kernel: dogs → orchestrator → learning loop.

    db_pool: asyncpg pool (optional — kernel works without DB, just no persistence).
    """
    logger.info("Building CYNIC kernel...")

    # Dogs (non-LLM — always available, no API keys needed)
    cynic_dog = CynicDog()
    dogs = {
        DogId.CYNIC:    cynic_dog,
        DogId.GUARDIAN: GuardianDog(),
        DogId.ANALYST:  AnalystDog(),
        DogId.JANITOR:  JanitorDog(),
    }

    axiom_arch = AxiomArchitecture()
    orchestrator = JudgeOrchestrator(
        dogs=dogs,
        axiom_arch=axiom_arch,
        cynic_dog=cynic_dog,
    )

    # Learning
    qtable = QTable()
    learning_loop = LearningLoop(qtable=qtable, pool=db_pool)
    learning_loop.start(get_core_bus())

    logger.info(
        "Kernel ready: %d dogs, learning loop active, pool=%s",
        len(dogs),
        "connected" if db_pool else "none",
    )

    return AppState(
        orchestrator=orchestrator,
        qtable=qtable,
        learning_loop=learning_loop,
        _pool=db_pool,
    )


# Process-level singleton — set during lifespan startup
_state: Optional[AppState] = None


def set_state(state: AppState) -> None:
    global _state
    _state = state


def get_state() -> AppState:
    if _state is None:
        raise RuntimeError("AppState not initialized — lifespan not started")
    return _state
