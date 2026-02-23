"""CYNIC Organism — 10-Layer Architecture

Layers operate in precedence order:
0. Identity (axioms, immutable DNA)
1. Unified Will (judgment engine)
2. Organs (dogs)
3. Nervous System (events)
4. Memory (storage)
5. Learning (feedback loops)
6. Autonomy (tiers)
7. Embodiment (boundaries)
8. Self-Knowledge (introspection)
9. Immune (veto)
10. Perception (senses)

Each layer has:
- __init__(...)
- public methods (no underscore)
- test suite (passes in isolation)
- clear boundary (no cross-layer direct calls)

This __init__.py is the Organism's PUBLIC API.
What the outside world sees.
"""

# Layer 0: Identity (IMPLEMENTED ✅)
from .identity import OrganismIdentity

# Layer 1: Judgment Engine (IMPLEMENTED ✅)
from .judgment_engine import JudgmentEngine, DogInput, UnifiedJudgment

# Future layers (coming Week 3-6)
# from .judgment_engine import JudgmentEngine
# from .organs import OrganSystem
# from .nervous_system import NervousSystem
# from .memory import MemorySubstrate
# from .learning_loop import LearningLoop
# from .autonomy import AutonomyTiers
# from .embodiment import Embodiment
# from .self_knowledge import SelfKnowledge
# from .immune import ImmuneSystem
# from .perception import PerceptionUnifier

__all__ = [
    "OrganismIdentity",
    "JudgmentEngine",
    "DogInput",
    "UnifiedJudgment",
]
