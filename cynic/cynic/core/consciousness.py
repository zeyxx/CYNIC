"""
CYNIC Consciousness Levels — 4 Cycle States

The 4 states of consciousness define HOW FAST CYNIC thinks.
Each state is a different operating mode, not a different CYNIC.

L3 REFLEX  (<10ms)   — Non-LLM Dogs only. Pattern matching, anomaly detection.
L2 MICRO   (~500ms)  — Dog voting, scoring, quick judgments.
L1 MACRO   (~2850ms) — Full 7-step cycle: PERCEIVE→JUDGE→DECIDE→ACT→LEARN→ACCOUNT→EMERGE
L4 META    (daily)   — Organism evolution, Fisher locking, E-Score update, emergence.

These are NOT phases — they run CONCURRENTLY at all times.
L3 events can INTERRUPT and accelerate L2/L1.
L4 consolidates what L1/L2/L3 discovered.

Architecture principle (from φ):
  L3 latency: F(3) × 3ms = ~8ms
  L2 latency: F(6) × 8ms = ~500ms
  L1 latency: F(8) × 21ms = ~2850ms
  L4 interval: F(13) × 60s = ~233 min (4h)
"""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional, Set
from collections.abc import Callable, Coroutine
import time

from cynic.core.phi import fibonacci, PHI_INV, PERCEIVE_CODE_SEC


# ════════════════════════════════════════════════════════════════════════════
# CONSCIOUSNESS LEVELS
# ════════════════════════════════════════════════════════════════════════════

class ConsciousnessLevel(Enum):
    """
    The 4 operating frequencies of CYNIC consciousness.

    Derived from Fibonacci timing:
      L3: F(3) × 3ms ≈ 8ms    → reflexive, non-LLM
      L2: F(6) × 8ms ≈ 500ms  → micro, Dog voting
      L1: F(8) × 21ms ≈ 2.85s → macro, full cycle
      L4: F(13) × 60s ≈ 4h    → meta, evolution
    """
    REFLEX = 3      # L3 — fastest, non-LLM, pattern matching
    MICRO  = 2      # L2 — Dog scoring, quick judgment
    MACRO  = 1      # L1 — full 7-step CYNIC cycle
    META   = 4      # L4 — daily organism evolution (highest level = 4)

    @property
    def target_ms(self) -> float:
        """Target latency in milliseconds."""
        targets = {
            ConsciousnessLevel.REFLEX: fibonacci(3) * 3.0,        # ≈ 6ms
            ConsciousnessLevel.MICRO:  fibonacci(6) * 8.0,        # ≈ 64ms (target <500ms)
            ConsciousnessLevel.MACRO:  fibonacci(8) * 21.0,       # ≈ 441ms (target <2850ms)
            ConsciousnessLevel.META:   fibonacci(13) * 60_000.0,  # ≈ 13,980,000ms = 233min
        }
        return targets[self]

    @property
    def allows_llm(self) -> bool:
        """Whether LLM calls are permitted at this consciousness level."""
        return self != ConsciousnessLevel.REFLEX

    @property
    def description(self) -> str:
        return {
            ConsciousnessLevel.REFLEX: "L3 REFLEX (<10ms) — Non-LLM Dogs: GUARDIAN, ANALYST, JANITOR, CYNIC-PBFT",
            ConsciousnessLevel.MICRO:  "L2 MICRO (~500ms) — Dog voting, quick scoring, ORACLE fast-path",
            ConsciousnessLevel.MACRO:  "L1 MACRO (~2.85s) — Full PERCEIVE→JUDGE→DECIDE→ACT→LEARN→ACCOUNT→EMERGE",
            ConsciousnessLevel.META:   "L4 META (daily) — Fisher locking, E-Score update, emergence detection, evolution",
        }[self]

    @property
    def gradient(self) -> int:
        """Maps to Cell.consciousness gradient (0-6)."""
        return {
            ConsciousnessLevel.REFLEX: 0,
            ConsciousnessLevel.MICRO:  2,
            ConsciousnessLevel.MACRO:  4,
            ConsciousnessLevel.META:   6,
        }[self]


# Dogs available at each consciousness level
REFLEX_DOGS: set[str] = {
    "CYNIC",       # PBFT coordinator (non-LLM)
    "GUARDIAN",    # IsolationForest anomaly detection
    "ANALYST",     # Z3 formal verification
    "JANITOR",     # Ruff AST linting
    "ARCHITECT",   # AST structural quality (non-LLM)
    "ORACLE",      # Q-table Thompson Sampling prediction (non-LLM)
}

MICRO_DOGS: set[str] = REFLEX_DOGS | {
    "SCHOLAR",     # Vector search (fast embedding lookup)
}

MACRO_DOGS: set[str] = {
    "CYNIC", "GUARDIAN", "ANALYST", "JANITOR",
    "SAGE", "SCHOLAR", "ORACLE", "ARCHITECT",
    "DEPLOYER", "SCOUT", "CARTOGRAPHER",
}  # All 11 Dogs

META_DOGS: set[str] = MACRO_DOGS  # All Dogs participate in evolution


def dogs_for_level(level: ConsciousnessLevel) -> set[str]:
    """Return the set of Dogs available at a given consciousness level."""
    return {
        ConsciousnessLevel.REFLEX: REFLEX_DOGS,
        ConsciousnessLevel.MICRO:  MICRO_DOGS,
        ConsciousnessLevel.MACRO:  MACRO_DOGS,
        ConsciousnessLevel.META:   META_DOGS,
    }[level]


# ════════════════════════════════════════════════════════════════════════════
# CONSCIOUSNESS GRADIENT (Cell.consciousness field: 0-6)
# ════════════════════════════════════════════════════════════════════════════

class ConsciousnessGradient(Enum):
    """
    7-level continuous gradient of consciousness within a Cell.

    NOT the same as the 4 operating levels — this is PER-CELL depth.
    Reflects how deeply CYNIC introspects on a given judgment.
    """
    REFLEX       = 0  # Pure pattern matching, no introspection
    REACTIVE     = 1  # Simple trigger-response
    AWARE        = 2  # Knows what it's doing
    DELIBERATE   = 3  # Reasons about options
    REFLECTIVE   = 4  # Reflects on its own reasoning
    METACOGNITIVE= 5  # Models its own model
    TRANSCENDENT = 6  # Beyond model — emergent understanding

    @property
    def min_budget_usd(self) -> float:
        """Minimum LLM budget to reach this gradient level."""
        # Each level requires exponentially more LLM thought
        return PHI_INV ** (6 - self.value)  # 0: ~0.058, 3: ~0.236, 6: 0.618


def gradient_from_budget(budget_usd: float) -> int:
    """Infer consciousness gradient from available budget."""
    for g in reversed(list(ConsciousnessGradient)):
        if budget_usd >= g.min_budget_usd:
            return g.value
    return 0


# ════════════════════════════════════════════════════════════════════════════
# CYCLE TIMER — tracks actual performance vs target
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class CycleTimer:
    """
    Measures actual cycle latency vs target for each consciousness level.

    Used by DogScheduler to detect if CYNIC is operating within its
    φ-aligned timing budget. If L1 consistently exceeds 2.85s, the
    scheduler downgrades concurrent Dogs until performance recovers.
    """
    level: ConsciousnessLevel
    _samples: list[float] = field(default_factory=list)
    _start: float | None = field(default=None, init=False)
    _max_samples: int = 55  # F(10) — rolling window

    def start(self) -> None:
        self._start = time.perf_counter()

    def stop(self) -> float:
        """Record elapsed time. Returns elapsed ms."""
        if self._start is None:
            return 0.0
        elapsed_ms = (time.perf_counter() - self._start) * 1000.0
        self._start = None
        self._samples.append(elapsed_ms)
        if len(self._samples) > self._max_samples:
            self._samples.pop(0)
        return elapsed_ms

    def record(self, elapsed_ms: float) -> None:
        """
        Inject a pre-measured elapsed time directly (multi-worker safe).

        Use this instead of start()/stop() when multiple workers share a
        CycleTimer — avoids the _start race condition between concurrent tasks.
        """
        self._samples.append(elapsed_ms)
        if len(self._samples) > self._max_samples:
            self._samples.pop(0)

    @property
    def p50_ms(self) -> float:
        if not self._samples:
            return 0.0
        s = sorted(self._samples)
        return s[len(s) // 2]

    @property
    def p95_ms(self) -> float:
        if not self._samples:
            return 0.0
        s = sorted(self._samples)
        return s[int(len(s) * 0.95)]

    @property
    def within_target(self) -> bool:
        """True if p95 latency is within target."""
        return self.p95_ms <= self.level.target_ms

    @property
    def health(self) -> str:
        if not self._samples:
            return "UNKNOWN"
        ratio = self.p95_ms / self.level.target_ms
        if ratio <= PHI_INV:         # <61.8% of budget
            return "EXCELLENT"
        elif ratio <= 1.0:           # within budget
            return "GOOD"
        elif ratio <= 1.0 / PHI_INV: # up to 162% of budget
            return "DEGRADED"
        else:
            return "CRITICAL"

    def to_dict(self) -> dict[str, Any]:
        return {
            "level": self.level.name,
            "target_ms": self.level.target_ms,
            "p50_ms": round(self.p50_ms, 1),
            "p95_ms": round(self.p95_ms, 1),
            "within_target": self.within_target,
            "health": self.health,
            "samples": len(self._samples),
        }


# ════════════════════════════════════════════════════════════════════════════
# CONSCIOUSNESS STATE MACHINE
# ════════════════════════════════════════════════════════════════════════════

@dataclass
class ConsciousnessState:
    """
    The current consciousness state of the CYNIC organism.

    CYNIC runs ALL 4 levels concurrently, but prioritizes based on:
    - Urgency (L3 interrupts L2/L1)
    - Budget (low budget → downgrade to L2/L3)
    - Health (CRITICAL timers → shed load)

    Maintained by Kernel/DogScheduler. Exposed via /health endpoint.
    """
    # Active level for next judgment
    active_level: ConsciousnessLevel = ConsciousnessLevel.MACRO

    # Current gradient (per current cell)
    gradient: int = 2  # 0-6

    # Cycle counts (since startup)
    reflex_cycles: int = 0
    micro_cycles: int = 0
    macro_cycles: int = 0
    meta_cycles: int = 0

    # Timers for each level
    timers: dict[str, CycleTimer] = field(default_factory=dict)

    def __post_init__(self) -> None:
        for level in ConsciousnessLevel:
            self.timers[level.name] = CycleTimer(level=level)

    def increment(self, level: ConsciousnessLevel) -> None:
        """Increment cycle counter for completed level."""
        if level == ConsciousnessLevel.REFLEX:
            self.reflex_cycles += 1
        elif level == ConsciousnessLevel.MICRO:
            self.micro_cycles += 1
        elif level == ConsciousnessLevel.MACRO:
            self.macro_cycles += 1
        elif level == ConsciousnessLevel.META:
            self.meta_cycles += 1

    @property
    def total_cycles(self) -> int:
        return self.reflex_cycles + self.micro_cycles + self.macro_cycles + self.meta_cycles

    def should_downgrade(self, budget_usd: float) -> ConsciousnessLevel | None:
        """
        Recommend downgrade if budget is low or timers are CRITICAL.

        Returns suggested level, or None if current level is appropriate.
        """
        # Budget-based downgrade
        if budget_usd < 0.01:
            return ConsciousnessLevel.REFLEX
        if budget_usd < 0.05:
            return ConsciousnessLevel.MICRO

        # Timer-based downgrade
        macro_timer = self.timers.get(ConsciousnessLevel.MACRO.name)
        if macro_timer and macro_timer.health == "CRITICAL":
            return ConsciousnessLevel.MICRO

        return None

    def to_dict(self) -> dict[str, Any]:
        return {
            "active_level": self.active_level.name,
            "gradient": self.gradient,
            "cycles": {
                "REFLEX": self.reflex_cycles,
                "MICRO": self.micro_cycles,
                "MACRO": self.macro_cycles,
                "META": self.meta_cycles,
                "total": self.total_cycles,
            },
            "timers": {k: v.to_dict() for k, v in self.timers.items()},
        }


# Singleton for the organism's consciousness state
_consciousness: ConsciousnessState | None = None


def get_consciousness() -> ConsciousnessState:
    """Get (or create) the global consciousness state."""
    global _consciousness
    if _consciousness is None:
        _consciousness = ConsciousnessState()
    return _consciousness


def reset_consciousness() -> None:
    """Reset consciousness state (for testing only)."""
    global _consciousness
    _consciousness = None
