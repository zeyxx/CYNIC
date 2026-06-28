from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any


class Gate(str, Enum):
    HUMAN_SUBMISSION = "human_submission"
    PUBLIC_IDENTITY = "public_identity"
    WALLET_OR_FUNDS = "wallet_or_funds"
    SPONSOR_CREDENTIALS = "sponsor_credentials"
    EXTERNAL_BROWSER = "external_browser"


@dataclass(frozen=True)
class Opportunity:
    slug: str
    title: str
    url: str
    deadline: str | None
    reward_amount: float
    reward_asset: str
    skills: tuple[str, ...]
    region: str
    sponsor: str
    description_text: str
    resources: tuple[str, ...] = field(default_factory=tuple)

    @property
    def deadline_dt(self) -> datetime | None:
        if not self.deadline:
            return None
        value = self.deadline.replace("Z", "+00:00")
        try:
            return datetime.fromisoformat(value)
        except ValueError:
            return None

    def days_until_deadline(self, now: datetime | None = None) -> float | None:
        deadline = self.deadline_dt
        if deadline is None:
            return None
        now = now or datetime.now(timezone.utc)
        return (deadline - now).total_seconds() / 86400


@dataclass(frozen=True)
class OpportunityScore:
    slug: str
    title: str
    score: float
    layer: str
    gates: tuple[Gate, ...]
    reasons: tuple[str, ...]
    next_action: str
    browser_task: dict[str, Any]
