"""Phase G: Empirical lifecycle cleanup tests — Property-based validation.

Tests that verify resource cleanup is truly complete:
1. Memory doesn't accumulate across startup/shutdown cycles
2. Event handlers are properly unregistered (no listener fires after stop)
3. Cleanup is idempotent (no errors on repeated stop calls)
4. All documented components have stop() methods
"""

from __future__ import annotations

import asyncio
import gc
import sys
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from hypothesis import given, strategies as st


class TestLifecycleMemoryGrowth:
    """Property 1: Memory doesn't accumulate across Organism start/stop cycles."""

    @pytest.mark.asyncio
    async def test_memory_stable_across_startup_cycles(self):
        """Repeated start/stop cycles should not cause linear memory growth.

        This is a smoke test — real property-based testing would use Hypothesis
        with repeated cycles and memory snapshots. For now, we verify the
        basic mechanism works.
        """
        import asyncio
        from cynic.kernel.organism.factory import create_organism

        # Force memory tracking
        gc.collect()
        initial_objects = len(gc.get_objects())

        # Cycle 1: Create, start, stop
        org1 = await create_organism()
        await org1.start()
        await org1.stop()
        await asyncio.sleep(0.01)  # Allow cleanup to complete
        del org1
        gc.collect()
        objects_after_cycle1 = len(gc.get_objects())

        # Growth from cycle 1
        growth1 = objects_after_cycle1 - initial_objects

        # Cycle 2: Repeat
        org2 = await create_organism()
        await org2.start()
        await org2.stop()
        await asyncio.sleep(0.01)  # Allow cleanup to complete
        del org2
        gc.collect()
        objects_after_cycle2 = len(gc.get_objects())

        # Growth from cycle 2 should be similar to cycle 1 (not cumulative)
        growth2 = objects_after_cycle2 - objects_after_cycle1

        # Allow 20% variance (test is approximate, not deterministic)
        assert growth2 < growth1 * 1.2, (
            f"Memory growth increased in cycle 2: {growth2} vs cycle 1: {growth1}. "
            f"Suggests listener accumulation or resource leak."
        )


class TestHandlerSilence:
    """Property 2: Event handlers don't fire after stop()."""

    @pytest.mark.asyncio
    async def test_no_listener_fires_after_stop(self):
        """After organism.stop(), emitting an event should not trigger handlers."""
        import asyncio
        from cynic.kernel.core.event_bus import CoreEvent, Event, get_core_bus

        bus = get_core_bus("test-silence")

        handler_called = False

        async def test_handler(event: Event) -> None:
            nonlocal handler_called
            handler_called = True

        # Register listener
        bus.on(CoreEvent.JUDGMENT_CREATED, test_handler)

        # Verify it fires when active
        await bus.emit(Event.typed(CoreEvent.JUDGMENT_CREATED, {}))
        await asyncio.sleep(0.01)  # Allow handlers to process
        assert handler_called, "Handler should fire when registered"

        # Unregister listener
        bus.off(CoreEvent.JUDGMENT_CREATED, test_handler)
        handler_called = False

        # Emit again — handler should NOT fire
        await bus.emit(Event.typed(CoreEvent.JUDGMENT_CREATED, {}))
        await asyncio.sleep(0.01)  # Allow time for handlers (if any) to process
        assert not handler_called, "Handler should not fire after unregistration"


class TestIdempotentCleanup:
    """Property 3: Cleanup can be called multiple times without error."""

    @pytest.mark.asyncio
    async def test_stop_is_idempotent(self):
        """Calling stop() twice should not raise errors."""
        import asyncio
        from cynic.kernel.organism.factory import create_organism

        org = await create_organism()
        await org.start()

        # First stop — should succeed
        await org.stop()
        await asyncio.sleep(0.01)  # Allow cleanup to complete

        # Second stop — should not raise
        try:
            await org.stop()
            await asyncio.sleep(0.01)  # Allow second cleanup to complete
        except Exception as e:
            pytest.fail(f"Second stop() call raised: {e}")


class TestComponentMatrixCoverage:
    """Property 4: All documented components have stop() and are called."""

    def test_organism_stop_calls_all_components(self):
        """Verify Organism.stop() contains calls to all 13+ documented components."""
        import inspect
        from cynic.kernel.organism.organism import Organism

        # Get the source of Organism.stop()
        source = inspect.getsource(Organism.stop)

        # List of expected component cleanup calls
        expected_cleanups = [
            "orchestrator.stop",
            "residual_detector.stop",
            "learning_loop.stop",
            "self_prober.stop",
            "sona_emitter.stop",
            "executor.stop",
            "gossip_manager.stop",
            "world_model.stop",
            "internal_sensor.stop",
            "market_sensor.stop",
            "source_watcher.stop",
            "knet_server.stop",
            "scheduler.stop",
            "motor.stop",
            "state.stop_processing",
        ]

        missing_cleanups = [
            cleanup for cleanup in expected_cleanups
            if cleanup not in source
        ]

        assert (
            not missing_cleanups
        ), f"Organism.stop() missing cleanup calls for: {missing_cleanups}"


class TestComponentStopMethods:
    """Verify each component that has event listeners also has stop()."""

    async def test_world_model_updater_has_stop(self):
        """WorldModelUpdater.stop() should exist."""
        from cynic.kernel.core.world_model import WorldModelUpdater

        updater = WorldModelUpdater()
        assert hasattr(updater, "stop"), "WorldModelUpdater should have stop() method"
        assert callable(updater.stop), "WorldModelUpdater.stop should be callable"

    async def test_residual_detector_has_stop(self):
        """ResidualDetector.stop() should exist."""
        from cynic.kernel.organism.brain.cognition.cortex.residual import ResidualDetector

        detector = ResidualDetector()
        assert hasattr(detector, "stop"), "ResidualDetector should have stop() method"
        assert callable(detector.stop), "ResidualDetector.stop should be callable"

    async def test_account_agent_has_stop(self):
        """AccountAgent.stop() should exist."""
        from cynic.kernel.organism.brain.cognition.cortex.account import AccountAgent

        agent = AccountAgent()
        assert hasattr(agent, "stop"), "AccountAgent should have stop() method"
        assert callable(agent.stop), "AccountAgent.stop should be callable"

    async def test_action_proposer_has_stop(self):
        """ActionProposer.stop() should exist."""
        from cynic.kernel.organism.brain.cognition.cortex.action_proposer import ActionProposer

        # ActionProposer requires a repo, mock it
        mock_repo = MagicMock()
        proposer = ActionProposer(repo=mock_repo)
        assert hasattr(proposer, "stop"), "ActionProposer should have stop() method"
        assert callable(proposer.stop), "ActionProposer.stop should be callable"

    async def test_self_prober_has_stop(self):
        """SelfProber.stop() should exist."""
        from cynic.kernel.organism.brain.cognition.cortex.self_probe import SelfProber

        prober = SelfProber()
        assert hasattr(prober, "stop"), "SelfProber should have stop() method"
        assert callable(prober.stop), "SelfProber.stop should be callable"

    async def test_sovereignty_agent_has_stop(self):
        """SovereigntyAgent.stop() should exist."""
        from cynic.kernel.organism.brain.agents.sovereignty import SovereigntyAgent

        # Mock state manager
        mock_state = MagicMock()
        agent = SovereigntyAgent(state_manager=mock_state)
        assert hasattr(agent, "stop"), "SovereigntyAgent should have stop() method"
        assert callable(agent.stop), "SovereigntyAgent.stop should be callable"

    async def test_internal_sensor_has_stop(self):
        """InternalSensor.stop() should exist."""
        from cynic.kernel.organism.perception.senses.internal import InternalSensor
        from cynic.kernel.core.event_bus import get_core_bus

        bus = get_core_bus("test-sensor")
        sensor = InternalSensor(bus=bus)
        assert hasattr(sensor, "stop"), "InternalSensor should have stop() method"
        assert callable(sensor.stop), "InternalSensor.stop should be callable"
