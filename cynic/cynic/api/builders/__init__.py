"""
CYNIC Organism Builders — Decompose _OrganismAwakener into composable units.

Phase 2B: Break state.py's _OrganismAwakener (987 LOC, 7 big methods) into
8 builders + 1 assembler.

Each builder:
- Creates one logical slice of the organism
- Has explicit dependencies (injected, not looked up)
- Tests in isolation via mocks
- Observable (logs what it's creating)

Pattern:
  Builder.build() → returns created component(s)
  OrganismAssembler.assemble() → composes all builders in correct order

Benefits over monolithic _OrganismAwakener:
- Single Responsibility (each builder owns one concern)
- Testability (mock dependencies, test builder in isolation)
- Composability (different organisms? Mix and match builders)
- Observability (each builder logs what it creates)
- Replaceability (swap ComponentBuilder for MockComponentBuilder in tests)
"""
from __future__ import annotations

from cynic.api.builders.base import BaseBuilder, BuilderContext
from cynic.api.builders.component import ComponentBuilder
from cynic.api.builders.cognition import CognitionBuilder
from cynic.api.builders.metabolic import MetabolicBuilder
from cynic.api.builders.sensory import SensoryBuilder
from cynic.api.builders.memory import MemoryBuilder
from cynic.api.builders.storage import StorageBuilder
from cynic.api.builders.wiring import WiringBuilder
from cynic.api.builders.assembler import OrganismAssembler

__all__ = [
    "BaseBuilder",
    "BuilderContext",
    "ComponentBuilder",
    "CognitionBuilder",
    "MetabolicBuilder",
    "SensoryBuilder",
    "MemoryBuilder",
    "StorageBuilder",
    "WiringBuilder",
    "OrganismAssembler",
]
