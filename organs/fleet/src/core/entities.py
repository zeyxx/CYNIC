from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class Node:
    name: str
    role: str
    status: str
    constellation: str | None = None
    aliases: tuple[str, ...] = ()
    host: str | None = None
    proxmox_id: str | None = None
    energy_profile: dict[str, Any] = field(default_factory=dict)

    @property
    def identities(self) -> set[str]:
        values = {self.name, *self.aliases}
        if self.proxmox_id:
            values.add(self.proxmox_id)
        return {value for value in values if value}


@dataclass(frozen=True)
class ObservedResource:
    provider: str
    name: str
    status: str
    resource_id: str | None = None
    kind: str | None = None
    constellation: str | None = None

    @property
    def identities(self) -> set[str]:
        values = {self.name}
        if self.resource_id:
            values.add(self.resource_id)
        return {value for value in values if value}


@dataclass(frozen=True)
class Anomaly:
    severity: str
    kind: str
    target: str
    message: str


@dataclass(frozen=True)
class FleetReport:
    summary: dict[str, Any]
    desired: list[dict[str, Any]]
    declared: list[dict[str, Any]]
    observed: list[dict[str, Any]]
    effective: list[dict[str, Any]]
    anomalies: list[dict[str, Any]]
    energy: dict[str, Any]
