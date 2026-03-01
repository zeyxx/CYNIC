"""CYNIC Organism â€” Living System Architecture

The organism is a unified system with specialized organs coordinating via events.

Structure:
â”œâ”€â”€ handlers/             Event handlers (Judgment, Senses, etc.)
â”œâ”€â”€ state_manager.py      Unified state (RAM/DB/File)
â””â”€â”€ organism.py           Central coordinator and factory (awaken)

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
