from __future__ import annotations

from pathlib import Path
from typing import Any, Dict

from ..core.ports import InfraRegistryPort
from .kernel_adapter_policy import load_registry


class RegistryFileAdapter(InfraRegistryPort):
    """Filesystem-backed registry adapter for lifecycle and related organs."""

    def __init__(self, registry_path: str | Path | None = None):
        self.registry_path = Path(registry_path).expanduser() if registry_path else None

    def load_registry(self) -> Dict[str, Any]:
        return load_registry(self.registry_path) if self.registry_path else load_registry()

    def resolve_node(self, role: str) -> Dict[str, Any] | None:
        registry = self.load_registry()
        nodes = registry.get("nodes", [])
        if isinstance(nodes, dict):
            nodes = list(nodes.values())
        if not isinstance(nodes, list):
            return None
        for node in nodes:
            if node.get("role") == role:
                return node
        return None

    def resolve_service(self, service_name: str) -> Dict[str, Any] | None:
        registry = self.load_registry()
        for collection_name in ("services", "organs", "nodes"):
            items = registry.get(collection_name, [])
            if isinstance(items, dict):
                items = list(items.values())
            if not isinstance(items, list):
                continue
            for item in items:
                if item.get("name") == service_name or item.get("service_id") == service_name:
                    return item
        return None
