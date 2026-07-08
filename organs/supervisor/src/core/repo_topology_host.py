#!/usr/bin/env python3
"""Validate the CYNIC repository topology contract."""

from __future__ import annotations

from pathlib import Path

from organs.supervisor.src.core import repo_topology_host_constants as _constants
from organs.supervisor.src.core.repo_topology_host_services import (
    api_layer_alignment_contract_service as _api_layer_alignment_contract_service,
    api_topology_contract_service as _api_topology_contract_service,
    openapi_route_contract_service as _openapi_route_contract_service,
    multiscale_organism_property_map_service as _multiscale_organism_property_map_service,
    meta_layer_contract_service as _meta_layer_contract_service,
    infrastructure_mcp_contract_service as _infrastructure_mcp_contract_service,
    lxc_exec_bridge_contract_service as _lxc_exec_bridge_contract_service,
    organ_layer_topology_service as _organ_layer_topology_service,
    storage_primary_activation_contract_service as _storage_primary_activation_contract_service,
    surface_inventory_service as _surface_inventory_service,
    topology_contract_service as _topology_contract_service,
    validate_contract_file_policy as _validate_contract_file_policy,
    validate_data_control_surface_inventory_contract as _validate_data_control_surface_inventory_contract,
    validate_diagnostic_surface_inventory_contract as _validate_diagnostic_surface_inventory_contract,
    validate_implementation_surface_inventory_contract as _validate_implementation_surface_inventory_contract,
    validate_infra_surface_inventory_contract as _validate_infra_surface_inventory_contract,
    validate_multiscale_organism_property_map_contract as _validate_multiscale_organism_property_map_contract,
    validate_meta_layer_contract as _validate_meta_layer_contract,
    validate_api_layer_alignment_contract as _validate_api_layer_alignment_contract,
    validate_api_topology_contract as _validate_api_topology_contract,
    validate_openapi_route_contract as _validate_openapi_route_contract,
    validate_infrastructure_mcp_contract as _validate_infrastructure_mcp_contract,
    validate_lxc_exec_bridge_contract as _validate_lxc_exec_bridge_contract,
    validate_organ_layer_topology_contract as _validate_organ_layer_topology_contract,
    validate_storage_primary_activation_contract as _validate_storage_primary_activation_contract,
    validate_rust_hexagonal_crate_policy as _validate_rust_hexagonal_crate_policy,
    validate_runtime_principals_contract as _validate_runtime_principals_contract,
    validate_service_surface_inventory_contract as _validate_service_surface_inventory_contract,
    validate_topology_contract as _validate_topology_contract,
    runtime_principals_contract_service as _runtime_principals_contract_service,
)
from organs.supervisor.src.core.repo_topology_host_validation import (
    Finding,
    declared_paths as _declared_paths,
    git_ls_files as _git_ls_files,
    git_ls_untracked as _git_ls_untracked,
    git_metadata_writable as _git_metadata_writable,
    is_generated_dependency_directory as _is_generated_dependency_directory,
    iter_immediate_organ_dirs as _iter_immediate_organ_dirs,
    load_json as _load_json,
    load_yaml as _load_yaml,
    path_matches_any as _path_matches_any,
    relative as _relative,
    runtime_state_exception_patterns as _runtime_state_exception_patterns,
    validate_data_constitution_rules as _validate_data_constitution_rules,
    validate_draft_contract_governance as _validate_draft_contract_governance,
    validate_exception_metadata as _validate_exception_metadata,
    validate_json_contracts as _validate_json_contracts,
    validate_schema_driven_contracts as _validate_schema_driven_contracts,
)
from organs.contracts.src.surface_contracts import read_toml_file_with_loader

for _name in dir(_constants):
    if _name.isupper():
        globals()[_name] = getattr(_constants, _name)

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    tomllib = None  # type: ignore[assignment]


def relative(path: Path) -> str:
    return _relative(path, WORKSPACE_ROOT)


def git_ls_files(*pathspecs: str) -> list[str]:
    return _git_ls_files(WORKSPACE_ROOT, *pathspecs)


def git_ls_untracked(*pathspecs: str) -> list[str]:
    return _git_ls_untracked(WORKSPACE_ROOT, *pathspecs)


def load_json(path: Path) -> object:
    return _load_json(path)


def load_yaml(path: Path) -> object:
    return _load_yaml(path)


def load_toml(path: Path) -> dict[str, object]:
    return read_toml_file_with_loader(path, tomllib)


def path_matches_any(path: str, patterns: set[str]) -> bool:
    return _path_matches_any(path, patterns)


def is_generated_dependency_directory(path: Path) -> bool:
    return _is_generated_dependency_directory(path)


def iter_immediate_organ_dirs(organs_root: Path) -> list[Path]:
    return _iter_immediate_organ_dirs(organs_root)


def validate_json_contracts() -> list[Finding]:
    return _validate_json_contracts(
        CONTRACTS_ROOT,
        relative,
        load_json_fn=load_json,
    )


def validate_schema_driven_contracts() -> list[Finding]:
    return _validate_schema_driven_contracts(
        CONTRACTS_ROOT,
        WORKSPACE_ROOT,
        relative,
        load_json_fn=load_json,
    )


def validate_data_constitution_rules() -> list[Finding]:
    return _validate_data_constitution_rules(
        workspace_root=WORKSPACE_ROOT,
        organs_root=ORGANS_ROOT,
        organ_maturity_inventory_path=ORGAN_MATURITY_INVENTORY_PATH,
        organ_manifest_schema_path=ORGAN_MANIFEST_SCHEMA_PATH,
        relative_fn=relative,
        skip_schema_validation_statuses={"quarantined", "legacy_blocked", "burn_candidate"},
        load_json_fn=load_json,
        load_yaml_fn=load_yaml,
    )


def validate_draft_contract_governance() -> list[Finding]:
    return _validate_draft_contract_governance(
        governed_contract_paths=GOVERNED_CONTRACT_PATHS,
        required_fields=DRAFT_CONTRACT_REQUIRED_FIELDS,
        relative_fn=relative,
        load_json_fn=load_json,
    )


def runtime_principals_contract_service():
    return _runtime_principals_contract_service(
        workspace_root=WORKSPACE_ROOT,
        contract_path=RUNTIME_PRINCIPALS_CONTRACT_PATH,
        agent_policy_path=AGENT_POLICY_PATH,
        required_fields=RUNTIME_PRINCIPALS_REQUIRED_FIELDS,
        load_json=load_json,
        load_toml=load_toml,
        relative=relative,
        finding=Finding,
    )


def validate_runtime_principals_contract() -> list[Finding]:
    return _validate_runtime_principals_contract(runtime_principals_contract_service())


def topology_contract_service():
    return _topology_contract_service(
        workspace_root=WORKSPACE_ROOT,
        contract_path=CONTRACT_PATH,
        contracts_root=CONTRACTS_ROOT,
        rust_hexagonal_crate_required_fields=RUST_HEXAGONAL_CRATE_REQUIRED_FIELDS,
        root_class_required_fields=ROOT_CLASS_REQUIRED_FIELDS,
        load_json=load_json,
        relative=relative,
        git_ls_untracked=git_ls_untracked,
        finding=Finding,
    )


def validate_contract_file_policy(contract: dict[str, object]) -> list[Finding]:
    return _validate_contract_file_policy(topology_contract_service(), contract)


def validate_rust_hexagonal_crate_policy(contract: dict[str, object]) -> list[Finding]:
    return _validate_rust_hexagonal_crate_policy(topology_contract_service(), contract)


def validate_topology_contract() -> list[Finding]:
    return _validate_topology_contract(topology_contract_service())


def organ_layer_topology_service():
    return _organ_layer_topology_service(
        workspace_root=WORKSPACE_ROOT,
        contract_path=ORGAN_LAYER_TOPOLOGY_PATH,
        organs_root=ORGANS_ROOT,
        load_json=load_json,
        relative=relative,
        finding=Finding,
    )


def multiscale_organism_property_map_service():
    return _multiscale_organism_property_map_service(
        workspace_root=WORKSPACE_ROOT,
        contract_path=MULTISCALE_ORGANISM_PROPERTY_MAP_PATH,
        load_json=load_json,
        relative=relative,
        finding=Finding,
    )


def meta_layer_contract_service():
    return _meta_layer_contract_service(
        workspace_root=WORKSPACE_ROOT,
        contract_path=META_LAYER_CONTRACT_PATH,
        load_json=load_json,
        relative=relative,
        finding=Finding,
    )


def api_layer_alignment_contract_service():
    return _api_layer_alignment_contract_service(
        workspace_root=WORKSPACE_ROOT,
        contract_path=API_LAYER_ALIGNMENT_CONTRACT_PATH,
        topology_path=API_TOPOLOGY_CONTRACT_PATH,
        meta_layer_path=META_LAYER_CONTRACT_PATH,
        organ_layer_path=ORGAN_LAYER_TOPOLOGY_PATH,
        load_json=load_json,
        relative=relative,
        finding=Finding,
    )


def api_topology_contract_service():
    return _api_topology_contract_service(
        workspace_root=WORKSPACE_ROOT,
        openapi_path=OPENAPI_CONTRACT_PATH,
        topology_path=API_TOPOLOGY_CONTRACT_PATH,
        alignment_path=API_LAYER_ALIGNMENT_CONTRACT_PATH,
        discovery_rs_path=API_DISCOVERY_RS_PATH,
        middleware_rs_path=API_MIDDLEWARE_RS_PATH,
        load_json=load_json,
        relative=relative,
        finding=Finding,
    )


def openapi_route_contract_service():
    return _openapi_route_contract_service(
        workspace_root=WORKSPACE_ROOT,
        router_path=KERNEL_REST_ROUTER_PATH,
        openapi_path=OPENAPI_CONTRACT_PATH,
        load_json=load_json,
        relative=relative,
        finding=Finding,
    )


def infrastructure_mcp_contract_service():
    return _infrastructure_mcp_contract_service(
        workspace_root=WORKSPACE_ROOT,
        contract_path=INFRASTRUCTURE_MCP_CONTRACT_PATH,
        load_json=load_json,
        relative=relative,
        finding=Finding,
    )


def lxc_exec_bridge_contract_service():
    return _lxc_exec_bridge_contract_service(
        workspace_root=WORKSPACE_ROOT,
        contract_path=LXC_EXEC_BRIDGE_CONTRACT_PATH,
        load_json=load_json,
        relative=relative,
        finding=Finding,
    )


def storage_primary_activation_contract_service():
    return _storage_primary_activation_contract_service(
        workspace_root=WORKSPACE_ROOT,
        contract_path=STORAGE_PRIMARY_ACTIVATION_CONTRACT_PATH,
        load_json=load_json,
        relative=relative,
        finding=Finding,
    )


def validate_organ_layer_topology_contract() -> list[Finding]:
    return _validate_organ_layer_topology_contract(organ_layer_topology_service())


def validate_multiscale_organism_property_map_contract() -> list[Finding]:
    return _validate_multiscale_organism_property_map_contract(multiscale_organism_property_map_service())


def validate_meta_layer_contract() -> list[Finding]:
    return _validate_meta_layer_contract(meta_layer_contract_service())


def validate_api_layer_alignment_contract() -> list[Finding]:
    return _validate_api_layer_alignment_contract(api_layer_alignment_contract_service())


def validate_api_topology_contract() -> list[Finding]:
    return _validate_api_topology_contract(api_topology_contract_service())


def validate_openapi_route_contract() -> list[Finding]:
    return _validate_openapi_route_contract(openapi_route_contract_service())


def validate_infrastructure_mcp_contract() -> list[Finding]:
    return _validate_infrastructure_mcp_contract(infrastructure_mcp_contract_service())


def validate_lxc_exec_bridge_contract() -> list[Finding]:
    return _validate_lxc_exec_bridge_contract(lxc_exec_bridge_contract_service())


def validate_storage_primary_activation_contract() -> list[Finding]:
    return _validate_storage_primary_activation_contract(storage_primary_activation_contract_service())


def surface_inventory_service():
    return _surface_inventory_service(
        workspace_root=WORKSPACE_ROOT,
        services_root=SERVICES_ROOT,
        service_surface_inventory_path=SERVICE_SURFACE_INVENTORY_PATH,
        diagnostic_surface_inventory_path=DIAGNOSTIC_SURFACE_INVENTORY_PATH,
        implementation_surface_inventory_path=IMPLEMENTATION_SURFACE_INVENTORY_PATH,
        infra_surface_inventory_path=INFRA_SURFACE_INVENTORY_PATH,
        data_control_surface_inventory_path=DATA_CONTROL_SURFACE_INVENTORY_PATH,
        service_surface_required_fields=SERVICE_SURFACE_REQUIRED_FIELDS,
        diagnostic_surface_required_fields=DIAGNOSTIC_SURFACE_REQUIRED_FIELDS,
        implementation_surface_required_fields=IMPLEMENTATION_SURFACE_REQUIRED_FIELDS,
        infra_surface_required_fields=INFRA_SURFACE_REQUIRED_FIELDS,
        data_control_surface_required_fields=DATA_CONTROL_SURFACE_REQUIRED_FIELDS,
        surface_status_transition_required_fields=SURFACE_STATUS_TRANSITION_REQUIRED_FIELDS,
        load_json=load_json,
        relative=relative,
        git_ls_files=git_ls_files,
        git_ls_untracked=git_ls_untracked,
        finding=Finding,
    )


def validate_service_surface_inventory_contract() -> list[Finding]:
    return _validate_service_surface_inventory_contract(surface_inventory_service())


def validate_diagnostic_surface_inventory_contract() -> list[Finding]:
    return _validate_diagnostic_surface_inventory_contract(surface_inventory_service())


def validate_implementation_surface_inventory_contract() -> list[Finding]:
    return _validate_implementation_surface_inventory_contract(surface_inventory_service())


def validate_infra_surface_inventory_contract() -> list[Finding]:
    return _validate_infra_surface_inventory_contract(surface_inventory_service())


def validate_data_control_surface_inventory_contract() -> list[Finding]:
    return _validate_data_control_surface_inventory_contract(surface_inventory_service())


def git_metadata_writable() -> bool:
    return _git_metadata_writable(WORKSPACE_ROOT)


def declared_paths(contract: dict[str, object], key: str) -> set[str]:
    return _declared_paths(contract, key)


def validate_exception_metadata(
    contract: dict[str, object],
    key: str,
    required: set[str],
    *,
    directory_paths: bool = False,
) -> list[Finding]:
    return _validate_exception_metadata(
        contract,
        key,
        required,
        directory_paths=directory_paths,
    )


def runtime_state_exception_patterns(contract: dict[str, object]) -> set[str]:
    return _runtime_state_exception_patterns(contract)
