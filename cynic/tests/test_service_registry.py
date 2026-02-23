"""
Tests for Tier 1 Nervous System — Service State Registry Component

Tests verify:
  - Component registration and state tracking
  - Judgment recording with metadata
  - Snapshot generation and health count accuracy
  - Stalled detection (components not updated recently)
  - Change detection (changed_since)
  - Judgment log queries with filtering
  - Thread safety (async lock protection)
  - Rolling cap enforcement (F(11)=89 judgments)
"""
import asyncio
import time
from unittest import IsolatedAsyncioTestCase

import pytest

from cynic.core.phi import fibonacci
from cynic.nervous import (
    ServiceStateRegistry,
    ComponentType,
    HealthStatus,
    get_service_registry,
    reset_service_registry,
)


class TestServiceStateRegistry(IsolatedAsyncioTestCase):
    """Test suite for ServiceStateRegistry."""

    async def asyncSetUp(self) -> None:
        """Reset singleton before each test."""
        reset_service_registry()
        self.registry = ServiceStateRegistry(stall_threshold_sec=2)

    # ────────────────────────────────────────────────────────────────────────
    # REGISTRATION TESTS
    # ────────────────────────────────────────────────────────────────────────

    async def test_register_component(self) -> None:
        """Component registration creates snapshot with HEALTHY status."""
        await self.registry.register(
            "guardian_dog",
            ComponentType.DOG,
            metrics={"score": 0.85},
        )

        comp = await self.registry.get_component("guardian_dog")
        assert comp is not None
        assert comp.name == "guardian_dog"
        assert comp.type == ComponentType.DOG
        assert comp.status == HealthStatus.HEALTHY
        assert comp.metrics["score"] == 0.85
        assert comp.last_judgment_id is None

    async def test_register_multiple_components(self) -> None:
        """Can register many components simultaneously."""
        dogs = ["guardian", "analyst", "janitor", "scholar", "sounder"]
        for dog in dogs:
            await self.registry.register(dog, ComponentType.DOG)

        snapshot = await self.registry.snapshot()
        assert snapshot.total_components == 5
        assert snapshot.healthy_count == 5

    async def test_register_different_types(self) -> None:
        """Can register different component types."""
        await self.registry.register("core_bus", ComponentType.BUS)
        await self.registry.register("perceive_worker", ComponentType.WORKER)
        await self.registry.register("qtable", ComponentType.LEARNER)
        await self.registry.register("surreal_db", ComponentType.STORAGE)

        snapshot = await self.registry.snapshot()
        assert snapshot.total_components == 4
        assert len(snapshot.components) == 4

    # ────────────────────────────────────────────────────────────────────────
    # JUDGMENT RECORDING TESTS
    # ────────────────────────────────────────────────────────────────────────

    async def test_record_judgment(self) -> None:
        """Recording judgment updates component snapshot."""
        await self.registry.register("guardian", ComponentType.DOG)

        await self.registry.record_judgment(
            "guardian",
            "j001",
            "WAG",
            75.5,
            {"dogs": 11},
        )

        comp = await self.registry.get_component("guardian")
        assert comp is not None
        assert comp.last_judgment_id == "j001"
        assert comp.last_judgment_verdict == "WAG"
        assert comp.last_judgment_q_score == 75.5

    async def test_record_judgment_on_unregistered_component(self) -> None:
        """Recording judgment on unregistered component is silently ignored."""
        # Should not crash
        await self.registry.record_judgment("unknown", "j001", "BARK", 25.0)

        log = await self.registry.get_judgment_log()
        assert len(log) == 0

    async def test_judgment_log_rolling_cap(self) -> None:
        """Judgment log enforces rolling cap at F(11)=89."""
        await self.registry.register("dog", ComponentType.DOG)

        # Record 100 judgments
        for i in range(100):
            await self.registry.record_judgment(
                "dog",
                f"j{i:03d}",
                "WAG",
                50.0 + (i % 10),
            )

        log = await self.registry.get_judgment_log()
        assert len(log) == fibonacci(11)  # 89
        assert log[0]["judgment_id"] == "j011"  # Oldest retained
        assert log[-1]["judgment_id"] == "j099"  # Newest

    async def test_judgment_updates_timestamp(self) -> None:
        """Recording judgment updates last_update_ms."""
        await self.registry.register("dog", ComponentType.DOG)

        before_ms = time.time() * 1000
        await self.registry.record_judgment("dog", "j001", "BARK", 35.0)
        after_ms = time.time() * 1000

        comp = await self.registry.get_component("dog")
        assert comp is not None
        assert before_ms <= comp.last_update_ms <= after_ms

    # ────────────────────────────────────────────────────────────────────────
    # SNAPSHOT AND HEALTH TESTS
    # ────────────────────────────────────────────────────────────────────────

    async def test_snapshot_health_counts(self) -> None:
        """Snapshot computes accurate health status counts."""
        await self.registry.register("dog1", ComponentType.DOG)
        await self.registry.register("dog2", ComponentType.DOG)
        await self.registry.register("dog3", ComponentType.DOG)

        snapshot = await self.registry.snapshot()

        assert snapshot.total_components == 3
        assert snapshot.healthy_count == 3
        assert snapshot.degraded_count == 0
        assert snapshot.stalled_count == 0
        assert snapshot.failed_count == 0

    async def test_stalled_detection(self) -> None:
        """Components not updated within threshold marked STALLED."""
        await self.registry.register("dog1", ComponentType.DOG)

        # Backdate the component to 3 seconds ago
        comp = await self.registry.get_component("dog1")
        comp.last_update_ms = time.time() * 1000 - 3000

        snapshot = await self.registry.snapshot()

        assert snapshot.healthy_count == 0
        assert snapshot.stalled_count == 1
        assert comp.status == HealthStatus.STALLED

    async def test_failed_status_not_auto_stalled(self) -> None:
        """Components marked FAILED are not overwritten to STALLED."""
        await self.registry.register("dog", ComponentType.DOG)
        await self.registry.mark_failed("dog", "connection lost")

        # Backdate it
        comp = await self.registry.get_component("dog")
        comp.last_update_ms = time.time() * 1000 - 5000

        snapshot = await self.registry.snapshot()

        assert snapshot.failed_count == 1
        assert snapshot.stalled_count == 0
        assert comp.status == HealthStatus.FAILED

    async def test_snapshot_immutable_copy(self) -> None:
        """Snapshot is independent of future registry changes."""
        await self.registry.register("dog1", ComponentType.DOG)
        snapshot1 = await self.registry.snapshot()

        await self.registry.register("dog2", ComponentType.DOG)
        snapshot2 = await self.registry.snapshot()

        assert snapshot1.total_components == 1
        assert snapshot2.total_components == 2

    # ────────────────────────────────────────────────────────────────────────
    # CHANGE DETECTION TESTS
    # ────────────────────────────────────────────────────────────────────────

    async def test_changed_since_empty(self) -> None:
        """changed_since returns empty if nothing changed."""
        await self.registry.register("dog", ComponentType.DOG)

        snapshot = await self.registry.snapshot()
        future_ms = snapshot.timestamp_ms + 1000

        changed = await self.registry.changed_since(future_ms)
        assert changed == {}

    async def test_changed_since_detects_updates(self) -> None:
        """changed_since detects components updated after timestamp."""
        await self.registry.register("dog1", ComponentType.DOG)

        snapshot = await self.registry.snapshot()
        baseline_ms = snapshot.timestamp_ms

        # Wait a bit and update
        await asyncio.sleep(0.01)
        await self.registry.register("dog2", ComponentType.DOG)
        await self.registry.record_metric("dog1", "score", 0.75)

        changed = await self.registry.changed_since(baseline_ms)
        assert "dog1" in changed  # Updated via metric
        assert "dog2" in changed  # Newly registered

    # ────────────────────────────────────────────────────────────────────────
    # JUDGMENT LOG QUERY TESTS
    # ────────────────────────────────────────────────────────────────────────

    async def test_judgment_log_filter_by_component(self) -> None:
        """Can filter judgment log by component name."""
        await self.registry.register("dog1", ComponentType.DOG)
        await self.registry.register("dog2", ComponentType.DOG)

        await self.registry.record_judgment("dog1", "j001", "BARK", 30.0)
        await self.registry.record_judgment("dog2", "j002", "WAG", 70.0)
        await self.registry.record_judgment("dog1", "j003", "GROWL", 45.0)

        dog1_judgments = await self.registry.get_judgment_log(component="dog1")
        assert len(dog1_judgments) == 2
        assert all(j["component"] == "dog1" for j in dog1_judgments)

    async def test_judgment_log_filter_by_verdict(self) -> None:
        """Can filter judgment log by verdict."""
        await self.registry.register("dog", ComponentType.DOG)

        await self.registry.record_judgment("dog", "j001", "BARK", 25.0)
        await self.registry.record_judgment("dog", "j002", "BARK", 30.0)
        await self.registry.record_judgment("dog", "j003", "WAG", 70.0)

        barks = await self.registry.get_judgment_log(verdict="BARK")
        assert len(barks) == 2
        assert all(j["verdict"] == "BARK" for j in barks)

    async def test_judgment_log_filter_by_timestamp(self) -> None:
        """Can filter judgment log by timestamp (since_ms)."""
        await self.registry.register("dog", ComponentType.DOG)

        await self.registry.record_judgment("dog", "j001", "BARK", 25.0)

        baseline_ms = time.time() * 1000
        await asyncio.sleep(0.01)

        await self.registry.record_judgment("dog", "j002", "WAG", 70.0)

        recent = await self.registry.get_judgment_log(since_ms=baseline_ms)
        assert len(recent) == 1
        assert recent[0]["judgment_id"] == "j002"

    async def test_judgment_log_combined_filters(self) -> None:
        """Can combine multiple filters on judgment log."""
        await self.registry.register("dog1", ComponentType.DOG)
        await self.registry.register("dog2", ComponentType.DOG)

        await self.registry.record_judgment("dog1", "j001", "BARK", 25.0)
        await self.registry.record_judgment("dog2", "j002", "BARK", 30.0)
        await asyncio.sleep(0.01)

        baseline_ms = time.time() * 1000
        await asyncio.sleep(0.01)

        await self.registry.record_judgment("dog1", "j003", "WAG", 70.0)
        await self.registry.record_judgment("dog2", "j004", "BARK", 35.0)

        # dog1's BARK judgments after baseline (j001 was before, j003 is WAG not BARK)
        result = await self.registry.get_judgment_log(
            component="dog1",
            verdict="BARK",
            since_ms=baseline_ms,
        )
        assert len(result) == 0  # No dog1 BARK judgments after baseline

    # ────────────────────────────────────────────────────────────────────────
    # METRIC RECORDING TESTS
    # ────────────────────────────────────────────────────────────────────────

    async def test_record_metric_on_registered_component(self) -> None:
        """record_metric updates component metrics."""
        await self.registry.register("dog", ComponentType.DOG)

        await self.registry.record_metric("dog", "score", 0.85)
        await self.registry.record_metric("dog", "visits", 42)

        comp = await self.registry.get_component("dog")
        assert comp is not None
        assert comp.metrics["score"] == 0.85
        assert comp.metrics["visits"] == 42

    async def test_record_metric_with_status_change(self) -> None:
        """record_metric can update component status."""
        await self.registry.register("dog", ComponentType.DOG)

        await self.registry.record_metric(
            "dog",
            "error_rate",
            0.15,
            status=HealthStatus.DEGRADED,
        )

        comp = await self.registry.get_component("dog")
        assert comp is not None
        assert comp.status == HealthStatus.DEGRADED

    async def test_record_metric_on_unregistered_ignored(self) -> None:
        """record_metric on unregistered component is ignored silently."""
        # Should not crash
        await self.registry.record_metric("unknown", "key", "value")

    # ────────────────────────────────────────────────────────────────────────
    # FAILURE MARKING TESTS
    # ────────────────────────────────────────────────────────────────────────

    async def test_mark_failed(self) -> None:
        """mark_failed sets status to FAILED and records reason."""
        await self.registry.register("dog", ComponentType.DOG)

        await self.registry.mark_failed("dog", "connection timeout")

        comp = await self.registry.get_component("dog")
        assert comp is not None
        assert comp.status == HealthStatus.FAILED
        assert comp.metrics.get("failure_reason") == "connection timeout"

    async def test_mark_failed_affects_snapshot(self) -> None:
        """Marked failed components reflected in snapshot."""
        await self.registry.register("dog1", ComponentType.DOG)
        await self.registry.register("dog2", ComponentType.DOG)

        await self.registry.mark_failed("dog1", "crash")

        snapshot = await self.registry.snapshot()
        assert snapshot.healthy_count == 1
        assert snapshot.failed_count == 1

    # ────────────────────────────────────────────────────────────────────────
    # THREAD SAFETY TESTS
    # ────────────────────────────────────────────────────────────────────────

    async def test_concurrent_operations(self) -> None:
        """Registry handles concurrent registration and updates safely."""
        tasks = []

        # 20 concurrent registrations
        for i in range(20):
            tasks.append(
                self.registry.register(f"dog{i}", ComponentType.DOG)
            )

        # 40 concurrent judgments
        for i in range(20):
            tasks.append(
                self.registry.record_judgment(f"dog{i}", f"j{i}", "WAG", 50.0)
            )

        await asyncio.gather(*tasks)

        snapshot = await self.registry.snapshot()
        assert snapshot.total_components == 20

        log = await self.registry.get_judgment_log()
        assert len(log) == 20

    # ────────────────────────────────────────────────────────────────────────
    # SERIALIZATION TESTS
    # ────────────────────────────────────────────────────────────────────────

    async def test_snapshot_serialization(self) -> None:
        """Snapshot can be converted to dict and back."""
        await self.registry.register("dog", ComponentType.DOG)
        await self.registry.record_judgment("dog", "j001", "BARK", 30.0)

        snapshot = await self.registry.snapshot()
        snapshot_dict = snapshot.to_dict()

        # Verify dict structure
        assert "timestamp_ms" in snapshot_dict
        assert "components" in snapshot_dict
        assert "total_components" in snapshot_dict
        assert snapshot_dict["total_components"] == 1

        # Deserialize and verify
        restored = snapshot.__class__.from_dict(snapshot_dict)
        assert restored.total_components == snapshot.total_components
        assert list(restored.components.keys()) == list(snapshot.components.keys())

    async def test_component_snapshot_round_trip(self) -> None:
        """ComponentSnapshot serialization round-trip."""
        await self.registry.register("dog", ComponentType.DOG, metrics={"x": 1.5})

        comp = await self.registry.get_component("dog")
        assert comp is not None

        comp_dict = comp.to_dict()
        restored = comp.__class__.from_dict(comp_dict)

        assert restored.name == comp.name
        assert restored.type == comp.type
        assert restored.status == comp.status
        assert restored.metrics == comp.metrics

    # ────────────────────────────────────────────────────────────────────────
    # SINGLETON TESTS
    # ────────────────────────────────────────────────────────────────────────

    async def test_singleton_accessor(self) -> None:
        """get_service_registry returns singleton instance."""
        reset_service_registry()

        r1 = get_service_registry()
        r2 = get_service_registry()

        assert r1 is r2

    async def test_singleton_reset(self) -> None:
        """reset_service_registry clears state."""
        r1 = get_service_registry()
        await r1.register("dog", ComponentType.DOG)

        reset_service_registry()

        r2 = get_service_registry()
        comp = await r2.get_component("dog")
        assert comp is None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
