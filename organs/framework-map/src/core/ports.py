"""Hexagonal ports for framework mapping."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Protocol


class CapabilityDiscoveryPort(Protocol):
    def discover(self, *, source_root: Path | None, binary: str) -> dict[str, Any]:
        """Return cynic.agent_framework_capabilities.v1 payload."""


class DependencyInventoryPort(Protocol):
    def inventory(self, roots: tuple[Path, ...]) -> dict[str, Any]:
        """Return cynic.external_dependency_inventory.v1 payload."""


class CommandMapDiscoveryPort(Protocol):
    def discover(
        self,
        *,
        source_root: Path | None,
        binary: str,
        source_inventory: dict[str, Any],
        repo_root: Path,
    ) -> dict[str, Any]:
        """Return cynic.hermes_agent_command_map.v1 payload."""


class SkillBridgeDiscoveryPort(Protocol):
    def discover(self, *, repo_root: Path, hermes_skill_root: Path | None = None) -> dict[str, Any]:
        """Return cynic.hermes_skill_bridge.v1 payload."""


class KanbanMirrorDiscoveryPort(Protocol):
    def discover(self, *, repo_root: Path, command_map: dict[str, Any] | None = None) -> dict[str, Any]:
        """Return cynic.hermes_kanban_kernel_mirror.v1 payload."""
