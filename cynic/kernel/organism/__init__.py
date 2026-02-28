"""CYNIC Organism — Living System Architecture

The organism is a unified system with specialized organs coordinating via events.

Structure:
├── handlers/             Event handlers (Judgment, Senses, etc.)
├── state_manager.py      Unified state (RAM/DB/File)
└── organism.py           Central coordinator and factory (awaken)

The organism is:
- SOURCE OF TRUTH for state
- AUTONOMOUS (runs independently)
- EVENT-DRIVEN (updates via events)
- OBSERVABLE (Exposes state via OrganismState)
"""

from .organism import Organism, awaken
from .state_manager import OrganismState, StateLayer

__all__ = [
    "Organism",
    "awaken",
    "OrganismState",
    "StateLayer",
]
