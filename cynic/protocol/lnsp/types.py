"""LNSP types, enums, and message schema definitions."""
from __future__ import annotations

from dataclasses import dataclass, field, asdict
from enum import Enum
from typing import Any, Optional
import time
from datetime import datetime


# ============================================================================
# Layer Enumeration
# ============================================================================

class Layer(Enum):
    """LNSP Protocol layer levels."""

    RAW = 1  # Layer 1: Raw observations (sensors)
    AGGREGATED = 2  # Layer 2: Aggregated state (regional ganglia)
    JUDGMENT = 3  # Layer 3: Judgments (axiom-based verdicts)
    ACTION = 4  # Layer 4: Actions (handlers + feedback)

    def __str__(self) -> str:
        return f"Layer.{self.name}"

    def __repr__(self) -> str:
        return f"<Layer: {self.name} ({self.value})>"


# ============================================================================
# Verdict Types (Dogs' Outputs)
# ============================================================================

class VerdictType(Enum):
    """Verdict verdicts from CYNIC's 11 Dogs (Sefirot)."""

    HOWL = "HOWL"  # Strong positive judgment
    GROWL = "GROWL"  # Moderate positive judgment
    WAG = "WAG"  # Neutral/uncertain judgment
    BARK = "BARK"  # Negative judgment

    def __str__(self) -> str:
        return self.value


# ============================================================================
# Observation Types (Layer 1 Raw Data)
# ============================================================================

class ObservationType(Enum):
    """Types of raw observations from sensors."""

    PROCESS_CREATED = "PROCESS_CREATED"  # Process/agent spawned
    PROCESS_TERMINATED = "PROCESS_TERMINATED"  # Process/agent shutdown
    METRIC_SAMPLE = "METRIC_SAMPLE"  # Quantitative measurement
    ECOSYSTEM_EVENT = "ECOSYSTEM_EVENT"  # External event (blockchain, API)
    HUMAN_INPUT = "HUMAN_INPUT"  # User/governance input
    ACTION_RESULT = "ACTION_RESULT"  # Result of previously taken action

    def __str__(self) -> str:
        return self.value


# ============================================================================
# Aggregation Types (Layer 2 Processed State)
# ============================================================================

class AggregationType(Enum):
    """Types of aggregated state information."""

    PROCESS_METRICS = "PROCESS_METRICS"  # Aggregated process stats
    SYSTEM_STATE = "SYSTEM_STATE"  # Overall system health
    ECOSYSTEM_STATE = "ECOSYSTEM_STATE"  # Aggregate community state
    HEALTH_SUMMARY = "HEALTH_SUMMARY"  # Organism vitals

    def __str__(self) -> str:
        return self.value


# ============================================================================
# Judgment Types (Layer 3 Verdicts)
# ============================================================================

class JudgmentType(Enum):
    """Types of judgments made by Dogs."""

    STATE_EVALUATION = "STATE_EVALUATION"  # Is the state healthy?
    EMERGENCE_ALERT = "EMERGENCE_ALERT"  # New pattern emerged
    PATTERN_DETECTED = "PATTERN_DETECTED"  # Recognizable pattern
    LEARNING_UPDATE = "LEARNING_UPDATE"  # Q-Table updated

    def __str__(self) -> str:
        return self.value


# ============================================================================
# Action Types (Layer 4 Handlers)
# ============================================================================

class ActionType(Enum):
    """Types of actions the organism can take."""

    APPLY_CONFIG = "APPLY_CONFIG"  # Update configuration
    DEPLOY_COMPONENT = "DEPLOY_COMPONENT"  # Spawn new agent/component
    EXTERNAL_CALL = "EXTERNAL_CALL"  # Call external API/blockchain
    SIGNAL_HUMAN = "SIGNAL_HUMAN"  # Signal to human operator

    def __str__(self) -> str:
        return self.value


# ============================================================================
# Message Header
# ============================================================================

@dataclass
class MessageHeader:
    """Header for all LNSP messages.

    Attributes:
        layer: Protocol layer (1-4)
        message_id: Unique message identifier (8-char hex)
        timestamp: Unix timestamp (seconds)
        source: Originating component name
        target: Optional target component name
        version: Protocol version (default: "1.0.0")
    """

    layer: Layer
    message_id: str  # 8-char hex
    timestamp: float  # Unix time
    source: str  # Component name
    target: Optional[str] = None
    version: str = "1.0.0"

    def to_dict(self) -> dict[str, Any]:
        """Convert header to dictionary."""
        return {
            "layer": self.layer.name,
            "layer_value": self.layer.value,
            "message_id": self.message_id,
            "timestamp": self.timestamp,
            "timestamp_iso": datetime.fromtimestamp(self.timestamp).isoformat(),
            "source": self.source,
            "target": self.target,
            "version": self.version,
        }


# ============================================================================
# Message Metadata
# ============================================================================

@dataclass
class Metadata:
    """Metadata attached to all LNSP messages.

    Attributes:
        instance_id: Unique organism instance ID
        region: Optional regional identifier (for distributed systems)
        route_trace: List of components this message traversed
        feedback: Whether this message expects feedback
        closes_action_id: Optional ID of action this message completes
    """

    instance_id: str  # Unique organism ID
    region: Optional[str] = None
    route_trace: list[str] = field(default_factory=list)
    feedback: bool = False
    closes_action_id: Optional[str] = None

    def to_dict(self) -> dict[str, Any]:
        """Convert metadata to dictionary."""
        return asdict(self)


# ============================================================================
# LNSP Message (Main Message Type)
# ============================================================================

@dataclass
class LNSPMessage:
    """A message in the Layered Nervous System Protocol.

    This is the base message type for all inter-agent communication in CYNIC.

    Attributes:
        header: MessageHeader with layer, ID, timestamp, source/target
        payload: Dictionary containing message data (type-specific)
        metadata: Metadata with instance_id, route_trace, feedback info
    """

    header: MessageHeader
    payload: dict[str, Any]
    metadata: Metadata

    def to_dict(self) -> dict[str, Any]:
        """Convert entire message to dictionary (including nested objects)."""
        return {
            "header": self.header.to_dict(),
            "payload": self.payload,
            "metadata": self.metadata.to_dict(),
        }

    def __repr__(self) -> str:
        return (
            f"<LNSPMessage: {self.header.layer.name} "
            f"id={self.header.message_id} "
            f"source={self.header.source} "
            f"target={self.header.target}>"
        )
