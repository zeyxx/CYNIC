"""Framework mapping use case."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import ReportSurfaceSpec
from core.architecture_patterns import architecture_patterns_from_reports
from core.compatibility import compatibility_from_reports
from core.cynic_integration import cynic_integration_from_reports
from core.ports import (
    CapabilityDiscoveryPort,
    CommandMapDiscoveryPort,
    DependencyInventoryPort,
    KanbanMirrorDiscoveryPort,
    SkillBridgeDiscoveryPort,
)


FRAMEWORK_MAP_REPORT_SCHEMA = "cynic.framework_map.report.v1"

FRAMEWORK_MAP_SURFACE = ReportSurfaceSpec(
    schema=FRAMEWORK_MAP_REPORT_SCHEMA,
    command="python3 organs/framework-map/src/entrypoints/cli.py map-hermes",
    json_command="python3 organs/framework-map/src/entrypoints/cli.py map-hermes",
    usage="map-hermes [--source-root external/hermes-agent] [--binary hermes]",
)


@dataclass(frozen=True)
class FrameworkTarget:
    framework: str
    source_root: Path
    binary: str


class FrameworkMapService:
    def __init__(
        self,
        *,
        capability_discovery: CapabilityDiscoveryPort,
        dependency_inventory: DependencyInventoryPort,
        command_map_discovery: CommandMapDiscoveryPort,
        skill_bridge_discovery: SkillBridgeDiscoveryPort,
        kanban_mirror_discovery: KanbanMirrorDiscoveryPort,
        repo_root: Path,
    ) -> None:
        self.capability_discovery = capability_discovery
        self.dependency_inventory = dependency_inventory
        self.command_map_discovery = command_map_discovery
        self.skill_bridge_discovery = skill_bridge_discovery
        self.kanban_mirror_discovery = kanban_mirror_discovery
        self.repo_root = repo_root

    def map_target(self, target: FrameworkTarget) -> dict[str, Any]:
        source_root = target.source_root if target.source_root.exists() else None
        capabilities = self.capability_discovery.discover(source_root=source_root, binary=target.binary)
        dependency_roots = (target.source_root,) if target.source_root.exists() else ()
        dependencies = self.dependency_inventory.inventory(dependency_roots)
        source_inventory = self._source_inventory(target)
        command_map = self.command_map_discovery.discover(
            source_root=source_root,
            binary=target.binary,
            source_inventory=source_inventory,
            repo_root=self.repo_root,
        )
        skill_bridge = self.skill_bridge_discovery.discover(repo_root=self.repo_root)
        kanban_mirror = self.kanban_mirror_discovery.discover(
            repo_root=self.repo_root,
            command_map=command_map,
        )
        architecture = architecture_patterns_from_reports(
            framework=target.framework,
            capabilities=capabilities,
            dependencies=dependencies,
        )
        compatibility = compatibility_from_reports(
            framework=target.framework,
            capabilities=capabilities,
            dependencies=dependencies,
            architecture_patterns=architecture,
        )
        cynic_integration = cynic_integration_from_reports(
            repo_root=self.repo_root,
            framework=target.framework,
            capabilities=capabilities,
            source_inventory=source_inventory,
            command_map=command_map,
            skill_bridge=skill_bridge,
            kanban_mirror=kanban_mirror,
        )
        coverage_gaps = []
        if not target.source_root.exists():
            coverage_gaps.append({"kind": "missing_source_root", "path": str(target.source_root)})
        coverage_gaps.extend(item for item in capabilities.get("coverage_gaps", []) if isinstance(item, dict))
        coverage_gaps.extend(item for item in dependencies.get("coverage_gaps", []) if isinstance(item, dict))

        return {
            "schema": FRAMEWORK_MAP_REPORT_SCHEMA,
            "organ": "organ-framework-map",
            "framework": target.framework,
            "source_root": str(target.source_root),
            "binary": target.binary,
            "mutates": False,
            "mutates_mempool": False,
            "source_inventory": source_inventory,
            "command_map": command_map,
            "skill_bridge": skill_bridge,
            "kanban_mirror": kanban_mirror,
            "capabilities": capabilities,
            "dependencies": dependencies,
            "architecture_patterns": architecture,
            "compatibility": compatibility,
            "cynic_integration": cynic_integration,
            "coverage_gap_count": len(coverage_gaps),
            "coverage_gaps": coverage_gaps,
            "surface_contract": FRAMEWORK_MAP_SURFACE.as_manifest(),
        }

    @staticmethod
    def _source_inventory(target: FrameworkTarget) -> dict[str, Any]:
        if target.framework != "hermes-agent":
            return {
                "schema": "cynic.framework_source_inventory.v1",
                "framework": target.framework,
                "coverage_gaps": [{"kind": "unsupported_framework", "framework": target.framework}],
            }
        from adapters.hermes_source_inventory import HermesSourceInventory

        return HermesSourceInventory().scan(target.source_root)
