"""
CYNIC API State — Gateway to the unified Organism.

One AppContainer per process. Initialized via FastAPI lifespan.
All routes get this via Depends(get_app_container).
"""
from __future__ import annotations

import json
import os
import time
import logging
from dataclasses import dataclass, field
from typing import Any, Optional, TYPE_CHECKING

if TYPE_CHECKING:
    import asyncpg

from cynic.core.phi import MAX_CONFIDENCE
from cynic.core.event_bus import get_core_bus, Event, CoreEvent
from cynic.organism.organism import Organism

logger = logging.getLogger("cynic.api.state")

_GUIDANCE_PATH = os.path.join(os.path.expanduser("~"), ".cynic", "guidance.json")

# Instance ID for multi-instance guidance isolation (set from lifespan startup)
_current_instance_id: str | None = None


def set_instance_id(instance_id: str) -> None:
    """Set the instance ID used for guidance-{id}.json isolation."""
    global _current_instance_id
    _current_instance_id = instance_id


# Backward compatibility alias
CynicOrganism = Organism


@dataclass
class AppContainer:
    """
    Instance-scoped application state.

    The AppContainer is the single gateway to the CYNIC Organism for the API.
    It encapsulates the organism instance and process-specific metadata.
    """
    organism: Organism
    instance_id: str  # Unique per process
    guidance_path: str  # ~/.cynic/guidance-{instance_id}.json
    started_at: float = field(default_factory=time.time)

    @property
    def uptime_s(self) -> float:
        return time.time() - self.started_at

    # --- Convenience properties mapping to Organism components ---
    @property
    def orchestrator(self): return self.organism.orchestrator
    @property
    def qtable(self): return self.organism.qtable
    @property
    def learning_loop(self): return self.organism.learning_loop
    @property
    def scheduler(self): return self.organism.scheduler
    @property
    def runner(self): return self.organism.runner
    @property
    def decide_agent(self): return self.organism.decide_agent
    @property
    def escore_tracker(self): return self.organism.escore_tracker
    @property
    def axiom_monitor(self): return self.organism.axiom_monitor
    @property
    def lod_controller(self): return self.organism.lod_controller
    @property
    def world_model(self): return self.organism.world_model
    @property
    def container(self): return self.organism.container


# Process-level singleton — set during lifespan startup
_app_container: Optional[AppContainer] = None


def set_app_container(c: AppContainer) -> None:
    """Set the app container during lifespan startup."""
    global _app_container
    _app_container = c


def get_app_container() -> AppContainer:
    """Get the app container (FastAPI dependency)."""
    if _app_container is None:
        raise RuntimeError("AppContainer not initialized — lifespan not started")
    return _app_container


# ── Legacy Compatibility Layer ───────────────────────────────────────────

def build_kernel(db_pool=None, registry=None) -> Organism:
    """DEPRECATED: Use awaken()."""
    return awaken(db_pool, registry)


_state: Optional[Organism] = None


def set_state(state: Organism) -> None:
    """Legacy state setter."""
    global _state
    _state = state


def get_state() -> Organism:
    """Legacy state getter."""
    if _state is None:
        raise RuntimeError("Organism not initialized")
    return _state


def awaken(db_pool=None, registry=None) -> Organism:
    """Gateway to the true awakening in cynic.organism.organism."""
    from cynic.organism.organism import awaken as organism_awaken
    return organism_awaken(db_pool, registry)


async def restore_state(container: AppContainer) -> None:
    """Trigger state recovery via OrganismState."""
    await container.organism.state.recover()
    logger.info("restore_state: OrganismState recovery triggered")
