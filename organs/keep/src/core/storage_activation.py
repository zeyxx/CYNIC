from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from organs.keep.src.adapters.bundle_filesystem import BundleFilesystemAdapter
from organs.contracts.src.surface_contracts import ReportSurfaceSpec, SurfaceRegistry, attach_surface_contract


@dataclass(frozen=True)
class StoragePrimarySpec:
    node: str
    service: str
    endpoint: str
    synapse_ip: str
    port: int


STORAGE_PRIMARY_PLAN_SURFACE = ReportSurfaceSpec(
    schema="cynic.storage_primary_activation.bundle.v1",
    command="cynic storage plan",
)
STORAGE_PRIMARY_STATUS_SURFACE = ReportSurfaceSpec(
    schema="cynic.storage_primary_activation.status.v1",
    command="cynic storage status",
)
STORAGE_PRIMARY_SURFACE_REGISTRY = SurfaceRegistry(
    reports=(STORAGE_PRIMARY_PLAN_SURFACE, STORAGE_PRIMARY_STATUS_SURFACE),
)


class StoragePrimaryActivation:
    def __init__(self, registry: dict[str, Any]) -> None:
        self.registry = registry
        self._bundle_filesystem = BundleFilesystemAdapter()

    def spec(self, node_name: str = "cynic-lxc-surrealdb") -> StoragePrimarySpec:
        node = self._node(node_name)
        service = self._service(node, "surrealdb")
        synapse = node.get("network", {}).get("synapse")
        if not isinstance(synapse, str) or not synapse:
            raise RuntimeError(f"{node_name} missing network.synapse")
        port = int(service.get("port", 8000))
        return StoragePrimarySpec(
            node=node_name,
            service="surrealdb",
            endpoint=f"http://organ-surrealdb.synapse.cynic:{port}",
            synapse_ip=synapse,
            port=port,
        )

    def manifest(self) -> dict[str, Any]:
        spec = self.spec()
        payload = {
            "schema": "cynic.storage_primary_activation.bundle.v1",
            "mutates": False,
            "target": {
                "node": spec.node,
                "service": spec.service,
                "endpoint": spec.endpoint,
                "synapse_ip": spec.synapse_ip,
                "port": spec.port,
            },
            "requires": [
                "lxc_exec_bridge.activation_status=active",
                "fleet.node.status(cynic-lxc-surrealdb).observed.status=running",
                "network.ipam.check(cynic-lxc-surrealdb).reserved=true",
                "network.mesh.status reaches organ-surrealdb.synapse.cynic",
            ],
            "forbidden": [
                "root/root credentials",
                "192.168.* storage endpoint",
                "remote installer piped to shell",
                "raw pct exec",
            ],
            "install_targets": {
                "unit": "/etc/systemd/system/surrealdb.service",
                "env": "/etc/cynic/surrealdb.env",
                "data_dir": "/var/lib/cynic/surrealdb",
            },
        }
        return attach_surface_contract(payload, STORAGE_PRIMARY_PLAN_SURFACE)

    def write_bundle(self, output_dir: Path, schema_path: Path) -> dict[str, Any]:
        manifest = self.manifest()
        files = {
            "surrealdb.service": self._unit(),
            "surrealdb.env.template": self._env_template(),
            "README.activation.md": self._readme(),
        }
        return self._bundle_filesystem.write_bundle(output_dir, schema_path, manifest, files)

    def activation_status(
        self,
        *,
        bundle_dir: Path | None = None,
        bridge_status: dict[str, Any] | None = None,
        fleet_report: dict[str, Any] | None = None,
        mesh_status: dict[str, Any] | None = None,
        health: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        spec = self.spec()
        checks = {
            "registry_declares_storage_node": True,
            "endpoint_is_synapse_dns": spec.endpoint == f"http://organ-surrealdb.synapse.cynic:{spec.port}",
            "bridge_active": (bridge_status or {}).get("status") == "active",
            "fleet_observes_node_running": self._fleet_observes_node_running(fleet_report, spec.node),
            "mesh_reaches_storage": self._mesh_reaches_storage(mesh_status, spec),
            "bundle_complete": self._bundle_complete(bundle_dir),
            "service_health_healthy": bool((health or {}).get("healthy")),
        }
        missing = [name for name, ok in checks.items() if not ok]
        payload = {
            "schema": "cynic.storage_primary_activation.status.v1",
            "status": "active" if not missing else "blocked",
            "target": {
                "node": spec.node,
                "service": spec.service,
                "endpoint": spec.endpoint,
                "synapse_ip": spec.synapse_ip,
            },
            "checks": checks,
            "missing": missing,
            "evidence": {
                "bridge_status": (bridge_status or {}).get("status"),
                "bundle_dir": str(bundle_dir) if bundle_dir else None,
                "mesh_status": self._mesh_evidence(mesh_status, spec),
                "health": health or {},
            },
            "proof_requirements": self._proof_requirements(spec, checks),
        }
        return attach_surface_contract(payload, STORAGE_PRIMARY_STATUS_SURFACE)

    def _proof_requirements(self, spec: StoragePrimarySpec, checks: dict[str, bool]) -> list[dict[str, Any]]:
        return [
            {
                "check": "bridge_active",
                "status": "satisfied" if checks["bridge_active"] else "missing",
                "command": "cynic bridge status --json",
                "expected": "status=active",
                "resolver_organ": "organ-fleet",
            },
            {
                "check": "fleet_observes_node_running",
                "status": "satisfied" if checks["fleet_observes_node_running"] else "missing",
                "command": "cynic fleet status --json",
                "expected": f"effective[{spec.node}].observed_status=running",
                "resolver_organ": "organ-fleet",
            },
            {
                "check": "mesh_reaches_storage",
                "status": "satisfied" if checks["mesh_reaches_storage"] else "missing",
                "command": "cynic mesh status --resolve-dns --include-records --json",
                "expected": f"organ-surrealdb.synapse.cynic resolves to {spec.synapse_ip}",
                "resolver_organ": "cynic-node",
            },
            {
                "check": "bundle_complete",
                "status": "satisfied" if checks["bundle_complete"] else "missing",
                "command": "cynic storage bundle --output-dir <dir>",
                "expected": "bundle contains manifest.json, schema.surql, surrealdb.service, surrealdb.env.template and README.activation.md",
                "resolver_organ": "organ-keep",
            },
            {
                "check": "service_health_healthy",
                "status": "satisfied" if checks["service_health_healthy"] else "missing",
                "command": "cynic storage status --json --health-json <health.json>",
                "expected": f"{spec.endpoint}/health reports healthy=true through Synapse",
                "resolver_organ": "organ-keep",
            },
        ]

    def _bundle_complete(self, bundle_dir: Path | None) -> bool:
        if bundle_dir is None:
            return False
        required = {
            "README.activation.md",
            "manifest.json",
            "schema.surql",
            "surrealdb.env.template",
            "surrealdb.service",
        }
        return bundle_dir.exists() and all((bundle_dir / name).exists() for name in required)

    def _fleet_observes_node_running(self, fleet_report: dict[str, Any] | None, node_name: str) -> bool:
        if not fleet_report:
            return False
        for item in fleet_report.get("effective", []):
            if item.get("name") == node_name:
                return item.get("observed_status") == "running" or item.get("effective_status") == "active"
        return False

    def _mesh_reaches_storage(
        self,
        mesh_status: dict[str, Any] | None,
        spec: StoragePrimarySpec,
    ) -> bool:
        if not mesh_status:
            return False
        if mesh_status.get("policy_status") != "ok" or mesh_status.get("dns_status") != "ok":
            return False
        expected_name = "organ-surrealdb.synapse.cynic"
        for item in mesh_status.get("dns_checks", []):
            for check in item.get("checks", []):
                if check.get("name") != expected_name:
                    continue
                return (
                    check.get("expected_ip") == spec.synapse_ip
                    and check.get("status") == "resolved_expected_ip"
                )
        return False

    def _mesh_evidence(
        self,
        mesh_status: dict[str, Any] | None,
        spec: StoragePrimarySpec,
    ) -> dict[str, Any]:
        if not mesh_status:
            return {}
        return {
            "policy_status": mesh_status.get("policy_status"),
            "dns_status": mesh_status.get("dns_status"),
            "runtime_status": mesh_status.get("runtime_status"),
            "target": "organ-surrealdb.synapse.cynic",
            "target_ip": spec.synapse_ip,
        }

    def _node(self, name: str) -> dict[str, Any]:
        for node in self.registry.get("nodes", []):
            if node.get("name") == name or name in node.get("aliases", []):
                return node
        raise RuntimeError(f"registry node not found: {name}")

    def _service(self, node: dict[str, Any], name: str) -> dict[str, Any]:
        for service in node.get("services", []):
            if service.get("name") == name:
                return service
        raise RuntimeError(f"service {name} not declared on {node.get('name')}")

    def _unit(self) -> str:
        return """[Unit]
Description=SurrealDB - CYNIC dedicated storage primary
After=network-online.target
Wants=network-online.target

[Service]
EnvironmentFile=/etc/cynic/surrealdb.env
ExecStart=/usr/local/bin/surreal start --bind 0.0.0.0:8000 --user ${SURREALDB_USER} --pass ${SURREALDB_PASS} surrealkv:///var/lib/cynic/surrealdb
Restart=on-failure
RestartSec=5
StateDirectory=cynic/surrealdb

[Install]
WantedBy=multi-user.target
"""

    def _env_template(self) -> str:
        return """# Copy to /etc/cynic/surrealdb.env on the storage node.
# Fill secrets out-of-band; do not commit real values.
SURREALDB_USER=<set-by-operator>
SURREALDB_PASS=<set-by-operator>
"""

    def _readme(self) -> str:
        return """# Storage Primary Activation

This bundle is non-mutating. Install only after the LXC exec bridge is active.

Required checks:

1. `fleet.node.status` observes `cynic-lxc-surrealdb` running.
2. `network.ipam.check` reserves `10.20.0.3` for `cynic-lxc-surrealdb`.
3. Synapse DNS reaches `organ-surrealdb.synapse.cynic`.
4. Install `surrealdb.service` and `/etc/cynic/surrealdb.env`.
5. Start service, then verify `service.health` on the Synapse endpoint.
"""
