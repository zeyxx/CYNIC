"""
Empirical Event Bus Integration Tests â€” ResidualDetector (NO MOCKS)

Verifies that the ResidualDetector correctly listens to real events
on the real EventBus and updates its internal state.
"""

import asyncio
import pytest
from cynic.kernel.core.event_bus import CoreEvent, Event
from cynic.kernel.organism.brain.cognition.cortex.residual import ResidualDetector

@pytest.fixture
def detector():
    """Create a fresh detector."""
    return ResidualDetector()

@pytest.mark.asyncio
async def test_detector_integration_empirical(detector):
    """Verify that detector processes real JUDGMENT_CREATED events."""
    detector.start() # Subscribes to real global core bus
    bus = get_core_bus("DEFAULT")
    
    # 1. EMIT real event
    # We need at least 2 dog votes for the detector to calculate stdev
    payload = {
        "judgment_id": "test_j1",
        "dog_votes": {"SAGE": 80.0, "ANALYST": 20.0}
    }
    
    await bus.emit(Event.typed(
        CoreEvent.JUDGMENT_CREATED,
        payload,
        source="test_harness"
    ))
    
    # 2. WAIT for async processing
    await asyncio.sleep(0.2)
    
    # 3. VERIFY
    stats = detector.stats()
    assert stats["history_size"] == 1
    assert stats["current_residual"] > 0

@pytest.mark.asyncio
async def test_detector_high_entropy_signal(detector):
    """Verify that detector emits RESIDUAL_HIGH after enough stable high variance."""
    detector.start()
    bus = get_core_bus("DEFAULT")
    
    # Capture high residual events
    captured = []
    bus.on(CoreEvent.RESIDUAL_HIGH, lambda e: captured.append(e))
    
    # Emit events with high variance (stdev > 38.2)
    # 80 and 20 -> stdev is ~42
    payload = {"dog_votes": {"SAGE": 80.0, "ANALYST": 20.0}}
    
    # We need RESIDUAL_STABLE_HIGH_N events (usually 5)
    from cynic.kernel.core.formulas import RESIDUAL_STABLE_HIGH_N
    
    for _ in range(RESIDUAL_STABLE_HIGH_N * 2):
        await bus.emit(Event.typed(CoreEvent.JUDGMENT_CREATED, payload, source="test"))
        await asyncio.sleep(0.01)
        
    assert len(captured) > 0
    assert captured[0].type == CoreEvent.RESIDUAL_HIGH
