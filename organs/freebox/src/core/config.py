"""Configuration values for Freebox integration."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class FreeboxConfig:
    """Domain configuration for Freebox API interactions."""

    freebox_url: str = "http://mafreebox.freebox.fr/api/v8"
    app_id: str = "fr.cynic.organ_freebox"
    app_name: str = "CYNIC Organ Freebox"
    app_version: str = "1.0.0"
    device_name: str = "CYNIC Kernel"
    token_file: Path = field(default_factory=lambda: Path("~/.freebox_token").expanduser())

    def authorization_payload(self) -> dict[str, str]:
        return {
            "app_id": self.app_id,
            "app_name": self.app_name,
            "app_version": self.app_version,
            "device_name": self.device_name,
        }

@dataclass(frozen=True)
class PortForwardIntent:
    """Explicit router mutation intent.

    The organ must not smuggle local network policy through hardcoded defaults.
    Callers provide the LAN target, WAN/LAN ports and approval evidence at the
    entrypoint boundary.
    """

    lan_ip: str
    wan_port_start: int
    wan_port_end: int
    lan_port: int
    protocol: str = "tcp"
    src_ip: str = "0.0.0.0"
    comment: str = "CYNIC approved edge exposure"

    def to_payload(self) -> dict[str, Any]:
        return {
            "enabled": True,
            "ip": self.lan_ip,
            "wan_port_start": self.wan_port_start,
            "wan_port_end": self.wan_port_end,
            "lan_port": self.lan_port,
            "lan_ip": self.lan_ip,
            "src_ip": self.src_ip,
            "ip_proto": self.protocol,
            "comment": self.comment,
        }


@dataclass(frozen=True)
class EdgeExposureEvidence:
    """Evidence required before a router mutation can leave the organ boundary."""

    registry_intent_id: str
    synapse_target: str

    @property
    def approved(self) -> bool:
        return bool(self.registry_intent_id.strip()) and self.synapse_target.endswith(".synapse.cynic")

    def to_dict(self) -> dict[str, str | bool]:
        return {
            "registry_intent_id": self.registry_intent_id,
            "synapse_target": self.synapse_target,
            "approved": self.approved,
        }
