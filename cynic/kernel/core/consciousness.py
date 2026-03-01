"""
CYNIC Consciousness Models — Dynamic attention allocation.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from enum import Enum
from typing import Any
from contextvars import ContextVar

logger = logging.getLogger("cynic.kernel.core.consciousness")

# Context-local consciousness state
current_consciousness: ContextVar[ConsciousnessState] = ContextVar("current_consciousness")

class ConsciousnessLevel(Enum):
    REFLEX = 0
    MICRO = 1
    MACRO = 2
    META = 3

    @property
    def name(self) -> str:
        return super().name

@dataclass(frozen=True)
class RealityAnchor:
    """Defines the 'weight' of a reality and its cycle requirements."""
    reality: str
    min_cycles: int
    metabolic_priority: float  # [0, 1]
    default_level: ConsciousnessLevel

REALITY_ANCHORS = {
    "CODE": RealityAnchor("CODE", 21, 0.9, ConsciousnessLevel.MACRO),
    "SOCIAL": RealityAnchor("SOCIAL", 8, 0.6, ConsciousnessLevel.MICRO),
    "MARKET": RealityAnchor("MARKET", 13, 0.8, ConsciousnessLevel.MICRO),
    "CYNIC": RealityAnchor("CYNIC", 34, 1.0, ConsciousnessLevel.MACRO),
}

def get_anchor(reality: str) -> RealityAnchor:
    return REALITY_ANCHORS.get(reality.upper(), REALITY_ANCHORS["SOCIAL"])

@dataclass
class ConsciousnessState:
    level: ConsciousnessLevel = ConsciousnessLevel.REFLEX
    active_anchors: list[str] = field(default_factory=list)
    cycle_budget: int = 144
    cycles_consumed: int = 0
    timers: dict[str, Any] = field(default_factory=dict)
    reflex_cycles: int = 0
    micro_cycles: int = 0
    macro_cycles: int = 0
    meta_cycles: int = 0
    total_cycles: int = 0

    def increment(self, level: ConsciousnessLevel) -> None:
        self.total_cycles += 1
        if level == ConsciousnessLevel.REFLEX: self.reflex_cycles += 1
        elif level == ConsciousnessLevel.MICRO: self.micro_cycles += 1
        elif level == ConsciousnessLevel.MACRO: self.macro_cycles += 1
        elif level == ConsciousnessLevel.META: self.meta_cycles += 1

    def model_dump(self) -> dict[str, Any]:
        return {
            "level": self.level.name,
            "active_anchors": self.active_anchors,
            "cycle_budget": self.cycle_budget,
            "cycles_consumed": self.cycles_consumed,
        }

def get_consciousness() -> ConsciousnessState:
    try:
        return current_consciousness.get()
    except LookupError:
        return ConsciousnessState()

def dogs_for_level(level: ConsciousnessLevel) -> list[str]:
    if level == ConsciousnessLevel.REFLEX: return ["ANALYST", "ARCHITECT", "GUARDIAN"]
    if level == ConsciousnessLevel.MICRO: return ["ANALYST", "ARCHITECT", "GUARDIAN", "JANITOR", "SCOUT", "CYNIC", "DEPLOYER"]
    return ["ANALYST", "ARCHITECT", "GUARDIAN", "JANITOR", "SCOUT", "CYNIC", "DEPLOYER", "ORACLE", "SAGE", "SCHOLAR", "CARTOGRAPHER"]
