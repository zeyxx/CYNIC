"""Agent proposal envelopes for organ-forge lifecycle tasks."""

from __future__ import annotations

import hashlib
import json
from typing import Any


AGENT_PROPOSAL_SCHEMA = "cynic.agent_proposal.v1"
PUBLIC_PROJECTION_SCHEMA = "cynic.agent_proposal_public_projection.v1"


def _fingerprint(payload: dict[str, Any]) -> str:
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()[:16]


def _proposal_id(prefix: str, content: dict[str, Any]) -> str:
    return f"{prefix}-{_fingerprint(content)}"


def build_lifecycle_agent_proposal(
    content: dict[str, Any],
    *,
    source_agent_id: str,
    producer: str = "organ-forge",
) -> dict[str, Any]:
    action = content.get("action") if isinstance(content.get("action"), dict) else {}
    intent = content.get("intent") or action.get("action") or "lifecycle"
    is_activation = bool(action)
    title = (
        f"Lifecycle activation: {action.get('action', 'unknown')}"
        if is_activation
        else f"Lifecycle request: {intent}"
    )
    summary = (
        f"Request {producer} to activate lifecycle evidence for {action.get('node', 'the organism')}."
        if is_activation
        else f"Request {producer} to execute {intent} through the registry-selected forge lifecycle."
    )
    proof_commands = [
        "python3 organs/forge/src/entrypoints/cli.py lifecycle-check",
        "python3 organs/forge/src/entrypoints/cli.py lifecycle-profile-payloads "
        f"{content.get('intent', 'gate-2')}",
    ]
    if is_activation:
        proof_commands = [
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-requests",
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers",
        ]
    return {
        "schema": AGENT_PROPOSAL_SCHEMA,
        "proposal_id": _proposal_id("forge-lifecycle", content),
        "source": {
            "agent_id": source_agent_id,
            "agent_kind": "hermes",
            "producer": producer,
            "inference_mode": "deterministic_lifecycle",
        },
        "domain": "organ-forge",
        "objective": content.get("objective", "Review organ-forge lifecycle request."),
        "visibility": "operator_private",
        "operator_review": {
            "state": "pending",
            "required_before_publication": True,
            "review_reasons": [
                "forge lifecycle tasks can mutate runtime state",
                "public projection requires proof and redaction",
            ],
        },
        "public_projection": {
            "schema": PUBLIC_PROJECTION_SCHEMA,
            "eligible": False,
            "redaction_status": "needs_review",
            "title": title,
            "summary": summary,
            "status_label": "pending_operator_review",
            "evidence_refs": proof_commands,
            "risk_label": (
                "deployment"
                if content.get("deploy")
                else "bootstrap"
                if content.get("bootstrap_phase")
                else "lifecycle_activation"
                if is_activation
                else "validation"
            ),
            "withheld_fields": [
                "raw lifecycle task content",
                "private runtime context",
                "operator local paths",
            ],
        },
        "targets": content.get("targets", []),
        "coord_targets": content.get("coord_targets", content.get("targets", [])),
        "proof": {
            "proof_commands": proof_commands,
            "evidence_refs": proof_commands,
        },
        "rollback": {
            "mode": "operator_review_or_git_revert",
            "scope": content.get("coord_targets", content.get("targets", [])),
        },
        "confidence": 1.0,
        "technical_payload_schema": (
            "cynic.lifecycle.activation_request.v1"
            if is_activation
            else "cynic.lifecycle.kernel_release_request.v1"
        ),
    }


def validate_lifecycle_agent_proposal(content: dict[str, Any]) -> list[str]:
    proposal = content.get("operator_interface_proposal")
    errors: list[str] = []
    if not isinstance(proposal, dict):
        return ["content.operator_interface_proposal missing"]
    if proposal.get("schema") != AGENT_PROPOSAL_SCHEMA:
        errors.append("content.operator_interface_proposal.schema invalid")
    if proposal.get("visibility") != "operator_private":
        errors.append("content.operator_interface_proposal.visibility must be operator_private")
    review = proposal.get("operator_review", {})
    if not isinstance(review, dict) or review.get("state") != "pending":
        errors.append("content.operator_interface_proposal.operator_review.state must be pending")
    public_projection = proposal.get("public_projection", {})
    if not isinstance(public_projection, dict):
        errors.append("content.operator_interface_proposal.public_projection missing")
    else:
        if public_projection.get("eligible") is not False:
            errors.append("content.operator_interface_proposal.public_projection.eligible must be false")
        if public_projection.get("redaction_status") != "needs_review":
            errors.append(
                "content.operator_interface_proposal.public_projection.redaction_status must be needs_review"
            )
    if proposal.get("targets") != content.get("targets", []):
        errors.append("content.operator_interface_proposal.targets must match content.targets")
    if proposal.get("coord_targets") != content.get("coord_targets", content.get("targets", [])):
        errors.append("content.operator_interface_proposal.coord_targets must match content.coord_targets")
    return errors
