"""
Federation Protocol â€" Data structures for P2P message passing.

Defines the FederationMessage protocol for sharing Q-Table snapshots,
judgment history, and unnameable patterns across federated CYNIC instances.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from types import MappingProxyType

FEDERATION_VERSION = "1.0"


@dataclass(frozen=True)
class FederationMessage:
    sender_id: str
    q_table_snapshot: dict
    total_judgments: int
    unnameable_patterns: list[str]
    sent_at: datetime
    version: str = FEDERATION_VERSION

    def __post_init__(self) -> None:
        object.__setattr__(self, "q_table_snapshot", MappingProxyType(self.q_table_snapshot))
        object.__setattr__(self, "unnameable_patterns", tuple(self.unnameable_patterns))

    def to_dict(self) -> dict:
        return {
            "sender_id": self.sender_id,
            "q_table_snapshot": self.q_table_snapshot,
            "total_judgments": self.total_judgments,
            "unnameable_patterns": self.unnameable_patterns,
            "sent_at": self.sent_at.isoformat(),
            "version": self.version,
        }

    @classmethod
    def from_dict(cls, data: dict) -> FederationMessage:
        try:
            return cls(
                sender_id=data["sender_id"],
                q_table_snapshot=data["q_table_snapshot"],
                total_judgments=data["total_judgments"],
                unnameable_patterns=data.get("unnameable_patterns", []),
                sent_at=datetime.fromisoformat(data["sent_at"]),
                version=data.get("version", FEDERATION_VERSION),
            )
        except KeyError as exc:
            raise ValueError(f"FederationMessage.from_dict missing required field: {exc}") from exc
