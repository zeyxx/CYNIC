"""
Îº-PULSE â€” Unified Message Format for Îº-NET Protocol.
Defines the structure of the data flowing through the organism's nerves.
"""

from __future__ import annotations

import time
from dataclasses import asdict, dataclass, field
from enum import Enum
from typing import Any


class PulseType(Enum):
    SOMATIC_SYNC = "SOMATIC_SYNC"  # Heartbeat + Metrics
    NEURAL_PULSE = "NEURAL_PULSE"  # Thinking + Axioms
    INTENT_SIGNAL = "INTENT_SIGNAL"  # Action proposal
    SENSORY_INPUT = "SENSORY_INPUT"  # Input from CLI/Human
    SYSTEM_ALERT = "SYSTEM_ALERT"  # Critical errors


@dataclass
class PulseMessage:
    """A single packet of data in the Îº-NET ecosystem."""

    type: PulseType
    organism_id: str = "cynic-core"
    version: str = "1.0"
    timestamp: float = field(default_factory=time.time)
    data: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        d = asdict(self)
        d["type"] = self.type.value
        return d

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> PulseMessage:
        return cls(
            type=PulseType(data["type"]),
            organism_id=data.get("organism_id", "cynic-core"),
            version=data.get("version", "1.0"),
            timestamp=data.get("timestamp", time.time()),
            data=data.get("data", {}),
        )
