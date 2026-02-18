"""
CYNIC API State — Kernel singleton wired at startup.

One AppState per process. Initialized via FastAPI lifespan.
All routes get this via Depends(get_app_state).
"""
from __future__ import annotations

import json
import os
import time
import logging
from dataclasses import dataclass, field
from typing import Dict, List, Optional

from cynic.core.axioms import AxiomArchitecture
from cynic.core.heuristic_scorer import HeuristicFacetScorer
from cynic.core.consciousness import ConsciousnessLevel
from cynic.core.consciousness import get_consciousness
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.core.phi import MAX_CONFIDENCE
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
from cynic.dogs.deployer import DeployerDog
from cynic.dogs.scout import ScoutDog
from cynic.judge.orchestrator import JudgeOrchestrator
from cynic.judge.residual import ResidualDetector
from cynic.judge.decide import DecideAgent
from cynic.learning.qlearning import QTable, LearningLoop
from cynic.perceive.workers import GitWatcher, HealthWatcher, SelfWatcher, MarketWatcher, SolanaWatcher, SocialWatcher
from cynic.scheduler import DogScheduler
from cynic.act.telemetry import TelemetryStore

logger = logging.getLogger("cynic.api.state")

_GUIDANCE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "guidance.json")


async def _on_judgment_created(event: Event) -> None:
    """
    Write guidance.json from ANY judgment source (hook, scheduler, API).

    This is the key feedback loop: JUDGMENT_CREATED fires after every judgment,
    including MACRO scheduler jobs where SAGE temporal MCTS ran.
    The hook reads guidance.json on next UserPromptSubmit → Claude Code gets wisdom.

    By subscribing here (not just in /judge and /perceive endpoints),
    SAGE's Ollama-powered judgments finally reach the feedback loop.
    """
    try:
        p = event.payload
        os.makedirs(os.path.dirname(_GUIDANCE_PATH), exist_ok=True)
        with open(_GUIDANCE_PATH, "w", encoding="utf-8") as fh:
            json.dump({
                "timestamp": time.time(),
                "state_key": p.get("state_key", ""),
                "verdict": p.get("verdict", "WAG"),
                "q_score": round(float(p.get("q_score", 0.0)), 3),
                "confidence": round(min(float(p.get("confidence", 0.0)), MAX_CONFIDENCE), 4),
                "reality": p.get("reality", "CODE"),
                "dog_votes": {
                    k: round(float(v), 3)
                    for k, v in (p.get("dog_votes") or {}).items()
                },
            }, fh)
    except Exception as exc:
        logger.debug("guidance.json write skipped: %s", exc)


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
    scheduler: DogScheduler
    started_at: float = field(default_factory=time.time)
    _pool: Optional[object] = None  # asyncpg pool (None if no DB)
    last_judgment: Optional[Dict] = None  # state_key, action, judgment_id — for /feedback
    decide_agent: Optional[object] = None  # DecideAgent — auto-decides on BARK/GROWL
    runner: Optional[object] = None        # ClaudeCodeRunner — spawns claude autonomously
    telemetry_store: TelemetryStore = field(default_factory=TelemetryStore)  # session data

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at

    @property
    def dogs(self) -> List[str]:
        return list(self.orchestrator.dogs.keys())


def build_kernel(db_pool=None, registry=None) -> AppState:
    """
    Wire the full kernel: dogs → orchestrator → learning loop.

    db_pool: asyncpg pool (optional — kernel works without DB, just no persistence).
    registry: LLMRegistry (optional — kernel works without LLM, heuristic mode only).
              When provided, LLM dogs get registry injected → temporal MCTS enabled.
    """
    logger.info("Building CYNIC kernel...")

    # Dogs (non-LLM — always available, no API keys needed)
    cynic_dog = CynicDog()
    # QTable must exist before OracleDog (Oracle reads it for predictions)
    qtable = QTable()
    dogs = {
        DogId.CYNIC:        cynic_dog,
        DogId.SAGE:         SageDog(),
        DogId.GUARDIAN:     GuardianDog(),
        DogId.ANALYST:      AnalystDog(),
        DogId.JANITOR:      JanitorDog(),
        DogId.ARCHITECT:    ArchitectDog(),
        DogId.ORACLE:       OracleDog(qtable=qtable),
        DogId.SCHOLAR:      ScholarDog(),
        DogId.CARTOGRAPHER: CartographerDog(),
        DogId.DEPLOYER:     DeployerDog(),
        DogId.SCOUT:        ScoutDog(),
    }

    # ── Inject LLMRegistry into all LLM-capable dogs ──────────────────────
    # Dogs use registry to get the best adapter (Ollama first, then Claude).
    # SageDog → Temporal MCTS (7 parallel Ollama calls, asyncio.gather)
    # Other LLM dogs → Phase 2 when they implement their LLM paths
    if registry is not None:
        llm_dogs_wired = 0
        for dog in dogs.values():
            if hasattr(dog, "set_llm_registry"):
                dog.set_llm_registry(registry)
                llm_dogs_wired += 1
        available_count = len(registry.get_available())
        logger.info(
            "LLMRegistry injected into %d dogs, %d adapters available",
            llm_dogs_wired, available_count,
        )
    else:
        logger.info("No LLMRegistry — all dogs run in heuristic mode")

    axiom_arch = AxiomArchitecture(facet_scorer=HeuristicFacetScorer())
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

    scheduler = DogScheduler(orchestrator=orchestrator)

    # ── DecideAgent — subscribes to JUDGMENT_CREATED, auto-decides on BARK/GROWL
    decide_agent = DecideAgent(qtable=qtable)
    decide_agent.start(get_core_bus())

    # ── EMERGENCE_DETECTED -> META cycle trigger ───────────────────────────
    # When ResidualDetector detects an emergence pattern, submit a META cell
    # to the scheduler so the organism can evolve in response.
    async def _on_emergence(event: Event) -> None:
        from cynic.core.judgment import Cell
        cell = Cell(
            reality="CYNIC", analysis="EMERGE", time_dim="PRESENT",
            content=event.payload,
            context="Emergence detected — META cycle triggered",
            risk=0.5, complexity=0.6, budget_usd=0.1,
            metadata={
                "source": "emergence_trigger",
                "pattern": event.payload.get("pattern_type", "UNKNOWN") if isinstance(event.payload, dict) else "UNKNOWN",
            },
        )
        scheduler.submit(cell, level=ConsciousnessLevel.META, source="emergence_trigger")

    get_core_bus().on(CoreEvent.EMERGENCE_DETECTED, _on_emergence)

    # ── Guidance feedback loop — ALL judgment sources ──────────────────────
    # Subscribes to JUDGMENT_CREATED from ANY source: /perceive (REFLEX),
    # /judge (MACRO), or DogScheduler background workers (MACRO with SAGE).
    # This ensures SAGE's Ollama temporal MCTS reaches the guidance.json loop.
    get_core_bus().on(CoreEvent.JUDGMENT_CREATED, _on_judgment_created)

    # ── PerceiveWorkers (autonomous sensors) ──────────────────────────────
    # Wired here; actually started when scheduler.start() is called (in lifespan).
    # CODE×PERCEIVE — git working tree changes
    scheduler.register_perceive_worker(GitWatcher())
    # CYNIC×PERCEIVE — timer degradation
    scheduler.register_perceive_worker(HealthWatcher())
    # CYNIC×LEARN — Q-Table health self-monitoring
    scheduler.register_perceive_worker(SelfWatcher(qtable_getter=lambda: qtable))
    # MARKET×PERCEIVE — SOL/USD price significant moves
    scheduler.register_perceive_worker(MarketWatcher())
    # SOLANA×PERCEIVE — mainnet slot + TPS anomalies
    scheduler.register_perceive_worker(SolanaWatcher())
    # SOCIAL×PERCEIVE — social signals from ~/.cynic/social.json feed
    scheduler.register_perceive_worker(SocialWatcher())

    logger.info(
        "Kernel ready: %d dogs, scheduler wired, learning loop + residual detector active, pool=%s, llm=%s",
        len(dogs),
        "connected" if db_pool else "none",
        f"{len(registry.get_available())} adapters" if registry else "none",
    )

    return AppState(
        orchestrator=orchestrator,
        qtable=qtable,
        learning_loop=learning_loop,
        residual_detector=residual_detector,
        scheduler=scheduler,
        _pool=db_pool,
        decide_agent=decide_agent,
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
