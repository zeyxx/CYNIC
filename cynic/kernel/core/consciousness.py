"""
CYNIC Consciousness Models — Dynamic attention allocation.

Levels align with PHI-bounded latency and cost targets.
Now anchored in Reality requirements.
"""
from __future__ import annotations

import logging
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional

logger = logging.getLogger("cynic.kernel.core.consciousness")

class ConsciousnessLevel(Enum):
    """
    Tiers of organism attention and resource allocation.
    """
    REFLEX = 0  # < 50ms, pattern match, 1 cycle
    MICRO  = 1  # < 1s, dog consensus, 5-13 cycles (Fibonacci)
    MACRO  = 2  # < 5s, deep fractal, 21-55 cycles
    META   = 3  # Long-term, self-evolution

    @property
    def name(self) -> str:
        return super().name

@dataclass(frozen=True)
class RealityAnchor:
    """Defines the 'weight' of a reality and its cycle requirements."""
    reality: str
    min_cycles: int
    metabolic_priority: float # [0, 1] (1.0 = high priority)
    default_level: ConsciousnessLevel

# ── ANCHOR DEFINITIONS (The Laws of Context) ──

REALITY_ANCHORS = {
    "CODE": RealityAnchor(
        reality="CODE", 
        min_cycles=21, 
        metabolic_priority=0.9, 
        default_level=ConsciousnessLevel.MACRO
    ),
    "SOCIAL": RealityAnchor(
        reality="SOCIAL", 
        min_cycles=8, 
        metabolic_priority=0.6, 
        default_level=ConsciousnessLevel.MICRO
    ),
    "MARKET": RealityAnchor(
        reality="MARKET", 
        min_cycles=13, 
        metabolic_priority=0.8, 
        default_level=ConsciousnessLevel.MICRO
    ),
    "CYNIC": RealityAnchor(
        reality="CYNIC", 
        min_cycles=34, 
        metabolic_priority=1.0, 
        default_level=ConsciousnessLevel.MACRO
    ),
}

def get_anchor(reality: str) -> RealityAnchor:
    """Get anchor for a reality, fallback to SOCIAL."""
    return REALITY_ANCHORS.get(reality.upper(), REALITY_ANCHORS["SOCIAL"])

@dataclass
class ConsciousnessState:
    """Current state of organism attention."""
    level: ConsciousnessLevel = ConsciousnessLevel.REFLEX
    active_anchors: List[str] = field(default_factory=list)
    cycle_budget: int = 144 # F(12) — Total available cycles per session
    cycles_consumed: int = 0
    
    def can_afford(self, level: ConsciousnessLevel, anchor: RealityAnchor) -> bool:
        """Check if metabolic capacity allows this level of thinking."""
        required = anchor.min_cycles
        if level == ConsciousnessLevel.MACRO:
            required *= 2
        return (self.cycle_budget - self.cycles_consumed) >= required

    def consume(self, count: int):
        self.cycles_consumed += count
        logger.debug("Consciousness: consumed %d cycles (Total: %d)", count, self.cycles_consumed)

    def increment(self, level: ConsciousnessLevel) -> None:
        """Backward compatibility: convert level to cycle cost."""
        costs = {
            ConsciousnessLevel.REFLEX: 1,
            ConsciousnessLevel.MICRO: 5,
            ConsciousnessLevel.MACRO: 21,
            ConsciousnessLevel.META: 55,
        }
        self.consume(costs.get(level, 1))

# Global singleton
_conscious_state: Optional[ConsciousnessState] = None

def get_consciousness() -> ConsciousnessState:
    """Get the global consciousness state singleton."""
    global _conscious_state
    if _conscious_state is None:
        _conscious_state = ConsciousnessState()
    return _conscious_state

def dogs_for_level(level: ConsciousnessLevel) -> list[str]:
    """
    Returns IDs of dogs active at a given level.
    
    REFLEX: Minimal heuristic dogs.
    MICRO: Standard 7 dogs (The Menorah).
    MACRO: All 11 dogs (The Tree of Life).
    """
    if level == ConsciousnessLevel.REFLEX:
        return ["ANALYST", "ARCHITECT", "GUARDIAN"]
    
    if level == ConsciousnessLevel.MICRO:
        return ["ANALYST", "ARCHITECT", "GUARDIAN", "JANITOR", "SCOUT", "CYNIC", "DEPLOYER"]
        
    # MACRO/META: The Full Tree
    return [
        "ANALYST", "ARCHITECT", "GUARDIAN", "JANITOR", "SCOUT", 
        "CYNIC", "DEPLOYER", "ORACLE", "SAGE", "SCHOLAR", "CARTOGRAPHER"
    ]
