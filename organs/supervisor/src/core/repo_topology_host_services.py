#!/usr/bin/env python3
"""Service factories for topology contract validation."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Callable

from organs.supervisor.src.core.runtime_principals_contract import (
    RuntimePrincipalsContractService,
)
from organs.supervisor.src.core.api_layer_alignment_contract import (
    ApiLayerAlignmentContractService,
)
from organs.supervisor.src.core.api_topology_contract import ApiTopologyContractService
from organs.supervisor.src.core.openapi_route_contract import OpenApiRouteContractService
from organs.supervisor.src.core.multiscale_organism_property_map import (
    MultiscaleOrganismPropertyMapService,
)
from organs.supervisor.src.core.meta_layer_contract import MetaLayerContractService
from organs.supervisor.src.core.infrastructure_mcp_contract import InfrastructureMcpContractService
from organs.supervisor.src.core.lxc_exec_bridge_contract import LxcExecBridgeContractService
from organs.supervisor.src.core.organ_layer_topology import OrganLayerTopologyService
from organs.supervisor.src.core.storage_primary_activation_contract import (
    StoragePrimaryActivationContractService,
)
from organs.supervisor.src.core.topology_contract import TopologyContractService

from organs.supervisor.src.core.repo_topology_host_validation import Finding


def runtime_principals_contract_service(
    *,
    workspace_root: Path,
    contract_path: Path,
    agent_policy_path: Path,
    required_fields: set[str],
    load_json: Callable[[Path], Any],
    load_toml: Callable[[Path], dict[str, Any]],
    relative: Callable[[Path], str],
    finding: type[Finding],
) -> RuntimePrincipalsContractService:
    return RuntimePrincipalsContractService(
        workspace_root=workspace_root,
        contract_path=contract_path,
        agent_policy_path=agent_policy_path,
        required_fields=required_fields,
        load_json=load_json,
        load_toml=load_toml,
        relative=relative,
        finding=finding,
    )


def validate_runtime_principals_contract(
    service: RuntimePrincipalsContractService,
) -> list[Finding]:
    return service.validate()


def topology_contract_service(
    *,
    workspace_root: Path,
    contract_path: Path,
    contracts_root: Path,
    rust_hexagonal_crate_required_fields: set[str],
    root_class_required_fields: set[str],
    load_json: Callable[[Path], Any],
    relative: Callable[[Path], str],
    git_ls_untracked: Callable[..., list[str]],
    finding: type[Finding],
) -> TopologyContractService:
    return TopologyContractService(
        workspace_root=workspace_root,
        contract_path=contract_path,
        contracts_root=contracts_root,
        rust_hexagonal_crate_required_fields=rust_hexagonal_crate_required_fields,
        root_class_required_fields=root_class_required_fields,
        load_json=load_json,
        relative=relative,
        git_ls_untracked=git_ls_untracked,
        finding=finding,
    )


def organ_layer_topology_service(
    *,
    workspace_root: Path,
    contract_path: Path,
    organs_root: Path,
    load_json: Callable[[Path], Any],
    relative: Callable[[Path], str],
    finding: type[Finding],
) -> OrganLayerTopologyService:
    return OrganLayerTopologyService(
        workspace_root=workspace_root,
        contract_path=contract_path,
        organs_root=organs_root,
        load_json=load_json,
        relative=relative,
        finding=finding,
    )


def multiscale_organism_property_map_service(
    *,
    workspace_root: Path,
    contract_path: Path,
    load_json: Callable[[Path], Any],
    relative: Callable[[Path], str],
    finding: type[Finding],
) -> MultiscaleOrganismPropertyMapService:
    return MultiscaleOrganismPropertyMapService(
        workspace_root=workspace_root,
        contract_path=contract_path,
        load_json=load_json,
        relative=relative,
        finding=finding,
    )


def meta_layer_contract_service(
    *,
    workspace_root: Path,
    contract_path: Path,
    load_json: Callable[[Path], Any],
    relative: Callable[[Path], str],
    finding: type[Finding],
) -> MetaLayerContractService:
    return MetaLayerContractService(
        workspace_root=workspace_root,
        contract_path=contract_path,
        load_json=load_json,
        relative=relative,
        finding=finding,
    )


def api_layer_alignment_contract_service(
    *,
    workspace_root: Path,
    contract_path: Path,
    topology_path: Path,
    meta_layer_path: Path,
    organ_layer_path: Path,
    load_json: Callable[[Path], Any],
    relative: Callable[[Path], str],
    finding: type[Finding],
) -> ApiLayerAlignmentContractService:
    return ApiLayerAlignmentContractService(
        workspace_root=workspace_root,
        contract_path=contract_path,
        topology_path=topology_path,
        meta_layer_path=meta_layer_path,
        organ_layer_path=organ_layer_path,
        load_json=load_json,
        relative=relative,
        finding=finding,
    )


def api_topology_contract_service(
    *,
    workspace_root: Path,
    openapi_path: Path,
    topology_path: Path,
    alignment_path: Path,
    discovery_rs_path: Path,
    middleware_rs_path: Path,
    load_json: Callable[[Path], Any],
    relative: Callable[[Path], str],
    finding: type[Finding],
) -> ApiTopologyContractService:
    return ApiTopologyContractService(
        workspace_root=workspace_root,
        openapi_path=openapi_path,
        topology_path=topology_path,
        alignment_path=alignment_path,
        discovery_rs_path=discovery_rs_path,
        middleware_rs_path=middleware_rs_path,
        load_json=load_json,
        relative=relative,
        finding=finding,
    )


def openapi_route_contract_service(
    *,
    workspace_root: Path,
    router_path: Path,
    openapi_path: Path,
    load_json: Callable[[Path], Any],
    relative: Callable[[Path], str],
    finding: type[Finding],
) -> OpenApiRouteContractService:
    return OpenApiRouteContractService(
        workspace_root=workspace_root,
        router_path=router_path,
        openapi_path=openapi_path,
        load_json=load_json,
        relative=relative,
        finding=finding,
    )


def infrastructure_mcp_contract_service(
    *,
    workspace_root: Path,
    contract_path: Path,
    load_json: Callable[[Path], Any],
    relative: Callable[[Path], str],
    finding: type[Finding],
) -> InfrastructureMcpContractService:
    return InfrastructureMcpContractService(
        workspace_root=workspace_root,
        contract_path=contract_path,
        load_json=load_json,
        relative=relative,
        finding=finding,
    )


def lxc_exec_bridge_contract_service(
    *,
    workspace_root: Path,
    contract_path: Path,
    load_json: Callable[[Path], Any],
    relative: Callable[[Path], str],
    finding: type[Finding],
) -> LxcExecBridgeContractService:
    return LxcExecBridgeContractService(
        workspace_root=workspace_root,
        contract_path=contract_path,
        load_json=load_json,
        relative=relative,
        finding=finding,
    )


def storage_primary_activation_contract_service(
    *,
    workspace_root: Path,
    contract_path: Path,
    load_json: Callable[[Path], Any],
    relative: Callable[[Path], str],
    finding: type[Finding],
) -> StoragePrimaryActivationContractService:
    return StoragePrimaryActivationContractService(
        workspace_root=workspace_root,
        contract_path=contract_path,
        load_json=load_json,
        relative=relative,
        finding=finding,
    )


def validate_contract_file_policy(
    service: TopologyContractService,
    contract: dict[str, Any],
) -> list[Finding]:
    return service.validate_contract_file_policy(contract)


def validate_rust_hexagonal_crate_policy(
    service: TopologyContractService,
    contract: dict[str, Any],
) -> list[Finding]:
    return service.validate_rust_hexagonal_crate_policy(contract)


def validate_topology_contract(service: TopologyContractService) -> list[Finding]:
    return service.validate_topology_contract()


def validate_organ_layer_topology_contract(service: OrganLayerTopologyService) -> list[Finding]:
    return service.validate()


def validate_multiscale_organism_property_map_contract(
    service: MultiscaleOrganismPropertyMapService,
) -> list[Finding]:
    return service.validate()


def validate_meta_layer_contract(service: MetaLayerContractService) -> list[Finding]:
    return service.validate()


def validate_api_layer_alignment_contract(
    service: ApiLayerAlignmentContractService,
) -> list[Finding]:
    return service.validate()


def validate_api_topology_contract(service: ApiTopologyContractService) -> list[Finding]:
    return service.validate()


def validate_openapi_route_contract(service: OpenApiRouteContractService) -> list[Finding]:
    return service.validate()


def validate_infrastructure_mcp_contract(service: InfrastructureMcpContractService) -> list[Finding]:
    return service.validate()


def validate_lxc_exec_bridge_contract(service: LxcExecBridgeContractService) -> list[Finding]:
    return service.validate()


def validate_storage_primary_activation_contract(
    service: StoragePrimaryActivationContractService,
) -> list[Finding]:
    return service.validate()


def surface_inventory_service(
    *,
    workspace_root: Path,
    services_root: Path,
    service_surface_inventory_path: Path,
    diagnostic_surface_inventory_path: Path,
    implementation_surface_inventory_path: Path,
    infra_surface_inventory_path: Path,
    data_control_surface_inventory_path: Path,
    service_surface_required_fields: set[str],
    diagnostic_surface_required_fields: set[str],
    implementation_surface_required_fields: set[str],
    infra_surface_required_fields: set[str],
    data_control_surface_required_fields: set[str],
    surface_status_transition_required_fields: set[str],
    load_json: Callable[[Path], Any],
    relative: Callable[[Path], str],
    git_ls_files: Callable[..., list[str]],
    git_ls_untracked: Callable[..., list[str]],
    finding: type[Finding],
) -> Any:
    from organs.supervisor.src.core.surface_inventories import SurfaceInventoryService

    return SurfaceInventoryService(
        workspace_root=workspace_root,
        services_root=services_root,
        service_surface_inventory_path=service_surface_inventory_path,
        diagnostic_surface_inventory_path=diagnostic_surface_inventory_path,
        implementation_surface_inventory_path=implementation_surface_inventory_path,
        infra_surface_inventory_path=infra_surface_inventory_path,
        data_control_surface_inventory_path=data_control_surface_inventory_path,
        service_surface_required_fields=service_surface_required_fields,
        diagnostic_surface_required_fields=diagnostic_surface_required_fields,
        implementation_surface_required_fields=implementation_surface_required_fields,
        infra_surface_required_fields=infra_surface_required_fields,
        data_control_surface_required_fields=data_control_surface_required_fields,
        surface_status_transition_required_fields=surface_status_transition_required_fields,
        load_json=load_json,
        relative=relative,
        git_ls_files=git_ls_files,
        git_ls_untracked=git_ls_untracked,
        finding=finding,
    )


def validate_service_surface_inventory_contract(
    service: Any,
) -> list[Finding]:
    return service.validate_service_surface_inventory_contract()


def validate_diagnostic_surface_inventory_contract(
    service: Any,
) -> list[Finding]:
    return service.validate_diagnostic_surface_inventory_contract()


def validate_implementation_surface_inventory_contract(
    service: Any,
) -> list[Finding]:
    return service.validate_implementation_surface_inventory_contract()


def validate_infra_surface_inventory_contract(service: Any) -> list[Finding]:
    return service.validate_infra_surface_inventory_contract()


def validate_data_control_surface_inventory_contract(
    service: Any,
) -> list[Finding]:
    return service.validate_data_control_surface_inventory_contract()
