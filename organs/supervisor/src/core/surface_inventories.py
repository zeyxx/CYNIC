#!/usr/bin/env python3
"""Core service for surface inventory validation."""

from __future__ import annotations

from collections.abc import Callable
from pathlib import Path
from typing import Any

from organs.supervisor.src.core.surface_inventories_common import (
    load_inventory_payload,
    validate_surface_status_transition as validate_surface_status_transition_base,
)
from organs.supervisor.src.core.surface_inventories_data_control import validate_data_control_surface_inventory_contract
from organs.supervisor.src.core.surface_inventories_diagnostic import validate_diagnostic_surface_inventory_contract
from organs.supervisor.src.core.surface_inventories_implementation import (
    validate_implementation_surface_inventory_contract,
)
from organs.supervisor.src.core.surface_inventories_infra import validate_infra_surface_inventory_contract
from organs.supervisor.src.core.surface_inventories_service import validate_service_surface_inventory_contract


class SurfaceInventoryService:
    """Validate all surface inventory contracts."""

    def __init__(
        self,
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
        finding: type,
    ) -> None:
        self.workspace_root = workspace_root
        self.services_root = services_root
        self.service_surface_inventory_path = service_surface_inventory_path
        self.diagnostic_surface_inventory_path = diagnostic_surface_inventory_path
        self.implementation_surface_inventory_path = implementation_surface_inventory_path
        self.infra_surface_inventory_path = infra_surface_inventory_path
        self.data_control_surface_inventory_path = data_control_surface_inventory_path
        self.service_surface_required_fields = service_surface_required_fields
        self.diagnostic_surface_required_fields = diagnostic_surface_required_fields
        self.implementation_surface_required_fields = implementation_surface_required_fields
        self.infra_surface_required_fields = infra_surface_required_fields
        self.data_control_surface_required_fields = data_control_surface_required_fields
        self.surface_status_transition_required_fields = surface_status_transition_required_fields
        self.load_json = load_json
        self.relative = relative
        self.git_ls_files = git_ls_files
        self.git_ls_untracked = git_ls_untracked
        self.finding = finding

    def _load_inventory_payload(
        self,
        path: Path,
        *,
        expected_schema: str,
        missing_kind: str,
        invalid_kind: str,
        missing_detail: str,
    ) -> tuple[dict[str, Any], list[Any]]:
        return load_inventory_payload(
            self,
            path=path,
            expected_schema=expected_schema,
            missing_kind=missing_kind,
            invalid_kind=invalid_kind,
            missing_detail=missing_detail,
        )

    def validate_surface_status_transition(
        self,
        *,
        inventory: dict[str, Any],
        inventory_path: Path,
        inventory_kind: str,
        item_path: str,
        status: Any,
    ) -> list[Any]:
        return validate_surface_status_transition_base(
            self,
            inventory=inventory,
            inventory_path=inventory_path,
            inventory_kind=inventory_kind,
            item_path=item_path,
            status=status,
        )

    def validate_service_surface_inventory_contract(self) -> list[Any]:
        return validate_service_surface_inventory_contract(self)

    def validate_diagnostic_surface_inventory_contract(self) -> list[Any]:
        return validate_diagnostic_surface_inventory_contract(self)

    def validate_implementation_surface_inventory_contract(self) -> list[Any]:
        return validate_implementation_surface_inventory_contract(self)

    def validate_infra_surface_inventory_contract(self) -> list[Any]:
        return validate_infra_surface_inventory_contract(self)

    def validate_data_control_surface_inventory_contract(self) -> list[Any]:
        return validate_data_control_surface_inventory_contract(self)
