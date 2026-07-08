"""Dependency convergence planning for organ-lifecycle."""

from __future__ import annotations

import hashlib
import json
from datetime import datetime, timezone
from typing import Any


INFERENCE_REQUEST_SCHEMA = "cynic.dependency_convergence_inference_request.v1"
PROPOSAL_SCHEMA = "cynic.dependency_convergence_proposal.v1"
AGENT_PROPOSAL_SCHEMA = "cynic.agent_proposal.v1"


def build_dependency_convergence_request(
    analysis: dict[str, Any],
    registry: dict[str, Any],
) -> dict[str, Any]:
    lifecycle = registry.get("lifecycle", {}).get("kernel_release", {})
    inference_node = next(
        (node for node in registry.get("nodes", []) if node.get("role") == "inference_node"),
        {},
    )
    return {
        "schema": INFERENCE_REQUEST_SCHEMA,
        "domain": "dependency_convergence",
        "created_at": datetime.now(timezone.utc).replace(microsecond=0).isoformat(),
        "sovereign_inference": {
            "required": True,
            "node": inference_node.get("name"),
            "constellation": inference_node.get("constellation", "inference"),
            "fallback": "deterministic_blocked_proposal",
        },
        "context": {
            "summary": analysis.get("summary", {}),
            "cycles": analysis.get("cycles", []),
            "unresolved_dependencies": analysis.get("unresolved_dependencies", []),
            "execution": {
                "state_layers": analysis.get("execution", {}).get("state_layers", {}),
                "canonical_stages": analysis.get("execution", {}).get("canonical_stages", []),
            },
            "registry_excerpt": {
                "orchestrator_organ": lifecycle.get("orchestrator_organ"),
                "task_kind": lifecycle.get("task_kind"),
                "task_domain": lifecycle.get("task_domain"),
                "request_profiles": lifecycle.get("request_profiles", {}),
            },
        },
        "output_contract": {
            "schema": PROPOSAL_SCHEMA,
            "required_fields": [
                "cycle",
                "mutation_strategy",
                "targets",
                "coord_targets",
                "expected_effect",
                "proof_commands",
                "rollback",
                "confidence",
            ],
        },
    }


def deterministic_dependency_convergence_proposal(request: dict[str, Any]) -> dict[str, Any]:
    context = request.get("context", {})
    cycles = context.get("cycles", [])
    unresolved = context.get("unresolved_dependencies", [])
    first_cycle = cycles[0] if cycles else []
    targets = ["infra/registry.json"]
    return {
        "schema": PROPOSAL_SCHEMA,
        "status": "blocked" if cycles or unresolved else "ok",
        "source": "deterministic_fallback",
        "requires_sovereign_inference": bool(cycles or unresolved),
        "cycle": first_cycle,
        "unresolved_dependencies": unresolved,
        "mutation_strategy": (
            "request_sovereign_inference_review"
            if cycles or unresolved
            else "no_mutation_required"
        ),
        "targets": targets if cycles or unresolved else [],
        "coord_targets": targets if cycles or unresolved else [],
        "expected_effect": "break dependency cycle or resolve missing edge without hidden runtime truth",
        "proof_commands": [
            "python3 organs/lifecycle/src/entrypoints/cli.py analyze-chain --registry infra/registry.json",
            "python3 organs/lifecycle/src/entrypoints/cli.py convergence-pack --registry infra/registry.json",
        ],
        "rollback": {
            "mode": "git_revert",
            "scope": targets,
        },
        "confidence": 0.0 if cycles or unresolved else 1.0,
    }


def build_dependency_convergence_pack(
    analysis: dict[str, Any],
    registry: dict[str, Any],
) -> dict[str, Any]:
    request = build_dependency_convergence_request(analysis, registry)
    proposal = deterministic_dependency_convergence_proposal(request)
    operator_proposal = build_agent_proposal_envelope(proposal, request)
    return {
        "schema": "cynic.dependency_convergence_pack.v1",
        "status": proposal["status"],
        "analysis_status": analysis.get("status", "unknown"),
        "inference_request": request,
        "proposal": proposal,
        "operator_interface_proposal": operator_proposal,
        "dispatch": {
            "kind": "hermes",
            "domain": "organ-lifecycle",
            "consumer": "organ-lifecycle-hermes-agent",
            "targets": proposal.get("targets", []),
            "coord_targets": proposal.get("coord_targets", []),
        },
    }


def build_agent_proposal_envelope(
    proposal: dict[str, Any],
    inference_request: dict[str, Any],
) -> dict[str, Any]:
    fingerprint = hashlib.sha256(
        json.dumps(
            {
                "proposal": proposal,
                "context": inference_request.get("context", {}),
            },
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8"),
    ).hexdigest()[:16]
    targets = proposal.get("targets", [])
    coord_targets = proposal.get("coord_targets", targets)
    requires_review = proposal.get("requires_sovereign_inference") is True
    title = "Dependency convergence proposal"
    summary = (
        "Sovereign inference review is required to resolve lifecycle dependency cycles or missing edges."
        if requires_review
        else "Deterministic lifecycle analysis found no dependency convergence mutation to propose."
    )
    return {
        "schema": AGENT_PROPOSAL_SCHEMA,
        "proposal_id": f"dependency-convergence-{fingerprint}",
        "source": {
            "agent_id": "organ-lifecycle-convergence",
            "agent_kind": "hermes",
            "producer": "organ-lifecycle",
            "inference_mode": "proposal_only",
        },
        "domain": "dependency_convergence",
        "objective": "Expose lifecycle dependency convergence proposals for operator review before mutation or publication.",
        "visibility": "operator_private",
        "operator_review": {
            "state": "pending" if requires_review else "approved",
            "required_before_publication": True,
            "review_reasons": [
                "agent proposals are operator-facing before public projection",
                "dependency mutations can affect organism lifecycle",
            ],
        },
        "public_projection": {
            "schema": "cynic.agent_proposal_public_projection.v1",
            "eligible": False,
            "redaction_status": "needs_review",
            "title": title,
            "summary": summary,
            "status_label": proposal.get("status", "unknown"),
            "evidence_refs": proposal.get("proof_commands", []),
            "risk_label": "lifecycle_mutation" if requires_review else "none",
            "withheld_fields": [
                "raw inference request",
                "private runtime context",
                "operator local paths",
            ],
        },
        "targets": targets,
        "coord_targets": coord_targets,
        "proof": {
            "proof_commands": proposal.get("proof_commands", []),
            "evidence_refs": proposal.get("proof_commands", []),
        },
        "rollback": proposal.get("rollback", {}),
        "confidence": proposal.get("confidence", 0.0),
        "technical_payload_schema": proposal.get("schema"),
    }


def build_dependency_convergence_hermes_task(pack: dict[str, Any]) -> dict[str, Any]:
    proposal = pack.get("proposal", {})
    operator_proposal = pack.get("operator_interface_proposal", {})
    inference_request = pack.get("inference_request", {})
    targets = proposal.get("targets", [])
    coord_targets = proposal.get("coord_targets", targets)
    return {
        "kind": "hermes",
        "domain": "organ-lifecycle",
        "agent_id": "organ-lifecycle-convergence",
        "consumer": "organ-lifecycle-hermes-agent",
        "priority": 95,
        "content": {
            "schema": "cynic.dependency_convergence_hermes_task.v1",
            "objective": "Use sovereign agentic inference to propose a bounded dependency convergence mutation.",
            "mode": "proposal_only",
            "mutation_allowed": False,
            "domain": "dependency_convergence",
            "targets": targets,
            "coord_targets": coord_targets,
            "inference_request": inference_request,
            "deterministic_proposal": proposal,
            "operator_interface_proposal": operator_proposal,
            "output_contract": inference_request.get("output_contract", {}),
            "done_when": [
                "return strict JSON matching cynic.dependency_convergence_proposal.v1",
                "do not edit files directly",
                "include proof_commands and rollback",
                "include or preserve cynic.agent_proposal.v1 operator_interface_proposal",
                "state whether deterministic proposal is sufficient or needs mutation",
            ],
        },
    }
