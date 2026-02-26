"""Unified state manager collecting from all three entities: CYNIC, Human, Machine.

SymbioticStateManager aggregates observations from:
- HumanStateTracker: Energy, focus, intentions, feedback, values, growth areas
- MachineMonitor: CPU, memory, disk, network, temperature, health
- CYNIC Core: Observations, thinking, planning, confidence, E-Score

Returns a unified, immutable SymbioticState snapshot on demand.
Provides global singleton instance and convenience functions.
"""
from __future__ import annotations

import asyncio
import logging
import time
from typing import Optional

from cynic.observability.human_state_tracker import HumanStateTracker
from cynic.observability.machine_monitor import MachineMonitor
from cynic.observability.models import SymbioticState

logger = logging.getLogger(__name__)

# Global singleton instance
_INSTANCE: Optional[SymbioticStateManager] = None
_LOCK = asyncio.Lock()


class SymbioticStateManager:
    """Collects and unifies state from Human, Machine, and CYNIC.

    Aggregates real-time observations from:
    - HumanStateTracker (energy, focus, intentions, feedback)
    - MachineMonitor (resources, health, constraints)
    - CYNIC Core (observations, thinking, planning, confidence, E-Score)

    Returns immutable SymbioticState snapshots representing the current
    state of the symbiosis at a point in time.

    All returned states are frozen dataclasses (immutable) with a timestamp.
    """

    def __init__(
        self,
        human_tracker: HumanStateTracker,
        machine_monitor: MachineMonitor,
    ):
        """Initialize SymbioticStateManager.

        Args:
            human_tracker: HumanStateTracker instance for collecting human state.
            machine_monitor: MachineMonitor instance for collecting machine state.
        """
        self._human_tracker = human_tracker
        self._machine_monitor = machine_monitor

    async def get_current_state(self) -> SymbioticState:
        """Collect and return current unified SymbioticState.

        Gathers data from all three entities (Human, Machine, CYNIC) and
        computes alignment score. Returns immutable snapshot.

        Returns:
            SymbioticState: Immutable unified state with timestamp.
        """
        # Collect human state
        human_state = await self._human_tracker.get_state()

        # Collect machine state
        machine_state = await self._machine_monitor.get_state()

        # Collect CYNIC state (placeholder for now)
        cynic_observations = self._get_cynic_observations()
        cynic_thinking = self._get_cynic_thinking()
        cynic_planning = self._get_cynic_planning()
        cynic_confidence = self._get_cynic_confidence()
        cynic_e_score = self._get_cynic_e_score()

        # Calculate alignment
        alignment = self._calculate_alignment(human_state, machine_state)

        # Detect machine constraints
        machine_constraints = await self._machine_monitor.detect_constraints()
        machine_capability_delta = self._get_machine_capability_delta()

        # Detect conflicts
        conflicts = self._detect_conflicts(human_state, machine_state)

        # Get mutual influences
        mutual_influences = self._get_mutual_influences()

        # Get shared objectives
        shared_objectives = self._get_shared_objectives(human_state)

        # Create and return immutable SymbioticState
        return SymbioticState(
            cynic_observations=cynic_observations,
            cynic_thinking=cynic_thinking,
            cynic_planning=cynic_planning,
            cynic_confidence=cynic_confidence,
            cynic_e_score=cynic_e_score,
            human_energy=human_state.energy,
            human_focus=human_state.focus,
            human_intentions=human_state.intentions,
            human_values=human_state.values,
            human_feedback=human_state.feedback,
            human_growth_areas=human_state.growth_areas,
            machine_resources={
                'cpu_percent': machine_state.cpu_percent,
                'memory_percent': machine_state.memory_percent,
                'disk_percent': machine_state.disk_percent,
                'network_bandwidth': machine_state.network_bandwidth,
                'temperature': machine_state.temperature,
            },
            machine_constraints={
                'warnings': machine_constraints,
            },
            machine_capability_delta=machine_capability_delta,
            machine_health=machine_state.health,
            alignment_score=alignment,
            conflicts=conflicts,
            mutual_influences=mutual_influences,
            shared_objectives=shared_objectives,
            timestamp=time.time(),
        )

    def _calculate_alignment(self, human_state, machine_state) -> float:
        """Calculate overall symbiotic alignment score [0, 1].

        Considers:
        - Human energy and focus (higher is better, max 10)
        - Machine health (all health indicators)
        - Resource availability
        - Absence of conflicts

        Returns:
            float: Alignment score [0.0, 1.0]
        """
        # Human component: average of energy and focus normalized to [0, 1]
        human_normalized = (human_state.energy + human_state.focus) / 20.0

        # Machine component: check health indicators
        machine_health_score = 0.0
        if machine_state.health.get('is_healthy', False):
            machine_health_score = 1.0
        else:
            # Partial score based on individual indicators
            health_checks = [
                machine_state.health.get('cpu_ok', False),
                machine_state.health.get('memory_ok', False),
                machine_state.health.get('disk_ok', False),
            ]
            machine_health_score = sum(health_checks) / len(health_checks)

        # Resource availability component: inverse of utilization
        # Good alignment = resources available (low utilization)
        cpu_available = 1.0 - (machine_state.cpu_percent / 100.0)
        memory_available = 1.0 - (machine_state.memory_percent / 100.0)
        disk_available = 1.0 - (machine_state.disk_percent / 100.0)
        resource_score = (cpu_available + memory_available + disk_available) / 3.0

        # Combined alignment (weighted average)
        # Prioritize: machine health (40%) > resources (30%) > human (30%)
        alignment = (
            machine_health_score * 0.4
            + resource_score * 0.3
            + human_normalized * 0.3
        )

        # Clamp to [0, 1]
        return max(0.0, min(1.0, alignment))

    def _detect_conflicts(self, human_state, machine_state) -> list[str]:
        """Detect conflicts or misalignments between entities.

        Returns:
            list[str]: List of conflict descriptions (empty if none).
        """
        conflicts = []

        # Low human energy + high machine load = conflict
        if human_state.energy < 3.0 and machine_state.cpu_percent > 70:
            conflicts.append(
                "Human energy depleted while machine is under heavy load"
            )

        # Low focus + complex intentions = conflict
        if (
            human_state.focus < 3.0
            and len(human_state.intentions) > 0
            and len(human_state.intentions) > 3
        ):
            conflicts.append("Low focus with excessive concurrent objectives")

        # Machine constraints + human intentions = conflict
        if machine_state.memory_percent > 80 and len(human_state.intentions) > 0:
            conflicts.append("Memory pressure prevents executing human intentions")

        return conflicts

    def _get_mutual_influences(self) -> list[tuple]:
        """Get mutual influences between the three entities.

        Returns:
            list[tuple]: List of (source, target, influence_type) tuples.
        """
        influences = []

        # CYNIC influences are not yet implemented in detail
        # Placeholder for future expansion

        return influences

    def _get_shared_objectives(self, human_state) -> list[str]:
        """Get objectives shared across all three entities.

        Returns:
            list[str]: List of shared objectives.
        """
        # For now, human's intentions are treated as shared objectives
        # This will be enhanced as CYNIC planning and machine optimization
        # capabilities become more sophisticated
        return human_state.intentions.copy()

    def _get_machine_capability_delta(self) -> list[str]:
        """Get recent changes in machine capabilities.

        Returns:
            list[str]: List of capability changes (empty for now).
        """
        # Placeholder: would track transitions (e.g., new GPU detected)
        return []

    def _get_cynic_observations(self) -> dict:
        """Get CYNIC's current observations.

        Returns:
            dict: CYNIC observations (placeholder).
        """
        return {
            'system_health': 'nominal',
            'attention_level': 'standard',
            'reasoning_depth': 'moderate',
        }

    def _get_cynic_thinking(self) -> str:
        """Get CYNIC's current thinking or analysis.

        Returns:
            str: CYNIC's thinking summary.
        """
        return "Monitoring symbiotic equilibrium across Human-Machine-CYNIC domains"

    def _get_cynic_planning(self) -> list[str]:
        """Get CYNIC's current planning items.

        Returns:
            list[str]: Current planning items.
        """
        return [
            "Maintain optimal resource allocation",
            "Monitor human-machine alignment",
            "Detect and prevent conflicts",
        ]

    def _get_cynic_confidence(self) -> float:
        """Get CYNIC's confidence level.

        Returns:
            float: Confidence [0.0, 1.0].
        """
        # Placeholder: would be derived from actual CYNIC uncertainty
        return 0.75

    def _get_cynic_e_score(self) -> float:
        """Get CYNIC's E-Score (energy/consciousness metric).

        Returns:
            float: E-Score [0.0, 1.0] or higher depending on scale.
        """
        # Placeholder: would connect to actual E-Score tracker
        # For now returns a reasonable default
        return 0.618  # Golden ratio as default


def get_state_manager() -> SymbioticStateManager:
    """Get or create global singleton SymbioticStateManager instance.

    Creates and caches a global instance on first call.
    Uses the default HumanStateTracker and MachineMonitor.

    Returns:
        SymbioticStateManager: Global singleton instance.
    """
    global _INSTANCE
    if _INSTANCE is None:
        _INSTANCE = SymbioticStateManager(
            human_tracker=HumanStateTracker(),
            machine_monitor=MachineMonitor(),
        )
    return _INSTANCE


async def get_current_state() -> SymbioticState:
    """Get current unified SymbioticState using global singleton manager.

    Convenience function that uses the global singleton manager
    to get the current state. Equivalent to:
        manager = get_state_manager()
        state = await manager.get_current_state()

    Returns:
        SymbioticState: Current immutable state snapshot.
    """
    manager = get_state_manager()
    return await manager.get_current_state()
