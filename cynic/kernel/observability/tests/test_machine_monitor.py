"""Tests for MachineMonitor."""
import pytest
from cynic.kernel.observability.machine_monitor import MachineMonitor, MachineState


@pytest.mark.asyncio
async def test_machine_monitor_initializes():
    """MachineMonitor can be created."""
    monitor = MachineMonitor()
    assert monitor is not None


@pytest.mark.asyncio
async def test_get_machine_state():
    """Can get current machine state."""
    monitor = MachineMonitor()
    state = await monitor.get_state()

    assert isinstance(state, MachineState)
    assert hasattr(state, 'cpu_percent')
    assert hasattr(state, 'memory_percent')
    assert hasattr(state, 'disk_percent')
    assert hasattr(state, 'network_bandwidth')
    assert hasattr(state, 'temperature')
    assert hasattr(state, 'health')
    assert hasattr(state, 'timestamp')

    # Validate value ranges
    assert 0 <= state.cpu_percent <= 100
    assert 0 <= state.memory_percent <= 100
    assert 0 <= state.disk_percent <= 100
    assert state.network_bandwidth >= 0
    assert isinstance(state.health, dict)
    assert state.timestamp > 0


@pytest.mark.asyncio
async def test_detect_constraints():
    """Can detect machine resource constraints."""
    monitor = MachineMonitor()
    constraints = await monitor.detect_constraints()

    assert isinstance(constraints, list)
    # Constraints should be string messages about resource limitations
    for constraint in constraints:
        assert isinstance(constraint, str)
