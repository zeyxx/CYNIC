#!/usr/bin/env python3
"""Validate the CYNIC repository topology contract."""

from __future__ import annotations

import argparse
import json
from dataclasses import asdict
from pathlib import Path
import sys
from typing import Any, Callable

from organs.contracts.src.project_root import resolve_repo_root

PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

import organs.supervisor.src.core.repo_topology_documents as _docs
import organs.supervisor.src.core.repo_topology_host as _host
import organs.supervisor.src.core.repo_topology_legacy as _legacy
import organs.supervisor.src.core.repo_topology_policy as _policy
from organs.supervisor.src.core.infra_consumption_contract import validate_infra_consumption_contract


for _name in [name for name in dir(_host) if name.isupper()]:
    globals()[_name] = getattr(_host, _name)

for _name in (
    "Finding",
    "load_json",
    "load_yaml",
    "relative",
    "git_ls_files",
    "git_ls_untracked",
    "git_metadata_writable",
    "path_matches_any",
    "declared_paths",
    "validate_exception_metadata",
    "runtime_state_exception_patterns",
):
    globals()[_name] = getattr(_host, _name)

_SYNC_NAMES = {
    name
    for name in globals()
    if name.isupper()
    or name in {
        "Finding",
        "load_json",
        "load_yaml",
        "relative",
        "git_ls_files",
        "git_ls_untracked",
        "git_metadata_writable",
        "path_matches_any",
        "declared_paths",
        "validate_exception_metadata",
        "runtime_state_exception_patterns",
    }
}


def _sync_host() -> None:
    for name, value in list(globals().items()):
        if name not in _SYNC_NAMES:
            continue
        if (
            name == "git_metadata_writable"
            and getattr(value, "__module__", None) == __name__
            and getattr(value, "__name__", None) == "git_metadata_writable"
        ):
            continue
        if getattr(_host, name, object()) is not value:
            setattr(_host, name, value)


def _wrap(func: Callable[..., Any]) -> Callable[..., Any]:
    def wrapped(*args: Any, **kwargs: Any) -> Any:
        _sync_host()
        return func(*args, **kwargs)

    return wrapped


def validate_json_contracts() -> list[Finding]:
    _sync_host()
    return _host.validate_json_contracts()


def validate_schema_driven_contracts() -> list[Finding]:
    _sync_host()
    return _host.validate_schema_driven_contracts()


def validate_data_constitution_rules() -> list[Finding]:
    _sync_host()
    return _host.validate_data_constitution_rules()


def validate_draft_contract_governance() -> list[Finding]:
    _sync_host()
    return _host.validate_draft_contract_governance()


def load_toml(path) -> dict[str, Any]:
    _sync_host()
    return _host.load_toml(path)


def runtime_principals_contract_service():
    _sync_host()
    return _host.runtime_principals_contract_service()


def validate_runtime_principals_contract() -> list[Finding]:
    _sync_host()
    return _host.validate_runtime_principals_contract()


def topology_contract_service():
    _sync_host()
    return _host.topology_contract_service()


def validate_contract_file_policy(contract: dict[str, Any]) -> list[Finding]:
    _sync_host()
    return _host.validate_contract_file_policy(contract)


def validate_rust_hexagonal_crate_policy(contract: dict[str, Any]) -> list[Finding]:
    _sync_host()
    return _host.validate_rust_hexagonal_crate_policy(contract)


def validate_topology_contract() -> list[Finding]:
    _sync_host()
    return _host.validate_topology_contract()


def validate_multiscale_organism_property_map_contract() -> list[Finding]:
    _sync_host()
    return _host.validate_multiscale_organism_property_map_contract()


def validate_meta_layer_contract() -> list[Finding]:
    _sync_host()
    return _host.validate_meta_layer_contract()


def validate_api_layer_alignment_contract() -> list[Finding]:
    _sync_host()
    return _host.validate_api_layer_alignment_contract()


def validate_api_topology_contract() -> list[Finding]:
    _sync_host()
    return _host.validate_api_topology_contract()


def validate_openapi_route_contract() -> list[Finding]:
    _sync_host()
    return _host.validate_openapi_route_contract()


def validate_infrastructure_mcp_contract() -> list[Finding]:
    _sync_host()
    return _host.validate_infrastructure_mcp_contract()


def validate_lxc_exec_bridge_contract() -> list[Finding]:
    _sync_host()
    return _host.validate_lxc_exec_bridge_contract()


def validate_storage_primary_activation_contract() -> list[Finding]:
    _sync_host()
    return _host.validate_storage_primary_activation_contract()


def validate() -> list[Finding]:
    _sync_host()
    return _policy.validate()


def zero_exception_report() -> dict[str, Any]:
    _sync_host()
    return _policy.zero_exception_report()


def git_membrane_closure_service():
    _sync_host()
    return _policy.git_membrane_closure_service()


def git_membrane_closure_summary(selected_wave: int | None = None) -> dict[str, Any]:
    _sync_host()
    return _policy.git_membrane_closure_summary(selected_wave=selected_wave)


def git_matter_report() -> dict[str, Any]:
    _sync_host()
    return _policy.git_matter_report()


def git_metadata_writable() -> bool:
    _sync_host()
    return _host.git_metadata_writable()


def git_membrane_closure_commands(selected_wave: int | None = None) -> dict[str, Any]:
    _sync_host()
    return _policy.git_membrane_closure_commands(selected_wave=selected_wave)


def git_membrane_closure_preflight() -> dict[str, Any]:
    _sync_host()
    return _policy.git_membrane_closure_preflight()


def git_membrane_closure_check(selected_wave: int | None = None) -> dict[str, Any]:
    _sync_host()
    return _policy.git_membrane_closure_check(selected_wave=selected_wave)


def __getattr__(name: str) -> Any:
    exports: dict[str, Callable[..., Any]] = {
        "validate_service_surface_inventory_contract": _wrap(_host.validate_service_surface_inventory_contract),
        "validate_diagnostic_surface_inventory_contract": _wrap(_host.validate_diagnostic_surface_inventory_contract),
        "validate_implementation_surface_inventory_contract": _wrap(_host.validate_implementation_surface_inventory_contract),
        "validate_infra_surface_inventory_contract": _wrap(_host.validate_infra_surface_inventory_contract),
        "validate_data_control_surface_inventory_contract": _wrap(_host.validate_data_control_surface_inventory_contract),
        "validate_multiscale_organism_property_map_contract": _wrap(_host.validate_multiscale_organism_property_map_contract),
        "validate_meta_layer_contract": _wrap(_host.validate_meta_layer_contract),
        "validate_api_layer_alignment_contract": _wrap(_host.validate_api_layer_alignment_contract),
        "validate_api_topology_contract": _wrap(_host.validate_api_topology_contract),
        "validate_openapi_route_contract": _wrap(_host.validate_openapi_route_contract),
        "validate_infrastructure_mcp_contract": _wrap(_host.validate_infrastructure_mcp_contract),
        "validate_lxc_exec_bridge_contract": _wrap(_host.validate_lxc_exec_bridge_contract),
        "validate_storage_primary_activation_contract": _wrap(_host.validate_storage_primary_activation_contract),
        "validate_git_membrane_closure_plan_contract": _wrap(_policy.validate_git_membrane_closure_plan_contract),
        "validate_theme_topology_contract": _wrap(_docs.validate_theme_topology_contract),
        "validate_document_theme_inventory_contract": _wrap(_docs.validate_document_theme_inventory_contract),
        "document_inventory_entries": _wrap(_docs.document_inventory_entries),
        "data_emergence_capsule_roots": _wrap(_docs.data_emergence_capsule_roots),
        "validate_evidence_capsule_contract": _wrap(_docs.validate_evidence_capsule_contract),
        "validate_closed_legacy_paths": _wrap(_legacy.validate_closed_legacy_paths),
        "validate_current_transitional_paths": _wrap(_legacy.validate_current_transitional_paths),
        "transitional_wrapper_exception_paths": _wrap(_legacy.transitional_wrapper_exception_paths),
        "validate_transitional_wrapper_file_exceptions": _wrap(_legacy.validate_transitional_wrapper_file_exceptions),
        "misplaced_organ_exception_paths": _wrap(_legacy.misplaced_organ_exception_paths),
        "validate_misplaced_organ_exceptions": _wrap(_legacy.validate_misplaced_organ_exceptions),
        "iter_repo_text_files": _wrap(_legacy.iter_repo_text_files),
        "validate_no_active_legacy_hermes_x_references": _wrap(_legacy.validate_no_active_legacy_hermes_x_references),
        "validate_legacy_hermes_x_wrapper_contract": _wrap(_legacy.validate_legacy_hermes_x_wrapper_contract),
        "validate_no_infra_hermes_x_direct_refs": _wrap(_legacy.validate_no_infra_hermes_x_direct_refs),
        "validate_legacy_hermes_x_systemd_migration": _wrap(_legacy.validate_legacy_hermes_x_systemd_migration),
        "validate_runtime_state_commit_exceptions": _wrap(_legacy.validate_runtime_state_commit_exceptions),
        "validate_local_data_runtime_outputs_ignored": _wrap(_legacy.validate_local_data_runtime_outputs_ignored),
        "provider_mutation_exception_paths": _wrap(_legacy.provider_mutation_exception_paths),
        "validate_provider_mutation_script_exceptions": _wrap(_legacy.validate_provider_mutation_script_exceptions),
        "validate_no_hardcoded_deployment_projection_paths": _wrap(_legacy.validate_no_hardcoded_deployment_projection_paths),
        "validate_prohibited_script_entry_files": _wrap(_legacy.validate_prohibited_script_entry_files),
        "validate_tracked_executable_file_policy": _wrap(_legacy.validate_tracked_executable_file_policy),
        "validate_top_level_source_file_policy": _wrap(_legacy.validate_top_level_source_file_policy),
        "validate_generated_dependency_directory_policy": _wrap(_legacy.validate_generated_dependency_directory_policy),
        "validate_entrypoint_thin_membrane_policy": _wrap(_legacy.validate_entrypoint_thin_membrane_policy),
        "validate_active_document_script_entrypoint_policy": _wrap(_legacy.validate_active_document_script_entrypoint_policy),
        "allowed_organ_maturity_statuses": _wrap(_policy.allowed_organ_maturity_statuses),
        "validate_organ_maturity_policy": _wrap(_policy.validate_organ_maturity_policy),
        "validate_organ_maturity_inventory_contract": _wrap(_policy.validate_organ_maturity_inventory_contract),
        "validate_python_organ_authority_inventory_contract": _wrap(_policy.validate_python_organ_authority_inventory_contract),
        "validate_organ_boundaries": _wrap(_policy.validate_organ_boundaries),
        "validate_organ_manifest_policy": _wrap(_policy.validate_organ_manifest_policy),
        "validate_organ_manifest_state_paths": _wrap(_policy.validate_organ_manifest_state_paths),
        "manifest_script_reference_path": _wrap(_policy.manifest_script_reference_path),
        "is_git_tracked_path": _wrap(_policy.is_git_tracked_path),
        "manifest_untracked_script_status_finding": _wrap(_policy.manifest_untracked_script_status_finding),
        "validate_manifest_script_references": _wrap(_policy.validate_manifest_script_references),
        "has_python_sources": _wrap(_policy.has_python_sources),
        "validate_python_organ_shape": _wrap(_policy.validate_python_organ_shape),
        "validate_mcp_no_absolute_project_paths": _wrap(_policy.validate_mcp_no_absolute_project_paths),
    }
    try:
        return exports[name]
    except KeyError as exc:
        raise AttributeError(name) from exc


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate CYNIC repo topology contract.")
    parser.add_argument("--json", action="store_true", help="Emit machine-readable report.")
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Fail when any topology finding exists, including medium-severity drift.",
    )
    parser.add_argument("--closure-summary", action="store_true", help="Emit machine-readable Git membrane closure summary.")
    parser.add_argument("--git-matter-report", action="store_true", help="Emit read-only Git matter groups and inference batches.")
    parser.add_argument("--closure-commands", action="store_true", help="Emit non-mutating git add commands grouped by closure wave.")
    parser.add_argument("--closure-check", action="store_true", help="Check whether the selected closure wave, or all waves, has no visible untracked files.")
    parser.add_argument("--closure-next", action="store_true", help="Emit non-mutating commands for the next open Git membrane closure wave.")
    parser.add_argument("--closure-preflight", action="store_true", help="Check whether this environment can execute Git membrane closure commands.")
    parser.add_argument("--closure-wave", type=int, help="Filter closure summary or commands to one wave number.")
    parser.add_argument("--zero-exception-report", action="store_true", help="Emit remaining topology exceptions that prevent a 100%% clean organism claim.")
    parser.add_argument("--zero-exception-check", action="store_true", help="Fail until no non-canonical organs or topology exceptions remain.")
    args = parser.parse_args()

    if args.closure_summary:
        print(json.dumps(git_membrane_closure_summary(selected_wave=args.closure_wave), indent=2, sort_keys=True))
        return 0
    if args.git_matter_report:
        print(json.dumps(git_matter_report(), indent=2, sort_keys=True))
        return 0
    if args.closure_next:
        summary = git_membrane_closure_summary()
        next_wave = summary.get("next_open_wave")
        selected_wave = int(next_wave["wave"]) if next_wave else None
        print(json.dumps(git_membrane_closure_commands(selected_wave=selected_wave), indent=2, sort_keys=True))
        return 0
    if args.closure_preflight:
        preflight = git_membrane_closure_preflight()
        print(json.dumps(preflight, indent=2, sort_keys=True))
        return 0 if preflight["ok"] else 1
    if args.closure_commands:
        print(json.dumps(git_membrane_closure_commands(selected_wave=args.closure_wave), indent=2, sort_keys=True))
        return 0
    if args.closure_check:
        check = git_membrane_closure_check(selected_wave=args.closure_wave)
        print(json.dumps(check, indent=2, sort_keys=True))
        return 0 if check["ok"] else 1
    if args.zero_exception_report:
        report = zero_exception_report()
        print(json.dumps(report, indent=2, sort_keys=True))
        return 0
    if args.zero_exception_check:
        report = zero_exception_report()
        print(json.dumps(report, indent=2, sort_keys=True))
        return 0 if report["ok"] else 1

    findings = validate()
    ok = not findings if args.strict else not any(item.severity == "critical" for item in findings)
    report = {
        "schema": "cynic.repo_topology_validation.v1",
        "mode": "strict" if args.strict else "critical",
        "ok": ok,
        "findings": [asdict(item) for item in findings],
    }

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        if report["ok"]:
            print("repo topology contract ok")
        else:
            print("repo topology contract failed")
            for item in findings:
                print(f"{item.severity}: {item.kind}: {item.path}: {item.detail}")

    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
