"""
TIER B2 Tests: Storage Tier Mapping

Tests the OPCODE_STORAGE_MAP routing and StorageTierPolicy class.

Hypotheses:
- T1: All opcodes mapped to storage tier(s)
- T2: Event routing returns correct tier(s)
- T3: Replication works for multi-tier events
- T4: Tier statistics track correctly
- T5: Event promotion works
"""
from __future__ import annotations

import pytest

from cynic.core.event_bus import CoreEvent, Event
from cynic.core.storage.tier_policy import (
    StorageTier,
    StorageTierPolicy,
    OPCODE_STORAGE_MAP,
)


# ═══════════════════════════════════════════════════════════════════════════
# T1: All opcodes mapped to storage tier(s)
# ═══════════════════════════════════════════════════════════════════════════

class TestT1_OpcodeMapping:
    """Verify all opcodes are mapped to storage tiers."""

    def test_t1_all_critical_opcodes_mapped(self):
        """T1.1: All 7 critical opcodes have tier mappings."""
        critical_opcodes = [
            CoreEvent.PERCEPTION_RECEIVED,
            CoreEvent.JUDGMENT_CREATED,
            CoreEvent.DECISION_MADE,
            CoreEvent.ACT_COMPLETED,
            CoreEvent.LEARNING_EVENT,
            CoreEvent.COST_ACCOUNTED,
            CoreEvent.EMERGENCE_DETECTED,
        ]
        for opcode in critical_opcodes:
            assert opcode in OPCODE_STORAGE_MAP, f"Missing mapping for {opcode}"

    def test_t1_perceive_opcode_in_hot(self):
        """T1.2: PERCEIVE (PERCEPTION_RECEIVED) maps to HOT tier."""
        tier = OPCODE_STORAGE_MAP[CoreEvent.PERCEPTION_RECEIVED]
        assert tier == StorageTier.HOT

    def test_t1_judge_opcode_in_multi_tier(self):
        """T1.3: JUDGE (JUDGMENT_CREATED) maps to HOT+WARM+COLD."""
        tiers = OPCODE_STORAGE_MAP[CoreEvent.JUDGMENT_CREATED]
        assert isinstance(tiers, list)
        assert StorageTier.HOT in tiers
        assert StorageTier.WARM in tiers
        assert StorageTier.COLD in tiers

    def test_t1_decide_opcode_in_warm(self):
        """T1.4: DECIDE (DECISION_MADE) maps to WARM tier."""
        tier = OPCODE_STORAGE_MAP[CoreEvent.DECISION_MADE]
        assert tier == StorageTier.WARM

    def test_t1_act_opcode_in_cold(self):
        """T1.5: ACT (ACT_COMPLETED) maps to COLD tier."""
        tier = OPCODE_STORAGE_MAP[CoreEvent.ACT_COMPLETED]
        assert tier == StorageTier.COLD

    def test_t1_learn_opcode_in_warm(self):
        """T1.6: LEARN (LEARNING_EVENT) maps to WARM tier."""
        tier = OPCODE_STORAGE_MAP[CoreEvent.LEARNING_EVENT]
        assert tier == StorageTier.WARM

    def test_t1_account_opcode_in_cold(self):
        """T1.7: ACCOUNT (COST_ACCOUNTED) maps to COLD tier."""
        tier = OPCODE_STORAGE_MAP[CoreEvent.COST_ACCOUNTED]
        assert tier == StorageTier.COLD

    def test_t1_emerge_opcode_in_multi_tier(self):
        """T1.8: EMERGE (EMERGENCE_DETECTED) maps to WARM+COLD."""
        tiers = OPCODE_STORAGE_MAP[CoreEvent.EMERGENCE_DETECTED]
        assert isinstance(tiers, list)
        assert StorageTier.WARM in tiers
        assert StorageTier.COLD in tiers

    def test_t1_mapping_completeness(self):
        """T1.9: Check mapping coverage (all events should have tiers defined)."""
        # Get total mapped opcodes
        total_mapped = len([e for e in CoreEvent if e in OPCODE_STORAGE_MAP])
        # All 7 critical opcodes must be mapped
        assert total_mapped >= 7, f"Expected at least 7 mapped opcodes, got {total_mapped}"


# ═══════════════════════════════════════════════════════════════════════════
# T2: Event routing returns correct tier(s)
# ═══════════════════════════════════════════════════════════════════════════

class TestT2_EventRouting:
    """Verify that StorageTierPolicy routes events to correct tiers."""

    def test_t2_policy_initialization(self):
        """T2.1: StorageTierPolicy initializes without error."""
        policy = StorageTierPolicy()
        assert policy is not None

    def test_t2_route_single_tier_event(self):
        """T2.2: Single-tier event returns list with one tier."""
        policy = StorageTierPolicy()
        tiers = policy.get_tiers_for_event(CoreEvent.PERCEPTION_RECEIVED)
        assert len(tiers) == 1
        assert tiers[0] == StorageTier.HOT

    @pytest.mark.asyncio
    async def test_t2_route_event_perceive(self):
        """T2.3: Route PERCEIVE event → HOT."""
        policy = StorageTierPolicy()

        event = Event(type=CoreEvent.PERCEPTION_RECEIVED, payload={})
        tiers = await policy.route_event(event)

        assert len(tiers) == 1
        assert tiers[0] == StorageTier.HOT

    @pytest.mark.asyncio
    async def test_t2_route_event_judge(self):
        """T2.4: Route JUDGE event → HOT+WARM+COLD."""
        policy = StorageTierPolicy()
        event = Event(type=CoreEvent.JUDGMENT_CREATED, payload={"q_score": 75.0})
        tiers = await policy.route_event(event)

        assert len(tiers) == 3
        assert StorageTier.HOT in tiers
        assert StorageTier.WARM in tiers
        assert StorageTier.COLD in tiers

    @pytest.mark.asyncio
    async def test_t2_route_event_decide(self):
        """T2.5: Route DECIDE event → WARM."""
        policy = StorageTierPolicy()
        event = Event(type=CoreEvent.DECISION_MADE, payload={})
        tiers = await policy.route_event(event)

        assert len(tiers) == 1
        assert tiers[0] == StorageTier.WARM

    @pytest.mark.asyncio
    async def test_t2_route_event_act(self):
        """T2.6: Route ACT event → COLD."""
        policy = StorageTierPolicy()
        event = Event(type=CoreEvent.ACT_COMPLETED, payload={})
        tiers = await policy.route_event(event)

        assert len(tiers) == 1
        assert tiers[0] == StorageTier.COLD

    @pytest.mark.asyncio
    async def test_t2_route_event_learn(self):
        """T2.7: Route LEARN event → WARM."""
        policy = StorageTierPolicy()
        event = Event(type=CoreEvent.LEARNING_EVENT, payload={})
        tiers = await policy.route_event(event)

        assert len(tiers) == 1
        assert tiers[0] == StorageTier.WARM

    @pytest.mark.asyncio
    async def test_t2_route_event_account(self):
        """T2.8: Route ACCOUNT event → COLD."""
        policy = StorageTierPolicy()
        event = Event(type=CoreEvent.COST_ACCOUNTED, payload={})
        tiers = await policy.route_event(event)

        assert len(tiers) == 1
        assert tiers[0] == StorageTier.COLD

    @pytest.mark.asyncio
    async def test_t2_route_event_emerge(self):
        """T2.9: Route EMERGE event → WARM+COLD."""
        policy = StorageTierPolicy()
        event = Event(type=CoreEvent.EMERGENCE_DETECTED, payload={})
        tiers = await policy.route_event(event)

        assert len(tiers) == 2
        assert StorageTier.WARM in tiers
        assert StorageTier.COLD in tiers


# ═══════════════════════════════════════════════════════════════════════════
# T3: Replication works for multi-tier events
# ═══════════════════════════════════════════════════════════════════════════

class TestT3_Replication:
    """Verify multi-tier replication for important events."""

    @pytest.mark.asyncio
    async def test_t3_judgment_replicated_to_all_tiers(self):
        """T3.1: JUDGMENT_CREATED is replicated to all 3 tiers."""
        policy = StorageTierPolicy()
        event = Event(
            type=CoreEvent.JUDGMENT_CREATED,
            payload={"q_score": 75.0, "verdict": "WAG"}
        )
        tiers = await policy.route_event(event)

        assert len(tiers) == 3, "JUDGMENT_CREATED must replicate to 3 tiers"

    @pytest.mark.asyncio
    async def test_t3_emergence_replicated_to_warm_cold(self):
        """T3.2: EMERGENCE_DETECTED is replicated to WARM and COLD."""
        policy = StorageTierPolicy()
        event = Event(
            type=CoreEvent.EMERGENCE_DETECTED,
            payload={"pattern_type": "SPIKE"}
        )
        tiers = await policy.route_event(event)

        assert len(tiers) == 2, "EMERGENCE_DETECTED must replicate to 2 tiers"
        assert StorageTier.WARM in tiers
        assert StorageTier.COLD in tiers

    @pytest.mark.asyncio
    async def test_t3_single_tier_events_not_replicated(self):
        """T3.3: Single-tier events (PERCEIVE, DECIDE, LEARN, ACCOUNT) not replicated."""
        policy = StorageTierPolicy()
        single_tier_events = [
            CoreEvent.PERCEPTION_RECEIVED,
            CoreEvent.DECISION_MADE,
            CoreEvent.LEARNING_EVENT,
            CoreEvent.COST_ACCOUNTED,
        ]

        for event_type in single_tier_events:
            event = Event(type=event_type, payload={})
            tiers = await policy.route_event(event)
            assert len(tiers) == 1, f"{event_type} should not be replicated"


# ═══════════════════════════════════════════════════════════════════════════
# T4: Tier statistics track correctly
# ═══════════════════════════════════════════════════════════════════════════

class TestT4_Statistics:
    """Verify that storage tier statistics track correctly."""

    @pytest.mark.asyncio
    async def test_t4_stats_empty_initially(self):
        """T4.1: New policy has zero event counts."""
        policy = StorageTierPolicy()
        stats = policy.stats()

        assert stats["tier_counts"]["hot"] == 0
        assert stats["tier_counts"]["warm"] == 0
        assert stats["tier_counts"]["cold"] == 0
        assert stats["total_events_routed"] == 0

    @pytest.mark.asyncio
    async def test_t4_stats_track_single_tier_routes(self):
        """T4.2: Stats update after routing single-tier events."""
        policy = StorageTierPolicy()

        # Route PERCEIVE (HOT)
        event1 = Event(type=CoreEvent.PERCEPTION_RECEIVED, payload={})
        await policy.route_event(event1)

        stats = policy.stats()
        assert stats["tier_counts"]["hot"] >= 1
        assert stats["total_events_routed"] >= 1

    @pytest.mark.asyncio
    async def test_t4_stats_track_multi_tier_routes(self):
        """T4.3: Stats count multi-tier events correctly."""
        policy = StorageTierPolicy()

        # Route JUDGMENT (replicated to 3 tiers)
        event = Event(type=CoreEvent.JUDGMENT_CREATED, payload={})
        await policy.route_event(event)

        stats = policy.stats()
        # One JUDGMENT_CREATED event should increment all 3 tiers
        assert stats["tier_counts"]["hot"] >= 1
        assert stats["tier_counts"]["warm"] >= 1
        assert stats["tier_counts"]["cold"] >= 1

    @pytest.mark.asyncio
    async def test_t4_stats_coverage_percentage(self):
        """T4.4: Stats show opcode coverage percentage."""
        policy = StorageTierPolicy()
        stats = policy.stats()

        assert "coverage_pct" in stats
        assert stats["coverage_pct"] > 0
        assert stats["coverage_pct"] <= 100

    @pytest.mark.asyncio
    async def test_t4_stats_opcode_count(self):
        """T4.5: Stats show opcode mapping count."""
        policy = StorageTierPolicy()
        stats = policy.stats()

        assert "opcode_coverage" in stats
        # Should be in format "N/M"
        coverage = stats["opcode_coverage"]
        assert "/" in coverage
        mapped, total = coverage.split("/")
        assert int(mapped) >= 7  # At least 7 critical opcodes


# ═══════════════════════════════════════════════════════════════════════════
# T5: Event promotion works
# ═══════════════════════════════════════════════════════════════════════════

class TestT5_Promotion:
    """Verify event promotion between tiers."""

    @pytest.mark.asyncio
    async def test_t5_promote_event_method_exists(self):
        """T5.1: StorageTierPolicy has promote_event method."""
        policy = StorageTierPolicy()
        assert hasattr(policy, "promote_event")
        assert callable(getattr(policy, "promote_event"))

    @pytest.mark.asyncio
    async def test_t5_promote_cold_to_warm(self):
        """T5.2: Can promote event from COLD to WARM."""
        policy = StorageTierPolicy()
        await policy.promote_event("jdg_123", StorageTier.COLD, StorageTier.WARM)
        # Should not raise an exception

    @pytest.mark.asyncio
    async def test_t5_promote_warm_to_hot(self):
        """T5.3: Can promote event from WARM to HOT."""
        policy = StorageTierPolicy()
        await policy.promote_event("jdg_123", StorageTier.WARM, StorageTier.HOT)
        # Should not raise an exception

    @pytest.mark.asyncio
    async def test_t5_promote_cold_to_hot(self):
        """T5.4: Can directly promote from COLD to HOT."""
        policy = StorageTierPolicy()
        await policy.promote_event("jdg_123", StorageTier.COLD, StorageTier.HOT)
        # Should not raise an exception


# ═══════════════════════════════════════════════════════════════════════════
# Integration Tests
# ═══════════════════════════════════════════════════════════════════════════

class TestIntegration_FullCycle:
    """Integration test: Route all 7 opcodes through tiers."""

    @pytest.mark.asyncio
    async def test_full_cycle_routing(self):
        """Route all 7 opcodes and verify tiers are correct."""
        policy = StorageTierPolicy()

        test_cases = [
            (CoreEvent.PERCEPTION_RECEIVED, [StorageTier.HOT]),
            (CoreEvent.JUDGMENT_CREATED, [StorageTier.HOT, StorageTier.WARM, StorageTier.COLD]),
            (CoreEvent.DECISION_MADE, [StorageTier.WARM]),
            (CoreEvent.ACT_COMPLETED, [StorageTier.COLD]),
            (CoreEvent.LEARNING_EVENT, [StorageTier.WARM]),
            (CoreEvent.COST_ACCOUNTED, [StorageTier.COLD]),
            (CoreEvent.EMERGENCE_DETECTED, [StorageTier.WARM, StorageTier.COLD]),
        ]

        for event_type, expected_tiers in test_cases:
            event = Event(type=event_type, payload={})
            actual_tiers = await policy.route_event(event)

            assert len(actual_tiers) == len(expected_tiers), \
                f"{event_type}: expected {len(expected_tiers)} tiers, got {len(actual_tiers)}"

            for expected_tier in expected_tiers:
                assert expected_tier in actual_tiers, \
                    f"{event_type}: expected {expected_tier} in {actual_tiers}"

        # Verify stats show all events were routed
        stats = policy.stats()
        total_routed = stats["total_events_routed"]
        assert total_routed >= 7, f"Expected at least 7 events routed, got {total_routed}"
