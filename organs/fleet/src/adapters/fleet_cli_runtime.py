"""Runtime helpers for the organ-fleet CLI."""

from __future__ import annotations

from dataclasses import asdict
from pathlib import Path

from organs.fleet.src.adapters.proxmox_adapter import ProxmoxJsonAdapter, ProxmoxListTextAdapter
from organs.fleet.src.adapters.registry_adapter import RegistryAdapter
from organs.fleet.src.adapters.watchdog_adapter import WatchdogObservationAdapter
from organs.fleet.src.core.service import FleetTruthService


def organ_dirs(root: Path) -> set[str]:
    organs_path = root / "organs"
    support_dirs = {"controllers", "state"}
    return {
        child.name
        for child in organs_path.iterdir()
        if child.is_dir() and not child.name.startswith(".") and child.name not in support_dirs
    }


def build_report_payload(
    *,
    root: Path,
    registry_path: Path,
    proxmox_json_path: Path | None,
    proxmox_text_path: Path | None,
    watchdog_json_path: Path | None = None,
) -> dict:
    registry = RegistryAdapter(registry_path)
    observed = [
        *registry.observed_provider_resources(),
        *WatchdogObservationAdapter(watchdog_json_path).resources(),
        *ProxmoxJsonAdapter(proxmox_json_path).resources(),
        *ProxmoxListTextAdapter(proxmox_text_path).resources(),
    ]
    report = FleetTruthService().build_report(
        declared_nodes=registry.nodes(),
        observed_resources=observed,
        registry_organs=registry.organs(),
        organ_dirs=organ_dirs(root),
    )
    return asdict(report)
