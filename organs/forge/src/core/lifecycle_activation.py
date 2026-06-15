"""Lifecycle activation planning and dispatch helpers for organ-forge."""

from __future__ import annotations

import json
from pathlib import Path


def _default_registry_path() -> Path:
    return Path(__file__).resolve().parents[4] / "infra" / "registry.json"


def _load_registry(path: str | Path | None = None) -> dict:
    target_path = Path(path) if path else _default_registry_path()
    return json.loads(target_path.read_text())


def _activation_action_for_blocker(blocker: dict) -> dict:
    category = blocker.get("category")
    if category == "activation" and blocker.get("reason") == "missing_env":
        return {
            "action": "provide_placeholder_env_evidence",
            "node": blocker.get("node"),
            "role": blocker.get("role"),
            "env": blocker.get("env"),
            "requirement": blocker.get("requirement"),
        }
    if category == "activation" and blocker.get("reason") == "missing_runtime_path_proof":
        return {
            "action": "verify_runtime_path_proof",
            "node": blocker.get("node"),
            "role": blocker.get("role"),
            "path": blocker.get("requirement"),
            "value": blocker.get("path"),
            "proof": blocker.get("proof"),
            "command": f"make lifecycle-verify-runtime-paths node={blocker.get('node')}",
            "ingestion_command": "make lifecycle-apply-activation-evidence-patch patch='<activation_evidence_patch_json>'",
        }
    if category == "activation" and blocker.get("reason") == "missing_service_evidence":
        return {
            "action": "record_service_activation_evidence",
            "node": blocker.get("node"),
            "role": blocker.get("role"),
            "service": blocker.get("requirement"),
            "contract": "infra/registry.json:lifecycle.kernel_release.activation_evidence_contract",
            "command": f"make lifecycle-service-evidence-patch node={blocker.get('node')}",
            "ingestion_command": "make lifecycle-apply-activation-evidence-patch patch='<activation_evidence_patch_json>'",
        }
    if category == "status":
        return {
            "action": "activate_registry_node",
            "node": blocker.get("node"),
            "role": blocker.get("role"),
            "from_status": blocker.get("actual_status"),
            "to_status": blocker.get("required_status"),
            "command": f"make lifecycle-activate-node node={blocker.get('node')}",
        }
    if category == "fleet" and blocker.get("kind") == "tailscale_probe":
        return {
            "action": "restore_fleet_probe",
            "provider": "tailscale",
            "required": blocker.get("required"),
            "actual": blocker.get("actual"),
            "error": blocker.get("error"),
        }
    if category == "fleet" and blocker.get("kind") == "tailscale_peer":
        return {
            "action": "join_or_wake_tailscale_peer",
            "node": blocker.get("node"),
            "required": blocker.get("required"),
            "actual": blocker.get("actual"),
        }
    return {
        "action": "resolve_lifecycle_blocker",
        "code": blocker.get("code"),
        "category": category,
    }


def build_lifecycle_activation_plan(
    registry: dict | None = None,
    inspect_fleet: bool = False,
) -> dict:
    registry = registry or _load_registry()
    from ..adapters.kernel_adapter import KernelRestClient, validate_lifecycle_policy

    validate_lifecycle_policy(registry)

    diagnostic = KernelRestClient.__new__(KernelRestClient)
    diagnostic.registry = registry
    diagnostic.lifecycle = diagnostic._required_lifecycle("kernel_release")
    diagnostic.build_node = diagnostic._required_node(
        diagnostic.lifecycle["build_runner_role"]
    )
    diagnostic.deploy_node = diagnostic._required_node(
        diagnostic.lifecycle["deploy_target_role"],
        deploy_target_flag=diagnostic.lifecycle.get("deploy_target_flag"),
    )
    diagnostic.build_target = diagnostic.build_node["name"]
    diagnostic.deploy_target = diagnostic.deploy_node["name"]
    diagnostic.kernel_endpoint_source = "not_resolved"
    diagnostic.kernel_bus_canonical = False
    diagnostic.kernel_probe = {"enabled": False, "status": "not_run"}
    diagnostic.fleet_probe = {"enabled": False, "status": "not_run"}
    diagnostic.strict_lifecycle = False
    diagnostic.base_url = "<unresolved>"
    if inspect_fleet:
        diagnostic.inspect_fleet()

    context = diagnostic.lifecycle_context()
    required_status = context.get("required_node_status")
    node_roles = {
        diagnostic.lifecycle["build_runner_role"]: diagnostic.build_node,
        diagnostic.lifecycle["deploy_target_role"]: diagnostic.deploy_node,
    }
    nodes = []
    for role, node in node_roles.items():
        requirement = diagnostic.lifecycle.get("activation_requirements", {}).get(role, {})
        nodes.append(
            {
                "name": node.get("name"),
                "role": role,
                "current_status": node.get("status", "unknown"),
                "required_status": required_status,
                "network": {
                    key: node.get("network", {}).get(key)
                    for key in requirement.get("network", [])
                },
                "required_services": requirement.get("services", []),
                "required_filesystem": {
                    key: diagnostic.lifecycle.get("runtime_paths", {}).get(key)
                    for key in requirement.get("filesystem", [])
                },
                "filesystem_proofs": {
                    key: diagnostic.lifecycle.get("runtime_path_proofs", {}).get(key, {})
                    for key in requirement.get("filesystem", [])
                },
            }
        )

    actions = []
    seen_actions = set()
    for blocker in context.get("lifecycle_blockers", []):
        action = _activation_action_for_blocker(blocker)
        action_key = json.dumps(action, sort_keys=True)
        if action_key in seen_actions:
            continue
        seen_actions.add(action_key)
        actions.append(action)
    for node in nodes:
        for path_name, path_value in node.get("required_filesystem", {}).items():
            action = {
                "action": "materialize_runtime_path",
                "node": node.get("name"),
                "role": node.get("role"),
                "path": path_name,
                "value": path_value,
                "proof": node.get("filesystem_proofs", {}).get(path_name, {}),
                "source": "infra/registry.json:lifecycle.kernel_release.runtime_paths",
            }
            action_key = json.dumps(action, sort_keys=True)
            if action_key in seen_actions:
                continue
            seen_actions.add(action_key)
            actions.append(action)

    profiles = context.get("request_profiles", {})
    next_requests = [
        {
            "profile": profile,
            "intents": intents,
            "make_target": f"{profile}-request",
        }
        for profile, intents in sorted(profiles.items())
    ]
    return {
        "status": "ok" if context.get("lifecycle_nodes_ready") else "blocked",
        "lifecycle": context.get("lifecycle"),
        "orchestrator_organ": context.get("orchestrator_organ"),
        "build_target": context.get("build_target"),
        "deploy_target": context.get("deploy_target"),
        "nodes": nodes,
        "blockers": context.get("lifecycle_blockers", []),
        "actions": actions,
        "next_requests": next_requests,
        "local_execution_policy": {
            "builds_on_operator_machine": False,
            "source": "infra/registry.json:lifecycle.kernel_release",
            "repo_root": diagnostic.lifecycle.get("runtime_paths", {}).get("repo_root"),
        },
    }


def build_lifecycle_activation_requests(
    registry: dict | None = None,
    inspect_fleet: bool = False,
) -> dict:
    registry = registry or _load_registry()
    plan = build_lifecycle_activation_plan(registry=registry, inspect_fleet=inspect_fleet)
    lifecycle_repo_targets = (
        registry.get("lifecycle", {})
        .get("kernel_release", {})
        .get("repo_targets", [])
    )
    requests = []
    for action in plan["actions"]:
        content = {
            "lifecycle": "kernel_release",
            "objective": f"activate {action.get('action')} for {action.get('node', 'lifecycle')}",
            "orchestrator_organ": plan["orchestrator_organ"],
            "domain": "organ-forge",
            "build_target": plan["build_target"],
            "build_runner_role": "build_runner",
            "deploy_target": plan["deploy_target"],
            "deploy_target_role": "kernel_primary",
            "required_node_status": "active",
            "activation_requirements": {
                "build_runner": {
                    "network": ["tailscale"],
                    "filesystem": ["repo_root"],
                    "services": ["organ-forge", "cargo-build", "gate-2"],
                },
                "kernel_primary": {
                    "network": ["tailscale"],
                    "filesystem": ["repo_root"],
                    "services": ["cynic-kernel", "surrealdb", "cynic-portal"],
                },
            },
            "fleet_requirements": {
                "provider": "tailscale",
                "required": True,
                "require_online": True,
            },
            "runtime_paths": {
                "repo_root": "%h/.cynic/repos/CYNIC",
            },
            "runtime_path_proofs": {
                "repo_root": {
                    "kind": "git_worktree",
                    "marker": ".git",
                    "sentinels": ["AGENTS.md", "infra/registry.json"],
                }
            },
            "activation_evidence_contract": {
                "patch_field": "activation_evidence_patch",
                "node_field": "node",
                "runtime_paths_field": "activation_evidence.runtime_paths",
                "services_field": "activation_evidence.services",
                "verified_requires": ["marker_exists", "sentinels_verified"],
                "service_active_requires": ["declared", "active"],
                "ingestion_mode": "validated_merge",
            },
            "targets": lifecycle_repo_targets,
            "coord_targets": lifecycle_repo_targets,
            "deploy": False,
            "action": action,
        }
        requests.append(
            {
                "kind": "hermes",
                "domain": "organ-forge",
                "agent_id": "organ-forge-cli",
                "content": json.dumps(content, separators=(",", ":")),
            }
        )

    return {
        "status": "ok" if plan.get("status") == "ok" else "blocked",
        "lifecycle": plan.get("lifecycle"),
        "orchestrator_organ": plan.get("orchestrator_organ"),
        "build_target": plan.get("build_target"),
        "deploy_target": plan.get("deploy_target"),
        "blockers": plan.get("blockers", []),
        "actions": plan.get("actions", []),
        "requests": requests,
        "next_requests": plan.get("next_requests", []),
    }


def validate_lifecycle_activation_request_payload(
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
    if payload.get("domain") != "organ-forge":
        errors.append("payload.domain does not match activation request domain")
    if payload.get("agent_id") != "organ-forge-cli":
        errors.append("payload.agent_id does not match activation request agent")

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
        "targets": repo_targets,
        "coord_targets": repo_targets,
        "deploy": False,
    }
    for key, expected_value in expected.items():
        if content.get(key) != expected_value:
            errors.append(f"content.{key} does not match lifecycle registry")

    if not isinstance(content.get("action"), dict) or not content["action"].get("action"):
        errors.append("content.action must be a structured activation action")
    if not content.get("objective"):
        errors.append("content.objective missing")

    if errors:
        raise RuntimeError("; ".join(errors))
    return {
        "status": "ok",
        "lifecycle": "kernel_release",
        "action": content["action"].get("action"),
        "objective": content["objective"],
    }


def dispatch_lifecycle_activation_requests(
    registry: dict | None = None,
    inspect_fleet: bool = False,
    kernel_client_factory=None,
) -> dict:
    registry = registry or _load_registry()
    plan = build_lifecycle_activation_requests(registry=registry, inspect_fleet=inspect_fleet)
    if not plan.get("requests"):
        raise RuntimeError("no lifecycle activation requests were generated")

    if kernel_client_factory is None:
        from ..adapters.kernel_adapter import KernelRestClient as kernel_client_factory

    client = kernel_client_factory(require_api_key=True, strict_lifecycle=False)
    if not getattr(client, "api_key", ""):
        raise RuntimeError("CYNIC_API_KEY is required for organ-forge kernel dispatch")

    responses = []
    for payload in plan["requests"]:
        validate_lifecycle_activation_request_payload(payload, registry)
        responses.append(client._post_agent_task(payload))

    return {
        "status": "ok",
        "plan_status": plan.get("status"),
        "lifecycle": plan.get("lifecycle"),
        "orchestrator_organ": plan.get("orchestrator_organ"),
        "build_target": plan.get("build_target"),
        "deploy_target": plan.get("deploy_target"),
        "blockers": plan.get("blockers", []),
        "actions": plan.get("actions", []),
        "requests": plan.get("requests", []),
        "responses": responses,
        "dispatched_count": len(responses),
        "next_requests": plan.get("next_requests", []),
        "local_execution_policy": plan.get("local_execution_policy", {}),
    }

