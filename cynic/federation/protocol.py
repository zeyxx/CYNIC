from __future__ import annotations
from dataclasses import dataclass
from datetime import datetime, UTC

FEDERATION_VERSION = "1.0"

@dataclass(frozen=True)
class FederationMessage:
    sender_id: str
    q_table_snapshot: dict
    total_judgments: int
    unnameable_patterns: list[str]
    sent_at: datetime
    version: str = FEDERATION_VERSION

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
    def from_dict(cls, data: dict) -> "FederationMessage":
        return cls(
            sender_id=data["sender_id"],
            q_table_snapshot=data["q_table_snapshot"],
            total_judgments=data["total_judgments"],
            unnameable_patterns=data.get("unnameable_patterns", []),
            sent_at=datetime.fromisoformat(data["sent_at"]),
            version=data.get("version", FEDERATION_VERSION),
        )
