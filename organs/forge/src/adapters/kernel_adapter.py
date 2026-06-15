import json
import logging
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from urllib import error, request

from ..core.ports import IKernelClient

logger = logging.getLogger(__name__)


def _default_registry_path() -> Path:
    return Path(__file__).resolve().parents[4] / "infra" / "registry.json"


def load_registry(path: str | Path | None = None) -> dict:
    registry_path = Path(path) if path else _default_registry_path()
    with registry_path.open() as f:
        return json.load(f)


def validate_lifecycle_policy(registry: dict | None = None) -> dict:
    registry = registry or load_registry()
    lifecycle = registry.get("lifecycle", {}).get("kernel_release")
    if not lifecycle:
        raise RuntimeError("missing lifecycle.kernel_release")

    nodes = registry.get("nodes", [])
    organs = registry.get("organs", [])
    build_role = lifecycle["build_runner_role"]
    deploy_role = lifecycle["deploy_target_role"]
    deploy_flag = lifecycle.get("deploy_target_flag", "deploy_target")

    build_node = next((node for node in nodes if node.get("role") == build_role), None)
    deploy_node = next(
        (
            node
            for node in nodes
            if node.get("role") == deploy_role and node.get(deploy_flag) is True
        ),
        None,
    )
    organ = next(
        (item for item in organs if item.get("name") == lifecycle["orchestrator_organ"]),
        None,
    )

    errors = []
    if not build_node:
        errors.append("lifecycle build runner role does not resolve")
    if not deploy_node:
        errors.append("lifecycle deploy target role does not resolve")
    if not organ:
        errors.append("lifecycle orchestrator organ does not resolve")
    if not lifecycle.get("task_kind") or not lifecycle.get("task_domain"):
        errors.append("lifecycle task routing incomplete")
    if not isinstance(lifecycle.get("repo_targets"), list) or not lifecycle["repo_targets"]:
        errors.append("lifecycle repo_targets missing")

    intents = lifecycle.get("intents", {})
    if not isinstance(intents, dict) or not intents:
        errors.append("lifecycle intents missing")
    for intent_name, intent_policy in intents.items():
        if not isinstance(intent_policy, dict):
            errors.append(f"intents.{intent_name} must be an object")
            continue
        if not intent_policy.get("objective"):
            errors.append(f"intents.{intent_name}.objective missing")
        if not isinstance(intent_policy.get("deploy"), bool):
            errors.append(f"intents.{intent_name}.deploy must be boolean")

    profiles = lifecycle.get("request_profiles", {})
    if not isinstance(profiles, dict) or not profiles:
        errors.append("lifecycle request_profiles missing")
    for profile_name, profile_intents in profiles.items():
        if not isinstance(profile_intents, list) or not profile_intents:
            errors.append(f"request_profiles.{profile_name} must be a non-empty intent list")
            continue
        for intent in profile_intents:
            if intent not in intents:
                errors.append(f"request_profiles.{profile_name} unknown intent {intent}")

    runtime_paths = lifecycle.get("runtime_paths", {})
    runtime_path_proofs = lifecycle.get("runtime_path_proofs", {})

    activation = lifecycle.get("activation_requirements", {})
    if set(activation) != {build_role, deploy_role}:
        errors.append("activation_requirements must cover build/deploy roles exactly")
    for role, requirement in activation.items():
        if not isinstance(requirement.get("network"), list) or not requirement["network"]:
            errors.append(f"activation_requirements.{role}.network missing")
        if not isinstance(requirement.get("services"), list) or not requirement["services"]:
            errors.append(f"activation_requirements.{role}.services missing")
        filesystem = requirement.get("filesystem", [])
        if not isinstance(filesystem, list) or "repo_root" not in filesystem:
            errors.append(f"activation_requirements.{role}.filesystem must include repo_root")
            filesystem = []
        for path_name in filesystem:
            if path_name not in runtime_paths:
                errors.append(
                    f"activation_requirements.{role}.filesystem references unknown runtime path {path_name}"
                )

    fleet = lifecycle.get("fleet_requirements", {})
    if fleet.get("provider") != "tailscale":
        errors.append("fleet_requirements.provider must be tailscale")
    if fleet.get("required") is not True:
        errors.append("fleet_requirements.required must be true")
    if fleet.get("require_online") is not True:
        errors.append("fleet_requirements.require_online must be true")

    probe = lifecycle.get("kernel_bus_probe", {})
    if not str(probe.get("path", "")).startswith("/"):
        errors.append("kernel_bus_probe.path must be absolute")
    if not isinstance(probe.get("accepted_status"), list) or not probe["accepted_status"]:
        errors.append("kernel_bus_probe.accepted_status missing")
    if int(probe.get("timeout_seconds", 0)) <= 0:
        errors.append("kernel_bus_probe.timeout_seconds must be positive")

    repo_root = runtime_paths.get("repo_root")
    if not isinstance(repo_root, str) or not repo_root:
        errors.append("runtime_paths.repo_root missing")
    elif "/Bureau/" in repo_root or repo_root.startswith("/home/"):
        errors.append("runtime_paths.repo_root must not point at an operator checkout")
    repo_root_proof = runtime_path_proofs.get("repo_root", {})
    if repo_root_proof.get("kind") != "git_worktree":
        errors.append("runtime_path_proofs.repo_root.kind must be git_worktree")
    if repo_root_proof.get("marker") != ".git":
        errors.append("runtime_path_proofs.repo_root.marker must be .git")
    if not isinstance(repo_root_proof.get("sentinels"), list) or not repo_root_proof["sentinels"]:
        errors.append("runtime_path_proofs.repo_root.sentinels missing")
    evidence_contract = lifecycle.get("activation_evidence_contract", {})
    if evidence_contract.get("patch_field") != "activation_evidence_patch":
        errors.append("activation_evidence_contract.patch_field must be activation_evidence_patch")
    if evidence_contract.get("node_field") != "node":
        errors.append("activation_evidence_contract.node_field must be node")
    if evidence_contract.get("runtime_paths_field") != "activation_evidence.runtime_paths":
        errors.append(
            "activation_evidence_contract.runtime_paths_field must be activation_evidence.runtime_paths"
        )
    if evidence_contract.get("services_field") != "activation_evidence.services":
        errors.append("activation_evidence_contract.services_field must be activation_evidence.services")
    verified_requires = evidence_contract.get("verified_requires", [])
    if sorted(verified_requires) != ["marker_exists", "sentinels_verified"]:
        errors.append(
            "activation_evidence_contract.verified_requires must include marker_exists and sentinels_verified"
        )
    service_active_requires = evidence_contract.get("service_active_requires", [])
    if sorted(service_active_requires) != ["active", "declared"]:
        errors.append(
            "activation_evidence_contract.service_active_requires must include declared and active"
        )
    if evidence_contract.get("ingestion_mode") != "validated_merge":
        errors.append("activation_evidence_contract.ingestion_mode must be validated_merge")

    if errors:
        raise RuntimeError("; ".join(errors))
    return {
        "status": "ok",
        "lifecycle": "kernel_release",
        "build_runner_role": build_role,
        "deploy_target_role": deploy_role,
        "orchestrator_organ": lifecycle["orchestrator_organ"],
        "fleet_provider": fleet["provider"],
        "repo_root": repo_root,
        "request_profiles": sorted(profiles),
    }


def lifecycle_profile_intents(
    profile: str,
    registry: dict | None = None,
) -> list[str]:
    registry = registry or load_registry()
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
    registry = registry or load_registry()
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
    registry = registry or load_registry()
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
    registry = registry or load_registry()
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


def build_lifecycle_activation_plan(
    registry: dict | None = None,
    inspect_fleet: bool = False,
) -> dict:
    registry = registry or load_registry()
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
    registry = registry or load_registry()
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


def build_lifecycle_runtime_paths(registry: dict | None = None) -> dict:
    plan = build_lifecycle_activation_plan(registry=registry, inspect_fleet=False)
    paths = []
    for node in plan["nodes"]:
        for path_name, path_value in node.get("required_filesystem", {}).items():
            paths.append(
                {
                    "node": node["name"],
                    "role": node["role"],
                    "path": path_name,
                    "value": path_value,
                    "proof": node.get("filesystem_proofs", {}).get(path_name, {}),
                    "required_status": node["required_status"],
                    "current_status": node["current_status"],
                    "materialized_by": "lifecycle_activation",
                    "source": "infra/registry.json:lifecycle.kernel_release.runtime_paths",
                }
            )
    return {
        "status": plan["status"],
        "lifecycle": plan["lifecycle"],
        "paths": paths,
        "blocked": plan["status"] != "ok",
        "blockers": plan["blockers"],
    }


def verify_lifecycle_runtime_paths(
    node_name: str,
    registry: dict | None = None,
    home: str | None = None,
) -> dict:
    if not node_name:
        raise RuntimeError("node_name is required")
    registry = registry or load_registry()
    paths_payload = build_lifecycle_runtime_paths(registry)
    home_path = home or str(Path.home())
    checks = []
    for path in paths_payload["paths"]:
        if path["node"] != node_name:
            continue
        expanded = _expand_systemd_user_path(path["value"], home_path)
        proof = path.get("proof", {})
        marker = proof.get("marker")
        sentinels = proof.get("sentinels", [])
        marker_path = os.path.join(expanded, marker) if marker else ""
        marker_ok = bool(marker_path) and os.path.exists(marker_path)
        sentinel_checks = [
            {
                "path": sentinel,
                "expanded": os.path.join(expanded, sentinel),
                "exists": os.path.exists(os.path.join(expanded, sentinel)),
            }
            for sentinel in sentinels
        ]
        checks.append(
            {
                "node": node_name,
                "role": path["role"],
                "path": path["path"],
                "value": path["value"],
                "expanded": expanded,
                "proof_kind": proof.get("kind"),
                "marker": marker,
                "marker_exists": marker_ok,
                "sentinels": sentinel_checks,
                "ok": (
                    proof.get("kind") == "git_worktree"
                    and marker_ok
                    and bool(sentinel_checks)
                    and all(item["exists"] for item in sentinel_checks)
                ),
            }
        )
    if not checks:
        raise RuntimeError(f"node {node_name} has no lifecycle runtime paths")
    ok = all(check["ok"] for check in checks)
    verified_at = (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )
    runtime_path_evidence = {
        check["path"]: {
            "verified": check["ok"],
            "verified_at": verified_at,
            "value": check["value"],
            "expanded": check["expanded"],
            "proof_kind": check["proof_kind"],
            "marker_exists": check["marker_exists"],
            "sentinels_verified": all(item["exists"] for item in check["sentinels"]),
        }
        for check in checks
    }
    return {
        "status": "ok" if ok else "blocked",
        "lifecycle": paths_payload["lifecycle"],
        "node": node_name,
        "checks": checks,
        "activation_evidence_patch": {
            "node": node_name,
            "activation_evidence": {
                "runtime_paths": runtime_path_evidence,
            },
        },
    }


def build_lifecycle_service_evidence_patch(
    node_name: str,
    registry: dict | None = None,
    service_states: dict | None = None,
) -> dict:
    if not node_name:
        raise RuntimeError("node_name is required")
    registry = registry or load_registry()
    validate_lifecycle_policy(registry)
    lifecycle = registry["lifecycle"]["kernel_release"]
    node = next(
        (item for item in registry.get("nodes", []) if item.get("name") == node_name),
        None,
    )
    if not node:
        raise RuntimeError(f"node {node_name} is not in registry")
    role = node.get("role")
    required_services = (
        lifecycle.get("activation_requirements", {})
        .get(role, {})
        .get("services", [])
    )
    if not required_services:
        raise RuntimeError(f"node {node_name} has no lifecycle services")
    declared_services = {
        service.get("name")
        for service in node.get("services", [])
        if isinstance(service, dict)
    }
    service_states = service_states or {}
    checked_at = (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )
    services = {}
    checks = []
    for service_name in required_services:
        state = service_states.get(service_name, {})
        declared = service_name in declared_services
        active = bool(state.get("active", False))
        checked_service_at = state.get("checked_at", checked_at)
        services[service_name] = {
            "declared": declared,
            "active": active,
            "checked_at": checked_service_at,
            "source": state.get("source", "lifecycle_service_evidence_patch"),
        }
        checks.append(
            {
                "node": node_name,
                "role": role,
                "service": service_name,
                "declared": declared,
                "active": active,
                "ok": declared and active,
            }
        )
    ok = all(check["ok"] for check in checks)
    return {
        "status": "ok" if ok else "blocked",
        "lifecycle": "kernel_release",
        "node": node_name,
        "role": role,
        "checks": checks,
        "activation_evidence_patch": {
            "node": node_name,
            "activation_evidence": {
                "services": services,
            },
        },
    }


def apply_lifecycle_activation_evidence_patch(
    patch: dict,
    registry: dict | None = None,
) -> dict:
    registry = registry or load_registry()
    validate_lifecycle_policy(registry)

    if not isinstance(patch, dict):
        raise RuntimeError("activation evidence patch must be a JSON object")
    node_name = patch.get("node")
    if not isinstance(node_name, str) or not node_name:
        raise RuntimeError("activation evidence patch node missing")

    node_index = next(
        (
            index
            for index, node in enumerate(registry.get("nodes", []))
            if node.get("name") == node_name
        ),
        None,
    )
    if node_index is None:
        raise RuntimeError(
            f"activation evidence patch node is not in registry: {node_name}"
        )

    lifecycle = registry["lifecycle"]["kernel_release"]
    node = registry["nodes"][node_index]
    role = node.get("role")
    required_paths = set(
        lifecycle.get("activation_requirements", {})
        .get(role, {})
        .get("filesystem", [])
    )
    evidence = patch.get("activation_evidence", {})
    runtime_paths = evidence.get("runtime_paths", {})
    services = evidence.get("services", {})
    if not isinstance(runtime_paths, dict):
        raise RuntimeError("activation evidence patch runtime_paths must be an object")
    if not isinstance(services, dict):
        raise RuntimeError("activation evidence patch services must be an object")
    if not runtime_paths and not services:
        raise RuntimeError("activation evidence patch runtime_paths or services missing")

    errors = []
    for path_name, path_evidence in runtime_paths.items():
        if path_name not in required_paths:
            errors.append(f"runtime path {path_name} is not required by node role {role}")
            continue
        if not isinstance(path_evidence, dict):
            errors.append(f"runtime path {path_name} evidence must be an object")
            continue
        expected_value = lifecycle.get("runtime_paths", {}).get(path_name)
        expected_proof = lifecycle.get("runtime_path_proofs", {}).get(path_name, {})
        if path_evidence.get("value") != expected_value:
            errors.append(f"runtime path {path_name} value does not match registry")
        if path_evidence.get("proof_kind") != expected_proof.get("kind"):
            errors.append(f"runtime path {path_name} proof_kind does not match registry")
        if not isinstance(path_evidence.get("verified"), bool):
            errors.append(f"runtime path {path_name} verified must be boolean")
        if not isinstance(path_evidence.get("marker_exists"), bool):
            errors.append(f"runtime path {path_name} marker_exists must be boolean")
        if not isinstance(path_evidence.get("sentinels_verified"), bool):
            errors.append(f"runtime path {path_name} sentinels_verified must be boolean")
        if not isinstance(path_evidence.get("verified_at"), str) or not path_evidence.get(
            "verified_at"
        ):
            errors.append(f"runtime path {path_name} verified_at missing")
        if path_evidence.get("verified") and (
            not path_evidence.get("marker_exists")
            or not path_evidence.get("sentinels_verified")
        ):
            errors.append(
                f"runtime path {path_name} cannot be verified without marker and sentinels"
            )
    required_services = set(
        lifecycle.get("activation_requirements", {})
        .get(role, {})
        .get("services", [])
    )
    declared_services = {
        service.get("name")
        for service in node.get("services", [])
        if isinstance(service, dict)
    }
    for service_name, service_evidence in services.items():
        if service_name not in required_services:
            errors.append(f"service {service_name} is not required by node role {role}")
            continue
        if service_name not in declared_services:
            errors.append(f"service {service_name} is not declared by node {node_name}")
            continue
        if not isinstance(service_evidence, dict):
            errors.append(f"service {service_name} evidence must be an object")
            continue
        if not isinstance(service_evidence.get("declared"), bool):
            errors.append(f"service {service_name} declared must be boolean")
        if not isinstance(service_evidence.get("active"), bool):
            errors.append(f"service {service_name} active must be boolean")
        if not isinstance(service_evidence.get("checked_at"), str) or not service_evidence.get(
            "checked_at"
        ):
            errors.append(f"service {service_name} checked_at missing")
        if service_evidence.get("active") and not service_evidence.get("declared"):
            errors.append(f"service {service_name} cannot be active without declared=true")
    if errors:
        raise RuntimeError("; ".join(errors))

    merged = json.loads(json.dumps(registry))
    target = merged["nodes"][node_index]
    target.setdefault("activation_evidence", {}).setdefault("runtime_paths", {}).update(
        runtime_paths
    )
    target.setdefault("activation_evidence", {}).setdefault("services", {}).update(services)
    return {
        "status": "ok",
        "node": node_name,
        "role": role,
        "accepted_runtime_paths": sorted(runtime_paths),
        "accepted_services": sorted(services),
        "registry": merged,
    }


def persist_lifecycle_activation_evidence_patch(
    patch: dict,
    registry_path: str | Path | None = None,
) -> dict:
    target_path = Path(registry_path) if registry_path else _default_registry_path()
    registry = load_registry(target_path)
    result = apply_lifecycle_activation_evidence_patch(patch, registry)
    tmp_path = target_path.with_name(f".{target_path.name}.tmp")
    tmp_path.write_text(
        json.dumps(result["registry"], indent=2, ensure_ascii=False) + "\n",
    )
    os.replace(tmp_path, target_path)
    return {
        "status": "ok",
        "registry_path": str(target_path),
        "node": result["node"],
        "role": result["role"],
        "accepted_runtime_paths": result["accepted_runtime_paths"],
        "accepted_services": result["accepted_services"],
    }


def activate_lifecycle_registry_node(
    node_name: str,
    registry: dict | None = None,
) -> dict:
    if not node_name:
        raise RuntimeError("node_name is required")
    registry = registry or load_registry()
    validate_lifecycle_policy(registry)
    lifecycle = registry["lifecycle"]["kernel_release"]
    required_status = lifecycle.get("required_node_status")
    if required_status != "active":
        raise RuntimeError("lifecycle required_node_status must be active")

    nodes = registry.get("nodes", [])
    node_index = next(
        (
            index
            for index, node in enumerate(nodes)
            if node.get("name") == node_name
        ),
        None,
    )
    if node_index is None:
        raise RuntimeError(f"node {node_name} is not in registry")
    node = nodes[node_index]
    lifecycle_roles = {
        lifecycle["build_runner_role"],
        lifecycle["deploy_target_role"],
    }
    role = node.get("role")
    if role not in lifecycle_roles:
        raise RuntimeError(f"node {node_name} is not part of lifecycle kernel_release")

    diagnostic = KernelRestClient.__new__(KernelRestClient)
    diagnostic.registry = registry
    diagnostic.lifecycle = lifecycle
    diagnostic.build_node = next(
        item for item in nodes if item.get("role") == lifecycle["build_runner_role"]
    )
    diagnostic.deploy_node = next(
        item
        for item in nodes
        if item.get("role") == lifecycle["deploy_target_role"]
        and item.get(lifecycle.get("deploy_target_flag", "deploy_target")) is True
    )
    node_gaps = [
        gap
        for gap in diagnostic._activation_evidence_gap_details()
        if gap.get("node") == node_name
    ]
    if node_gaps:
        raise RuntimeError(
            "node activation evidence is incomplete "
            f"({'; '.join(gap['code'] for gap in node_gaps)})"
        )

    activated_at = (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )
    merged = json.loads(json.dumps(registry))
    target = merged["nodes"][node_index]
    previous_status = target.get("status", "unknown")
    target["status"] = required_status
    target["status_updated_at"] = activated_at
    target["status_source"] = "lifecycle_node_activation"
    return {
        "status": "ok",
        "node": node_name,
        "role": role,
        "from_status": previous_status,
        "to_status": required_status,
        "registry": merged,
    }


def persist_lifecycle_registry_node_activation(
    node_name: str,
    registry_path: str | Path | None = None,
) -> dict:
    target_path = Path(registry_path) if registry_path else _default_registry_path()
    registry = load_registry(target_path)
    result = activate_lifecycle_registry_node(node_name, registry)
    tmp_path = target_path.with_name(f".{target_path.name}.tmp")
    tmp_path.write_text(
        json.dumps(result["registry"], indent=2, ensure_ascii=False) + "\n",
    )
    os.replace(tmp_path, target_path)
    return {
        "status": "ok",
        "registry_path": str(target_path),
        "node": result["node"],
        "role": result["role"],
        "from_status": result["from_status"],
        "to_status": result["to_status"],
    }


def _expand_systemd_user_path(value: str, home: str) -> str:
    if value.startswith("%h/"):
        return os.path.join(home, value[3:])
    if value == "%h":
        return home
    return value


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


def lifecycle_health_status_result(
    status_code: int,
    registry: dict | None = None,
) -> dict:
    registry = registry or load_registry()
    lifecycle = registry.get("lifecycle", {}).get("kernel_release")
    if not lifecycle:
        raise RuntimeError("missing lifecycle.kernel_release")
    probe = lifecycle.get("kernel_bus_probe", {})
    accepted = probe.get("accepted_status", [200])
    if not isinstance(accepted, list) or not accepted:
        raise RuntimeError("kernel_bus_probe.accepted_status missing")
    return {
        "status": "ok" if status_code in accepted else "rejected",
        "http_status": status_code,
        "accepted_status": accepted,
        "accepted": status_code in accepted,
    }


def normalize_kernel_base_url(raw: str) -> str:
    if not raw:
        raise RuntimeError("CYNIC_REST_ADDR is required")
    if raw.startswith(("http://", "https://")):
        return raw.rstrip("/")
    return f"http://{raw.rstrip('/')}"


class KernelRestClient(IKernelClient):
    def __init__(
        self,
        uri: str = None,
        require_api_key: bool = True,
        strict_lifecycle: bool = True,
    ):
        self.registry = self._load_registry()
        self.lifecycle = self._required_lifecycle("kernel_release")
        self.build_node = self._required_node(self.lifecycle["build_runner_role"])
        self.deploy_node = self._required_node(
            self.lifecycle["deploy_target_role"],
            deploy_target_flag=self.lifecycle.get("deploy_target_flag"),
        )
        self.build_target = self.build_node["name"]
        self.deploy_target = self.deploy_node["name"]
        self.kernel_endpoint_source = "unknown"
        self.kernel_bus_canonical = True
        self.kernel_probe = {"enabled": False, "status": "not_run"}
        self.fleet_probe = {"enabled": False, "status": "not_run"}
        self.strict_lifecycle = strict_lifecycle
        if strict_lifecycle:
            self._require_ready_nodes()
        self.base_url = self._kernel_addr(uri)
        self.api_key = os.environ.get("CYNIC_" + "API" + "_KEY", "")
        if require_api_key and not self.api_key:
            raise RuntimeError("CYNIC_API_KEY is required for organ-forge kernel dispatch")
        if strict_lifecycle:
            self._probe_kernel_bus()

    def _load_registry(self) -> dict:
        return load_registry()

    def _required_lifecycle(self, name: str) -> dict:
        lifecycle = self.registry.get("lifecycle", {}).get(name)
        if not lifecycle:
            raise RuntimeError(f"registry missing lifecycle.{name}")
        return lifecycle

    def _required_node(self, role: str, deploy_target_flag: str | None = None) -> dict:
        for node in self.registry.get("nodes", []):
            if node.get("role") != role:
                continue
            if deploy_target_flag and node.get(deploy_target_flag) is not True:
                continue
            return node
        suffix = f" with {deploy_target_flag}=true" if deploy_target_flag else ""
        raise RuntimeError(f"registry missing node role={role}{suffix}")

    def _require_ready_nodes(self) -> None:
        required = self.lifecycle.get("required_node_status")
        if not required:
            return
        missing_evidence = self._activation_evidence_gaps()
        if missing_evidence:
            raise RuntimeError(
                "registry lifecycle activation evidence is incomplete "
                f"({'; '.join(missing_evidence)})"
            )
        self._require_fleet_presence()
        not_ready = [
            f"{node.get('name')}:{node.get('status', 'unknown')}"
            for node in (self.build_node, self.deploy_node)
            if node.get("status") != required
        ]
        if not_ready:
            raise RuntimeError(
                "registry lifecycle nodes are not ready "
                f"(required status={required}; found {', '.join(not_ready)})"
            )

    def _require_fleet_presence(self) -> None:
        requirements = self.lifecycle.get("fleet_requirements", {})
        if not requirements.get("required", False):
            return
        provider = requirements.get("provider")
        if provider != "tailscale":
            raise RuntimeError(f"unsupported lifecycle fleet provider: {provider}")
        self.inspect_fleet()
        if self.fleet_probe.get("status") != "ok":
            reason = self.fleet_probe.get("error", self.fleet_probe.get("status", "unknown"))
            raise RuntimeError(f"registry lifecycle fleet probe failed ({reason})")
        missing = self.fleet_probe.get("missing", [])
        offline = [
            name
            for name, peer in self.fleet_probe.get("matches", {}).items()
            if requirements.get("require_online", True) and not peer.get("online")
        ]
        if missing or offline:
            details = []
            if missing:
                details.append(f"missing={','.join(missing)}")
            if offline:
                details.append(f"offline={','.join(offline)}")
            raise RuntimeError(
                "registry lifecycle fleet targets are not ready "
                f"({'; '.join(details)})"
            )

    def _activation_evidence_gaps(self) -> list[str]:
        return [
            gap["code"]
            for gap in self._activation_evidence_gap_details()
        ]

    def _activation_evidence_gap_details(self) -> list[dict]:
        requirements = self.lifecycle.get("activation_requirements", {})
        role_nodes = {
            self.lifecycle["build_runner_role"]: self.build_node,
            self.lifecycle["deploy_target_role"]: self.deploy_node,
        }
        gaps = []
        for role, requirement in requirements.items():
            node = role_nodes.get(role)
            if not node:
                gaps.append(
                    {
                        "code": f"{role}:node_not_resolved",
                        "kind": "node",
                        "role": role,
                        "node": None,
                        "requirement": "resolved_node",
                    }
                )
                continue
            network = node.get("network", {})
            for network_key in requirement.get("network", []):
                value = network.get(network_key)
                if not value:
                    gaps.append(
                        {
                            "code": f"{node.get('name')}:{network_key}_missing",
                            "kind": "network",
                            "role": role,
                            "node": node.get("name"),
                            "requirement": network_key,
                            "reason": "missing_registry_value",
                        }
                    )
                    continue
                if isinstance(value, str) and value.startswith("<") and value.endswith(">"):
                    env_name = value[1:-1]
                    if not os.environ.get(env_name):
                        gaps.append(
                            {
                                "code": f"{node.get('name')}:{network_key}_env_{env_name}_missing",
                                "kind": "network",
                                "role": role,
                                "node": node.get("name"),
                                "requirement": network_key,
                                "reason": "missing_env",
                                "env": env_name,
                            }
                        )
            service_names = {
                service.get("name")
                for service in node.get("services", [])
                if isinstance(service, dict)
            }
            node_service_evidence = (
                node.get("activation_evidence", {})
                .get("services", {})
            )
            for service_name in requirement.get("services", []):
                if service_name not in service_names:
                    gaps.append(
                        {
                            "code": f"{node.get('name')}:service_{service_name}_missing",
                            "kind": "service",
                            "role": role,
                            "node": node.get("name"),
                            "requirement": service_name,
                            "reason": "missing_registry_service",
                        }
                    )
                    continue
                evidence = node_service_evidence.get(service_name, {})
                if not evidence.get("active"):
                    gaps.append(
                        {
                            "code": f"{node.get('name')}:service_{service_name}_inactive",
                            "kind": "service",
                            "role": role,
                            "node": node.get("name"),
                            "requirement": service_name,
                            "reason": "missing_service_evidence",
                        }
                    )
            runtime_paths = self.lifecycle.get("runtime_paths", {})
            runtime_path_proofs = self.lifecycle.get("runtime_path_proofs", {})
            node_runtime_evidence = (
                node.get("activation_evidence", {})
                .get("runtime_paths", {})
            )
            for path_name in requirement.get("filesystem", []):
                evidence = node_runtime_evidence.get(path_name, {})
                if not evidence.get("verified"):
                    gaps.append(
                        {
                            "code": f"{node.get('name')}:runtime_path_{path_name}_unverified",
                            "kind": "filesystem",
                            "role": role,
                            "node": node.get("name"),
                            "requirement": path_name,
                            "reason": "missing_runtime_path_proof",
                            "path": runtime_paths.get(path_name),
                            "proof": runtime_path_proofs.get(path_name, {}),
                        }
                    )
        return gaps

    def _kernel_addr(self, explicit: str | None) -> str:
        raw = explicit or os.environ.get("CYNIC_REST_ADDR", "")
        self.kernel_endpoint_source = "explicit" if explicit else "env:CYNIC_REST_ADDR"
        if not raw:
            kernel = self._required_node(
                self.lifecycle["deploy_target_role"],
                deploy_target_flag=self.lifecycle.get("deploy_target_flag"),
            )
            raw = kernel.get("network", {}).get("tailscale", "")
            self.kernel_endpoint_source = "registry:kernel_primary.network.tailscale"
        if not raw:
            raise RuntimeError("CYNIC_REST_ADDR is required")
        if raw.startswith("<") and raw.endswith(">"):
            env_name = raw[1:-1]
            raw = os.environ.get(env_name, "")
            self.kernel_endpoint_source = f"env:{env_name}"
            if not raw:
                if not self.strict_lifecycle:
                    raw = f"<{env_name}>"
                else:
                    raise RuntimeError(
                        f"{env_name} is required to resolve registry kernel_primary endpoint"
                    )
        if raw.startswith("<") and raw.endswith(">"):
            if self.strict_lifecycle:
                raise RuntimeError(f"{env_name} is required to resolve registry kernel_primary endpoint")
            return raw
        if raw.startswith(("localhost:", "127.")) or raw.startswith(
            ("http://localhost:", "http://127.")
        ):
            self.kernel_bus_canonical = False
            override = self.lifecycle.get("local_kernel_bus_override_env")
            if self.strict_lifecycle and (not override or os.environ.get(override) != "1"):
                raise RuntimeError(
                    "local kernel bus is not canonical; point CYNIC_REST_ADDR to registry kernel_primary"
                )
        return normalize_kernel_base_url(raw)

    @staticmethod
    def _redact_url(url: str) -> str:
        scheme = ""
        rest = url
        if "://" in url:
            scheme, rest = url.split("://", 1)
            scheme += "://"
        host_port = rest.split("/", 1)[0]
        host, sep, port = host_port.partition(":")
        if host in {"localhost"} or host.startswith("127."):
            redacted_host = host
        elif host.startswith("100."):
            redacted_host = "100.x.x.x"
        elif host.startswith("192.168."):
            redacted_host = "192.168.x.x"
        else:
            redacted_host = "<redacted-host>"
        return f"{scheme}{redacted_host}{sep}{port}" if sep else f"{scheme}{redacted_host}"

    def lifecycle_context(self) -> dict:
        required_status = self.lifecycle.get("required_node_status")
        activation_gap_details = self._activation_evidence_gap_details()
        activation_gaps = [gap["code"] for gap in activation_gap_details]
        status_nodes_ready = (
            not any(
                node.get("status") != required_status
                for node in (self.build_node, self.deploy_node)
            )
            if required_status
            else True
        )
        fleet_nodes_ready = self._fleet_nodes_ready()
        lifecycle_blockers = self._lifecycle_blockers(
            activation_gap_details=activation_gap_details,
            required_status=required_status,
        )
        return {
            "lifecycle": "kernel_release",
            "orchestrator_organ": self.lifecycle["orchestrator_organ"],
            "task_kind": self.lifecycle["task_kind"],
            "task_domain": self.lifecycle["task_domain"],
            "build_target": self.build_target,
            "build_target_status": self.build_node.get("status", "unknown"),
            "deploy_target": self.deploy_target,
            "deploy_target_status": self.deploy_node.get("status", "unknown"),
            "kernel_endpoint_source": self.kernel_endpoint_source,
            "kernel_endpoint": self._redact_url(self.base_url),
            "kernel_bus_canonical": self.kernel_bus_canonical,
            "kernel_probe": self.kernel_probe,
            "fleet_probe": self.fleet_probe,
            "fleet_requirements": self.lifecycle.get("fleet_requirements", {}),
            "required_node_status": required_status,
            "activation_requirements": self.lifecycle.get("activation_requirements", {}),
            "request_profiles": self.lifecycle.get("request_profiles", {}),
            "intents": self.lifecycle.get("intents", {}),
            "activation_evidence_gaps": activation_gaps,
            "activation_evidence_gap_details": activation_gap_details,
            "status_nodes_ready": status_nodes_ready,
            "fleet_nodes_ready": fleet_nodes_ready,
            "lifecycle_nodes_ready": (
                status_nodes_ready and not activation_gaps and fleet_nodes_ready
            ),
            "lifecycle_blockers": lifecycle_blockers,
            "requires_e2e": self.lifecycle.get("requires_e2e", True),
            "repo_targets": self.lifecycle.get("repo_targets", []),
        }

    def _lifecycle_blockers(
        self,
        activation_gap_details: list[dict],
        required_status: str | None,
    ) -> list[dict]:
        blockers = [
            {"category": "activation", **gap}
            for gap in activation_gap_details
        ]
        if required_status:
            for node in (self.build_node, self.deploy_node):
                actual = node.get("status", "unknown")
                if actual != required_status:
                    blockers.append(
                        {
                            "category": "status",
                            "kind": "node_status",
                            "node": node.get("name"),
                            "role": node.get("role"),
                            "required_status": required_status,
                            "actual_status": actual,
                            "code": f"{node.get('name')}:status_{actual}_not_{required_status}",
                        }
                    )
        fleet_requirements = self.lifecycle.get("fleet_requirements", {})
        if fleet_requirements.get("required", False):
            fleet_status = self.fleet_probe.get("status", "not_run")
            if fleet_status != "ok":
                blockers.append(
                    {
                        "category": "fleet",
                        "kind": "tailscale_probe",
                        "required": "ok",
                        "actual": fleet_status,
                        "error": self.fleet_probe.get("error"),
                        "code": f"fleet_probe:{fleet_status}",
                    }
                )
            for node_name in self.fleet_probe.get("missing", []):
                blockers.append(
                    {
                        "category": "fleet",
                        "kind": "tailscale_peer",
                        "node": node_name,
                        "required": "present",
                        "actual": "missing",
                        "code": f"{node_name}:fleet_missing",
                    }
                )
            if fleet_requirements.get("require_online", True):
                for node_name, peer in self.fleet_probe.get("matches", {}).items():
                    if not peer.get("online"):
                        blockers.append(
                            {
                                "category": "fleet",
                                "kind": "tailscale_peer",
                                "node": node_name,
                                "required": "online",
                                "actual": "offline",
                                "code": f"{node_name}:fleet_offline",
                            }
                        )
        return blockers

    def _fleet_nodes_ready(self) -> bool:
        requirements = self.lifecycle.get("fleet_requirements", {})
        if not requirements.get("required", False):
            return True
        if self.fleet_probe.get("status") != "ok":
            return False
        if self.fleet_probe.get("missing"):
            return False
        if requirements.get("require_online", True):
            return all(
                peer.get("online") is True
                for peer in self.fleet_probe.get("matches", {}).values()
            )
        return True

    def inspect_fleet(self) -> None:
        target_names = {self.build_target, self.deploy_target}
        self.fleet_probe = {
            "enabled": True,
            "status": "pending",
            "targets": sorted(target_names),
        }
        try:
            result = subprocess.run(
                ["tailscale", "status", "--json"],
                check=False,
                capture_output=True,
                text=True,
                timeout=10,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            self.fleet_probe.update({"status": "error", "error": type(exc).__name__})
            return
        if result.returncode != 0:
            self.fleet_probe.update({"status": "error", "error": "tailscale_status_failed"})
            return
        try:
            status = json.loads(result.stdout)
        except json.JSONDecodeError:
            self.fleet_probe.update({"status": "error", "error": "tailscale_json_invalid"})
            return
        peers = list((status.get("Peer") or {}).values())
        self_node = status.get("Self")
        if isinstance(self_node, dict):
            peers.append(self_node)
        by_name = {}
        for peer in peers:
            names = {
                str(peer.get("HostName", "")).lower(),
                str(peer.get("DNSName", "")).split(".", 1)[0].lower(),
            }
            for name in names:
                if name:
                    by_name[name] = peer
        matches = {}
        missing = []
        for target in sorted(target_names):
            peer = by_name.get(target.lower())
            if not peer:
                missing.append(target)
                continue
            matches[target] = {
                "online": bool(peer.get("Online")),
                "os": peer.get("OS", "unknown"),
                "name": peer.get("HostName") or peer.get("DNSName", "").split(".", 1)[0],
            }
        self.fleet_probe.update(
            {
                "status": "ok",
                "matches": matches,
                "missing": missing,
                "tailnet_running": status.get("BackendState") == "Running",
            }
        )

    def _probe_kernel_bus(self) -> None:
        probe = self.lifecycle.get("kernel_bus_probe") or {}
        if not probe:
            return
        path = probe.get("path", "/health")
        timeout = int(probe.get("timeout_seconds", 10))
        accepted = set(probe.get("accepted_status", [200]))
        headers = {}
        if probe.get("requires_bearer_auth", True):
            if not self.api_key:
                raise RuntimeError("CYNIC_API_KEY is required for kernel bus probe")
            headers["Authorization"] = f"Bearer {self.api_key}"
        req = request.Request(
            f"{self.base_url}{path}",
            method="GET",
            headers=headers,
        )
        self.kernel_probe = {
            "enabled": True,
            "path": path,
            "accepted_status": sorted(accepted),
            "status": "pending",
        }
        try:
            with request.urlopen(req, timeout=timeout) as resp:
                status = resp.status
        except error.HTTPError as exc:
            status = exc.code
        except OSError as exc:
            self.kernel_probe.update({"status": "error", "error": type(exc).__name__})
            raise RuntimeError(f"kernel bus probe failed: {type(exc).__name__}")
        self.kernel_probe.update({"status": "ok" if status in accepted else "rejected", "http_status": status})
        if status not in accepted:
            raise RuntimeError(
                f"kernel bus probe rejected HTTP {status}; accepted={sorted(accepted)}"
            )

    def _post_agent_task(self, payload: dict) -> dict:
        body = json.dumps(payload).encode("utf-8")
        req = request.Request(
            f"{self.base_url}/agent-tasks",
            data=body,
            method="POST",
            headers={
                "Authorization": f"Bearer {self.api_key}",
                "Content-Type": "application/json",
            },
        )
        try:
            with request.urlopen(req, timeout=10) as resp:
                text = resp.read().decode("utf-8", errors="replace")
                return json.loads(text)
        except error.HTTPError as exc:
            text = exc.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"agent task dispatch failed: HTTP {exc.code} {text[:200]}")
        except (OSError, json.JSONDecodeError) as exc:
            raise RuntimeError(f"agent task dispatch failed: {exc}")

    def _content_for_intent(self, intent: str, patch: str) -> dict:
        payload = build_lifecycle_task_payload(
            intent=intent,
            domain=self.lifecycle["task_domain"],
            agent_id="organ-forge-cli",
            registry=self.registry,
        )
        content = json.loads(payload["content"])
        if patch:
            content["patch_sha256"] = __import__("hashlib").sha256(patch.encode("utf-8")).hexdigest()
        return content

    async def dispatch_task_and_stream(self, intent: str, patch: str):
        payload = {
            "kind": self.lifecycle["task_kind"],
            "domain": self.lifecycle["task_domain"],
            "agent_id": "organ-forge-cli",
            "content": json.dumps(self._content_for_intent(intent, patch), separators=(",", ":")),
        }
        validate_lifecycle_task_payload(payload, self.registry)
        response = self._post_agent_task(payload)
        yield json.dumps(response, separators=(",", ":")) + "\n"
