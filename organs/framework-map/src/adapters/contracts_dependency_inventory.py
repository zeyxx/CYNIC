"""Dependency inventory adapter backed by organ-contracts."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from organs.contracts.src.adapters.dependency_inventory import DependencyInventoryScanner
from organs.contracts.src.core.agent_framework_capabilities import utc_now_iso
from organs.contracts.src.core.external_dependency_inventory import ExternalDependencyInventory


class ContractsDependencyInventory:
    def inventory(self, roots: tuple[Path, ...]) -> dict[str, Any]:
        if not roots:
            return ExternalDependencyInventory(
                observed_at=utc_now_iso(),
                scan_roots=(),
                dependencies=(),
                manifests=(),
                coverage_gaps=({"kind": "missing_source_root", "reason": "no dependency roots were provided"},),
            ).to_dict()
        return DependencyInventoryScanner(roots).scan().to_dict()
