#!/usr/bin/env python3
"""
Tier 2 INFRASTRUCTURE: Talaria event ingestion bridge.
Typed Talaria event helpers for CYNIC /observe ingestion."""
from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
from hashlib import sha256
import json
from typing import Any, Literal

TalariaScope = Literal[
    "talaria.pow.team",
    "talaria.poh.user",
    "talaria.chess.signal",
    "talaria.reputation.public",
    "talaria.governance.review",
    "talaria.futarchy.market",
    "talaria.alignment.capital",
    "talaria.incident.trace",
    "talaria.comms.signal",
]
TalariaVisibility = Literal["public", "internal", "redacted"]
TalariaActorKind = Literal["human", "agent", "system", "market", "wallet", "app_user"]
TalariaConfidence = Literal["observed", "deduced", "inferred", "conjecture"]


@dataclass(frozen=True)
class TalariaEvent:
    """Canonical Talaria event envelope before projection to CYNIC /observe."""

    scope: TalariaScope
    actor: str
    actor_kind: TalariaActorKind
    kind: str
    title: str
    summary: str
    source: str
    confidence: TalariaConfidence
    visibility: TalariaVisibility
    value: dict[str, Any] = field(default_factory=dict)
    source_url: str | None = None
    evidence_hash: str | None = None
    repo: str | None = None
    commit_sha: str | None = None
    proposal_id: str | None = None
    wallet_address: str | None = None
    subject_id: str | None = None
    depends_on: list[str] = field(default_factory=list)
    created_at: str = field(
        default_factory=lambda: datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    )
    id: str | None = None

    def envelope(self) -> dict[str, Any]:
        """Return the stable JSON-compatible Talaria event envelope."""
        data: dict[str, Any] = {
            "id": self.id or self.stable_id(),
            "scope": self.scope,
            "actor": self.actor,
            "actorKind": self.actor_kind,
            "kind": self.kind,
            "title": self.title,
            "summary": self.summary,
            "source": self.source,
            "confidence": self.confidence,
            "visibility": self.visibility,
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
            "dependsOn": self.depends_on or None,
        }
        for key, val in optional.items():
            if val:
                data[key] = val
        return data

    def stable_id(self) -> str:
        """Deterministic ID from non-secret event identity fields."""
        seed = "|".join([
            self.scope,
            self.kind,
            self.actor,
            self.subject_id or "",
            self.wallet_address or "",
            self.source,
            self.created_at,
        ])
        return "talaria_" + sha256(seed.encode("utf-8")).hexdigest()[:24]

    def observe_payload(self, agent_id: str, status: str = "observed") -> dict[str, Any]:
        """Project this event to the current CYNIC /observe request shape."""
        envelope = self.envelope()
        envelope_json = json.dumps(envelope, sort_keys=True, separators=(",", ":"))
        tags = [self.scope, self.kind, f"visibility:{self.visibility}"]
        return {
            "tool": self.kind[:64],
            "target": self.subject_id or self.wallet_address or envelope["id"],
            "domain": self.scope,
            "status": status,
            "context": f"{self.summary}\n\nTALARIA_EVENT={envelope_json}",
            "project": "talaria",
            "agent_id": agent_id,
            "tags": tags,
            "value": envelope,
            "confidence": self.confidence,
            "depends_on": self.depends_on,
        }


def evidence_hash(value: dict[str, Any]) -> str:
    """Hash a JSON-like evidence dict without importing a heavier canonicalizer."""
    return sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
