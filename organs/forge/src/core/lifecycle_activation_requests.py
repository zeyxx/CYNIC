"""Lifecycle activation request payloads and kernel dispatch helpers."""

from __future__ import annotations

import json
from hashlib import sha256
from pathlib import Path

from organs.contracts.src.surface_contracts import json_object_or_raise
from organs.contracts.src.surface_contracts import read_json_file_or_default
from .agent_proposals import (
    build_lifecycle_agent_proposal,
    validate_lifecycle_agent_proposal,
)
from .lifecycle_activation import build_lifecycle_activation_plan
from .project_root import resolve_project_root_from_script
from organs.contracts.src.surface_contracts import (
    ReportSurfaceSpec,
    SurfaceRegistry,
    attach_surface_contract,
)

ACTIVATION_REQUEST_CONSUMER = "organ-forge-hermes-agent"
ACTIVATION_SCHEMA_VERSION = "activation.v1"

LIFECYCLE_BLOCKERS_SURFACE = ReportSurfaceSpec(
    schema="cynic.lifecycle.activation_summary.v1",
    command="cynic activate --dry-run",
)
LIFECYCLE_REQUESTS_SURFACE = ReportSurfaceSpec(
    schema="cynic.lifecycle.activation_requests.v1",
    command="cynic activate",
)
LIFECYCLE_DISPATCH_SURFACE = ReportSurfaceSpec(
    schema="cynic.lifecycle.activation_dispatch.v1",
    command="cynic activate --json",
)
LIFECYCLE_SURFACE_REGISTRY = SurfaceRegistry(
    reports=(
        LIFECYCLE_BLOCKERS_SURFACE,
        LIFECYCLE_REQUESTS_SURFACE,
        LIFECYCLE_DISPATCH_SURFACE,
    ),
)


def _default_registry_path() -> Path:
    return resolve_project_root_from_script(__file__) / "infra" / "registry.json"


def _load_registry(path: str | Path | None = None) -> dict:
    target_path = Path(path) if path else _default_registry_path()
    return read_json_file_or_default(target_path, {})


def _unique_targets(targets: list[str]) -> list[str]:
    unique = []
    for target in targets:
        if target and target not in unique:
            unique.append(target)
    return unique


def _activation_request_targets(action: dict) -> dict:
    action_name = action.get("action")
    registry_targets = ["infra/registry.json"]
    fleet_targets = ["organs/fleet"]
    fleet_node_targets = ["organs/fleet", "crates/cynic-node"]
    network_runtime_targets = ["organs/fleet", "crates/cynic-node"]

    if action_name in {
        "provide_placeholder_env_evidence",
        "verify_runtime_path_proof",
        "record_service_activation_evidence",
        "activate_registry_node",
        "materialize_runtime_path",
    }:
        targets = registry_targets
    elif action_name == "refresh_fleet_inventory":
        targets = fleet_targets
    elif action_name == "restore_fleet_probe":
        targets = fleet_targets
    elif action_name in {
        "provide_runtime_observation",
        "join_or_wake_synapse_peer",
    }:
        targets = network_runtime_targets
    elif action_name in {
        "restore_canonical_dns_projection",
        "install_network_actuator_bundle",
        "activate_synapse_route",
        "activate_wireguard_mesh",
    }:
        targets = fleet_node_targets
    else:
        targets = ["organs/forge", "infra/registry.json"]

    return {
        "targets": _unique_targets(targets),
        "coord_targets": _unique_targets(targets),
    }


def _activation_request_priority(action: dict) -> int:
    action_name = action.get("action")
    if action_name in {"provide_placeholder_env_evidence", "record_service_activation_evidence"}:
        return 99
    if action_name == "restore_fleet_probe":
        return 98
    if action_name in {"refresh_fleet_inventory", "provide_runtime_observation"}:
        return 97
    if action_name in {
        "restore_canonical_dns_projection",
        "install_network_actuator_bundle",
        "activate_synapse_route",
        "activate_wireguard_mesh",
    }:
        return 97
    if action_name == "materialize_runtime_path":
        return 96
    if action_name in {"verify_runtime_path_proof", "activate_registry_node"}:
        return 95
    return 90


def build_lifecycle_activation_requests(
    registry: dict | None = None,
    inspect_fleet: bool = False,
    include_network_probe: bool = False,
) -> dict:
    registry = registry or _load_registry()
    plan = build_lifecycle_activation_plan(
        registry=registry,
        inspect_fleet=inspect_fleet,
        include_network_probe=include_network_probe,
    )
    requests = []
    for action in plan["actions"]:
        scoped_targets = _activation_request_targets(action)
        content = {
            "lifecycle": "kernel_release",
            "activation_schema_version": ACTIVATION_SCHEMA_VERSION,
            "objective": f"activate {action.get('action')} for {action.get('node', 'lifecycle')}",
            "orchestrator_organ": plan["orchestrator_organ"],
            "domain": "organ-forge",
            "build_target": plan["build_target"],
            "build_runner_role": "build_runner",
            "deploy_target": plan["deploy_target"],
            "deploy_target_role": "kernel_primary",
            "required_node_status": "active",
            "activation_requirements": plan.get("activation_requirements", {}),
            "fleet_requirements": plan.get("fleet_requirements", {}),
            "runtime_paths": plan.get("runtime_paths", {}),
            "runtime_path_proofs": plan.get("runtime_path_proofs", {}),
            "activation_evidence_contract": {
                "patch_field": "activation_evidence_patch",
                "node_field": "node",
                "runtime_paths_field": "activation_evidence.runtime_paths",
                "services_field": "activation_evidence.services",
                "verified_requires": ["marker_exists", "sentinels_verified"],
                "service_active_requires": ["declared", "active"],
                "ingestion_mode": "validated_merge",
            },
            "targets": scoped_targets["targets"],
            "coord_targets": scoped_targets["coord_targets"],
            "deploy": False,
            "action": action,
        }
        content["correlation_id"] = _activation_correlation_id({"content": content})
        content["operator_interface_proposal"] = build_lifecycle_agent_proposal(
            content,
            source_agent_id="organ-forge-cli",
        )
        requests.append(
            {
                "kind": "hermes",
                "domain": "organ-forge",
                "agent_id": "organ-forge-cli",
                "consumer": ACTIVATION_REQUEST_CONSUMER,
                "priority": _activation_request_priority(action),
                "content": json.dumps(content, separators=(",", ":")),
                "surface_contract": LIFECYCLE_REQUESTS_SURFACE.as_manifest(),
            }
        )

    payload = {
        "status": "ok" if plan.get("status") == "ok" else "blocked",
        "activation_schema_version": ACTIVATION_SCHEMA_VERSION,
        "lifecycle": plan.get("lifecycle"),
        "orchestrator_organ": plan.get("orchestrator_organ"),
        "build_target": plan.get("build_target"),
        "deploy_target": plan.get("deploy_target"),
        "status_nodes_ready": plan.get("status_nodes_ready"),
        "fleet_nodes_ready": plan.get("fleet_nodes_ready"),
        "lifecycle_nodes_ready": plan.get("lifecycle_nodes_ready"),
        "blockers": plan.get("blockers", []),
        "actions": plan.get("actions", []),
        "root_install_sequence": plan.get("root_install_sequence", []),
        "requests": requests,
        "request_summary": [_activation_request_summary(payload) for payload in requests],
        "next_requests": plan.get("next_requests", []),
        "surface_contract": LIFECYCLE_REQUESTS_SURFACE.as_manifest(),
    }
    return payload


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
    if payload.get("consumer") != ACTIVATION_REQUEST_CONSUMER:
        errors.append("payload.consumer does not match activation request consumer")

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
    expected = {
        "lifecycle": "kernel_release",
        "activation_schema_version": ACTIVATION_SCHEMA_VERSION,
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
        "deploy": False,
    }
    for key, expected_value in expected.items():
        if content.get(key) != expected_value:
            errors.append(f"content.{key} does not match lifecycle registry")

    if not isinstance(content.get("action"), dict) or not content["action"].get("action"):
        errors.append("content.action must be a structured activation action")
    else:
        expected_targets = _activation_request_targets(content["action"])
        if content.get("targets") != expected_targets["targets"]:
            errors.append("content.targets does not match activation action scope")
        if content.get("coord_targets") != expected_targets["coord_targets"]:
            errors.append("content.coord_targets does not match activation action scope")
        if payload.get("priority") != _activation_request_priority(content["action"]):
            errors.append("payload.priority does not match activation action priority")
        if content.get("correlation_id") != _activation_correlation_id(payload):
            errors.append("content.correlation_id does not match activation action scope")
    if not content.get("objective"):
        errors.append("content.objective missing")
    errors.extend(validate_lifecycle_agent_proposal(content))

    if errors:
        raise RuntimeError("; ".join(errors))
    payload = {
        "status": "ok",
        "lifecycle": "kernel_release",
        "action": content["action"].get("action"),
        "objective": content["objective"],
        "surface_contract": LIFECYCLE_REQUESTS_SURFACE.as_manifest(),
    }
    return payload


def _activation_request_key(payload: dict) -> str:
    raw_content = payload.get("content", "")
    content = json_object_or_raise(raw_content, RuntimeError("payload.content is not valid JSON"))
    return json.dumps(
        {
            "lifecycle": content.get("lifecycle"),
            "action": content.get("action"),
            "targets": content.get("targets", []),
            "coord_targets": content.get("coord_targets", []),
        },
        sort_keys=True,
        separators=(",", ":"),
    )


def _activation_correlation_id(payload: dict) -> str:
    return "activation:" + sha256(_activation_request_key(payload).encode("utf-8")).hexdigest()[:16]


def _pending_activation_task_by_key(tasks: list[dict]) -> dict[str, dict]:
    indexed = {}
    for task in tasks:
        if not isinstance(task, dict):
            continue
        try:
            key = _activation_request_key({"content": task.get("content", "")})
        except RuntimeError:
            continue
        indexed.setdefault(key, task)
    return indexed


def _activation_request_summary(payload: dict) -> dict:
    raw_content = payload.get("content", "")
    content = json_object_or_raise(raw_content, RuntimeError("payload.content is not valid JSON"))
    action = content.get("action", {})
    summary = {
        "correlation_id": _activation_correlation_id(payload),
        "action": action.get("action"),
        "node": action.get("node"),
        "hostname": action.get("hostname"),
        "role": action.get("role"),
        "code": action.get("code"),
        "targets": content.get("targets", []),
        "coord_targets": content.get("coord_targets", []),
        "consumer": payload.get("consumer"),
        "priority": payload.get("priority"),
        "resolver_organ": action.get("resolver_organ"),
        "resolver_consumer": action.get("resolver_consumer"),
        "command": action.get("command"),
    }
    for key in (
        "operator_action",
        "bundle_command",
        "install_plan_command",
        "install_commands",
        "manual_root_install_command",
        "activation_plan_command",
        "activation_status_command",
        "postflight_commands",
        "validation_command",
        "missing",
        "helper_path",
        "sudoers_rule",
        "bundle_complete",
        "next_actions",
        "preferred_transport",
        "fallback_transport",
        "transport_command_env",
        "transport_command_default",
        "remote_exec_request",
        "mcp_reload_command",
        "legacy_mcp_reload_command",
        "emergency_repair_command",
        "dispatch_profile",
        "dispatch_tool",
        "request_intents",
        "request_make_target",
        "proof_requirements",
        "debounce_seconds",
        "repair_local_debug_binary",
        "canonical_binary",
        "stale_source",
        "recommended_build_profile",
        "transport_failure_mode",
        "transport_timeout_seconds",
        "response_error_message",
        "response_error_code",
        "required_bridge",
        "transport_reason",
        "transport_missing",
        "declared_role",
        "declared_transport",
    ):
        if key in action:
            summary[key] = action.get(key)
    return summary


def dispatch_lifecycle_activation_requests(
    registry: dict | None = None,
    inspect_fleet: bool = False,
    include_network_probe: bool = False,
    kernel_client_factory=None,
) -> dict:
    registry = registry or _load_registry()
    plan = build_lifecycle_activation_requests(
        registry=registry,
        inspect_fleet=inspect_fleet,
        include_network_probe=include_network_probe,
    )
    if not plan.get("requests"):
        raise RuntimeError("no lifecycle activation requests were generated")

    if kernel_client_factory is None:
        from ..adapters.kernel_adapter import KernelRestClient as kernel_client_factory

    client = kernel_client_factory(require_api_key=True, strict_lifecycle=False)
    if not getattr(client, "api_key", ""):
        raise RuntimeError("CYNIC_API_KEY is required for organ-forge kernel dispatch")

    pending_listing = client._list_agent_tasks(kind="hermes", domain="organ-forge")
    pending_by_key = _pending_activation_task_by_key(pending_listing.get("tasks", []))
    responses = []
    dispatched_tasks = []
    skipped_existing = []
    for payload in plan["requests"]:
        validate_lifecycle_activation_request_payload(payload, registry)
        payload_content = json_object_or_raise(payload["content"], RuntimeError("payload.content is not valid JSON"))
        request_key = _activation_request_key(payload)
        existing_task = pending_by_key.get(request_key)
        if existing_task:
            skipped_existing.append(
                {
                    "task_id": existing_task.get("id"),
                    "correlation_id": _activation_correlation_id(payload),
                    "action": payload_content["action"].get("action"),
                    "reason": "pending_duplicate",
                }
            )
            continue
        response = client._post_agent_task(payload)
        task_id = response.get("task_id") or response.get("id")
        if not task_id:
            raise RuntimeError(f"activation task dispatch response missing task_id: {response}")
        client._assert_agent_task_visible(
            task_id,
            kind=payload["kind"],
            domain=payload["domain"],
        )
        responses.append(response)
        dispatched_tasks.append(
            {
                "task_id": task_id,
                "kind": payload.get("kind"),
                "domain": payload.get("domain"),
                "consumer": payload.get("consumer"),
                "priority": payload.get("priority"),
                "correlation_id": payload_content.get("correlation_id"),
                "action": payload_content["action"].get("action"),
                "hostname": payload_content["action"].get("hostname"),
                "targets": payload_content.get("targets", []),
                "coord_targets": payload_content.get("coord_targets", []),
                "resolver_organ": payload_content["action"].get("resolver_organ"),
                "resolver_consumer": payload_content["action"].get("resolver_consumer"),
                "command": payload_content["action"].get("command"),
                "bundle_command": payload_content["action"].get("bundle_command"),
                "install_plan_command": payload_content["action"].get("install_plan_command"),
                "install_commands": payload_content["action"].get("install_commands"),
                "manual_root_install_command": payload_content["action"].get("manual_root_install_command"),
                "activation_plan_command": payload_content["action"].get("activation_plan_command"),
                "activation_status_command": payload_content["action"].get("activation_status_command"),
                "postflight_commands": payload_content["action"].get("postflight_commands"),
                "validation_command": payload_content["action"].get("validation_command"),
                "preferred_transport": payload_content["action"].get("preferred_transport"),
                "fallback_transport": payload_content["action"].get("fallback_transport"),
                "transport_command_env": payload_content["action"].get("transport_command_env"),
                "transport_command_default": payload_content["action"].get("transport_command_default"),
                "remote_exec_request": payload_content["action"].get("remote_exec_request"),
                "mcp_reload_command": payload_content["action"].get("mcp_reload_command"),
                "legacy_mcp_reload_command": payload_content["action"].get("legacy_mcp_reload_command"),
                "emergency_repair_command": payload_content["action"].get("emergency_repair_command"),
                "dispatch_profile": payload_content["action"].get("dispatch_profile"),
                "dispatch_tool": payload_content["action"].get("dispatch_tool"),
                "request_intents": payload_content["action"].get("request_intents"),
                "request_make_target": payload_content["action"].get("request_make_target"),
                "proof_requirements": payload_content["action"].get("proof_requirements"),
                "debounce_seconds": payload_content["action"].get("debounce_seconds"),
                "repair_local_debug_binary": payload_content["action"].get("repair_local_debug_binary"),
                "canonical_binary": payload_content["action"].get("canonical_binary"),
                "stale_source": payload_content["action"].get("stale_source"),
                "recommended_build_profile": payload_content["action"].get("recommended_build_profile"),
                "transport_failure_mode": payload_content["action"].get("transport_failure_mode"),
                "transport_timeout_seconds": payload_content["action"].get("transport_timeout_seconds"),
                "response_error_message": payload_content["action"].get("response_error_message"),
                "response_error_code": payload_content["action"].get("response_error_code"),
                "required_bridge": payload_content["action"].get("required_bridge"),
                "transport_reason": payload_content["action"].get("transport_reason"),
                "transport_missing": payload_content["action"].get("transport_missing"),
                "declared_role": payload_content["action"].get("declared_role"),
                "declared_transport": payload_content["action"].get("declared_transport"),
            }
        )
        pending_by_key[request_key] = {"id": task_id, "content": payload["content"]}

    payload = {
        "status": "ok",
        "activation_schema_version": ACTIVATION_SCHEMA_VERSION,
        "plan_status": plan.get("status"),
        "lifecycle": plan.get("lifecycle"),
        "orchestrator_organ": plan.get("orchestrator_organ"),
        "build_target": plan.get("build_target"),
        "deploy_target": plan.get("deploy_target"),
        "blockers": plan.get("blockers", []),
        "actions": plan.get("actions", []),
        "requests": plan.get("requests", []),
        "request_summary": plan.get("request_summary", []),
        "responses": responses,
        "dispatched_tasks": dispatched_tasks,
        "dispatched_count": len(responses),
        "skipped_existing": skipped_existing,
        "skipped_existing_count": len(skipped_existing),
        "dispatch_summary": {
            "requested": len(plan.get("requests", [])),
            "dispatched": len(responses),
            "skipped_existing": len(skipped_existing),
            "consumer": ACTIVATION_REQUEST_CONSUMER,
            "task_ids": [task.get("task_id") for task in dispatched_tasks],
            "correlation_ids": [
                item.get("correlation_id")
                for item in plan.get("request_summary", [])
                if item.get("correlation_id")
            ],
        },
        "next_requests": plan.get("next_requests", []),
        "local_execution_policy": plan.get("local_execution_policy", {}),
        "surface_contract": LIFECYCLE_DISPATCH_SURFACE.as_manifest(),
    }
    return payload
