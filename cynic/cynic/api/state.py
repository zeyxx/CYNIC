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
from cynic.dogs.architect import ArchitectDog
from cynic.dogs.oracle import OracleDog
from cynic.dogs.sage import SageDog
from cynic.dogs.scholar import ScholarDog
from cynic.dogs.cartographer import CartographerDog
from cynic.judge.orchestrator import JudgeOrchestrator
from cynic.judge.residual import ResidualDetector
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
    residual_detector: ResidualDetector
    started_at: float = field(default_factory=time.time)
    _pool: Optional[object] = None  # asyncpg pool (None if no DB)
    last_judgment: Optional[Dict] = None  # state_key, action, judgment_id — for /feedback

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
    # QTable must exist before OracleDog (Oracle reads it for predictions)
    qtable = QTable()
    dogs = {
        DogId.CYNIC:      cynic_dog,
        DogId.SAGE:       SageDog(),
        DogId.GUARDIAN:   GuardianDog(),
        DogId.ANALYST:    AnalystDog(),
        DogId.JANITOR:    JanitorDog(),
        DogId.ARCHITECT:  ArchitectDog(),
        DogId.ORACLE:     OracleDog(qtable=qtable),
        DogId.SCHOLAR:    ScholarDog(),
        DogId.CARTOGRAPHER: CartographerDog(),
    }

    axiom_arch = AxiomArchitecture()
    learning_loop = LearningLoop(qtable=qtable, pool=db_pool)
    learning_loop.start(get_core_bus())

    # ResidualDetector — subscribes to JUDGMENT_CREATED, emits EMERGENCE_DETECTED
    residual_detector = ResidualDetector()
    residual_detector.start(get_core_bus())

    orchestrator = JudgeOrchestrator(
        dogs=dogs,
        axiom_arch=axiom_arch,
        cynic_dog=cynic_dog,
        residual_detector=residual_detector,
    )

    logger.info(
        "Kernel ready: %d dogs, learning loop + residual detector active, pool=%s",
        len(dogs),
        "connected" if db_pool else "none",
    )

    return AppState(
        orchestrator=orchestrator,
        qtable=qtable,
        learning_loop=learning_loop,
        residual_detector=residual_detector,
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
