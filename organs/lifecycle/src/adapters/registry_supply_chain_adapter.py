from __future__ import annotations

from pathlib import Path
from typing import Any, Dict

from ..core.ports import SupplyChainAnalysisPort
from ..core.supply_chain_analysis import RegistrySupplyChainAnalyzer
from .registry_adapter import RegistryFileAdapter


class RegistrySupplyChainAdapter(SupplyChainAnalysisPort):
    """Concrete supply-chain analyzer backed by the lifecycle registry file."""

    def __init__(self, registry_path: str | Path | None = None, registry_data: dict | None = None):
        self.registry_port = RegistryFileAdapter(registry_path)
        self.registry_data = registry_data

    def analyze_supply_chain(self) -> Dict[str, Any]:
        if self.registry_data is not None:
            class _InMemoryRegistry:
                def __init__(self, data: dict):
                    self.data = data

                def load_registry(self) -> dict:
                    return self.data

            registry_port = _InMemoryRegistry(self.registry_data)
        else:
            registry_port = self.registry_port
        return RegistrySupplyChainAnalyzer(registry_port).analyze_supply_chain()
