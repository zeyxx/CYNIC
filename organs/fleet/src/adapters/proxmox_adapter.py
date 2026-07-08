from __future__ import annotations

import json
import re
from pathlib import Path

from organs.contracts.src.surface_contracts import read_json_file_or_default

from ..core.entities import ObservedResource


class ProxmoxJsonAdapter:
    def __init__(self, path: Path | None) -> None:
        self.path = path

    def resources(self) -> list[ObservedResource]:
        if self.path is None:
            return []
        payload = read_json_file_or_default(self.path, {})
        resources = []
        for raw in payload.get("resources", []):
            resources.append(
                ObservedResource(
                    provider="proxmox",
                    name=raw.get("name", ""),
                    status=raw.get("status", "unknown"),
                    resource_id=raw.get("id"),
                    kind=raw.get("kind"),
                ),
            )
        return [resource for resource in resources if resource.name]


class ProxmoxListTextAdapter:
    VM_PATTERN = re.compile(
        r"^- ID: (?P<id>\d+), Name: (?P<name>[^,]+), Status: (?P<status>[^,]+),",
    )
    NODE_PATTERN = re.compile(r"^=== Node: (?P<name>[^=]+?) ===$")

    def __init__(self, path: Path | None) -> None:
        self.path = path

    def resources(self) -> list[ObservedResource]:
        if self.path is None:
            return []
        return self.parse(self.path.read_text(encoding="utf-8"))

    def parse(self, text: str) -> list[ObservedResource]:
        resources = []
        current_kind: str | None = None

        for raw_line in text.splitlines():
            line = raw_line.strip()
            node_match = self.NODE_PATTERN.match(line)
            if node_match is not None:
                resources.append(
                    ObservedResource(
                        provider="proxmox",
                        name=node_match.group("name").strip(),
                        status="running",
                        kind="hypervisor",
                    ),
                )
                continue
            if line == "VMs:":
                current_kind = "vm"
                continue
            if line == "Containers (LXC):":
                current_kind = "lxc"
                continue
            if not line.startswith("- ID: ") or current_kind is None:
                continue

            match = self.VM_PATTERN.match(line)
            if match is None:
                continue

            numeric_id = match.group("id")
            resource_id = f"{'qemu' if current_kind == 'vm' else 'lxc'}/{numeric_id}"
            resources.append(
                ObservedResource(
                    provider="proxmox",
                    name=match.group("name"),
                    status=match.group("status"),
                    resource_id=resource_id,
                    kind=current_kind,
                ),
            )

        return resources
