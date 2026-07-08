"""Lifecycle activation planning and dispatch helpers for organ-forge."""

from __future__ import annotations

import json
from pathlib import Path

from .entities import FleetProbe
from .lifecycle_bootstrap import build_runner_bootstrap_contract
from .lifecycle_bootstrap_source import probe_bootstrap_source
from .lifecycle_consumer_status import probe_local_consumer_runtime
from .lifecycle_diagnostic_payloads import blockers_with_runtime_dependencies
from .lifecycle_readiness import probe_local_network_runtime
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


def _mcp_reload_policy(lifecycle: dict) -> dict:
    controller = lifecycle.get("mcp_freshness_controller", {})
    request_profiles = lifecycle.get("request_profiles", {})
    profile = controller.get("dispatch_profile", "mcp-reload")
    return {
        "dispatch_profile": profile,
        "dispatch_tool": controller.get("dispatch_tool", "cynic_lifecycle_dispatch"),
        "fallback_command": controller.get("fallback_command", "cynic mcp-reload"),
        "legacy_fallback_command": controller.get(
            "legacy_fallback_command",
            "make mcp-reload-request",
        ),
        "proof_requirements": controller.get("proof_requirements", []),
        "debounce_seconds": controller.get("debounce_seconds", 900),
        "request_intents": request_profiles.get(profile, []),
        "request_make_target": f"{profile}-request",
    }


def _network_actuator_activation_commands() -> dict:
    install_commands = [
        "install -d -o root -g root -m 0755 /usr/local/libexec",
        "install -d -o root -g root -m 0755 /etc/cynic",
        "install -d -o root -g root -m 0755 /etc/sudoers.d",
        "install -o root -g root -m 0755 /tmp/cynic-network-actuator/cynic-network-actuator /usr/local/libexec/cynic-network-actuator",
        "install -o root -g root -m 0644 /tmp/cynic-network-actuator/network-actuator.policy.json /etc/cynic/network-actuator.policy.json",
        "install -o root -g root -m 0440 /tmp/cynic-network-actuator/cynic-network-actuator.sudoers /etc/sudoers.d/cynic-network-actuator",
        "visudo -cf /etc/sudoers.d/cynic-network-actuator",
    ]
    return {
        "validation_command": "cargo run -q -p cynic-node -- network actuator-status --json",
        "install_plan_command": "python3 organs/fleet/src/entrypoints/cli.py network-actuator install-plan",
        "activation_plan_command": "python3 organs/fleet/src/entrypoints/cli.py network-actuator activation-plan --bundle-dir /tmp/cynic-network-actuator",
        "bundle_command": "python3 organs/fleet/src/entrypoints/cli.py network-actuator bundle --output-dir /tmp/cynic-network-actuator",
        "install_commands": install_commands,
        "manual_root_install_command": " && ".join(install_commands),
        "postflight_commands": [
            "cargo run -q -p cynic-node -- network actuator-status --json",
            "cargo run -q -p cynic-node -- observe --json",
        ],
    }


def _activation_action_for_blocker(blocker: dict, lifecycle: dict | None = None) -> dict:
    lifecycle = lifecycle or {}
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
    if category == "network" and blocker.get("kind") == "runtime_blocker":
        blocker_name = blocker.get("blocker")
        if blocker_name == "canonical_dns_projection_drift":
            return {
                "action": "restore_canonical_dns_projection",
                "hostname": blocker.get("hostname"),
                "code": blocker.get("code"),
                "resolver_organ": blocker.get("resolver_organ"),
                "resolver_consumer": blocker.get("resolver_consumer"),
                "command": blocker.get("command"),
                "operator_action": "render and activate the canonical Synapse DNS projection, then rerun cynic-node observe",
                "bundle_command": "python3 organs/fleet/src/entrypoints/cli.py dns bundle --registry infra/registry.json --output-dir /tmp/cynic-synapse-dns",
                "activation_status_command": "python3 organs/fleet/src/entrypoints/cli.py dns activation-status --resolve-dns",
                "validation_command": "cargo run -q -p cynic-node -- observe --json",
            }
        if blocker_name == "synapse_route_missing":
            return {
                "action": "activate_synapse_route",
                "hostname": blocker.get("hostname"),
                "code": blocker.get("code"),
                "resolver_organ": blocker.get("resolver_organ"),
                "resolver_consumer": blocker.get("resolver_consumer"),
                "command": blocker.get("command"),
                "activation_plan_command": "python3 organs/fleet/src/entrypoints/cli.py network-actuator activation-plan --bundle-dir /tmp/cynic-network-actuator",
                "bundle_command": "python3 organs/fleet/src/entrypoints/cli.py network-actuator bundle --output-dir /tmp/cynic-network-actuator",
                "validation_command": "cargo run -q -p cynic-node -- network repair --dry-run",
            }
        if blocker_name == "wireguard_mesh_absent":
            return {
                "action": "activate_wireguard_mesh",
                "hostname": blocker.get("hostname"),
                "code": blocker.get("code"),
                "resolver_organ": blocker.get("resolver_organ"),
                "resolver_consumer": blocker.get("resolver_consumer"),
                "command": blocker.get("command"),
                "activation_plan_command": "python3 organs/fleet/src/entrypoints/cli.py network-actuator activation-plan --bundle-dir /tmp/cynic-network-actuator",
                "bundle_command": "python3 organs/fleet/src/entrypoints/cli.py network-actuator bundle --output-dir /tmp/cynic-network-actuator",
                "validation_command": "cargo run -q -p cynic-node -- network repair --dry-run",
            }
    if category == "network" and blocker.get("kind") == "actuator_activation":
        return {
            "action": "install_network_actuator_bundle",
            "hostname": blocker.get("hostname"),
            "code": blocker.get("code"),
            "resolver_organ": blocker.get("resolver_organ"),
            "resolver_consumer": blocker.get("resolver_consumer"),
            "command": blocker.get("command"),
            "next_actions": blocker.get("next_actions", []),
            "root_authority": blocker.get("root_authority"),
            "operator_action": "run installation through a root-approved channel on the target node, then rerun activation-status and cynic-node evidence",
            **_network_actuator_activation_commands(),
        }
    if category == "network" and blocker.get("kind") == "host_exec_activation":
        return {
            "action": "install_host_exec_bundle",
            "hostname": blocker.get("hostname"),
            "code": blocker.get("code"),
            "resolver_organ": blocker.get("resolver_organ"),
            "resolver_consumer": blocker.get("resolver_consumer"),
            "command": blocker.get("command"),
            "missing": blocker.get("missing", []),
            "helper_path": blocker.get("helper_path"),
            "sudoers_rule": blocker.get("sudoers_rule"),
            "bundle_complete": blocker.get("bundle_complete"),
            "next_actions": blocker.get("next_actions", []),
            "proof_requirements": blocker.get("proof_requirements", []),
            "operator_action": "install and authorize the canonical host-exec helper bundle on the target host, then rerun host-exec activation-status",
            **_host_exec_activation_commands(),
        }
    if category == "network" and blocker.get("kind") == "root_authority":
        return {
            "action": "obtain_root_install_channel",
            "hostname": blocker.get("hostname"),
            "code": blocker.get("code"),
            "resolver_organ": blocker.get("resolver_organ"),
            "resolver_consumer": blocker.get("resolver_consumer"),
            "command": blocker.get("command"),
            "required": blocker.get("required"),
            "actual": blocker.get("actual"),
            "operator_action": "use the registry-approved root or remote execution channel instead of local sudo-prompted mutation",
        }
    if category == "network" and blocker.get("kind") == "remote_exec_transport":
        mcp_reload_policy = _mcp_reload_policy(lifecycle)
        transport_probe = blocker.get("transport_probe") if isinstance(blocker.get("transport_probe"), dict) else {}
        repair_local_debug_binary = transport_probe.get("repair_local_debug_binary") is True
        transport_metadata = (
            transport_probe.get("metadata") if isinstance(transport_probe.get("metadata"), dict) else {}
        )
        transport_failure_mode = transport_probe.get("error") or transport_metadata.get("error")
        response_error_message = transport_probe.get("response_error_message")
        required_bridge = transport_probe.get("required_bridge")
        transport_reason = transport_probe.get("reason")
        transport_missing = transport_probe.get("missing")
        declared_role = transport_probe.get("declared_role")
        declared_transport = (
            transport_probe.get("declared_transport")
            if isinstance(transport_probe.get("declared_transport"), dict)
            else None
        )
        declared_transport_kind = (
            declared_transport.get("kind") if isinstance(declared_transport, dict) else None
        )
        host_exec_commands = {}
        operator_action = "refresh the MCP-backed fleet_node_exec transport before attempting remote root installation"
        if transport_failure_mode == "remote_exec_transport_no_payload" and transport_metadata.get("error") == "timeout":
            operator_action = (
                "investigate the MCP fleet_node_exec call path or kernel reachability: the local launcher is fresh but the tool call timed out before returning payload"
            )
        elif transport_failure_mode == "mcp_tool_error" and response_error_message:
            operator_action = (
                f"repair the fleet_node_exec registry/provider contract before retrying remote root installation: {response_error_message}"
            )
        elif transport_probe.get("status") == "precondition_missing":
            if required_bridge:
                operator_action = required_bridge
            if (
                transport_missing == "host_exec_transport"
                or declared_role in {"cortex_host", "inference_node", "hypervisor"}
                or "host exec transport" in str(response_error_message or "").lower()
                or "host exec transport" in str(required_bridge or "").lower()
            ):
                host_exec_commands = _host_exec_activation_commands()
                operator_action = (
                    "build and install the canonical host-exec helper bundle on the host-native surface, "
                    "then promote host_exec_transport from manual_operator_surface to an executable transport"
                )
        elif transport_probe.get("status") == "transport_failed" and declared_transport_kind == "ssh_sudo_helper":
            host_exec_commands = _host_exec_activation_commands()
            operator_action = (
                "install and authorize the canonical host-exec helper bundle on this host-native surface "
                "so the declared ssh_sudo_helper transport can run without a password prompt"
            )
        return {
            "action": "refresh_remote_exec_transport",
            "hostname": blocker.get("hostname"),
            "code": blocker.get("code"),
            "resolver_organ": blocker.get("resolver_organ"),
            "resolver_consumer": blocker.get("resolver_consumer"),
            "command": blocker.get("command"),
            "required": blocker.get("required"),
            "actual": blocker.get("actual"),
            "transport_probe": blocker.get("transport_probe"),
            "operator_action": operator_action,
            "mcp_reload_command": mcp_reload_policy["fallback_command"],
            "legacy_mcp_reload_command": mcp_reload_policy["legacy_fallback_command"],
            "emergency_repair_command": "make mcp-repair-local",
            "dispatch_profile": mcp_reload_policy["dispatch_profile"],
            "dispatch_tool": mcp_reload_policy["dispatch_tool"],
            "request_intents": mcp_reload_policy["request_intents"],
            "request_make_target": mcp_reload_policy["request_make_target"],
            "proof_requirements": mcp_reload_policy["proof_requirements"],
            "debounce_seconds": mcp_reload_policy["debounce_seconds"],
            "repair_local_debug_binary": repair_local_debug_binary,
            "canonical_binary": transport_probe.get("canonical_binary"),
            "stale_source": transport_probe.get("stale_source"),
            "recommended_build_profile": "debug_repair" if repair_local_debug_binary else "release",
            "transport_failure_mode": transport_failure_mode,
            "transport_timeout_seconds": transport_metadata.get("timeout_seconds"),
            "response_error_message": response_error_message,
            "response_error_code": transport_probe.get("response_error_code"),
            "required_bridge": required_bridge,
            "transport_reason": transport_reason,
            "transport_missing": transport_missing,
            "declared_role": declared_role,
            "declared_transport": declared_transport,
            **host_exec_commands,
        }
    return {
        "action": "resolve_lifecycle_blocker",
        "code": blocker.get("code"),
        "category": category,
    }


def _fleet_exec_transport(lifecycle: dict) -> dict:
    bootstrap = lifecycle.get("build_policy", {}).get("bootstrap", {})
    return {
        "preferred_transport": bootstrap.get("preferred_transport", "fleet_node_exec"),
        "fallback_transport": bootstrap.get("fallback_transport", "parent_provider_exec"),
        "transport_command_env": bootstrap.get(
            "transport_command_env",
            "CYNIC_FLEET_NODE_EXEC_COMMAND",
        ),
        "transport_command_default": bootstrap.get(
            "transport_command_default",
            "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
        ),
    }


def _host_exec_activation_commands() -> dict:
    install_commands = [
        "install -d -o root -g root -m 0755 /usr/local/libexec",
        "install -d -o root -g root -m 0755 /etc/cynic",
        "install -d -o root -g root -m 0755 /etc/sudoers.d",
        "install -o root -g root -m 0755 /tmp/cynic-host-exec/cynic-host-exec /usr/local/libexec/cynic-host-exec",
        "install -o root -g root -m 0644 /tmp/cynic-host-exec/host-exec.policy.json /etc/cynic/host-exec.policy.json",
        "install -o root -g root -m 0440 /tmp/cynic-host-exec/cynic-host-exec.sudoers /etc/sudoers.d/cynic-host-exec",
        "visudo -cf /etc/sudoers.d/cynic-host-exec",
    ]
    return {
        "validation_command": "python3 organs/fleet/src/entrypoints/cli.py host-exec validate-policy --json",
        "install_plan_command": "python3 organs/fleet/src/entrypoints/cli.py host-exec install-plan",
        "activation_plan_command": "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-plan --bundle-dir /tmp/cynic-host-exec",
        "bundle_command": "python3 organs/fleet/src/entrypoints/cli.py host-exec bundle --output-dir /tmp/cynic-host-exec",
        "activation_status_command": "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-status --bundle-dir /tmp/cynic-host-exec",
        "install_commands": install_commands,
        "manual_root_install_command": " && ".join(install_commands),
        "postflight_commands": [
            "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-status --bundle-dir /tmp/cynic-host-exec",
            "python3 organs/forge/src/entrypoints/cli.py lifecycle-blockers",
        ],
    }


def _network_actuator_remote_install_request(hostname: str) -> dict:
    command = (
        "install -d -o root -g root -m 0755 /usr/local/libexec"
        " && install -d -o root -g root -m 0755 /etc/cynic"
        " && install -d -o root -g root -m 0755 /etc/sudoers.d"
        " && install -o root -g root -m 0755 /tmp/cynic-network-actuator/cynic-network-actuator /usr/local/libexec/cynic-network-actuator"
        " && install -o root -g root -m 0644 /tmp/cynic-network-actuator/network-actuator.policy.json /etc/cynic/network-actuator.policy.json"
        " && install -o root -g root -m 0440 /tmp/cynic-network-actuator/cynic-network-actuator.sudoers /etc/sudoers.d/cynic-network-actuator"
        " && visudo -cf /etc/sudoers.d/cynic-network-actuator"
    )
    return {
        "tool": "fleet_node_exec",
        "node": hostname,
        "command": command,
        "timeout_seconds": 120,
        "purpose": "install network actuator bundle through registry-approved remote execution",
        "agent_id": "organ-forge-network-root-channel",
    }


def _root_channel_probe_request(hostname: str) -> dict:
    return {
        "tool": "fleet_node_exec",
        "node": hostname,
        "command": "sudo -n true",
        "timeout_seconds": 30,
        "purpose": "verify remote noninteractive root channel for network actuator installation",
        "agent_id": "organ-forge-network-root-channel",
    }


def _root_install_sequence(actions: list[dict[str, object]]) -> list[dict[str, object]]:
    indexed = {
        str(action.get("action")): action
        for action in actions
        if isinstance(action, dict) and action.get("action")
    }
    sequence: list[dict[str, object]] = []
    for order, action_name in enumerate(
        ("install_host_exec_bundle", "install_network_actuator_bundle"),
        start=1,
    ):
        action = indexed.get(action_name)
        if not isinstance(action, dict):
            continue
        sequence.append(
            {
                "order": order,
                "action": action_name,
                "hostname": action.get("hostname"),
                "dependency": "required_before_remote_network_repair"
                if action_name == "install_host_exec_bundle"
                else "requires_host_exec_transport",
                "manual_root_install_command": action.get("manual_root_install_command"),
                "install_commands": action.get("install_commands", []),
                "postflight_commands": action.get("postflight_commands", []),
                "validation_command": action.get("validation_command"),
                "bundle_command": action.get("bundle_command"),
                "activation_status_command": action.get("activation_status_command"),
                "remote_exec_request": action.get("remote_exec_request"),
                "required_commands": ["install", "visudo"],
                "required_bundle_files": (
                    [
                        "/tmp/cynic-host-exec/cynic-host-exec",
                        "/tmp/cynic-host-exec/host-exec.policy.json",
                        "/tmp/cynic-host-exec/cynic-host-exec.sudoers",
                        "/tmp/cynic-host-exec/manifest.json",
                    ]
                    if action_name == "install_host_exec_bundle"
                    else [
                        "/tmp/cynic-network-actuator/cynic-network-actuator",
                        "/tmp/cynic-network-actuator/network-actuator.policy.json",
                        "/tmp/cynic-network-actuator/cynic-network-actuator.sudoers",
                        "/tmp/cynic-network-actuator/manifest.json",
                    ]
                ),
            }
        )
    return sequence


def build_lifecycle_activation_plan(
    registry: dict | None = None,
    inspect_fleet: bool = False,
    include_network_probe: bool = False,
) -> dict:
    registry = registry or _load_registry()
    from ..adapters.kernel_adapter import KernelRestClient
    from ..adapters.registry_adapter import validate_lifecycle_policy

    validate_lifecycle_policy(registry)
    lifecycle = registry.get("lifecycle", {}).get("kernel_release", {})
    transport = _fleet_exec_transport(lifecycle)

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
    network_probe = (
        probe_local_network_runtime()
        if include_network_probe
        else {"enabled": False, "status": "not_run", "blockers": []}
    )
    blockers, bootstrap_source, runtime_probe, network_probe = blockers_with_runtime_dependencies(
        {
            "blockers": context.get("lifecycle_blockers", []),
            "bootstrap_source": bootstrap_source,
            "runtime_probe": runtime_probe,
            "network_probe": network_probe,
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
        action = _activation_action_for_blocker(blocker, lifecycle)
        if action.get("action") in {
            "activate_synapse_route",
            "activate_wireguard_mesh",
            "install_network_actuator_bundle",
            "obtain_root_install_channel",
        } and action.get("hostname"):
            action = {
                **action,
                **transport,
            }
            if action.get("action") == "install_network_actuator_bundle":
                action["remote_exec_request"] = _network_actuator_remote_install_request(
                    action["hostname"]
                )
            elif action.get("action") == "obtain_root_install_channel":
                action["remote_exec_request"] = _root_channel_probe_request(action["hostname"])
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
        "network_probe": network_probe,
        "nodes": nodes,
        "blockers": blockers,
        "actions": actions,
        "root_install_sequence": _root_install_sequence(actions),
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
