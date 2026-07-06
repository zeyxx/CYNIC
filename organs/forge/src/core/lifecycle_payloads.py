"""Pure lifecycle payload helpers for organ-forge.

This module keeps request payload construction and validation separate from the
transport/client adapter and from lifecycle runtime evidence concerns.
"""

from __future__ import annotations

import json
from pathlib import Path

from organs.contracts.src.surface_contracts import json_object_or_raise
from organs.contracts.src.surface_contracts import read_json_file_or_default

from .agent_proposals import (
    build_lifecycle_agent_proposal,
    validate_lifecycle_agent_proposal,
)
from .project_root import resolve_project_root_from_script


def _default_registry_path() -> Path:
    return resolve_project_root_from_script(__file__) / "infra" / "registry.json"


def _load_registry(path: str | Path | None = None) -> dict:
    target_path = Path(path) if path else _default_registry_path()
    return read_json_file_or_default(target_path, {})


def lifecycle_profile_intents(
    profile: str,
    registry: dict | None = None,
) -> list[str]:
    registry = registry or _load_registry()
    lifecycle = registry.get("lifecycle", {}).get("kernel_release")
    if not lifecycle:
        raise RuntimeError("missing lifecycle.kernel_release")
    profiles = lifecycle.get("request_profiles", {})
    intents = profiles.get(profile)
    if not intents:
        raise RuntimeError(f"unknown lifecycle request profile: {profile}")
    unknown = [intent for intent in intents if intent not in lifecycle.get("intents", {})]
    if unknown:
        raise RuntimeError(
            f"lifecycle request profile {profile} has unknown intents: {','.join(unknown)}"
        )
    return intents


def build_lifecycle_task_payload(
    intent: str,
    domain: str,
    agent_id: str,
    registry: dict | None = None,
) -> dict:
    registry = registry or _load_registry()
    lifecycle = registry.get("lifecycle", {}).get("kernel_release")
    if not lifecycle:
        raise RuntimeError("missing lifecycle.kernel_release")

    nodes = registry.get("nodes", [])
    build_node = next(
        (
            node
            for node in nodes
            if node.get("role") == lifecycle["build_runner_role"]
        ),
        None,
    )
    deploy_node = next(
        (
            node
            for node in nodes
            if node.get("role") == lifecycle["deploy_target_role"]
            and node.get(lifecycle.get("deploy_target_flag", "deploy_target")) is True
        ),
        None,
    )
    if not build_node:
        raise RuntimeError("lifecycle build runner role does not resolve")
    if not deploy_node:
        raise RuntimeError("lifecycle deploy target role does not resolve")

    intent_policy = lifecycle.get("intents", {}).get(intent)
    if not intent_policy:
        raise RuntimeError(f"unknown lifecycle intent: {intent}")

    repo_targets = lifecycle.get("repo_targets", [])
    intent_targets = intent_policy.get("targets", repo_targets)
    intent_coord_targets = intent_policy.get("coord_targets", intent_targets)
    content = {
        "lifecycle": "kernel_release",
        "intent": intent,
        "objective": intent_policy["objective"],
        "orchestrator_organ": lifecycle["orchestrator_organ"],
        "domain": lifecycle["task_domain"],
        "build_target": build_node["name"],
        "build_runner_role": lifecycle["build_runner_role"],
        "build_policy": lifecycle.get("build_policy", {}),
        "deploy_target": deploy_node["name"],
        "deploy_target_role": lifecycle["deploy_target_role"],
        "required_node_status": lifecycle.get("required_node_status"),
        "activation_requirements": lifecycle.get("activation_requirements", {}),
        "runtime_paths": lifecycle.get("runtime_paths", {}),
        "runtime_path_proofs": lifecycle.get("runtime_path_proofs", {}),
        "activation_evidence_contract": lifecycle.get("activation_evidence_contract", {}),
        "fleet_requirements": lifecycle.get("fleet_requirements", {}),
        "mcp_freshness_controller": lifecycle.get("mcp_freshness_controller", {}),
        "requires_e2e": lifecycle.get("requires_e2e", True),
        "targets": intent_targets,
        "coord_targets": intent_coord_targets,
        "deploy": intent_policy.get("deploy", False),
    }
    if intent_policy.get("bootstrap_phase") is True:
        content["bootstrap_phase"] = True
    content["operator_interface_proposal"] = build_lifecycle_agent_proposal(
        content,
        source_agent_id=agent_id,
    )
    return {
        "kind": lifecycle["task_kind"],
        "domain": domain,
        "agent_id": agent_id,
        "content": json.dumps(content, separators=(",", ":")),
    }


def validate_lifecycle_task_payload(
    payload: dict,
    registry: dict | None = None,
) -> dict:
    registry = registry or _load_registry()
    lifecycle = registry.get("lifecycle", {}).get("kernel_release")
    if not lifecycle:
        raise RuntimeError("missing lifecycle.kernel_release")

    errors = []
    if payload.get("kind") != lifecycle.get("task_kind"):
        errors.append("payload.kind does not match lifecycle task_kind")
    if not payload.get("domain"):
        errors.append("payload.domain missing")
    if not payload.get("agent_id"):
        errors.append("payload.agent_id missing")

    raw_content = payload.get("content", "")
    try:
        content = json_object_or_raise(raw_content, RuntimeError("payload.content is not valid JSON"))
    except RuntimeError as exc:
        if str(exc) == "payload.content is not valid JSON":
            raise
        raise RuntimeError(f"payload.content is not valid JSON: {exc}") from exc

    nodes = registry.get("nodes", [])
    build_node = next(
        (node for node in nodes if node.get("role") == lifecycle["build_runner_role"]),
        None,
    )
    deploy_node = next(
        (
            node
            for node in nodes
            if node.get("role") == lifecycle["deploy_target_role"]
            and node.get(lifecycle.get("deploy_target_flag", "deploy_target")) is True
        ),
        None,
    )
    intent = content.get("intent")
    intent_policy = lifecycle.get("intents", {}).get(intent)
    repo_targets = lifecycle.get("repo_targets", [])
    expected_targets = (
        intent_policy.get("targets", repo_targets) if intent_policy else repo_targets
    )
    expected_coord_targets = (
        intent_policy.get("coord_targets", expected_targets)
        if intent_policy
        else expected_targets
    )

    expected = {
        "lifecycle": "kernel_release",
        "orchestrator_organ": lifecycle.get("orchestrator_organ"),
        "domain": lifecycle.get("task_domain"),
        "build_target": build_node.get("name") if build_node else None,
        "build_runner_role": lifecycle.get("build_runner_role"),
        "build_policy": lifecycle.get("build_policy", {}),
        "deploy_target": deploy_node.get("name") if deploy_node else None,
        "deploy_target_role": lifecycle.get("deploy_target_role"),
        "required_node_status": lifecycle.get("required_node_status"),
        "activation_requirements": lifecycle.get("activation_requirements", {}),
        "runtime_paths": lifecycle.get("runtime_paths", {}),
        "runtime_path_proofs": lifecycle.get("runtime_path_proofs", {}),
        "activation_evidence_contract": lifecycle.get("activation_evidence_contract", {}),
        "fleet_requirements": lifecycle.get("fleet_requirements", {}),
        "mcp_freshness_controller": lifecycle.get("mcp_freshness_controller", {}),
        "requires_e2e": lifecycle.get("requires_e2e", True),
        "targets": expected_targets,
        "coord_targets": expected_coord_targets,
    }
    for key, expected_value in expected.items():
        if content.get(key) != expected_value:
            errors.append(f"content.{key} does not match lifecycle registry")

    if not intent_policy:
        errors.append("content.intent is unknown")
    else:
        if content.get("objective") != intent_policy.get("objective"):
            errors.append("content.objective does not match lifecycle intent")
        if content.get("deploy") != intent_policy.get("deploy", False):
            errors.append("content.deploy does not match lifecycle intent")
        if intent_policy.get("bootstrap_phase") is True and content.get("bootstrap_phase") is not True:
            errors.append("content.bootstrap_phase does not match lifecycle intent")
        if intent_policy.get("bootstrap_phase") is not True and content.get("bootstrap_phase") is True:
            errors.append("content.bootstrap_phase is only allowed for bootstrap intents")

    if not isinstance(content.get("coord_targets"), list) or not content.get("coord_targets"):
        errors.append("content.coord_targets must be a non-empty list")
    errors.extend(validate_lifecycle_agent_proposal(content))

    if errors:
        raise RuntimeError("; ".join(errors))
    return {
        "status": "ok",
        "lifecycle": "kernel_release",
        "intent": intent,
        "coord_targets": content["coord_targets"],
    }


def build_lifecycle_profile_payloads(
    profile: str,
    domain: str,
    agent_id: str,
    registry: dict | None = None,
) -> dict:
    registry = registry or _load_registry()
    intents = lifecycle_profile_intents(profile, registry)
    payloads = [
        build_lifecycle_task_payload(
            intent=intent,
            domain=domain,
            agent_id=agent_id,
            registry=registry,
        )
        for intent in intents
    ]
    for payload in payloads:
        content = json.loads(payload["content"])
        content["request_profile"] = profile
        payload["content"] = json.dumps(content, separators=(",", ":"))
    validations = [
        validate_lifecycle_task_payload(payload, registry)
        for payload in payloads
    ]
    return {
        "status": "ok",
        "profile": profile,
        "intents": intents,
        "payloads": payloads,
        "validations": validations,
    }
