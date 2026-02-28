"""LNSP message factory functions for creating messages at each layer."""
from __future__ import annotations

import time
import uuid
from typing import Any

from .types import (
    ActionType,
    AggregationType,
    JudgmentType,
    Layer,
    LNSPMessage,
    MessageHeader,
    Metadata,
    ObservationType,
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
    data: dict[str, Any],
    source: str,
    instance_id: str = "instance:local",
    region: str | None = None,
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
        data: Observation data (should include relevant fields for the type)
        source: Originating sensor/component name
        instance_id: Unique organism instance ID (default: "instance:local")
        region: Optional region identifier

    Returns:
        LNSPMessage with Layer 1 configuration

    Example:
        >>> msg = create_raw_observation(
        ...     ObservationType.METRIC_SAMPLE,
        ...     data={"cpu": 45.2, "memory": 78.9},
        ...     source="METRICS_COLLECTOR",
        ...     instance_id="org_001",
        ... )
    """
    route_trace: list[str] = []

    header = MessageHeader(
        layer=Layer.RAW,
        message_id=_generate_message_id(),
        timestamp=_get_timestamp(),
        source=source,
        target=None,
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
        **data,
    }

    return LNSPMessage(header=header, payload=enriched_payload, metadata=metadata)


# ============================================================================
# Layer 2: Aggregated State
# ============================================================================

def create_aggregated_state(
    aggregation_type: AggregationType,
    data: dict[str, Any],
    source: str,
    based_on: list[str],
    instance_id: str = "instance:local",
    region: str | None = None,
) -> LNSPMessage:
    """Create a Layer 2 (AGGREGATED) state message.

    Aggregated state represents processed and combined observations:
    - Process metrics (aggregated)
    - System health status
    - Ecosystem-wide state
    - Organism vitals summary

    Args:
        aggregation_type: Type of aggregation (AggregationType enum)
        data: Aggregated state data
        source: Originating component (usually a ganglia/aggregator)
        based_on: List of source message IDs this aggregation is based on
        instance_id: Unique organism instance ID (default: "instance:local")
        region: Optional region identifier

    Returns:
        LNSPMessage with Layer 2 configuration

    Example:
        >>> msg = create_aggregated_state(
        ...     AggregationType.SYSTEM_STATE,
        ...     data={"health": "GOOD", "uptime": 86400},
        ...     source="SYSTEM_GANGLIA",
        ...     based_on=["msg_001", "msg_002"],
        ...     instance_id="org_001",
        ... )
    """
    route_trace: list[str] = []

    header = MessageHeader(
        layer=Layer.AGGREGATED,
        message_id=_generate_message_id(),
        timestamp=_get_timestamp(),
        source=source,
        target=None,
        version="1.0.0",
    )

    metadata = Metadata(
        instance_id=instance_id,
        region=region,
        route_trace=route_trace + [source],
        feedback=False,
    )

    # Add aggregation type and causality tracking to payload
    enriched_payload = {
        "aggregation_type": aggregation_type.value,
        "based_on": based_on,
        **data,
    }

    return LNSPMessage(header=header, payload=enriched_payload, metadata=metadata)


# ============================================================================
# Layer 3: Judgments
# ============================================================================

def create_judgment(
    judgment_type: JudgmentType,
    data: dict[str, Any],
    source: str,
    verdict: VerdictType | None = None,
    q_score: float = 0.0,
    confidence: float = 0.0,
    axiom_scores: dict[str, float] | None = None,
    target: str | None = None,
    based_on: list[str] | None = None,
    instance_id: str = "instance:local",
    region: str | None = None,
) -> LNSPMessage:
    """Create a Layer 3 (JUDGMENT) message.

    Judgments are verdicts from the Dogs (sefirot) using axioms:
    - State evaluations (is state healthy?)
    - Emergence alerts (new pattern?)
    - Pattern detection (recognized pattern?)
    - Learning updates (Q-Table changed?)

    Args:
        judgment_type: Type of judgment (JudgmentType enum)
        data: Judgment reasoning and details (stored nested under "data" key)
        source: Originating Dog/Judge name
        verdict: The verdict (HOWL, GROWL, WAG, BARK). Optional.
        q_score: Confidence/quality score (0-1). Default 0.0.
        confidence: Confidence metric (0-1). Default 0.0.
        axiom_scores: Dict mapping axiom names to scores (0-1). Optional.
        target: Target component (usually EXECUTIVE). Optional.
        based_on: List of source message IDs this judgment is based on. Optional.
        instance_id: Unique organism instance ID (default: "instance:local")
        region: Optional region identifier

    Returns:
        LNSPMessage with Layer 3 configuration

    Example:
        >>> msg = create_judgment(
        ...     JudgmentType.STATE_EVALUATION,
        ...     verdict=VerdictType.HOWL,
        ...     q_score=92.5,
        ...     confidence=0.95,
        ...     axiom_scores={"BURN": 0.9, "CONSENT": 0.85},
        ...     data={"reason": "All metrics in normal range"},
        ...     source="SAGE_DOG",
        ...     target="EXECUTIVE",
        ...     based_on=["agg_001"],
        ...     instance_id="org_001",
        ... )
    """
    route_trace: list[str] = []

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

    # Build payload: explicit fields at top level, data dict nested under "data"
    enriched_payload: dict[str, Any] = {
        "judgment_type": judgment_type.value,
        "q_score": q_score,
        "confidence": confidence,
        "axiom_scores": axiom_scores if axiom_scores is not None else {},
        "based_on": based_on if based_on is not None else [],
        "data": data,
    }
    if verdict is not None:
        enriched_payload["verdict"] = verdict.value

    return LNSPMessage(header=header, payload=enriched_payload, metadata=metadata)


# ============================================================================
# Layer 4: Actions
# ============================================================================

def create_action(
    action_type: ActionType,
    target: str,
    action_data: dict[str, Any],
    source: str,
    based_on_verdict: str,
    instance_id: str = "instance:local",
    region: str | None = None,
) -> LNSPMessage:
    """Create a Layer 4 (ACTION) message.

    Actions represent directives to handlers and executors:
    - Apply configuration changes
    - Deploy new components
    - Call external APIs/blockchain
    - Signal to human operators

    Args:
        action_type: Type of action (ActionType enum)
        target: Target executor component
        action_data: Action parameters and execution details
        source: Originating decision component (usually BRAIN/EXECUTIVE)
        based_on_verdict: The verdict message ID that triggered this action
        instance_id: Unique organism instance ID (default: "instance:local")
        region: Optional region identifier

    Returns:
        LNSPMessage with Layer 4 configuration

    Example:
        >>> msg = create_action(
        ...     ActionType.EXTERNAL_CALL,
        ...     target="BLOCKCHAIN_EXECUTOR",
        ...     action_data={"call": "submit_governance_vote", "args": [...]},
        ...     source="EXECUTIVE",
        ...     based_on_verdict="verdict_msg_001",
        ...     instance_id="org_001",
        ... )
    """
    route_trace: list[str] = []

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
        closes_action_id=None,
    )

    # Add action-specific fields and verdict causality tracking to payload
    enriched_payload = {
        "action_type": action_type.value,
        "based_on_verdict": based_on_verdict,
        **action_data,
    }

    return LNSPMessage(header=header, payload=enriched_payload, metadata=metadata)
