from __future__ import annotations

import os
from pathlib import Path

from ..adapters.kernel_observation_adapter import KernelObservationAdapter
from ..adapters.registry_supply_chain_adapter import RegistrySupplyChainAdapter
from ..core.runtime import LifecycleRuntime
from ..core.ports import ObservationPort


def _registry_path() -> Path | None:
    project_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if project_root:
        return Path(project_root).expanduser().resolve() / "infra" / "registry.json"
    return None


def build_lifecycle_runtime() -> LifecycleRuntime:
    registry_path = _registry_path()
    analyzer = RegistrySupplyChainAdapter(registry_path=registry_path)

    kernel_addr = os.environ.get("CYNIC_REST_ADDR")
    api_key = os.environ.get("CYNIC_API_KEY")
    observation_port: ObservationPort | None = None
    if kernel_addr:
        observation_port = KernelObservationAdapter(kernel_addr, api_key=api_key)

    return LifecycleRuntime(
        analyzer=analyzer,
        observation_port=observation_port,
    )
