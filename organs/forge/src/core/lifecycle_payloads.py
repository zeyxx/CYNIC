"""Pure lifecycle payload helpers for organ-forge.

This module keeps request payload construction and validation separate from the
transport/client adapter and from lifecycle runtime evidence concerns.
"""

from __future__ import annotations

import json
from pathlib import Path


def _default_registry_path() -> Path:
    return Path(__file__).resolve().parents[4] / "infra" / "registry.json"


def _load_registry(path: str | Path | None = None) -> dict:
    target_path = Path(path) if path else _default_registry_path()
    return json.loads(target_path.read_text())


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
    content = {
        "lifecycle": "kernel_release",
        "intent": intent,
        "objective": intent_policy["objective"],
        "orchestrator_organ": lifecycle["orchestrator_organ"],
        "domain": lifecycle["task_domain"],
        "build_target": build_node["name"],
        "build_runner_role": lifecycle["build_runner_role"],
        "deploy_target": deploy_node["name"],
        "deploy_target_role": lifecycle["deploy_target_role"],
        "required_node_status": lifecycle.get("required_node_status"),
        "activation_requirements": lifecycle.get("activation_requirements", {}),
        "runtime_paths": lifecycle.get("runtime_paths", {}),
        "runtime_path_proofs": lifecycle.get("runtime_path_proofs", {}),
        "activation_evidence_contract": lifecycle.get("activation_evidence_contract", {}),
        "fleet_requirements": lifecycle.get("fleet_requirements", {}),
        "requires_e2e": lifecycle.get("requires_e2e", True),
        "targets": repo_targets,
        "coord_targets": repo_targets,
        "deploy": intent_policy.get("deploy", False),
    }
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
        content = json.loads(raw_content) if isinstance(raw_content, str) else raw_content
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"payload.content is not valid JSON: {exc}") from exc
    if not isinstance(content, dict):
        raise RuntimeError("payload.content must be a JSON object")

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

    expected = {
        "lifecycle": "kernel_release",
        "orchestrator_organ": lifecycle.get("orchestrator_organ"),
        "domain": lifecycle.get("task_domain"),
        "build_target": build_node.get("name") if build_node else None,
        "build_runner_role": lifecycle.get("build_runner_role"),
        "deploy_target": deploy_node.get("name") if deploy_node else None,
        "deploy_target_role": lifecycle.get("deploy_target_role"),
        "required_node_status": lifecycle.get("required_node_status"),
        "activation_requirements": lifecycle.get("activation_requirements", {}),
        "runtime_paths": lifecycle.get("runtime_paths", {}),
        "runtime_path_proofs": lifecycle.get("runtime_path_proofs", {}),
        "activation_evidence_contract": lifecycle.get("activation_evidence_contract", {}),
        "fleet_requirements": lifecycle.get("fleet_requirements", {}),
        "requires_e2e": lifecycle.get("requires_e2e", True),
        "targets": repo_targets,
        "coord_targets": repo_targets,
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

    if not isinstance(content.get("coord_targets"), list) or not content.get("coord_targets"):
        errors.append("content.coord_targets must be a non-empty list")

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

