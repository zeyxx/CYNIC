"""Tests for SymbioticStateManager."""
import pytest
import time
from types import MappingProxyType

from cynic.observability.symbiotic_state_manager import (
    SymbioticStateManager,
    get_state_manager,
    get_current_state,
)
from cynic.observability.models import SymbioticState
from cynic.observability.human_state_tracker import HumanStateTracker
from cynic.observability.machine_monitor import MachineMonitor


@pytest.mark.asyncio
async def test_state_manager_initializes():
    """SymbioticStateManager can be created."""
    tracker = HumanStateTracker()
    monitor = MachineMonitor()
    manager = SymbioticStateManager(tracker, monitor)
    assert manager is not None
    assert manager._human_tracker is tracker
    assert manager._machine_monitor is monitor


@pytest.mark.asyncio
async def test_get_current_state_returns_symbiotic_state():
    """get_current_state() returns SymbioticState instance."""
    tracker = HumanStateTracker()
    monitor = MachineMonitor()
    manager = SymbioticStateManager(tracker, monitor)

    state = await manager.get_current_state()

    assert isinstance(state, SymbioticState)
    assert state.timestamp > 0
    assert isinstance(state.cynic_observations, dict)
    assert isinstance(state.cynic_thinking, str)
    assert isinstance(state.cynic_planning, list)
    assert 0 <= state.cynic_confidence <= 1.0
    assert 0 <= state.cynic_e_score <= 1.0
    assert 0 <= state.human_energy <= 10.0
    assert 0 <= state.human_focus <= 10.0
    assert isinstance(state.machine_resources, dict)
    assert isinstance(state.machine_health, dict)
    assert 0 <= state.alignment_score <= 1.0


@pytest.mark.asyncio
async def test_state_is_immutable():
    """SymbioticState is immutable (frozen dataclass)."""
    tracker = HumanStateTracker()
    monitor = MachineMonitor()
    manager = SymbioticStateManager(tracker, monitor)

    state = await manager.get_current_state()

    # Attempt to modify should raise FrozenInstanceError or AttributeError
    with pytest.raises((AttributeError, Exception)):
        state.alignment_score = 0.5  # type: ignore

    with pytest.raises((AttributeError, Exception)):
        state.human_energy = 7.5  # type: ignore


@pytest.mark.asyncio
async def test_global_get_state_manager():
    """get_state_manager() returns singleton instance."""
    manager1 = get_state_manager()
    manager2 = get_state_manager()

    assert manager1 is manager2


@pytest.mark.asyncio
async def test_global_get_current_state():
    """get_current_state() uses global singleton manager."""
    state1 = await get_current_state()
    state2 = await get_current_state()

    assert isinstance(state1, SymbioticState)
    assert isinstance(state2, SymbioticState)
    # States at different times should have different timestamps
    assert state2.timestamp >= state1.timestamp


@pytest.mark.asyncio
async def test_alignment_calculation():
    """Alignment score is calculated from human, machine, and CYNIC factors."""
    tracker = HumanStateTracker()
    monitor = MachineMonitor()
    manager = SymbioticStateManager(tracker, monitor)

    # Set human to good state
    await tracker.set_energy(8.0)
    await tracker.set_focus(8.0)

    state = await manager.get_current_state()

    # Alignment should factor in human energy/focus and machine health
    assert state.alignment_score >= 0.0
    assert state.alignment_score <= 1.0
    # With good human state, alignment should be reasonably high
    assert state.alignment_score > 0.3


@pytest.mark.asyncio
async def test_cynic_state_fields_populated():
    """CYNIC state fields are populated in SymbioticState."""
    tracker = HumanStateTracker()
    monitor = MachineMonitor()
    manager = SymbioticStateManager(tracker, monitor)

    state = await manager.get_current_state()

    # All CYNIC fields should be present
    assert state.cynic_observations is not None
    assert state.cynic_thinking is not None
    assert state.cynic_planning is not None
    assert state.cynic_confidence >= 0.0
    assert state.cynic_e_score >= 0.0


@pytest.mark.asyncio
async def test_human_state_fields_populated():
    """Human state fields are collected and populated."""
    tracker = HumanStateTracker()
    await tracker.set_energy(7.5)
    await tracker.set_focus(8.0)
    await tracker.set_intentions(["test objective 1"])
    await tracker.set_values(["integrity", "learning"])

    monitor = MachineMonitor()
    manager = SymbioticStateManager(tracker, monitor)

    state = await manager.get_current_state()

    assert state.human_energy == 7.5
    assert state.human_focus == 8.0
    assert "test objective 1" in state.human_intentions
    assert "integrity" in state.human_values


@pytest.mark.asyncio
async def test_machine_state_fields_populated():
    """Machine state fields are collected and populated."""
    tracker = HumanStateTracker()
    monitor = MachineMonitor()
    manager = SymbioticStateManager(tracker, monitor)

    state = await manager.get_current_state()

    # All machine fields should be present
    assert 'cpu_percent' in state.machine_resources
    assert 'memory_percent' in state.machine_resources
    assert 'disk_percent' in state.machine_resources
    assert isinstance(state.machine_health, dict)
    assert 'is_healthy' in state.machine_health


@pytest.mark.asyncio
async def test_timestamps_are_recent():
    """State timestamp is recent (within last second)."""
    tracker = HumanStateTracker()
    monitor = MachineMonitor()
    manager = SymbioticStateManager(tracker, monitor)

    before = time.time()
    state = await manager.get_current_state()
    after = time.time()

    assert before <= state.timestamp <= after
