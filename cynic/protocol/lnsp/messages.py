"""LNSP message factory functions for creating messages at each layer."""
from __future__ import annotations

import uuid
import time
from typing import Any, Optional

from .types import (
    Layer,
    LNSPMessage,
    MessageHeader,
    Metadata,
    ObservationType,
    AggregationType,
    JudgmentType,
    ActionType,
    VerdictType,
)


def _generate_message_id() -> str:
    """Generate a unique 8-character message ID."""
    return uuid.uuid4().hex[:8]


def _get_timestamp() -> float:
    """Get current Unix timestamp."""
    return time.time()


# ============================================================================
# Layer 1: Raw Observations
# ============================================================================

def create_raw_observation(
    observation_type: ObservationType,
    source: str,
    instance_id: str,
    payload: dict[str, Any],
    target: Optional[str] = None,
    region: Optional[str] = None,
    route_trace: Optional[list[str]] = None,
) -> LNSPMessage:
    """Create a Layer 1 (RAW) observation message.

    Raw observations are direct sensor readings from the ecosystem:
    - Process creation/termination
    - Metric samples
    - External events
    - Human input
    - Action results

    Args:
        observation_type: Type of observation (ObservationType enum)
        source: Originating sensor/component name
        instance_id: Unique organism instance ID
        payload: Observation data (should include relevant fields for the type)
        target: Optional target component
        region: Optional region identifier
        route_trace: Optional existing route trace to append to

    Returns:
        LNSPMessage with Layer 1 configuration

    Example:
        >>> msg = create_raw_observation(
        ...     ObservationType.METRIC_SAMPLE,
        ...     source="METRICS_COLLECTOR",
        ...     instance_id="org_001",
        ...     payload={"cpu": 45.2, "memory": 78.9},
        ... )
    """
    if route_trace is None:
        route_trace = []

    header = MessageHeader(
        layer=Layer.RAW,
        message_id=_generate_message_id(),
        timestamp=_get_timestamp(),
        source=source,
        target=target,
        version="1.0.0",
    )

    metadata = Metadata(
        instance_id=instance_id,
        region=region,
        route_trace=route_trace + [source],
        feedback=False,
    )

    # Add observation type to payload
    enriched_payload = {
        "observation_type": observation_type.value,
        **payload,
    }

    return LNSPMessage(header=header, payload=enriched_payload, metadata=metadata)


# ============================================================================
# Layer 2: Aggregated State
# ============================================================================

def create_aggregated_state(
    aggregation_type: AggregationType,
    source: str,
    instance_id: str,
    payload: dict[str, Any],
    target: Optional[str] = None,
    region: Optional[str] = None,
    route_trace: Optional[list[str]] = None,
) -> LNSPMessage:
    """Create a Layer 2 (AGGREGATED) state message.

    Aggregated state represents processed and combined observations:
    - Process metrics (aggregated)
    - System health status
    - Ecosystem-wide state
    - Organism vitals summary

    Args:
        aggregation_type: Type of aggregation (AggregationType enum)
        source: Originating component (usually a ganglia/aggregator)
        instance_id: Unique organism instance ID
        payload: Aggregated state data
        target: Optional target component
        region: Optional region identifier
        route_trace: Optional existing route trace to append to

    Returns:
        LNSPMessage with Layer 2 configuration

    Example:
        >>> msg = create_aggregated_state(
        ...     AggregationType.SYSTEM_STATE,
        ...     source="SYSTEM_GANGLIA",
        ...     instance_id="org_001",
        ...     payload={"health": "GOOD", "uptime": 86400},
        ... )
    """
    if route_trace is None:
        route_trace = []

    header = MessageHeader(
        layer=Layer.AGGREGATED,
        message_id=_generate_message_id(),
        timestamp=_get_timestamp(),
        source=source,
        target=target,
        version="1.0.0",
    )

    metadata = Metadata(
        instance_id=instance_id,
        region=region,
        route_trace=route_trace + [source],
        feedback=False,
    )

    # Add aggregation type to payload
    enriched_payload = {
        "aggregation_type": aggregation_type.value,
        **payload,
    }

    return LNSPMessage(header=header, payload=enriched_payload, metadata=metadata)


# ============================================================================
# Layer 3: Judgments
# ============================================================================

def create_judgment(
    judgment_type: JudgmentType,
    verdict: VerdictType,
    source: str,
    instance_id: str,
    payload: dict[str, Any],
    target: Optional[str] = None,
    region: Optional[str] = None,
    route_trace: Optional[list[str]] = None,
    q_score: Optional[float] = None,
) -> LNSPMessage:
    """Create a Layer 3 (JUDGMENT) message.

    Judgments are verdicts from the Dogs (sefirot) using axioms:
    - State evaluations (is state healthy?)
    - Emergence alerts (new pattern?)
    - Pattern detection (recognized pattern?)
    - Learning updates (Q-Table changed?)

    Args:
        judgment_type: Type of judgment (JudgmentType enum)
        verdict: The verdict (HOWL, GROWL, WAG, BARK)
        source: Originating Dog/Judge name
        instance_id: Unique organism instance ID
        payload: Judgment reasoning and details
        target: Optional target component
        region: Optional region identifier
        route_trace: Optional existing route trace to append to
        q_score: Optional confidence score (0-100)

    Returns:
        LNSPMessage with Layer 3 configuration

    Example:
        >>> msg = create_judgment(
        ...     JudgmentType.STATE_EVALUATION,
        ...     verdict=VerdictType.HOWL,
        ...     source="SAGE_DOG",
        ...     instance_id="org_001",
        ...     payload={"reason": "All metrics in normal range"},
        ...     q_score=92.5,
        ... )
    """
    if route_trace is None:
        route_trace = []

    header = MessageHeader(
        layer=Layer.JUDGMENT,
        message_id=_generate_message_id(),
        timestamp=_get_timestamp(),
        source=source,
        target=target,
        version="1.0.0",
    )

    metadata = Metadata(
        instance_id=instance_id,
        region=region,
        route_trace=route_trace + [source],
        feedback=True,  # Judgments expect feedback
    )

    # Add judgment-specific fields to payload
    enriched_payload = {
        "judgment_type": judgment_type.value,
        "verdict": verdict.value,
        "q_score": q_score,
        **payload,
    }

    return LNSPMessage(header=header, payload=enriched_payload, metadata=metadata)


# ============================================================================
# Layer 4: Actions
# ============================================================================

def create_action(
    action_type: ActionType,
    source: str,
    instance_id: str,
    payload: dict[str, Any],
    target: Optional[str] = None,
    region: Optional[str] = None,
    route_trace: Optional[list[str]] = None,
    closes_action_id: Optional[str] = None,
) -> LNSPMessage:
    """Create a Layer 4 (ACTION) message.

    Actions represent directives to handlers and executors:
    - Apply configuration changes
    - Deploy new components
    - Call external APIs/blockchain
    - Signal to human operators

    Args:
        action_type: Type of action (ActionType enum)
        source: Originating decision component (usually BRAIN/EXECUTIVE)
        instance_id: Unique organism instance ID
        payload: Action parameters and execution details
        target: Optional target executor component
        region: Optional region identifier
        route_trace: Optional existing route trace to append to
        closes_action_id: Optional ID of action this message completes

    Returns:
        LNSPMessage with Layer 4 configuration

    Example:
        >>> msg = create_action(
        ...     ActionType.EXTERNAL_CALL,
        ...     source="EXECUTIVE",
        ...     instance_id="org_001",
        ...     target="BLOCKCHAIN_EXECUTOR",
        ...     payload={"call": "submit_governance_vote", "args": [...]},
        ... )
    """
    if route_trace is None:
        route_trace = []

    header = MessageHeader(
        layer=Layer.ACTION,
        message_id=_generate_message_id(),
        timestamp=_get_timestamp(),
        source=source,
        target=target,
        version="1.0.0",
    )

    metadata = Metadata(
        instance_id=instance_id,
        region=region,
        route_trace=route_trace + [source],
        feedback=True,  # Actions expect feedback (result confirmation)
        closes_action_id=closes_action_id,
    )

    # Add action-specific fields to payload
    enriched_payload = {
        "action_type": action_type.value,
        **payload,
    }

    return LNSPMessage(header=header, payload=enriched_payload, metadata=metadata)
