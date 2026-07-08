import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

from organs.fleet.src.adapters.proxmox_adapter import ProxmoxListTextAdapter
from organs.fleet.src.adapters.registry_adapter import RegistryAdapter
from organs.fleet.src.core.entities import Node, ObservedResource
from organs.fleet.src.core.service import FleetTruthService
from organs.fleet.src.entrypoints import cli


class FleetTruthTests(unittest.TestCase):
    def test_active_node_without_observation_is_anomaly(self):
        report = FleetTruthService().build_report(
            declared_nodes=[
                Node(
                    name="cynic-vm-anvil",
                    role="build_runner",
                    status="active",
                    energy_profile={"class": "burst"},
                ),
            ],
            observed_resources=[],
            registry_organs={"organ-fleet"},
            organ_dirs={"fleet"},
        )

        self.assertTrue(
            any(
                anomaly["kind"] == "active_without_runtime_proof"
                for anomaly in report.anomalies
            )
        )
        self.assertEqual(report.effective[0]["effective_status"], "stale")
        self.assertEqual(report.summary["status"], "degraded")
        self.assertEqual(report.summary["critical_targets"], ["cynic-vm-anvil"])

    def test_observed_identity_satisfies_active_node(self):
        report = FleetTruthService().build_report(
            declared_nodes=[
                Node(
                    name="cynic-vm-anvil",
                    role="build_runner",
                    status="active",
                    aliases=("forge",),
                    proxmox_id="qemu/102",
                    energy_profile={"class": "burst"},
                ),
            ],
            observed_resources=[
                ObservedResource(
                    provider="proxmox",
                    name="cynic-vm-anvil",
                    status="running",
                    resource_id="qemu/102",
                ),
            ],
            registry_organs={"organ-fleet"},
            organ_dirs={"fleet"},
        )

        self.assertEqual(report.effective[0]["effective_status"], "active")
        self.assertFalse(
            [
                anomaly
                for anomaly in report.anomalies
                if anomaly["kind"] == "active_without_runtime_proof"
            ]
        )
        self.assertEqual(report.summary["status"], "ok")

    def test_proxmox_mcp_text_output_is_standardized_runtime_evidence(self):
        text = """=== Node: pve ===
VMs:
- ID: 102, Name: cynic-vm-anvil, Status: running, Mem: 4096MB
- ID: 140, Name: cynic-vm-v2, Status: running, Mem: 4096MB
Containers (LXC):
- ID: 100, Name: organ-headscale, Status: running, Mem: 1024MB
- ID: 105, Name: cynic-builder, Status: running, Mem: 8192MB
"""

        resources = ProxmoxListTextAdapter(None).parse(text)

        self.assertEqual(
            [
                (resource.resource_id, resource.name, resource.status, resource.kind)
                for resource in resources
            ],
            [
                (None, "pve", "running", "hypervisor"),
                ("qemu/102", "cynic-vm-anvil", "running", "vm"),
                ("qemu/140", "cynic-vm-v2", "running", "vm"),
                ("lxc/100", "organ-headscale", "running", "lxc"),
                ("lxc/105", "cynic-builder", "running", "lxc"),
            ],
        )

    def test_fleet_cli_accepts_proxmox_list_text_from_stdin(self):
        text = """=== Node: pve ===
VMs:
- ID: 102, Name: cynic-vm-anvil, Status: running, Mem: 4096MB
Containers (LXC):
- ID: 100, Name: organ-headscale, Status: running, Mem: 1024MB
"""
        argv = [
            "fleet",
            "report",
            "--json",
            "--registry",
            "infra/registry.json",
            "--proxmox-list-text",
            "-",
        ]
        stdout = io.StringIO()
        with patch.object(sys, "argv", argv), patch.object(sys, "stdin", io.StringIO(text)):
            with redirect_stdout(stdout):
                status = cli.main()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        effective = {item["name"]: item for item in payload["effective"]}
        self.assertEqual(effective["cynic-vm-anvil"]["effective_status"], "active")
        self.assertEqual(effective["organ-headscale"]["effective_status"], "active")

    def test_registry_provider_network_counts_as_bounded_provider_observation(self):
        resources = RegistryAdapter(Path("infra/registry.json")).observed_provider_resources()
        by_name = {resource.name: resource for resource in resources}

        self.assertEqual(by_name["cynic-lxc-git"].provider, "operator_observation")
        self.assertEqual(by_name["cynic-lxc-git"].status, "running")
        self.assertEqual(by_name["cynic-lxc-git"].resource_id, "lxc/101")
        self.assertEqual(by_name["cynic-lxc-git"].constellation, "forge")

    def test_fleet_cli_accepts_watchdog_constellation_evidence(self):
        observation = {
            "tool": "cynic-node-watchdog",
            "value": {
                "schema": "cynic.node_watchdog_observation.v1",
                "status": "ok",
                "node": {
                    "name": "cynic-vm-v2",
                    "role": "kernel_primary",
                    "constellation": "kernel",
                },
            },
        }
        with tempfile.NamedTemporaryFile("w", encoding="utf-8") as handle:
            json.dump(observation, handle)
            handle.flush()
            argv = [
                "fleet",
                "report",
                "--json",
                "--registry",
                "infra/registry.json",
                "--watchdog-json",
                handle.name,
            ]
            stdout = io.StringIO()
            with patch.object(sys, "argv", argv):
                with redirect_stdout(stdout):
                    status = cli.main()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        effective = {item["name"]: item for item in payload["effective"]}
        self.assertEqual(effective["cynic-vm-v2"]["provider"], "cynic-node-watchdog")
        self.assertEqual(effective["cynic-vm-v2"]["observed_constellation"], "kernel")
        self.assertGreaterEqual(
            payload["summary"]["constellations"]["observed"].get("kernel", 0),
            1,
        )


if __name__ == "__main__":
    unittest.main()
