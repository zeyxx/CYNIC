"""Talaria Observatory event helpers.

CYNIC owns Talaria observatory semantics. Product systems such as B&C emit
signals; CYNIC normalizes them into Talaria events and observations.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any


class TalariaScope(str, Enum):
    POW_TEAM = "talaria.pow.team"
    POH_USER = "talaria.poh.user"
    CHESS_SIGNAL = "talaria.chess.signal"
    REPUTATION_PUBLIC = "talaria.reputation.public"
    GOVERNANCE_REVIEW = "talaria.governance.review"
    FUTARCHY_MARKET = "talaria.futarchy.market"
    ALIGNMENT_CAPITAL = "talaria.alignment.capital"
    INCIDENT_TRACE = "talaria.incident.trace"
    COMMS_SIGNAL = "talaria.comms.signal"


class TalariaVisibility(str, Enum):
    PUBLIC = "public"
    INTERNAL = "internal"
    REDACTED = "redacted"


class TalariaActorKind(str, Enum):
    HUMAN = "human"
    AGENT = "agent"
    SYSTEM = "system"
    MARKET = "market"
    WALLET = "wallet"
    APP_USER = "app_user"


class TalariaConfidence(str, Enum):
    OBSERVED = "observed"
    DEDUCED = "deduced"
    INFERRED = "inferred"
    CONJECTURE = "conjecture"


@dataclass(frozen=True)
class TalariaEvent:
    scope: TalariaScope
    actor: str
    actor_kind: TalariaActorKind
    kind: str
    title: str
    summary: str
    source: str
    confidence: TalariaConfidence = TalariaConfidence.OBSERVED
    visibility: TalariaVisibility = TalariaVisibility.INTERNAL
    created_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    source_url: str | None = None
    evidence_hash: str | None = None
    repo: str | None = None
    commit_sha: str | None = None
    proposal_id: str | None = None
    wallet_address: str | None = None
    subject_id: str | None = None
    confidence_band: str | None = None
    value: dict[str, Any] = field(default_factory=dict)
    depends_on: list[str] = field(default_factory=list)

    def to_value(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "schema": "talaria.event.v1",
            "scope": self.scope.value,
            "actor": self.actor,
            "actorKind": self.actor_kind.value,
            "kind": self.kind,
            "title": self.title,
            "summary": self.summary,
            "source": self.source,
            "confidence": self.confidence.value,
            "visibility": self.visibility.value,
            "createdAt": self.created_at,
            "value": self.value,
        }
        optional = {
            "sourceUrl": self.source_url,
            "evidenceHash": self.evidence_hash,
            "repo": self.repo,
            "commitSha": self.commit_sha,
            "proposalId": self.proposal_id,
            "walletAddress": self.wallet_address,
            "subjectId": self.subject_id,
            "confidenceBand": self.confidence_band,
            "dependsOn": self.depends_on or None,
        }
        for key, value in optional.items():
            if value is not None:
                data[key] = value
        return data

    def to_observe_payload(self, *, agent_id: str, status: str = "observed") -> dict[str, Any]:
        tags = [
            "talaria",
            self.scope.value,
            self.kind,
            f"visibility:{self.visibility.value}",
        ]
        if self.confidence_band:
            tags.append(f"confidence:{self.confidence_band}")

        return {
            "project": "talaria",
            "agent_id": agent_id,
            "tool": self.kind[:64],
            "target": self.subject_id or self.wallet_address or self.actor,
            "domain": self.scope.value,
            "status": status,
            "context": self.summary,
            "tags": tags,
            "value": self.to_value(),
            "confidence": self.confidence.value,
            "depends_on": self.depends_on,
        }


def blitzchill_poh_observed_event(
    *,
    wallet_address: str,
    verified: bool,
    source_url: str,
    raw: dict[str, Any],
) -> TalariaEvent:
    games_completed = raw.get("gamesCompleted") or raw.get("games_completed") or raw.get("games")
    archetype = raw.get("archetype") or raw.get("archetypeId")
    confidence_value = raw.get("confidence")
    summary = "B&C PoH verification observed"
    if verified:
        summary = "B&C reports wallet as PoH verified"
    elif verified is False:
        summary = "B&C reports wallet as not PoH verified"

    value = {
        "producer": "blitz-and-chill",
        "verified": verified,
        "gamesCompleted": games_completed,
        "archetype": archetype,
        "producerConfidence": confidence_value,
        "raw": raw,
    }

    return TalariaEvent(
        scope=TalariaScope.POH_USER,
        actor=wallet_address,
        actor_kind=TalariaActorKind.WALLET,
        kind="bnc.poh.verification_observed",
        title="B&C PoH verification observed",
        summary=summary,
        source="blitz-and-chill",
        source_url=source_url,
        wallet_address=wallet_address,
        subject_id=f"solana:{wallet_address}",
        confidence=TalariaConfidence.OBSERVED,
        visibility=TalariaVisibility.INTERNAL,
        value=value,
    )
