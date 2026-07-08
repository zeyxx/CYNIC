"""Hermes capability adapter backed by organ-contracts."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from organs.contracts.src.adapters.hermes_agent_capabilities import HermesAgentCapabilityScanner


class HermesCapabilityDiscovery:
    def discover(self, *, source_root: Path | None, binary: str) -> dict[str, Any]:
        return HermesAgentCapabilityScanner(
            binary=binary,
            source_root=source_root,
        ).scan().to_dict()
