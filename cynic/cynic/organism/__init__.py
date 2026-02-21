"""CYNIC Organism — Living System Architecture

The organism is a unified system with specialized organs coordinating via events.

Structure:
├── layers/               Layer 0-10 (Identity, Judgment, Organs, Nervous, Memory, Learning, Autonomy, Embodiment, Self-Knowledge, Immune, Perception)
├── brain/                Central nervous system (orchestrator, judge)
├── motor/                Action execution (CLI, SDK, runners)
├── nervous/              Event buses and nervous system
├── memory/               Storage and persistence
├── metabolism/           Resource accounting and budgets
├── immune/               Safety gates and veto system
├── perception/           Sensory input (code, git, social, market, solana)
├── sensory/              Raw sensor implementations
├── actuators/            Action executors (runners, clients)
├── conscious_state.py    Read-only state interface (Phase 1)
└── README.md             Organism anatomy documentation

Public API:
- conscious_state.get_conscious_state()  → ConsciousState singleton
- conscious_state.ConsciousState         → Read-only state interface
- layers.OrganismIdentity                → Axiom constraints (Layer 0)
- layers.JudgmentEngine                  → Unified will (Layer 1)

The organism is:
- SOURCE OF TRUTH for state
- AUTONOMOUS (runs independently)
- EVENT-DRIVEN (updates via events, not API calls)
- OBSERVABLE (API queries ConsciousState, doesn't control organism)
"""

from .conscious_state import ConsciousState, get_conscious_state
from .layers import OrganismIdentity, JudgmentEngine, DogInput, UnifiedJudgment

__all__ = [
    "ConsciousState",
    "get_conscious_state",
    "OrganismIdentity",
    "JudgmentEngine",
    "DogInput",
    "UnifiedJudgment",
]
