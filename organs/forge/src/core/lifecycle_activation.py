"""Lifecycle activation planning and dispatch helpers for organ-forge."""

from __future__ import annotations

import json
from pathlib import Path

from .entities import FleetProbe
from .lifecycle_bootstrap import build_runner_bootstrap_contract
from .lifecycle_bootstrap_source import probe_bootstrap_source
from .lifecycle_consumer_status import probe_local_consumer_runtime
from .lifecycle_diagnostic_payloads import blockers_with_runtime_dependencies
from .project_root import resolve_project_root_from_script
from organs.contracts.src.surface_contracts import ReportSurfaceSpec, attach_surface_contract
from organs.contracts.src.surface_contracts import read_json_file_or_default


LIFECYCLE_ACTIVATION_PLAN_SURFACE = ReportSurfaceSpec(
    schema="cynic.lifecycle.activation_summary.v1",
    command="cynic activate --dry-run",
)


def _default_registry_path() -> Path:
    return resolve_project_root_from_script(__file__) / "infra" / "registry.json"


def _load_registry(path: str | Path | None = None) -> dict:
    target_path = Path(path) if path else _default_registry_path()
    return read_json_file_or_default(target_path, {})


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
    if category == "fleet" and blocker.get("kind") == "synapse_probe":
        action = {
            "action": "restore_fleet_probe",
            "provider": blocker.get("provider", "synapse"),
            "required": blocker.get("required"),
            "actual": blocker.get("actual"),
            "error": blocker.get("error"),
            "resolver_organ": blocker.get("resolver_organ"),
            "resolver_consumer": blocker.get("resolver_consumer"),
        }
        if blocker.get("operator_action"):
            action["operator_action"] = blocker.get("operator_action")
        if blocker.get("command"):
            action["command"] = blocker.get("command")
        return action
    if category == "fleet" and blocker.get("kind") == "fleet_inventory":
        return {
            "action": "refresh_fleet_inventory",
            "provider": blocker.get("provider", "organ-fleet"),
            "required": blocker.get("required"),
            "actual": blocker.get("actual"),
            "error": blocker.get("error"),
            "resolver_organ": blocker.get("resolver_organ"),
            "resolver_consumer": blocker.get("resolver_consumer"),
            "command": blocker.get("command")
            or "python3 organs/fleet/src/entrypoints/cli.py report --json --registry infra/registry.json",
        }
    if category == "fleet" and blocker.get("kind") == "fleet_inventory_evidence_gap":
        return {
            "action": "provide_runtime_observation",
            "provider": blocker.get("provider"),
            "code": blocker.get("code"),
        }
    if category == "fleet" and blocker.get("kind") == "synapse_peer":
        return {
            "action": "join_or_wake_synapse_peer",
            "node": blocker.get("node"),
            "required": blocker.get("required"),
            "actual": blocker.get("actual"),
        }
    if category == "bootstrap_source":
        reason = blocker.get("code")
        detail = blocker.get("detail", {})
        action = {
            "action": "inspect_bootstrap_source",
            "code": reason,
            "selected_remote": detail.get("selected_remote"),
            "selected_ref": detail.get("selected_ref"),
            "command": "python3 organs/forge/src/entrypoints/cli.py lifecycle-bootstrap-source-check",
        }
        if reason == "bootstrap_source_unreachable":
            action["repair_goal"] = "restore bootstrap source transport or fallback remote reachability"
        elif reason == "bootstrap_source_fallback_env_missing":
            action["repair_goal"] = "inject the fallback Forgejo bootstrap remote into the organ-forge runtime environment"
            action["operator_action"] = (
                "set CYNIC_FORGEJO_BOOTSTRAP_REMOTE from `git remote get-url forgejo`, regenerate ~/.config/cynic/env, then restart organ-forge-hermes-agent.service"
            )
        elif reason == "bootstrap_source_contract_stale":
            action["repair_goal"] = "publish a bootstrap source ref whose lifecycle contract matches current DNA"
        return action
    if category == "runtime_consumer":
        detail = blocker.get("detail", {})
        return {
            "action": "repair_runtime_consumer",
            "code": blocker.get("code"),
            "runtime_user": detail.get("runtime_user"),
            "problems": detail.get("problems", []),
            "cargo_available": detail.get("cargo_available"),
            "runtime_user_active": detail.get("runtime_user_active"),
            "operator_action": blocker.get("operator_action"),
            "command": "python3 organs/forge/src/entrypoints/cli.py lifecycle-consumer-status",
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
    from ..adapters.kernel_adapter import KernelRestClient
    from ..adapters.registry_adapter import validate_lifecycle_policy

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
    diagnostic.kernel_probe = FleetProbe.not_run()
    diagnostic.fleet_probe = FleetProbe.not_run()
    diagnostic.strict_lifecycle = False
    diagnostic.base_url = "<unresolved>"
    if inspect_fleet:
        diagnostic.inspect_fleet()

    context = diagnostic.lifecycle_context()
    bootstrap_source = probe_bootstrap_source(registry=registry)
    runtime_probe = probe_local_consumer_runtime()
    blockers, bootstrap_source, runtime_probe = blockers_with_runtime_dependencies(
        {
            "blockers": context.get("lifecycle_blockers", []),
            "bootstrap_source": bootstrap_source,
            "runtime_probe": runtime_probe,
        }
    )
    required_status = context.get("required_node_status")
    node_roles = diagnostic._activation_role_nodes()
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
    for blocker in blockers:
        action = _activation_action_for_blocker(blocker)
        action_key = json.dumps(action, sort_keys=True)
        if action_key in seen_actions:
            continue
        seen_actions.add(action_key)
        actions.append(action)
    for node in nodes:
        for path_name, path_value in node.get("required_filesystem", {}).items():
            runtime_evidence = (
                node_roles.get(node.get("role"), {})
                .get("activation_evidence", {})
                .get("runtime_paths", {})
                .get(path_name, {})
            )
            if runtime_evidence.get("verified") is True:
                continue
            action = {
                "action": "materialize_runtime_path",
                "node": node.get("name"),
                "role": node.get("role"),
                "path": path_name,
                "value": path_value,
                "proof": node.get("filesystem_proofs", {}).get(path_name, {}),
                "source": "infra/registry.json:lifecycle.kernel_release.runtime_paths",
            }
            if node.get("role") == diagnostic.lifecycle.get("build_runner_role"):
                registry_node = node_roles.get(node.get("role"), {})
                action["bootstrap_contract"] = build_runner_bootstrap_contract(
                    diagnostic.lifecycle,
                    registry_node,
                )
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
    payload = {
        "status": "ok" if context.get("lifecycle_nodes_ready") and not blockers else "blocked",
        "lifecycle": context.get("lifecycle"),
        "orchestrator_organ": context.get("orchestrator_organ"),
        "build_target": context.get("build_target"),
        "deploy_target": context.get("deploy_target"),
        "status_nodes_ready": context.get("status_nodes_ready"),
        "fleet_nodes_ready": context.get("fleet_nodes_ready"),
        "lifecycle_nodes_ready": context.get("lifecycle_nodes_ready"),
        "bootstrap_source": bootstrap_source,
        "runtime_probe": runtime_probe,
        "nodes": nodes,
        "blockers": blockers,
        "actions": actions,
        "next_requests": next_requests,
        "activation_requirements": diagnostic.lifecycle.get("activation_requirements", {}),
        "fleet_requirements": diagnostic.lifecycle.get("fleet_requirements", {}),
        "runtime_paths": diagnostic.lifecycle.get("runtime_paths", {}),
        "runtime_path_proofs": diagnostic.lifecycle.get("runtime_path_proofs", {}),
        "local_execution_policy": {
            "builds_on_operator_machine": False,
            "source": "infra/registry.json:lifecycle.kernel_release",
            "repo_root": diagnostic.lifecycle.get("runtime_paths", {}).get("repo_root"),
        },
    }
    return attach_surface_contract(payload, LIFECYCLE_ACTIVATION_PLAN_SURFACE)
