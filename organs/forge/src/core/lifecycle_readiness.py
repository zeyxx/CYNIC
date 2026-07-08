"""Lifecycle readiness helpers for organ-forge."""

from __future__ import annotations

import json
import os
import subprocess

from organs.contracts.src.surface_contracts import json_object_or_default

from .entities import FleetProbe


def _run_json_shell(command: str, *, timeout: int = 30) -> tuple[dict | None, dict]:
    try:
        result = subprocess.run(
            ["bash", "-lc", command],
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        metadata = {
            "command": command,
            "exit_code": result.returncode,
            "stdout": result.stdout.strip(),
            "stderr": result.stderr.strip(),
        }
    except subprocess.TimeoutExpired as exc:
        return None, {
            "command": command,
            "exit_code": None,
            "stdout": (exc.stdout or "").strip(),
            "stderr": (exc.stderr or "").strip(),
            "timeout_seconds": timeout,
            "error": "timeout",
        }
    if result.returncode != 0:
        return None, metadata
    try:
        return json.loads(result.stdout), metadata
    except json.JSONDecodeError as exc:
        metadata["stderr"] = (
            f"{metadata['stderr']}\njson_decode_error:{exc}"
            if metadata["stderr"]
            else f"json_decode_error:{exc}"
        )
        return None, metadata


def _run_json_program(
    argv: list[str],
    request: dict[str, object],
    *,
    timeout: int = 30,
) -> tuple[dict | None, dict]:
    try:
        result = subprocess.run(
            argv,
            input=json.dumps(request, separators=(",", ":")),
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        metadata = {
            "argv": argv,
            "exit_code": result.returncode,
            "stdout": result.stdout.strip(),
            "stderr": result.stderr.strip(),
        }
    except subprocess.TimeoutExpired as exc:
        return None, {
            "argv": argv,
            "exit_code": None,
            "stdout": (exc.stdout or "").strip(),
            "stderr": (exc.stderr or "").strip(),
            "timeout_seconds": timeout,
            "error": "timeout",
        }
    if not result.stdout.strip():
        return None, metadata
    try:
        return json.loads(result.stdout), metadata
    except json.JSONDecodeError as exc:
        metadata["stderr"] = (
            f"{metadata['stderr']}\njson_decode_error:{exc}"
            if metadata["stderr"]
            else f"json_decode_error:{exc}"
        )
        return None, metadata


def _probe_mcp_launcher_freshness(timeout: int = 30) -> dict | None:
    env = os.environ.copy()
    env["CYNIC_MCP_CHECK_ONLY"] = "1"
    result = subprocess.run(
        ["python3", ".cortex/mcp/run_cynic_mcp.py"],
        check=False,
        capture_output=True,
        text=True,
        timeout=timeout,
        env=env,
    )
    stdout = result.stdout.strip()
    if not stdout:
        return None
    try:
        payload = json.loads(stdout)
    except json.JSONDecodeError:
        return None
    return payload if isinstance(payload, dict) else None


def probe_fleet_node_exec_transport(hostname: str, timeout: int = 30) -> dict:
    request = {
        "tool": "fleet_node_exec",
        "node": hostname,
        "command": "hostname",
        "timeout_seconds": timeout,
        "purpose": "verify root-capable remote execution transport for network repair",
        "agent_id": "organ-forge-network-root-channel",
    }
    payload, metadata = _run_json_program(
        ["python3", "organs/forge/src/entrypoints/fleet_node_exec_wrapper.py"],
        request,
        timeout=timeout,
    )
    if not isinstance(payload, dict):
        return {
            "enabled": True,
            "status": "blocked",
            "error": "remote_exec_transport_no_payload",
            "request": request,
            "metadata": metadata,
        }
    probe = {
        "enabled": True,
        "status": payload.get("status", "blocked"),
        "ok": payload.get("ok", False),
        "error": payload.get("error"),
        "transport": payload.get("transport"),
        "stderr": payload.get("stderr"),
        "canonical_binary": payload.get("canonical_binary"),
        "repair_local_debug_binary": payload.get("repair_local_debug_binary"),
        "stale_source": payload.get("stale_source"),
        "missing": payload.get("missing"),
        "reason": payload.get("reason"),
        "required_bridge": payload.get("required_bridge"),
        "declared_role": payload.get("declared_role"),
        "declared_transport": payload.get("declared_transport"),
        "response": payload.get("response"),
        "request": request,
        "metadata": metadata,
    }
    response = payload.get("response") if isinstance(payload.get("response"), dict) else {}
    response_error = response.get("error") if isinstance(response, dict) else {}
    if isinstance(response_error, dict):
        probe["response_error_code"] = response_error.get("code")
        probe["response_error_message"] = response_error.get("message")
    if probe.get("error") == "mcp_initialize_failed":
        launcher_check = _probe_mcp_launcher_freshness(timeout=timeout)
        if isinstance(launcher_check, dict):
            probe["launcher_check"] = launcher_check
            for key in ("canonical_binary", "repair_local_debug_binary", "stale_source", "status"):
                if probe.get(key) in (None, False) and key in launcher_check:
                    target_key = "launcher_status" if key == "status" else key
                    probe[target_key] = launcher_check.get(key)
                elif key == "status":
                    probe["launcher_status"] = launcher_check.get("status")
    return probe


def probe_local_network_runtime(timeout: int = 30) -> dict:
    runtime_status_command = (
        "cargo run -q -p cynic-node -- network runtime-status --json --resolve-dns"
    )
    host_exec_command = "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-status --bundle-dir /tmp/cynic-host-exec"
    runtime_status, runtime_status_meta = _run_json_shell(runtime_status_command, timeout=timeout)
    host_exec_activation, host_exec_meta = _run_json_shell(host_exec_command, timeout=timeout)

    blockers = []
    hostname = None
    observation = None
    actuator = None
    if isinstance(runtime_status, dict):
        hostname = runtime_status.get("hostname")
        observation = runtime_status.get("observation")
        actuator = runtime_status.get("actuator_activation")
        blockers = list(runtime_status.get("blockers", []))
    if isinstance(actuator, dict) and actuator.get("status") != "active":
        root_authority = actuator.get("root_authority", {})
        if isinstance(root_authority, dict) and root_authority.get("sudo_noninteractive_available") is False:
            transport_probe = probe_fleet_node_exec_transport(hostname or "unknown", timeout=30)
            if isinstance(host_exec_activation, dict) and host_exec_activation.get("status") != "active":
                blockers.append(
                    {
                        "category": "network",
                        "kind": "host_exec_activation",
                        "hostname": hostname,
                        "actual": host_exec_activation.get("status"),
                        "required": "active",
                        "code": f"network_host_exec:{host_exec_activation.get('status', 'unknown')}",
                        "resolver_organ": "organ-fleet",
                        "resolver_consumer": "organ-forge-hermes-agent",
                        "command": host_exec_command,
                        "missing": host_exec_activation.get("missing", []),
                        "helper_path": host_exec_activation.get("helper_path"),
                        "sudoers_rule": host_exec_activation.get("sudoers_rule"),
                        "next_actions": host_exec_activation.get("next_actions", []),
                        "proof_requirements": host_exec_activation.get("proof_requirements", []),
                        "bundle_complete": host_exec_activation.get("bundle_complete"),
                    }
                )
            blockers.append(
                {
                    "category": "network",
                    "kind": "root_authority",
                    "hostname": hostname,
                    "actual": "sudo_prompt_required",
                    "required": "sudo_noninteractive_available",
                    "code": "network_root_authority:sudo_noninteractive_unavailable",
                    "resolver_organ": "organ-fleet",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": root_authority.get("command", "sudo -n true"),
                }
            )
            if transport_probe.get("status") != "ok":
                blockers.append(
                    {
                        "category": "network",
                        "kind": "remote_exec_transport",
                        "hostname": hostname,
                        "actual": transport_probe.get("error") or transport_probe.get("status"),
                        "required": "ok",
                        "code": f"network_remote_exec_transport:{transport_probe.get('error') or transport_probe.get('status')}",
                        "resolver_organ": "organ-forge",
                        "resolver_consumer": "organ-forge-hermes-agent",
                        "command": "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
                        "transport_probe": transport_probe,
                    }
                )

    if runtime_status is None:
        return {
            "enabled": True,
            "status": "error",
            "error": "network_runtime_probe_failed",
            "commands": [runtime_status_meta],
            "blockers": [],
        }
    return {
        "enabled": True,
        "status": runtime_status.get("status", "blocked") if isinstance(runtime_status, dict) else "blocked",
        "hostname": hostname,
        "observation": observation,
        "actuator_activation": actuator,
        "dns_activation": runtime_status.get("dns_activation") if isinstance(runtime_status, dict) else None,
        "provider_fallback_status": runtime_status.get("provider_fallback_status") if isinstance(runtime_status, dict) else None,
        "host_exec_activation": host_exec_activation,
        "commands": [runtime_status_meta, host_exec_meta],
        "blockers": blockers,
    }


def build_activation_evidence_gap_details(
    lifecycle: dict,
    build_node: dict,
    deploy_node: dict,
    *,
    env: dict | None = None,
    extra_role_nodes: dict | None = None,
) -> list[dict]:
    env = env or os.environ
    requirements = lifecycle.get("activation_requirements", {})
    role_nodes = {
        lifecycle["build_runner_role"]: build_node,
        lifecycle["deploy_target_role"]: deploy_node,
    }
    role_nodes.update(extra_role_nodes or {})
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
                if not env.get(env_name):
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
        runtime_paths = lifecycle.get("runtime_paths", {})
        runtime_path_proofs = lifecycle.get("runtime_path_proofs", {})
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


def activation_evidence_gaps(
    lifecycle: dict,
    build_node: dict,
    deploy_node: dict,
    *,
    env: dict | None = None,
    extra_role_nodes: dict | None = None,
) -> list[str]:
    return [
        gap["code"]
        for gap in build_activation_evidence_gap_details(
            lifecycle,
            build_node,
            deploy_node,
            env=env,
            extra_role_nodes=extra_role_nodes,
        )
    ]


def inspect_fleet(
    build_target: str,
    deploy_target: str,
    *,
    timeout: int = 10,
) -> dict:
    target_names = {build_target, deploy_target}
    matches = {}
    missing = []
    
    # ORGAN-WIREGUARD-V3: Agnostic network probe with Deep Sovereignty.
    # We resolve the name directly from the registry SSoT to bypass local DNS / hosts.
    try:
        import json
        from pathlib import Path
        
        # Locate the registry.json dynamically
        # Assuming we are in CYNIC/organs/forge/src/core/lifecycle_readiness.py
        registry_path = Path(__file__).resolve().parent.parent.parent.parent.parent / "infra" / "registry.json"
        
        registry_data = {}
        if registry_path.exists():
            with open(registry_path, "r", encoding="utf-8") as f:
                registry_data = json.load(f)
                
        # Build a name-to-ip map
        ip_map = {}
        for node in registry_data.get("nodes", []):
            name = node.get("name")
            ip = node.get("network", {}).get("local") or node.get("ip")
            if name and ip and ip != "dhcp":
                ip_map[name] = ip
                
    except Exception as e:
        ip_map = {}

    for target in sorted(target_names):
        try:
            # Use the registry IP if found, otherwise fallback to the raw target name
            target_ip = ip_map.get(target, target)
            
            result = subprocess.run(
                ["ping", "-c", "1", "-W", "1", target_ip],
                check=False,
                capture_output=True,
                text=True,
                timeout=timeout,
            )
            if result.returncode == 0:
                matches[target] = {
                    "online": True,
                    "os": "unknown",  # OS agnostic
                    "name": target,
                }
            else:
                missing.append(target)
        except FileNotFoundError:
            return FleetProbe.from_dict(
                {
                    "enabled": True,
                    "status": "degraded",
                    "error": "synapse_cli_unavailable",
                    "targets": sorted(target_names),
                }
            )
        except Exception:
            missing.append(target)
            
    if missing:
        return FleetProbe.from_dict({"enabled": True, "status": "degraded", "error": "nodes_unreachable", "targets": missing})

    return FleetProbe.from_dict({
        "enabled": True,
        "status": "ok",
        "missing": missing,
        "matches": matches,
        "targets": sorted(target_names),
    })


def fleet_nodes_ready(
    lifecycle: dict,
    fleet_probe: dict,
    fleet_inventory_probe: dict,
) -> bool:
    requirements = lifecycle.get("fleet_requirements", {})
    if not requirements.get("required", False):
        return True
    if not fleet_inventory_probe:
        return False
    if fleet_inventory_probe.get("status") != "ok":
        return False
    summary = fleet_inventory_probe.get("summary", {})
    if isinstance(summary, dict):
        if summary.get("missing_nodes", 0) > 0 or summary.get("unexpected_nodes", 0) > 0:
            return False
    if requirements.get("provider") == "organ-fleet":
        return True
    if fleet_probe.get("status") != "ok":
        return False
    if fleet_probe.get("missing", []):
        return False
    if requirements.get("require_online", True):
        return all(
            peer.get("online") is True
            for peer in fleet_probe.get("matches", {}).values()
        )
    return True


def lifecycle_blockers(
    lifecycle: dict,
    build_node: dict,
    deploy_node: dict,
    *,
    activation_gap_details: list[dict],
    required_status: str | None,
    fleet_probe: dict,
    fleet_inventory_probe: dict,
    status_nodes: list[dict] | None = None,
) -> list[dict]:
    blockers = [
        {"category": "activation", **gap}
        for gap in activation_gap_details
    ]
    if required_status:
        nodes = status_nodes if status_nodes is not None else [build_node, deploy_node]
        seen_nodes = set()
        for node in nodes:
            node_name = node.get("name")
            if node_name in seen_nodes:
                continue
            seen_nodes.add(node_name)
            actual = node.get("status", "unknown")
            if actual != required_status:
                blockers.append(
                    {
                        "category": "status",
                        "kind": "node_status",
                        "node": node_name,
                        "role": node.get("role"),
                        "required_status": required_status,
                        "actual_status": actual,
                        "code": f"{node_name}:status_{actual}_not_{required_status}",
                    }
                )
    fleet_requirements = lifecycle.get("fleet_requirements", {})
    if fleet_requirements.get("required", False):
        provider = fleet_requirements.get("provider", "synapse")
        if provider == "synapse":
            fleet_status = fleet_probe.get("status", "not_run")
            if fleet_status != "ok":
                blocker = {
                    "category": "fleet",
                    "kind": "synapse_probe",
                    "provider": "synapse",
                    "required": "ok",
                    "actual": fleet_status,
                    "error": fleet_probe.get("error"),
                    "code": f"fleet_probe:{fleet_status}",
                    "resolver_organ": "organ-fleet",
                    "resolver_consumer": "organ-forge-hermes-agent",
                }
                if fleet_probe.get("error") == "synapse_cli_unavailable":
                    blocker.update(
                        {
                            "operator_action": "restore the organ-fleet synapse probe surface on the registry build runner environment",
                            "command": "python3 organs/fleet/src/entrypoints/cli.py report --json --registry infra/registry.json",
                        }
                    )
                blockers.append(blocker)
            for node_name in fleet_probe.get("missing", []):
                blockers.append(
                    {
                        "category": "fleet",
                        "kind": "synapse_peer",
                        "node": node_name,
                        "required": "present",
                        "actual": "missing",
                        "code": f"{node_name}:fleet_missing",
                    }
                )
            if fleet_requirements.get("require_online", True):
                for node_name, peer in fleet_probe.get("matches", {}).items():
                    if not peer.get("online"):
                        blockers.append(
                            {
                                "category": "fleet",
                                "kind": "synapse_peer",
                                "node": node_name,
                                "required": "online",
                                "actual": "offline",
                                "code": f"{node_name}:fleet_offline",
                            }
                        )
        inventory_status = fleet_inventory_probe.get("status", "not_run")
        if inventory_status != "ok":
            blockers.append(
                {
                    "category": "fleet",
                    "kind": "fleet_inventory",
                    "provider": "organ-fleet",
                    "required": "ok",
                    "actual": inventory_status,
                    "error": fleet_inventory_probe.get("error"),
                    "code": f"fleet_inventory:{inventory_status}",
                    "resolver_organ": "organ-fleet",
                    "resolver_consumer": "organ-forge-hermes-agent",
                    "command": "python3 organs/fleet/src/entrypoints/cli.py report --json --registry infra/registry.json",
                }
            )
        summary = fleet_inventory_probe.get("summary", {})
        if isinstance(summary, dict):
            for gap in summary.get("provider_evidence_gaps", []):
                blockers.append(
                    {
                        "category": "fleet",
                        "kind": "fleet_inventory_evidence_gap",
                        "provider": "fleet_inventory",
                        "code": str(gap),
                    }
                )
            for node_name in summary.get("declaration_missing_nodes", []):
                blockers.append(
                    {
                        "category": "fleet",
                        "kind": "fleet_inventory_node",
                        "node": str(node_name),
                        "required": "declared",
                        "actual": "missing",
                        "code": f"{node_name}:fleet_inventory_missing",
                    }
                )
            for node_name in summary.get("declaration_unexpected_nodes", []):
                blockers.append(
                    {
                        "category": "fleet",
                        "kind": "fleet_inventory_node",
                        "node": str(node_name),
                        "required": "declared",
                        "actual": "unexpected",
                        "code": f"{node_name}:fleet_inventory_unexpected",
                    }
                )
    return blockers
