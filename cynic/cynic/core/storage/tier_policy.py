"""
TIER B2: Storage Tier Mapping — Route events to HOT/WARM/COLD tiers.

Maps each opcode (CoreEvent type) to storage tier(s).
- HOT: Recent, frequently accessed (memory + fresh writes)
- WARM: Standard archival (indexed database tables)
- COLD: Historical, rarely accessed (compressed/archived)

Timing:
- PERCEPTION_RECEIVED → HOT (immediate processing)
- JUDGMENT_CREATED → [HOT, WARM, COLD] (replicated everywhere)
- DECISION_MADE → WARM (stable, not urgent)
- ACTION_EXECUTED → COLD (historical record)
- LEARNING_SIGNAL_PROCESSED → WARM (learning is important, not urgent)
- COST_ACCOUNTED → COLD (financial record, archived)
- EMERGENCE_DETECTED → [WARM, COLD] (replicate for analysis)
"""
from __future__ import annotations

import logging
from enum import Enum
from typing import Any

from cynic.core.event_bus import CoreEvent, Event

logger = logging.getLogger(__name__)


# ════════════════════════════════════════════════════════════════════════════
# STORAGE TIER ENUMERATION
# ════════════════════════════════════════════════════════════════════════════

class StorageTier(str, Enum):
    """Storage tier classification for event persistence."""

    HOT = "hot"  # Memory + fresh writes (instant access, recent)
    WARM = "warm"  # Indexed database (normal access, standard archival)
    COLD = "cold"  # Compressed/archived (rare access, historical)


# ════════════════════════════════════════════════════════════════════════════
# OPCODE → STORAGE TIER MAPPING
# ════════════════════════════════════════════════════════════════════════════

OPCODE_STORAGE_MAP: dict[CoreEvent, StorageTier | list[StorageTier]] = {
    # Step 1: PERCEIVE
    CoreEvent.PERCEPTION_RECEIVED: StorageTier.HOT,

    # Step 2: JUDGE
    CoreEvent.JUDGMENT_CREATED: [StorageTier.HOT, StorageTier.WARM, StorageTier.COLD],

    # Step 3: DECIDE
    CoreEvent.DECISION_MADE: StorageTier.WARM,

    # Step 4: ACT
    CoreEvent.ACT_REQUESTED: StorageTier.HOT,
    CoreEvent.ACT_COMPLETED: StorageTier.COLD,

    # Step 5: LEARN
    CoreEvent.LEARNING_EVENT: StorageTier.WARM,
    CoreEvent.Q_TABLE_UPDATED: StorageTier.WARM,

    # Step 6: ACCOUNT
    CoreEvent.COST_ACCOUNTED: StorageTier.COLD,

    # Step 7: EMERGE
    CoreEvent.EMERGENCE_DETECTED: [StorageTier.WARM, StorageTier.COLD],

    # Cross-cutting concerns
    CoreEvent.AXIOM_ACTIVATED: StorageTier.WARM,
    CoreEvent.TRANSCENDENCE: [StorageTier.WARM, StorageTier.COLD],

    # Self-improvement
    CoreEvent.SELF_IMPROVEMENT_PROPOSED: StorageTier.WARM,

    # Consciousness
    CoreEvent.CONSCIOUSNESS_CHANGED: StorageTier.HOT,

    # Other events
    CoreEvent.USER_FEEDBACK: StorageTier.WARM,
    CoreEvent.RESIDUAL_HIGH: StorageTier.COLD,
}


# ════════════════════════════════════════════════════════════════════════════
# STORAGE TIER POLICY
# ════════════════════════════════════════════════════════════════════════════

class StorageTierPolicy:
    """
    Routes events to appropriate storage tiers based on opcode type.

    This policy ensures that:
    - Immediate events (PERCEIVE, JUDGE) stay in HOT tier for fast access
    - Standard events go to WARM tier for normal archival
    - Historical events go to COLD tier for long-term storage

    Replication: Some events (like JUDGMENT_CREATED) are replicated across
    multiple tiers for redundancy and analysis purposes.
    """

    def __init__(self):
        """Initialize the storage tier policy."""
        self._tier_counts: dict[StorageTier, int] = {
            tier: 0 for tier in StorageTier
        }
        logger.info("*sniff* StorageTierPolicy initialized")

    def get_tiers_for_event(self, event_type: CoreEvent) -> list[StorageTier]:
        """
        Get storage tier(s) for a given event type.

        Args:
            event_type: The CoreEvent type to route

        Returns:
            List of StorageTier values (may be one or more tiers)
        """
        mapping = OPCODE_STORAGE_MAP.get(event_type)

        if mapping is None:
            logger.warning(f"Unknown event type: {event_type} — defaulting to WARM")
            return [StorageTier.WARM]

        if isinstance(mapping, StorageTier):
            return [mapping]

        if isinstance(mapping, list):
            return mapping

        # Fallback
        return [StorageTier.WARM]

    async def route_event(self, event: Event) -> list[StorageTier]:
        """
        Route an event to its target storage tier(s).

        Args:
            event: The Event to route

        Returns:
            List of StorageTier values used for this event
        """
        tiers = self.get_tiers_for_event(event.type)

        for tier in tiers:
            self._tier_counts[tier] = self._tier_counts.get(tier, 0) + 1

        if len(tiers) > 1:
            logger.debug(
                f"Event {event.type} replicated to {len(tiers)} tiers: "
                f"{', '.join(t.value for t in tiers)}"
            )
        else:
            logger.debug(f"Event {event.type} → {tiers[0].value}")

        return tiers

    async def promote_event(
        self, event_id: str, from_tier: StorageTier, to_tier: StorageTier
    ) -> None:
        """
        Promote an event from one tier to another (e.g., COLD → WARM → HOT).

        Useful for events that become "hot" after being in cold storage.
        For example, a judgment that users start frequently querying should
        be promoted from COLD to WARM or HOT for better performance.

        Args:
            event_id: The event identifier
            from_tier: Source tier
            to_tier: Destination tier

        Note: Actual implementation would be in specific storage backends.
        """
        logger.info(f"Promoting event {event_id} from {from_tier.value} to {to_tier.value}")

    def stats(self) -> dict[str, Any]:
        """
        Get statistics on storage tier usage.

        Returns:
            Dictionary with counts per tier and mapping completeness
        """
        total_mapped = len(
            [e for e in CoreEvent if e in OPCODE_STORAGE_MAP]
        )
        total_events = len(list(CoreEvent))

        return {
            "tier_counts": {tier.value: count for tier, count in self._tier_counts.items()},
            "total_events_routed": sum(self._tier_counts.values()),
            "opcode_coverage": f"{total_mapped}/{total_events}",
            "coverage_pct": round(100 * total_mapped / max(total_events, 1), 1),
        }


# ════════════════════════════════════════════════════════════════════════════
# GLOBAL INSTANCE
# ════════════════════════════════════════════════════════════════════════════

_global_policy: StorageTierPolicy | None = None


def get_tier_policy() -> StorageTierPolicy:
    """Get or create the global storage tier policy instance."""
    global _global_policy
    if _global_policy is None:
        _global_policy = StorageTierPolicy()
    return _global_policy


def reset_tier_policy() -> None:
    """Reset the global storage tier policy (for testing)."""
    global _global_policy
    _global_policy = None


def set_tier_policy(policy: StorageTierPolicy | None) -> None:
    """Set the global storage tier policy (for testing/injection)."""
    global _global_policy
    _global_policy = policy
