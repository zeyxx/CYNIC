"""Data models for symbiotic observability."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class SymbioticState:
    """Unified state of CYNIC + Human + Machine symbiosis.

    Immutable data model representing a snapshot of the symbiotic system
    at a single point in time, combining:
    - CYNIC's awareness (observations, thinking, planning, confidence, E-Score)
    - Human's state (energy, focus, intentions, values, feedback, growth areas)
    - Machine's state (resources, constraints, capability delta, health)
    - Symbiotic relationship metrics (alignment, conflicts, influences, objectives)
    """

    # CYNIC's awareness
    cynic_observations: dict[str, Any]
    """CYNIC's current observations from sensors and reasoning."""

    cynic_thinking: str
    """CYNIC's current thinking or analysis."""

    cynic_planning: list[str]
    """CYNIC's current planning items."""

    cynic_confidence: float
    """CYNIC's confidence level in current state [0.0, 1.0]."""

    cynic_e_score: float
    """CYNIC's E-Score (φ-bounded energy/consciousness metric)."""

    # Human's state
    human_energy: float
    """Human's energy level [0.0, 10.0]."""

    human_focus: float
    """Human's focus level [0.0, 10.0]."""

    human_intentions: list[str]
    """Human's current intentions and goals."""

    human_values: list[str]
    """Human's core values and principles."""

    human_feedback: list[str]
    """Recent feedback from human to CYNIC."""

    human_growth_areas: dict[str, float]
    """Areas where human wants to grow and their focus (0.0-1.0)."""

    # Machine's state
    machine_resources: dict[str, float]
    """Available machine resources (CPU%, RAM%, disk%, etc.)."""

    machine_constraints: dict[str, Any]
    """Current machine constraints and warnings."""

    machine_capability_delta: list[str]
    """Recent changes in machine capabilities."""

    machine_health: dict[str, bool]
    """Machine health indicators (is_healthy, disk_ok, memory_ok, etc.)."""

    # Symbiotic relationship
    alignment_score: float
    """Overall symbiotic alignment score [0.0, 1.0]."""

    conflicts: list[str]
    """Current conflicts or misalignments between CYNIC, Human, and Machine."""

    mutual_influences: list[tuple]
    """Influences between the three entities (who affects what)."""

    shared_objectives: list[str]
    """Objectives shared across all three entities."""

    timestamp: float
    """Unix timestamp of when this state snapshot was captured."""
