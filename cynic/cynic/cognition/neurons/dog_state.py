"""
DogState — Each Dog is a Mini-CYNIC with φ-Explicit Fractal Structure

When CYNIC scales from 1 organism to 11 dogs, each dog needs its own:
  - Cognition (local judgment engine)
  - Metabolism (local action execution)
  - Senses (local perception)
  - Memory (local learning)

DogState mirrors CognitionCore/MetabolicCore/SensoryCore/MemoryCore structure,
but miniaturized: dogs don't need all the guardrails, just the essentials.

This enables:
  1. **Autonomy**: Each dog judges its domain independently
  2. **Gossip**: Dogs exchange compressed context, not raw decisions
  3. **Consensus**: Orchestrator aggregates dog votes (not re-judges)
  4. **Scaling**: Cost ∝ log(N), not N (no orchestrator bottleneck per dog)
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

# ═══════════════════════════════════════════════════════════════════════════
# DOG COGNITION — Mini-BRAIN
# ═══════════════════════════════════════════════════════════════════════════


@dataclass
class DogCognitionState:
    """
    Local judgment engine for one dog.

    Unlike CognitionCore (8 required fields), DogCognition is minimal:
    - Can run PERCEIVE→JUDGE→DECIDE→ACT independently
    - No guardrails (those are organism-level)
    - Own Q-learning for domain-specific patterns
    """
    local_qtable: dict[str, float] = field(default_factory=dict)
    # state_key → Q-value for domain-specific decisions

    judgment_count: int = 0
    # How many judgments this dog has made (local counter)

    confidence_history: list[float] = field(default_factory=list)
    # Recent confidence values (rolling window for stability)

    last_verdict: Optional[str] = None
    # Most recent verdict (BARK/GROWL/WAG/HOWL)

    last_q_score: float = 0.0
    # Most recent Q-score


# ═══════════════════════════════════════════════════════════════════════════
# DOG METABOLISM — Mini-BODY
# ═══════════════════════════════════════════════════════════════════════════


@dataclass
class DogMetabolismState:
    """
    Local action execution for one dog.

    Actions that this specific dog can execute in its domain:
    - Code analysis → suggest refactoring
    - Security check → flag vulnerability
    - Documentation → generate docstring
    """
    pending_actions: list[dict[str, Any]] = field(default_factory=list)
    # Actions queued for execution

    executed_count: int = 0
    # How many actions this dog has executed

    action_latency_ms: list[float] = field(default_factory=list)
    # Recent execution times (for performance tracking)


# ═══════════════════════════════════════════════════════════════════════════
# DOG SENSES — Mini-NERVOUS SYSTEM
# ═══════════════════════════════════════════════════════════════════════════


@dataclass
class DogSensoryState:
    """
    Local perception for one dog's domain.

    Each dog only observes signals relevant to its specialty:
    - SAGE observes knowledge graphs
    - ANALYST observes formal verification signals
    - GUARDIAN observes security anomalies
    """
    observed_signals: list[dict[str, Any]] = field(default_factory=list)
    # Domain-specific signals this dog has observed

    signal_count: int = 0
    # How many signals processed

    compressed_context: str = ""
    # Compressed summary of recent observations (for gossip protocol)

    context_timestamp: float = 0.0
    # When was this context last updated


# ═══════════════════════════════════════════════════════════════════════════
# DOG MEMORY — Mini-ARCHIVE
# ═══════════════════════════════════════════════════════════════════════════


@dataclass
class DogMemoryState:
    """
    Local learning and reflection for one dog.

    Dogs remember:
    - What patterns worked in their domain
    - What they were wrong about (residuals)
    - Who they've talked to (gossip history)
    """
    learned_patterns: dict[str, float] = field(default_factory=dict)
    # pattern_name → effectiveness_score

    residual_cases: list[dict[str, Any]] = field(default_factory=list)
    # Cases where this dog's judgment was wrong (for learning)

    gossip_peers: set[str] = field(default_factory=set)
    # Other dogs this dog exchanges context with

    trust_scores: dict[str, float] = field(default_factory=dict)
    # peer_dog_id → how much we trust their judgments


# ═══════════════════════════════════════════════════════════════════════════
# DOGSTATE — COMPLETE MINI-CYNIC
# ═══════════════════════════════════════════════════════════════════════════


@dataclass
class DogState:
    """
    Complete fractal state for one dog.

    Each of 11 dogs has its own DogState, mirroring the 4-façade organism structure:
      - cognition: DogCognitionState (independent judgment)
      - metabolism: DogMetabolismState (independent action)
      - senses: DogSensoryState (domain perception)
      - memory: DogMemoryState (learning from domain)

    Dogs run their 7-step cycle (PERCEIVE→JUDGE→DECIDE→ACT→LEARN→RESIDUAL→EVOLVE)
    independently and in parallel, then gossip summaries with siblings.

    φ-Explicit: Structure mirrors organism's 4 façades.
    Fractal: Each dog is autonomous, not dependent on orchestrator for decisions.
    Gossip: Dogs exchange compressed_context + verdict, not raw observations.
    """
    dog_id: str
    # Which dog this is (SAGE, ANALYST, GUARDIAN, etc.)

    cognition: DogCognitionState = field(default_factory=DogCognitionState)
    metabolism: DogMetabolismState = field(default_factory=DogMetabolismState)
    senses: DogSensoryState = field(default_factory=DogSensoryState)
    memory: DogMemoryState = field(default_factory=DogMemoryState)

    created_at: float = field(default_factory=lambda: __import__("time").time())
    last_judgment_at: float = 0.0
    # Timestamps for lifecycle tracking

    is_active: bool = True
    # Can this dog judge, or is it paused?

    def reset_for_testing(self) -> None:
        """Clear all state for testing (unit test cleanup)."""
        self.cognition = DogCognitionState()
        self.metabolism = DogMetabolismState()
        self.senses = DogSensoryState()
        self.memory = DogMemoryState()
        self.last_judgment_at = 0.0

    def to_dict(self) -> dict[str, Any]:
        """Serialize for storage or gossip."""
        return {
            "dog_id": self.dog_id,
            "cognition": {
                "judgment_count": self.cognition.judgment_count,
                "last_verdict": self.cognition.last_verdict,
                "last_q_score": round(self.cognition.last_q_score, 3),
            },
            "metabolism": {
                "executed_count": self.metabolism.executed_count,
            },
            "memory": {
                "gossip_peers": list(self.memory.gossip_peers),
            },
            "is_active": self.is_active,
            "last_judgment_at": self.last_judgment_at,
        }

    @property
    def uptime_s(self) -> float:
        """How long has this dog been alive?"""
        import time
        return time.time() - self.created_at
