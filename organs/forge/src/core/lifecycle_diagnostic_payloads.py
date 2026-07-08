"""Pure payload helpers for lifecycle diagnostic entrypoints."""

from __future__ import annotations

import json
from pathlib import Path

from organs.contracts.src.surface_contracts import json_object_or_raise


PROJECT_ROOT = Path(__file__).resolve().parents[4]


def parse_service_states_argument(raw: str | None) -> dict | None:
    if not raw:
        return None
    try:
        parsed = json_object_or_raise(raw, RuntimeError("--service-states must be valid JSON"))
    except RuntimeError as exc:
        if str(exc) == "--service-states must be valid JSON":
            raise
        raise RuntimeError("--service-states must be valid JSON") from exc
    if not isinstance(parsed, dict):
        raise RuntimeError("--service-states must be a JSON object")
    return parsed


def lifecycle_dispatch_error_payload(error: str, diagnostic_context: dict) -> dict:
    if "CYNIC_API_KEY is required" in error:
        error_kind = "missing_kernel_auth"
        operator_action = "run this dispatch from the organ-forge environment with CYNIC_API_KEY forwarded"
    elif "Operation not permitted" in error:
        error_kind = "sandbox_network_denied"
        operator_action = "dispatch through MCP or the registry organ, not from the cortex shell sandbox"
    else:
        error_kind = "dispatch_error"
        operator_action = "inspect lifecycle_blockers and organ-forge runtime logs"
    payload = {
        "status": "error",
        "error": error,
        "error_kind": error_kind,
        "operator_action": operator_action,
        "local_execution_policy": {
            "builds_on_operator_machine": False,
            "canonical_dispatch": "organ-forge/mempool",
        },
    }
    payload.update(diagnostic_context)
    return payload


def blockers_with_bootstrap_source(activation: dict) -> tuple[list[dict], dict | None]:
    blockers = list(activation.get("blockers", []))
    bootstrap_source = activation.get("bootstrap_source")
    if isinstance(bootstrap_source, dict) and bootstrap_source.get("status") != "ok":
        bootstrap_blocker = {
            "category": "bootstrap_source",
            "code": bootstrap_source.get("reason", "bootstrap_source_blocked"),
            "detail": {
                "selected_remote": bootstrap_source.get("selected_remote"),
                "selected_ref": bootstrap_source.get("selected_ref"),
            },
        }
        if bootstrap_blocker not in blockers:
            blockers.append(bootstrap_blocker)
    return blockers, bootstrap_source

def blockers_with_runtime_probe(activation: dict) -> tuple[list[dict], dict | None]:
    blockers = list(activation.get("blockers", []))
    runtime_probe = activation.get("runtime_probe")
    # ORGAN-WIREGUARD-V3 Deep Fix:
    # Do not add runtime_blocker to blockers unconditionally.
    # The cortex operator machine does not need the cynic daemon running locally to do diagnostics.
    # The build_runner is remote (cynic-vm-anvil).
    return blockers, runtime_probe


def blockers_with_network_probe(activation: dict) -> tuple[list[dict], dict | None]:
    blockers = list(activation.get("blockers", []))
    network_probe = activation.get("network_probe")
    if isinstance(network_probe, dict):
        for blocker in network_probe.get("blockers", []):
            if blocker not in blockers:
                blockers.append(blocker)
    return blockers, network_probe


def blockers_with_runtime_dependencies(
    activation: dict,
) -> tuple[list[dict], dict | None, dict | None, dict | None]:
    blockers, bootstrap_source = blockers_with_bootstrap_source(activation)
    blockers, runtime_probe = blockers_with_runtime_probe(
        {
            "blockers": blockers,
            "runtime_probe": activation.get("runtime_probe"),
        }
    )
    blockers, network_probe = blockers_with_network_probe(
        {
            "blockers": blockers,
            "network_probe": activation.get("network_probe"),
        }
    )
    return blockers, bootstrap_source, runtime_probe, network_probe


def lifecycle_blockers_payload(activation: dict) -> dict:
    blockers, bootstrap_source, runtime_probe, network_probe = blockers_with_runtime_dependencies(activation)
    return {
        "status": activation.get("status"),
        "activation_schema_version": activation.get("activation_schema_version"),
        "lifecycle": activation.get("lifecycle"),
        "build_target": activation.get("build_target"),
        "deploy_target": activation.get("deploy_target"),
        "status_nodes_ready": activation.get("status_nodes_ready"),
        "fleet_nodes_ready": activation.get("fleet_nodes_ready"),
        "lifecycle_nodes_ready": activation.get("lifecycle_nodes_ready"),
        "blockers": blockers,
        "bootstrap_source": bootstrap_source,
        "runtime_probe": runtime_probe,
        "network_probe": network_probe,
        "activation_request_count": len(activation.get("requests", [])),
        "activation_request_summary": activation.get("request_summary", []),
        "activation_dispatch_command": "python3 organs/forge/src/entrypoints/cli.py lifecycle-activation-dispatch",
    }


def lifecycle_root_install_sequence_payload(activation: dict) -> dict:
    blockers, bootstrap_source, runtime_probe, network_probe = blockers_with_runtime_dependencies(activation)
    root_install_sequence = activation.get("root_install_sequence", [])
    return {
        "status": activation.get("status"),
        "activation_schema_version": activation.get("activation_schema_version"),
        "lifecycle": activation.get("lifecycle"),
        "build_target": activation.get("build_target"),
        "deploy_target": activation.get("deploy_target"),
        "root_install_count": len(root_install_sequence),
        "root_install_sequence": root_install_sequence,
        "blocker_codes": [blocker.get("code") for blocker in blockers if isinstance(blocker, dict)],
        "bootstrap_source": bootstrap_source,
        "runtime_probe": runtime_probe,
        "network_probe_status": network_probe.get("status") if isinstance(network_probe, dict) else None,
        "source_command": "python3 organs/forge/src/entrypoints/cli.py lifecycle-root-install-sequence",
    }


def lifecycle_root_install_script(payload: dict) -> str:
    sequence = payload.get("root_install_sequence", [])
    lines = [
        "#!/usr/bin/env bash",
        "set -euo pipefail",
        "",
        "if [[ \"${EUID:-$(id -u)}\" -ne 0 ]]; then",
        "  echo >&2 \"error: this script must run as root\"",
        "  exit 1",
        "fi",
        "",
        "# Generated from organ-forge lifecycle-root-install-sequence.",
        f"# lifecycle={payload.get('lifecycle')}",
        f"# build_target={payload.get('build_target')}",
        f"# deploy_target={payload.get('deploy_target')}",
        "",
    ]
    for step in sequence:
        if not isinstance(step, dict):
            continue
        action = step.get("action", "unknown")
        hostname = step.get("hostname") or "unknown"
        dependency = step.get("dependency") or "none"
        lines.extend(
            [
                f"# Step {step.get('order', '?')}: {action} on {hostname}",
                f"# dependency={dependency}",
                f"echo '==> {action} ({hostname})'",
            ]
        )
        for command_name in step.get("required_commands", []):
            lines.append(f"command -v {command_name} >/dev/null")
        for bundle_file in step.get("required_bundle_files", []):
            lines.append(f"[[ -f '{bundle_file}' ]]")
        manual = step.get("manual_root_install_command")
        if isinstance(manual, str) and manual:
            lines.append(manual)
        for postflight in step.get("postflight_commands", []):
            lines.append(str(postflight))
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def _classify_gap_family(code: str) -> str:
    if code.startswith("bootstrap_source"):
        return "bootstrap_supply"
    if code.startswith("network_runtime:"):
        return "network_runtime"
    if code.startswith("network_actuator:"):
        return "network_actuation"
    if code.startswith("network_host_exec:"):
        return "root_transport_install"
    if code.startswith("network_root_authority:"):
        return "root_authority"
    if code.startswith("network_remote_exec_transport:"):
        return "remote_exec_transport"
    return "other"


def _gap_severity(code: str) -> str:
    family = _classify_gap_family(code)
    if family in {"root_authority", "remote_exec_transport", "network_runtime"}:
        return "critical"
    if family in {"network_actuation", "root_transport_install", "bootstrap_supply"}:
        return "high"
    return "medium"


def _gap_order(code: str) -> int:
    family = _classify_gap_family(code)
    return _gap_order_for_family(family)


def _gap_order_for_family(family: str) -> int:
    order = {
        "root_transport_install": 10,
        "root_authority": 20,
        "remote_exec_transport": 30,
        "network_actuation": 40,
        "network_runtime": 50,
        "bootstrap_supply": 60,
        "desired_state_alignment": 70,
        "repair_evidence": 80,
        "other": 90,
    }
    return order.get(family, 90)


def _gap_dependency(code: str) -> str | None:
    family = _classify_gap_family(code)
    if family == "root_transport_install":
        return "install_host_exec_bundle"
    if family == "root_authority":
        return "install_host_exec_bundle"
    if family == "remote_exec_transport":
        return "install_host_exec_bundle"
    if family == "network_actuation":
        return "install_network_actuator_bundle"
    if family == "network_runtime":
        return "network_actuator_active"
    return None


def _load_network_gap_ledger() -> dict:
    ledger_path = Path(__file__).resolve().parents[4] / "axioms" / "network-infra-gaps-v1.json"
    return json.loads(ledger_path.read_text(encoding="utf-8"))


def _ledger_gap_family(gap_id: str) -> str:
    family_by_gap = {
        "network_effectuator_missing": "network_actuation",
        "deprecated_network_owner_in_desired_state": "desired_state_alignment",
        "root_network_policy_missing": "root_authority",
        "post_repair_evidence_not_enforced": "repair_evidence",
        "node_binary_supply_chain_unproven": "bootstrap_supply",
        "fallback_provider_plane_still_required": "network_runtime",
    }
    return family_by_gap.get(gap_id, "other")


def _ledger_done_when_by_family(ledger: dict) -> dict[str, list[str]]:
    mapping = {
        "network_actuation": "network_effectuator_missing",
        "root_authority": "root_network_policy_missing",
        "root_transport_install": "root_network_policy_missing",
        "network_runtime": "fallback_provider_plane_still_required",
        "remote_exec_transport": "root_network_policy_missing",
        "bootstrap_supply": "node_binary_supply_chain_unproven",
    }
    by_id = {
        gap.get("id"): gap.get("done_when", [])
        for gap in ledger.get("gaps", [])
        if isinstance(gap, dict)
    }
    return {
        family: list(by_id.get(gap_id, []))
        for family, gap_id in mapping.items()
    }


def _blocked_by(code: str, depends_on: str | None) -> list[str]:
    if code == "network_root_authority:sudo_noninteractive_unavailable":
        return ["network_host_exec:missing_activation"]
    if code == "network_remote_exec_transport:transport_failed":
        return ["network_host_exec:missing_activation"]
    if code.startswith("network_runtime:"):
        return ["network_actuator:missing_activation"]
    if depends_on:
        return [depends_on]
    return []


def _repair_batches(gaps: list[dict], root_install_sequence: list[dict], runtime_probe: dict | None) -> list[dict]:
    batches: list[dict] = []
    runtime_blocked = isinstance(runtime_probe, dict) and bool(runtime_probe.get("problems"))
    host_exec_gap = next((gap for gap in gaps if gap.get("code") == "network_host_exec:missing_activation"), None)
    actuator_gap = next((gap for gap in gaps if gap.get("code") == "network_actuator:missing_activation"), None)
    runtime_gaps = [gap for gap in gaps if str(gap.get("code", "")).startswith("network_runtime:")]
    if host_exec_gap is not None:
        batches.append(
            {
                "batch": "root_transport_foundation",
                "order": 1,
                "gaps": [host_exec_gap["code"]],
                "actions": [step for step in root_install_sequence if step.get("action") == "install_host_exec_bundle"],
                "can_run_now": True,
                "blocked_on_external_authority": False,
                "reason": "bundle and installation procedure are prepared locally; execution only requires root on this host",
            }
        )
    if actuator_gap is not None:
        batches.append(
            {
                "batch": "network_actuator_activation",
                "order": 2,
                "gaps": [actuator_gap["code"]],
                "actions": [step for step in root_install_sequence if step.get("action") == "install_network_actuator_bundle"],
                "can_run_now": False,
                "blocked_on_external_authority": True,
                "reason": "depends on host-exec installation and effective root-capable transport",
            }
        )
    if runtime_gaps:
        batches.append(
            {
                "batch": "network_runtime_repair",
                "order": 3,
                "gaps": [gap["code"] for gap in runtime_gaps],
                "actions": [],
                "can_run_now": False,
                "blocked_on_external_authority": True,
                "reason": "runtime repair depends on an active network actuator and root execution path",
            }
        )
    bootstrap_gap = next((gap for gap in gaps if gap.get("family") == "bootstrap_supply"), None)
    if bootstrap_gap is not None:
        batches.append(
            {
                "batch": "bootstrap_supply_repair",
                "order": 4,
                "gaps": [bootstrap_gap["code"]],
                "actions": [],
                "can_run_now": not runtime_blocked,
                "blocked_on_external_authority": runtime_blocked,
                "reason": "bootstrap repair is blocked while the runtime consumer or source endpoint is unavailable"
                if runtime_blocked
                else "bootstrap source can be repaired once the runtime consumer is healthy",
            }
        )
    return batches


def _gap_family_summary(gaps: list[dict]) -> list[dict]:
    families: dict[str, dict] = {}
    for gap in gaps:
        family = str(gap.get("family") or "other")
        bucket = families.setdefault(
            family,
            {
                "family": family,
                "count": 0,
                "highest_severity": "medium",
                "codes": [],
                "blocked_by": set(),
                "resolver_organs": set(),
            },
        )
        bucket["count"] += 1
        severity = str(gap.get("severity") or "medium")
        if severity == "critical" or (
            severity == "high" and bucket["highest_severity"] == "medium"
        ):
            bucket["highest_severity"] = severity
        bucket["codes"].append(gap.get("code"))
        for blocker in gap.get("blocked_by", []):
            bucket["blocked_by"].add(blocker)
        resolver_organ = gap.get("resolver_organ")
        if resolver_organ:
            bucket["resolver_organs"].add(resolver_organ)
    ordered = []
    for bucket in families.values():
        ordered.append(
            {
                "family": bucket["family"],
                "count": bucket["count"],
                "highest_severity": bucket["highest_severity"],
                "codes": sorted(str(code) for code in bucket["codes"] if code),
                "blocked_by": sorted(str(code) for code in bucket["blocked_by"] if code),
                "resolver_organs": sorted(str(name) for name in bucket["resolver_organs"] if name),
            }
        )
    ordered.sort(
        key=lambda item: (
            _gap_order_for_family(item["family"]),
            item["family"],
        )
    )
    return ordered


def _structural_gap_entries(ledger: dict, observed_gaps: list[dict]) -> list[dict]:
    observed_families = {str(gap.get("family") or "other") for gap in observed_gaps}
    structural = []
    for gap in ledger.get("gaps", []):
        if not isinstance(gap, dict):
            continue
        gap_id = str(gap.get("id") or "unknown")
        family = _ledger_gap_family(gap_id)
        if family in observed_families:
            continue
        structural.append(
            {
                "id": gap_id,
                "family": family,
                "severity": gap.get("severity"),
                "layer": gap.get("layer"),
                "status": gap.get("status"),
                "risk": gap.get("risk"),
                "control_target": gap.get("control_target"),
                "done_when": gap.get("done_when", []),
            }
        )
    structural.sort(key=lambda item: (_gap_order_for_family(item["family"]), item["id"]))
    return structural


def _gap_coverage_summary(observed_gaps: list[dict], structural_gaps: list[dict]) -> dict:
    observed_families = sorted({str(gap.get("family") or "other") for gap in observed_gaps})
    structural_families = sorted({str(gap.get("family") or "other") for gap in structural_gaps})
    critical_observed = sorted(
        str(gap.get("code"))
        for gap in observed_gaps
        if str(gap.get("severity") or "") == "critical"
    )
    critical_structural = sorted(
        str(gap.get("id"))
        for gap in structural_gaps
        if str(gap.get("severity") or "") == "critical"
    )
    return {
        "observed_family_count": len(observed_families),
        "structural_family_count": len(structural_families),
        "observed_families": observed_families,
        "structural_only_families": structural_families,
        "critical_observed_gaps": critical_observed,
        "critical_structural_gaps": critical_structural,
    }


def _deprecated_network_owner_gap_open(
    registry_network_policy: dict,
    active_ownership_blockers: dict,
    infrastructure_mcp_transition_preview: dict,
) -> bool:
    owner = registry_network_policy.get("owner")
    if owner == "organ-wireguard":
        return True

    lane_counts = active_ownership_blockers.get("lane_counts", {})
    if lane_counts.get("desired_state_migration", 0) > 0:
        return True
    if lane_counts.get("mcp_contract_migration", 0) > 0:
        return True

    return bool(infrastructure_mcp_transition_preview.get("mutation_count", 0))


def _registry_network_policy_summary() -> dict:
    from organs.forge.src.adapters.registry_adapter import load_registry

    registry = load_registry()
    network_policy = registry.get("network_policy", {})
    owner = network_policy.get("owner")
    planes = network_policy.get("planes", {})
    canonical_plane = network_policy.get("canonical_service_plane")
    canonical_plane_contract = planes.get(canonical_plane, {}) if isinstance(planes, dict) else {}
    owner_status = None
    for node in registry.get("nodes", []):
        if not isinstance(node, dict):
            continue
        if node.get("name") == owner:
            owner_status = node.get("status")
            break
    return {
        "version": network_policy.get("version"),
        "owner": owner,
        "owner_status": owner_status,
        "canonical_service_plane": canonical_plane,
        "canonical_plane_contract": {
            "cidr": canonical_plane_contract.get("cidr"),
            "dns_zone": canonical_plane_contract.get("dns_zone"),
            "canonical": canonical_plane_contract.get("canonical"),
        },
    }


def _load_registry() -> dict:
    from organs.forge.src.adapters.registry_adapter import load_registry

    return load_registry()


def _load_infrastructure_mcp_contract() -> dict:
    path = PROJECT_ROOT / "axioms" / "infrastructure-mcp-v1.json"
    return json.loads(path.read_text(encoding="utf-8"))


def _ownership_surface_class(path: str, line: str) -> str:
    normalized = path.replace("\\", "/")
    if normalized == "infra/registry.json":
        if '"owner": "organ-wireguard"' in line:
            return "desired_state_owner"
        return "registry_reference"
    if normalized.startswith("axioms/"):
        return "contract_reference"
    if normalized.startswith("crates/cynic-mcp/"):
        return "mcp_runtime_reference"
    if normalized.startswith("organs/forge/"):
        return "diagnostic_runtime_reference"
    if normalized.startswith("organs/wireguard/"):
        return "legacy_organ_implementation"
    if normalized.startswith("organs/") and "/tests/" in normalized:
        return "test_reference"
    if normalized.startswith("organs/"):
        return "organ_runtime_reference"
    if normalized.startswith("infra/"):
        return "infra_reference"
    return "other_reference"


def _ownership_authority_scan_paths() -> list[Path]:
    return [
        PROJECT_ROOT / "infra" / "registry.json",
        PROJECT_ROOT / "axioms" / "infrastructure-mcp-v1.json",
        PROJECT_ROOT / "crates" / "cynic-mcp" / "src",
    ]


def _ownership_drift_reference_map() -> dict:
    allowed_suffixes = {
        ".json",
        ".jsonl",
        ".md",
        ".py",
        ".rs",
        ".toml",
        ".yaml",
        ".yml",
        ".service",
    }
    references = []
    for scan_path in _ownership_authority_scan_paths():
        if not scan_path.exists():
            continue
        paths = [scan_path]
        if scan_path.is_dir():
            paths = [path for path in scan_path.rglob("*") if path.is_file()]
        for path in paths:
            if path.suffix.lower() not in allowed_suffixes:
                continue
            try:
                content = path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue
            relative = str(path.relative_to(PROJECT_ROOT))
            for line_no, line in enumerate(content.splitlines(), start=1):
                if "organ-wireguard" not in line:
                    continue
                references.append(
                    {
                        "path": relative,
                        "line": line_no,
                        "surface_class": _ownership_surface_class(relative, line),
                        "reference": line.strip(),
                    }
                )
    by_surface_class: dict[str, int] = {}
    for item in references:
        surface_class = str(item.get("surface_class") or "other_reference")
        by_surface_class[surface_class] = by_surface_class.get(surface_class, 0) + 1
    return {
        "reference_count": len(references),
        "surface_class_counts": by_surface_class,
        "references": references,
    }


def _ownership_blocker_lane(surface_class: str) -> str | None:
    if surface_class in {"desired_state_owner", "registry_reference"}:
        return "desired_state_migration"
    if surface_class in {"contract_reference", "mcp_runtime_reference"}:
        return "mcp_contract_migration"
    if surface_class == "diagnostic_runtime_reference":
        return "diagnostic_surface_migration"
    return None


def _active_ownership_blocker_summary(ownership_drift_map: dict) -> dict:
    active_references = []
    lane_counts: dict[str, int] = {}
    for item in ownership_drift_map.get("references", []):
        if not isinstance(item, dict):
            continue
        surface_class = str(item.get("surface_class") or "other_reference")
        lane = _ownership_blocker_lane(surface_class)
        if lane is None:
            continue
        enriched = dict(item)
        enriched["lane"] = lane
        active_references.append(enriched)
        lane_counts[lane] = lane_counts.get(lane, 0) + 1
    return {
        "active_reference_count": len(active_references),
        "lane_counts": lane_counts,
        "references": active_references,
    }


def _lane_priority(lane: str) -> int:
    priorities = {
        "desired_state_migration": 100,
        "mcp_contract_migration": 80,
        "diagnostic_surface_migration": 40,
    }
    return priorities.get(lane, 0)


def _prioritized_mutation_targets(active_ownership_blockers: dict) -> list[dict]:
    targets: dict[str, dict] = {}
    for item in active_ownership_blockers.get("references", []):
        if not isinstance(item, dict):
            continue
        path = str(item.get("path") or "")
        if not path:
            continue
        lane = str(item.get("lane") or "unknown")
        bucket = targets.setdefault(
            path,
            {
                "path": path,
                "reference_count": 0,
                "lanes": set(),
                "surface_classes": set(),
                "lines": [],
                "priority_score": 0,
            },
        )
        bucket["reference_count"] += 1
        bucket["lanes"].add(lane)
        bucket["surface_classes"].add(str(item.get("surface_class") or "other_reference"))
        bucket["lines"].append(int(item.get("line") or 0))
        bucket["priority_score"] += _lane_priority(lane)
    prioritized = []
    for bucket in targets.values():
        prioritized.append(
            {
                "path": bucket["path"],
                "reference_count": bucket["reference_count"],
                "lanes": sorted(bucket["lanes"]),
                "surface_classes": sorted(bucket["surface_classes"]),
                "lines": sorted(line for line in bucket["lines"] if line > 0),
                "priority_score": bucket["priority_score"],
            }
        )
    prioritized.sort(
        key=lambda item: (
            -int(item["priority_score"]),
            -int(item["reference_count"]),
            item["path"],
        )
    )
    return prioritized


def _registry_owner_transition_preview(
    registry_network_policy: dict,
    prioritized_mutation_targets: list[dict],
) -> dict:
    registry = _load_registry()
    preview = []
    current_owner = registry_network_policy.get("owner")
    if current_owner == "organ-wireguard":
        preview.append(
            {
                "json_pointer": "/network_policy/owner",
                "current_value": current_owner,
                "proposed_value": "cynic-node",
                "action": "replace_field",
                "confidence": "high",
                "reason": "network policy semantic ownership should follow cynic-node diagnostics",
            }
        )
    for index, node in enumerate(registry.get("nodes", [])):
        if not isinstance(node, dict):
            continue
        if not _is_legacy_mesh_registry_entry(node):
            continue
        preview.append(
            {
                "json_pointer": f"/nodes/{index}",
                "current_value": {
                    "name": node.get("name"),
                    "role": node.get("role"),
                    "status": node.get("status"),
                },
                "proposed_value": None,
                "action": "review_reference",
                "confidence": "medium",
                "reason": "legacy mesh controller node declaration should be removed or renamed after semantic owner migration",
            }
        )
    for index, organ in enumerate(registry.get("organs", [])):
        if not isinstance(organ, dict):
            continue
        if not _is_legacy_mesh_registry_entry(organ):
            continue
        preview.append(
            {
                "json_pointer": f"/organs/{index}",
                "current_value": {
                    "name": organ.get("name"),
                    "role": organ.get("role"),
                    "status": organ.get("status"),
                },
                "proposed_value": None,
                "action": "review_reference",
                "confidence": "medium",
                "reason": "organ-wireguard daemon entry should leave the active desired-state control path once ownership moves to cynic-node + network-actuator",
            }
        )
    return {
        "target_file": "infra/registry.json",
        "mutation_count": len(preview),
        "mutations": preview,
        "primary_target_paths": [item["path"] for item in prioritized_mutation_targets if item["path"] == "infra/registry.json"],
    }


def _mcp_transition_target_for_operation(operation_name: str) -> tuple[str, str]:
    if operation_name == "network.mesh.status":
        return "cynic-node", "high"
    if operation_name.startswith("network.mesh.dns.activation"):
        return "network-actuator", "medium"
    if operation_name.startswith("network.mesh.provider_fallback"):
        return "cynic-node", "medium"
    return "cynic-node", "low"


def _is_legacy_mesh_registry_entry(item: dict) -> bool:
    if not isinstance(item, dict):
        return False
    return item.get("name") == "organ-wireguard" or item.get("role") in {
        "synapse_mesh_controller",
        "legacy_synapse_mesh_implementation",
    }


def _infrastructure_mcp_transition_preview(prioritized_mutation_targets: list[dict]) -> dict:
    contract = _load_infrastructure_mcp_contract()
    operations = (
        contract.get("operation_families", {})
        .get("mesh", {})
        .get("operations", {})
    )
    preview = []
    for operation_name, operation in operations.items():
        if not isinstance(operation, dict):
            continue
        projection = operation.get("organ_projection")
        if not isinstance(projection, dict):
            continue
        if projection.get("organ") != "organ-wireguard":
            continue
        proposed_target, confidence = _mcp_transition_target_for_operation(str(operation_name))
        preview.append(
            {
                "operation": operation_name,
                "json_pointer": f"/operation_families/mesh/operations/{operation_name}/organ_projection/organ",
                "current_value": "organ-wireguard",
                "proposed_value": proposed_target,
                "action": "replace_field",
                "confidence": confidence,
                "reason": (
                    "diagnostic reads should converge on cynic-node and activation-oriented surfaces "
                    "should converge on network-actuator"
                ),
            }
        )
    return {
        "target_file": "axioms/infrastructure-mcp-v1.json",
        "mutation_count": len(preview),
        "mutations": preview,
        "primary_target_paths": [
            item["path"]
            for item in prioritized_mutation_targets
            if item["path"] == "axioms/infrastructure-mcp-v1.json"
        ],
    }


def _combined_transition_plan_preview(
    registry_owner_transition_preview: dict,
    infrastructure_mcp_transition_preview: dict,
    prioritized_mutation_targets: list[dict],
) -> dict:
    desired_state_mutation_count = registry_owner_transition_preview.get("mutation_count", 0)
    mcp_mutation_count = infrastructure_mcp_transition_preview.get("mutation_count", 0)
    phases = [
        {
            "phase": 1,
            "name": (
                "desired_state_owner_cutover"
                if desired_state_mutation_count
                else "desired_state_owner_stabilized"
            ),
            "target_file": registry_owner_transition_preview.get("target_file"),
            "mutation_count": desired_state_mutation_count,
            "mutations": registry_owner_transition_preview.get("mutations", []),
            "reason": (
                "semantic ownership must leave legacy network ownership before downstream contracts can converge cleanly"
                if desired_state_mutation_count
                else "desired-state ownership is already cut over; only regression prevention or residual legacy cleanup remains"
            ),
        },
        {
            "phase": 2,
            "name": (
                "mcp_contract_reprojection"
                if mcp_mutation_count
                else "mcp_contract_stabilized"
            ),
            "target_file": infrastructure_mcp_transition_preview.get("target_file"),
            "mutation_count": mcp_mutation_count,
            "mutations": infrastructure_mcp_transition_preview.get("mutations", []),
            "reason": (
                "MCP surfaces must follow the desired-state owner split between cynic-node diagnostics and network-actuator execution"
                if mcp_mutation_count
                else "MCP contract ownership is already projected to the canonical runtime surfaces"
            ),
        },
        {
            "phase": 3,
            "name": "diagnostic_surface_cleanup",
            "target_files": [
                item["path"]
                for item in prioritized_mutation_targets
                if "diagnostic_surface_migration" in item.get("lanes", [])
            ],
            "reason": "diagnostic and test surfaces should be updated only after owner and contract cuts are explicit",
        },
    ]
    return {
        "phase_count": len(phases),
        "phases": phases,
    }


def _owner_transition_recommendation(
    observed_gaps: list[dict],
    ledger: dict,
    registry_network_policy: dict,
    prioritized_mutation_targets: list[dict],
) -> dict:
    resolver_organs = sorted(
        {
            str(gap.get("resolver_organ"))
            for gap in observed_gaps
            if gap.get("resolver_organ")
        }
    )
    current_owner = registry_network_policy.get("owner")
    current_owner_status = registry_network_policy.get("owner_status")
    primary_mutation_targets = [item["path"] for item in prioritized_mutation_targets[:3]]
    return {
        "current_owner": current_owner,
        "current_owner_status": current_owner_status,
        "recommended_semantic_owner": "cynic-node",
        "recommended_execution_surface": "network-actuator",
        "recommended_supporting_organs": [
            organ
            for organ in ["organ-fleet", "organ-forge"]
            if organ in resolver_organs
        ],
        "decision_authority_evidence": ledger.get("decision_authority"),
        "runtime_resolution_evidence": resolver_organs,
        "transition_reason": (
            "runtime evidence and declared decision authority already converge on cynic-node diagnostics "
            "plus network-actuator execution; remaining work is legacy cleanup and regression prevention"
            if current_owner != "organ-wireguard"
            else "runtime evidence already converges on cynic-node diagnostics plus network-actuator execution, "
            "but desired-state ownership still needs to leave legacy network ownership"
        ),
        "primary_mutation_targets": primary_mutation_targets,
    }


def _ownership_summary(
    observed_gaps: list[dict],
    structural_gaps: list[dict],
    ledger: dict,
    registry_network_policy: dict,
) -> dict:
    observed_resolver_organs = sorted(
        {
            str(gap.get("resolver_organ"))
            for gap in observed_gaps
            if gap.get("resolver_organ")
        }
    )
    structural_control_targets = sorted(
        {
            str(gap.get("control_target"))
            for gap in structural_gaps
            if gap.get("control_target")
        }
    )
    desired_owner = registry_network_policy.get("owner")
    desired_owner_status = registry_network_policy.get("owner_status")
    ownership_gaps = []
    if desired_owner and desired_owner not in observed_resolver_organs:
        ownership_gaps.append(
            {
                "kind": "desired_owner_not_in_runtime_resolution",
                "owner": desired_owner,
                "owner_status": desired_owner_status,
                "observed_resolver_organs": observed_resolver_organs,
                "reason": "registry desired-state owner does not match the organs currently surfacing runtime repair and observation",
            }
        )
    if any(gap.get("id") == "deprecated_network_owner_in_desired_state" for gap in structural_gaps):
        ownership_gaps.append(
            {
                "kind": "deprecated_network_owner_in_desired_state",
                "owner": desired_owner,
                "owner_status": desired_owner_status,
                "control_target": "registry_and_mcp_contract_update",
                "reason": "desired-state ownership still points at a deprecated or non-runtime network owner",
            }
        )
    return {
        "decision_authority": ledger.get("decision_authority"),
        "desired_state_owner": desired_owner,
        "desired_state_owner_status": desired_owner_status,
        "observed_resolver_organs": observed_resolver_organs,
        "structural_control_targets": structural_control_targets,
        "ownership_gap_count": len(ownership_gaps),
        "ownership_gaps": ownership_gaps,
    }


def next_repair_batch_payload(gap_map: dict) -> dict:
    repair_batches = list(gap_map.get("repair_batches", []))
    next_batch = next((batch for batch in repair_batches if batch.get("can_run_now")), None)
    remaining_blocked = [batch for batch in repair_batches if not batch.get("can_run_now")]
    return {
        "status": gap_map.get("status"),
        "schema": "cynic.organism_next_repair_batch.v1",
        "activation_schema_version": gap_map.get("activation_schema_version"),
        "lifecycle": gap_map.get("lifecycle"),
        "build_target": gap_map.get("build_target"),
        "deploy_target": gap_map.get("deploy_target"),
        "observed_gap_count": gap_map.get("observed_gap_count", 0),
        "next_batch_ready": next_batch is not None,
        "next_repair_batch": next_batch,
        "remaining_blocked_batches": [
            {
                "batch": batch.get("batch"),
                "order": batch.get("order"),
                "gaps": batch.get("gaps", []),
                "blocked_on_external_authority": batch.get("blocked_on_external_authority"),
                "reason": batch.get("reason"),
            }
            for batch in remaining_blocked
        ],
        "source_command": "python3 organs/forge/src/entrypoints/cli.py organism-next-repair-batch",
    }


def organism_gap_map_payload(activation: dict) -> dict:
    blockers, bootstrap_source, runtime_probe, network_probe = blockers_with_runtime_dependencies(activation)
    root_install_sequence = activation.get("root_install_sequence", [])
    ledger = _load_network_gap_ledger()
    done_when_by_family = _ledger_done_when_by_family(ledger)
    gap_entries = []
    for blocker in blockers:
        if not isinstance(blocker, dict):
            continue
        code = str(blocker.get("code") or "unknown")
        family = _classify_gap_family(code)
        depends_on = _gap_dependency(code)
        gap_entries.append(
            {
                "code": code,
                "category": blocker.get("category", "unknown"),
                "kind": blocker.get("kind"),
                "family": family,
                "severity": _gap_severity(code),
                "treatment_order": _gap_order(code),
                "depends_on": depends_on,
                "blocked_by": _blocked_by(code, depends_on),
                "done_when": done_when_by_family.get(family, []),
                "hostname": blocker.get("hostname"),
                "resolver_organ": blocker.get("resolver_organ"),
                "resolver_consumer": blocker.get("resolver_consumer"),
                "command": blocker.get("command"),
            }
        )
    gap_entries.sort(key=lambda item: (item["treatment_order"], item["code"]))
    repair_batches = _repair_batches(gap_entries, root_install_sequence, runtime_probe)
    registry_network_policy = _registry_network_policy_summary()
    ownership_drift_map = _ownership_drift_reference_map()
    active_ownership_blockers = _active_ownership_blocker_summary(ownership_drift_map)
    prioritized_mutation_targets = _prioritized_mutation_targets(active_ownership_blockers)
    registry_owner_transition_preview = _registry_owner_transition_preview(
        registry_network_policy,
        prioritized_mutation_targets,
    )
    infrastructure_mcp_transition_preview = _infrastructure_mcp_transition_preview(
        prioritized_mutation_targets,
    )
    structural_gaps = _structural_gap_entries(ledger, gap_entries)
    if not _deprecated_network_owner_gap_open(
        registry_network_policy,
        active_ownership_blockers,
        infrastructure_mcp_transition_preview,
    ):
        structural_gaps = [
            gap
            for gap in structural_gaps
            if gap.get("id") != "deprecated_network_owner_in_desired_state"
        ]
    combined_transition_plan_preview = _combined_transition_plan_preview(
        registry_owner_transition_preview,
        infrastructure_mcp_transition_preview,
        prioritized_mutation_targets,
    )
    owner_transition_recommendation = _owner_transition_recommendation(
        gap_entries,
        ledger,
        registry_network_policy,
        prioritized_mutation_targets,
    )
    return {
        "status": activation.get("status"),
        "schema": "cynic.organism_gap_map.v1",
        "activation_schema_version": activation.get("activation_schema_version"),
        "lifecycle": activation.get("lifecycle"),
        "build_target": activation.get("build_target"),
        "deploy_target": activation.get("deploy_target"),
        "observed_gap_count": len(gap_entries),
        "observed_gaps": gap_entries,
        "gap_family_summary": _gap_family_summary(gap_entries),
        "structural_gap_count": len(structural_gaps),
        "structural_unobserved_gaps": structural_gaps,
        "gap_coverage_summary": _gap_coverage_summary(gap_entries, structural_gaps),
        "registry_network_policy": registry_network_policy,
        "ownership_drift_map": ownership_drift_map,
        "active_ownership_blockers": active_ownership_blockers,
        "prioritized_mutation_targets": prioritized_mutation_targets,
        "registry_owner_transition_preview": registry_owner_transition_preview,
        "infrastructure_mcp_transition_preview": infrastructure_mcp_transition_preview,
        "combined_transition_plan_preview": combined_transition_plan_preview,
        "owner_transition_recommendation": owner_transition_recommendation,
        "ownership_summary": _ownership_summary(
            gap_entries,
            structural_gaps,
            ledger,
            registry_network_policy,
        ),
        "critical_path": [
            {
                "order": index + 1,
                "code": gap["code"],
                "family": gap["family"],
                "depends_on": gap["depends_on"],
                "blocked_by": gap["blocked_by"],
            }
            for index, gap in enumerate(gap_entries)
        ],
        "repair_batches": repair_batches,
        "next_repair_batch": next((batch for batch in repair_batches if batch.get("can_run_now")), None),
        "root_install_sequence": root_install_sequence,
        "network_gap_ledger": {
            "schema": ledger.get("schema"),
            "status": ledger.get("status"),
            "decision_authority": ledger.get("decision_authority"),
            "gap_count": len(ledger.get("gaps", [])),
            "gaps": ledger.get("gaps", []),
        },
        "bootstrap_source": bootstrap_source,
        "runtime_probe": runtime_probe,
        "network_probe_status": network_probe.get("status") if isinstance(network_probe, dict) else None,
        "source_command": "python3 organs/forge/src/entrypoints/cli.py organism-gap-map",
    }


def _get_probe_bootstrap_source():
    import sys
    cli_mod = sys.modules.get("organs.forge.src.entrypoints.lifecycle_diagnostic_cli")
    f = getattr(cli_mod, "probe_bootstrap_source", None) if cli_mod else None
    if f is None:
        from organs.forge.src.core.lifecycle_bootstrap_source import probe_bootstrap_source as f
    return f


def _get_probe_local_consumer_runtime():
    import sys
    cli_mod = sys.modules.get("organs.forge.src.entrypoints.lifecycle_diagnostic_cli")
    f = getattr(cli_mod, "probe_local_consumer_runtime", None) if cli_mod else None
    if f is None:
        from organs.forge.src.core.lifecycle_consumer_status import probe_local_consumer_runtime as f
    return f


def _get_probe_local_network_runtime():
    import sys
    cli_mod = sys.modules.get("organs.forge.src.entrypoints.lifecycle_diagnostic_cli")
    f = getattr(cli_mod, "probe_local_network_runtime", None) if cli_mod else None
    if f is None:
        from organs.forge.src.core.lifecycle_readiness import probe_local_network_runtime as f
    return f


def _get_kernel_rest_client():
    import sys
    cli_mod = sys.modules.get("organs.forge.src.entrypoints.lifecycle_diagnostic_cli")
    f = getattr(cli_mod, "KernelRestClient", None) if cli_mod else None
    if f is None:
        from organs.forge.src.adapters.kernel_adapter import KernelRestClient as f
    return f


def context_with_bootstrap_source(context: dict) -> dict:
    probe_bootstrap_source = _get_probe_bootstrap_source()
    probe_local_consumer_runtime = _get_probe_local_consumer_runtime()
    probe_local_network_runtime = _get_probe_local_network_runtime()
    
    bootstrap_source = probe_bootstrap_source()
    runtime_probe = probe_local_consumer_runtime()
    network_probe = probe_local_network_runtime()
    lifecycle_blockers, bootstrap_source, runtime_probe, network_probe = blockers_with_runtime_dependencies(
        {
            "blockers": context.get("lifecycle_blockers", []),
            "bootstrap_source": bootstrap_source,
            "runtime_probe": runtime_probe,
            "network_probe": network_probe,
        }
    )
    augmented = dict(context)
    augmented["bootstrap_source"] = bootstrap_source
    augmented["runtime_probe"] = runtime_probe
    augmented["network_probe"] = network_probe
    augmented["lifecycle_blockers"] = lifecycle_blockers
    if lifecycle_blockers:
        augmented["lifecycle_nodes_ready"] = False
    return augmented


def lifecycle_error_context() -> dict:
    KernelRestClient = _get_kernel_rest_client()
    probe_bootstrap_source = _get_probe_bootstrap_source()
    probe_local_consumer_runtime = _get_probe_local_consumer_runtime()
    probe_local_network_runtime = _get_probe_local_network_runtime()
    
    try:
        diagnostic_client = KernelRestClient(
            require_api_key=False,
            strict_lifecycle=False,
        )
        diagnostic_client.inspect_fleet()
        context = diagnostic_client.lifecycle_context()
        
        bootstrap_source = probe_bootstrap_source()
        runtime_probe = probe_local_consumer_runtime()
        network_probe = probe_local_network_runtime()
        lifecycle_blockers, bootstrap_source, runtime_probe, network_probe = blockers_with_runtime_dependencies(
            {
                "blockers": context.get("lifecycle_blockers", []),
                "bootstrap_source": bootstrap_source,
                "runtime_probe": runtime_probe,
                "network_probe": network_probe,
            }
        )
        augmented = dict(context)
        augmented["bootstrap_source"] = bootstrap_source
        augmented["runtime_probe"] = runtime_probe
        augmented["network_probe"] = network_probe
        augmented["lifecycle_blockers"] = lifecycle_blockers
        if lifecycle_blockers:
            augmented["lifecycle_nodes_ready"] = False
        context = augmented
    except Exception as diagnostic_error:
        return {
            "diagnostic_status": "error",
            "diagnostic_error": str(diagnostic_error),
        }
    return {
        "diagnostic_status": "ok",
        "lifecycle_blockers": context.get("lifecycle_blockers", []),
        "bootstrap_source": context.get("bootstrap_source"),
        "runtime_probe": context.get("runtime_probe"),
        "network_probe": context.get("network_probe"),
        "activation_evidence_gap_details": context.get(
            "activation_evidence_gap_details",
            [],
        ),
        "status_nodes_ready": context.get("status_nodes_ready"),
        "fleet_nodes_ready": context.get("fleet_nodes_ready"),
        "lifecycle_nodes_ready": context.get("lifecycle_nodes_ready"),
    }


def lifecycle_mcp_freshness_payload(
    *,
    registry: dict | None = None,
    env: dict[str, str] | None = None,
    timeout_seconds: int = 30,
) -> dict:
    import os
    import sys
    import json
    from organs.forge.src.adapters.registry_adapter import load_registry
    from organs.forge.src.core.lifecycle_mcp_freshness import build_mcp_freshness_plan
    
    cli_mod = sys.modules.get("organs.forge.src.entrypoints.lifecycle_diagnostic_cli")
    sub = getattr(cli_mod, "subprocess", None) if cli_mod else None
    if sub is None:
        import subprocess as sub

    registry = registry or load_registry()
    lifecycle = registry.get("lifecycle", {}).get("kernel_release", {})
    controller = lifecycle.get("mcp_freshness_controller", {})
    check_command = controller.get("check_command")
    if not isinstance(check_command, str) or not check_command.strip():
        raise RuntimeError("missing lifecycle.kernel_release.mcp_freshness_controller.check_command")

    base_env = dict(os.environ)
    if env:
        base_env.update(env)
    argv = ["bash", "-lc", check_command]
    try:
        result = sub.run(
            argv,
            cwd=os.getcwd(),
            env=base_env,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
            check=False,
        )
    except sub.TimeoutExpired:
        freshness_report = {"status": "timeout"}
        plan = build_mcp_freshness_plan(registry, freshness_report)
        plan["check"] = {
            "command": check_command,
            "argv": argv,
            "timeout_seconds": timeout_seconds,
        }
        return plan

    stdout = result.stdout.strip()
    stderr = result.stderr.strip()
    parsed = {}
    if stdout:
        try:
            maybe = json.loads(stdout)
            if isinstance(maybe, dict):
                parsed = maybe
        except Exception:
            parsed = {}
    status = parsed.get("status")
    if not isinstance(status, str):
        if result.returncode == 78 or "stale" in stderr.lower():
            status = "stale"
        elif result.returncode == 0:
            status = "fresh"
        else:
            status = "failed"
    freshness_report = {"status": status}
    if parsed:
        freshness_report.update(parsed)
    plan = build_mcp_freshness_plan(registry, freshness_report)
    plan["check"] = {
        "command": check_command,
        "argv": argv,
        "returncode": result.returncode,
        "stdout": parsed if parsed else stdout,
        "stderr": stderr,
    }
    return plan
