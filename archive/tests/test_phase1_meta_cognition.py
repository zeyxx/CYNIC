"""
Phase 1: Meta-Cognition Integration Tests
==========================================

Verify that:
1. MetaCognitionHandler initializes and subscribes to SONA_TICK
2. Meta-cognition responds to organism heartbeats
3. Learning rate adjustments work
4. Health trend analysis is accurate
5. Stagnation detection works

Integration test (real components, not mocked).
"""
import asyncio

import pytest

from cynic.kernel.core.event_bus import CoreEvent
from cynic.kernel.core.events_schema import SonaTickPayload
from cynic.kernel.organism.organism import awaken


@pytest.mark.asyncio
async def test_phase1_meta_cognition_initialized():
    """Test that meta-cognition handler is created and listening."""
    organism = awaken()

    # Verify handler exists
    assert hasattr(organism, 'meta_cognition'), "organism should have meta_cognition attribute"
    assert organism.meta_cognition is not None
    assert organism.meta_cognition.handler_id == "meta_cognition"


@pytest.mark.asyncio
async def test_phase1_meta_cognition_responds_to_sona():
    """Test that meta-cognition handler executes on SONA_TICK."""
    organism = awaken()
    handler = organism.meta_cognition

    # Manually emit a SONA_TICK to test handler response
    bus = get_core_bus("DEFAULT")

    # (We'd capture handler activity here once logging is integrated)

    # Emit a test SONA_TICK payload
    test_payload = SonaTickPayload(
        instance_id="test",
        q_table_entries=50,
        total_judgments=100,
        learning_rate=0.15,
        ewc_consolidated=10,
        uptime_s=120.0,
        interval_s=2040.0,
        tick_number=1,
    )

    # Emit the event
    from cynic.kernel.core.event_bus import Event
    await bus.emit(Event.typed(
        CoreEvent.SONA_TICK,
        test_payload,
        source="test",
    ))

    # Wait for async handler to process
    await asyncio.sleep(0.5)

    # Verify handler processed the tick
    assert handler._ticks_processed >= 1
    assert len(handler._health_window) >= 1


@pytest.mark.asyncio
async def test_phase1_health_metrics_extraction():
    """Test that health metrics are correctly extracted from SONA_TICK."""
    organism = awaken()
    handler = organism.meta_cognition

    # Create test payload
    test_payload = SonaTickPayload(
        instance_id="test",
        q_table_entries=75,
        total_judgments=200,
        learning_rate=0.12,
        ewc_consolidated=15,
        uptime_s=300.0,
        interval_s=2040.0,
        tick_number=1,
    )

    # Execute handler directly
    result = await handler.execute(sona_tick=test_payload)

    assert result.success
    assert "health_snapshot" in result.output
    assert result.output["health_snapshot"]["uptime_s"] == 300.0
    assert result.output["health_snapshot"]["q_table_saturation"] == 0.075  # 75/1000


@pytest.mark.asyncio
async def test_phase1_trend_analysis():
    """Test health trend analysis logic."""
    organism = awaken()
    handler = organism.meta_cognition

    # Build a trend: increasing judgments
    from cynic.kernel.organism.brain.cognition.cortex.handlers.meta_cognition import (
        OrganismHealthMetrics,
    )

    for i in range(10):
        health = OrganismHealthMetrics(
            uptime_s=60.0 + i * 10,
            q_table_entries=50 + i * 5,
            total_judgments=100 + i * 20,  # Rising
            learning_rate=0.15,
            ewc_consolidated=10,
            tick_number=i,
        )
        handler._health_window.append(health)

    # Analyze trend
    trend = handler._analyze_health_trends()

    assert trend["judgments_trend"] == "rising"
    assert trend["q_saturation"] == 0.095  # Last entry: 95/1000
    assert trend["judgments_per_second"] > 0  # Positive slope


@pytest.mark.asyncio
async def test_phase1_stagnation_detection():
    """Test that stagnation is detected when judgment flow stops."""
    organism = awaken()
    handler = organism.meta_cognition

    from cynic.kernel.organism.brain.cognition.cortex.handlers.meta_cognition import (
        OrganismHealthMetrics,
    )

    # Build stagnant trend: same judgments, no Q growth
    for i in range(10):
        health = OrganismHealthMetrics(
            uptime_s=60.0 + i * 10,
            q_table_entries=50,  # NO GROWTH
            total_judgments=100,  # NO GROWTH
            learning_rate=0.15,
            ewc_consolidated=10,
            tick_number=i,
        )
        handler._health_window.append(health)

    trend = handler._analyze_health_trends()
    is_stagnant = handler._detect_stagnation(trend)

    assert is_stagnant, "Should detect stagnation when judgments and Q don't grow"


@pytest.mark.asyncio
async def test_phase1_learning_rate_adjustment():
    """Test that learning rate adjustments are computed correctly."""
    organism = awaken()
    handler = organism.meta_cognition

    # Trend: rising judgments, good saturation
    trend = {
        "judgments_trend": "rising",
        "q_saturation": 0.8,  # High saturation
        "learning_health": 0.6,  # Good health
        "judgments_per_second": 0.05,
    }

    adjustment = handler._compute_Î±_adjustment(trend)

    # Should increase Î± when learning well
    assert adjustment > 0, "Should increase learning rate when system is healthy"
    assert adjustment <= 0.618, "Should be Ï†-bounded"


@pytest.mark.asyncio
async def test_phase1_meta_cognition_stats():
    """Test observability stats from meta-cognition handler."""
    organism = awaken()
    handler = organism.meta_cognition

    stats = handler.stats()

    assert "handler_id" in stats
    assert stats["handler_id"] == "meta_cognition"
    assert "ticks_processed" in stats
    assert "health_window_size" in stats


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
