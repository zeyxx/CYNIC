import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

from organs.fleet.src.entrypoints import cli
from organs.fleet.src.core.network_surfaces import mesh_status_payload


class FleetDnsSurfaceTests(unittest.TestCase):
    def test_cli_emits_dns_plan_with_fleet_surface_contract(self):
        argv = ["fleet", "dns", "plan"]
        stdout = io.StringIO()
        cynic_node_payload = {
            "schema": "cynic.organ_fleet.dns_activation_plan.v1",
            "mutates": False,
            "target": {
                "zone": "synapse.cynic",
                "hosts": "/etc/coredns/synapse.hosts",
                "corefile": "/etc/coredns/Corefile",
                "service": "/etc/systemd/system/coredns-synapse.service",
                "resolved_dropin": "/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf",
            },
            "required_owner": "root:root",
            "required_modes": {
                "hosts": "0644",
                "corefile": "0644",
                "service": "0644",
                "resolved_dropin": "0644",
            },
            "provider_policy": {},
            "package_preconditions": [],
            "activation_checks": [],
            "forbidden": [],
        }
        with patch(
            "organs.fleet.src.core.cli_runtime._cynic_node_json",
            return_value=cynic_node_payload,
        ):
            with patch.object(sys, "argv", argv), redirect_stdout(stdout):
                status = cli.main()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["schema"], "cynic.organ_fleet.dns_activation_plan.v1")
        self.assertEqual(
            payload["surface_contract"]["schema"],
            "cynic.organ_fleet.dns_activation_plan.v1",
        )

    def test_cli_dns_plan_falls_back_to_python_when_cynic_node_unavailable(self):
        argv = ["fleet", "dns", "plan"]
        stdout = io.StringIO()
        with patch(
            "organs.fleet.src.core.cli_runtime._cynic_node_json",
            return_value=None,
        ):
            with patch.object(sys, "argv", argv), redirect_stdout(stdout):
                status = cli.main()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["schema"], "cynic.organ_fleet.dns_activation_plan.v1")

    def test_cli_emits_dns_bundle_with_fleet_surface_contract(self):
        with tempfile.TemporaryDirectory() as tmp:
            bundle_dir = Path(tmp) / "dns-bundle"
            argv = [
                "fleet",
                "dns",
                "bundle",
                "--registry",
                "infra/registry.json",
                "--output-dir",
                str(bundle_dir),
            ]
            stdout = io.StringIO()
            cynic_node_payload = {
                "schema": "cynic.organ_fleet.dns_activation_bundle.v1",
                "mutates": False,
                "target": {
                    "zone": "synapse.cynic",
                    "hosts": "/etc/coredns/synapse.hosts",
                    "corefile": "/etc/coredns/Corefile",
                    "service": "/etc/systemd/system/coredns-synapse.service",
                    "resolved_dropin": "/etc/systemd/resolved.conf.d/cynic-synapse-dns.conf",
                },
                "required_owner": "root:root",
                "required_modes": {
                    "hosts": "0644",
                    "corefile": "0644",
                    "service": "0644",
                    "resolved_dropin": "0644",
                },
                "provider_policy": {
                    "canonical_provider": "coredns",
                    "canonical_binary": "coredns",
                    "selection_source": "organ-fleet Synapse DNS activation contract",
                    "service_contract": "*.synapse.cynic",
                    "approved_sources": ["distribution_package", "approved_internal_package_mirror"],
                    "known_noncanonical_candidates": ["dnsmasq", "unbound", "bind9", "systemd-resolved"],
                    "promotion_rule": "A candidate provider becomes usable only after a dedicated adapter contract, bundle renderer, service template, and activation-status checks are added.",
                },
                "package_preconditions": [],
                "activation_checks": [],
                "forbidden": [],
                "bundle_dir": str(bundle_dir),
                "bundle_files": [
                    "Corefile",
                    "README.activation.md",
                    "coredns-synapse.service",
                    "cynic-synapse-dns.conf",
                    "manifest.json",
                    "synapse.hosts",
                ],
            }
            with patch(
                "organs.fleet.src.core.cli_runtime._cynic_node_json",
                return_value=cynic_node_payload,
            ):
                with patch.object(sys, "argv", argv), redirect_stdout(stdout):
                    status = cli.main()

            payload = json.loads(stdout.getvalue())

        self.assertEqual(status, 0)
        self.assertEqual(payload["schema"], "cynic.organ_fleet.dns_activation_bundle.v1")
        self.assertEqual(
            payload["surface_contract"]["schema"],
            "cynic.organ_fleet.dns_activation_bundle.v1",
        )
        self.assertEqual(payload["bundle_dir"], str(bundle_dir))

    def test_cli_dns_bundle_falls_back_to_python_when_cynic_node_unavailable(self):
        with tempfile.TemporaryDirectory() as tmp:
            bundle_dir = Path(tmp) / "dns-bundle"
            argv = [
                "fleet",
                "dns",
                "bundle",
                "--registry",
                "infra/registry.json",
                "--output-dir",
                str(bundle_dir),
            ]
            stdout = io.StringIO()
            with patch(
                "organs.fleet.src.core.cli_runtime._cynic_node_json",
                return_value=None,
            ):
                with patch.object(sys, "argv", argv), redirect_stdout(stdout):
                    status = cli.main()

            payload = json.loads(stdout.getvalue())
            readme = (bundle_dir / "README.activation.md").read_text(encoding="utf-8")
            hosts = (bundle_dir / "synapse.hosts").read_text(encoding="utf-8")

        self.assertEqual(status, 0)
        self.assertEqual(payload["schema"], "cynic.organ_fleet.dns_activation_bundle.v1")
        self.assertIn("organ-fleet", readme)
        self.assertIn("python3 organs/fleet/src/entrypoints/cli.py", readme)
        self.assertIn("pve.synapse.cynic", hosts)
        self.assertIn("organ-forgejo.synapse.cynic", hosts)
        self.assertIn("# AUTO-GENERATED BY CYNIC-NODE NETWORK SURFACE", hosts)

    def test_cli_emits_dns_activation_status_with_fleet_surface_contract(self):
        with tempfile.TemporaryDirectory() as tmp:
            bundle_dir = Path(tmp) / "dns-bundle"
            bundle_argv = [
                "fleet",
                "dns",
                "bundle",
                "--registry",
                "infra/registry.json",
                "--output-dir",
                str(bundle_dir),
            ]
            with patch.object(sys, "argv", bundle_argv), redirect_stdout(io.StringIO()):
                cli.main()

            status_argv = [
                "fleet",
                "dns",
                "activation-status",
                "--hosts-path",
                str(bundle_dir / "synapse.hosts"),
                "--corefile-path",
                str(bundle_dir / "Corefile"),
                "--service-path",
                str(bundle_dir / "coredns-synapse.service"),
                "--resolved-dropin-path",
                str(bundle_dir / "cynic-synapse-dns.conf"),
            ]
            stdout = io.StringIO()
            cynic_node_payload = {
                "schema": "cynic.organ_fleet.dns_activation_status.v1",
                "status": "active",
                "checks": {
                    "hosts_exists": True,
                    "corefile_exists": True,
                    "service_exists": True,
                    "resolved_dropin_exists": True,
                    "coredns_binary_available": True,
                    "canonical_dns_provider_available": True,
                    "hosts_contains_storage_record": True,
                    "corefile_references_hosts": True,
                    "service_references_corefile": True,
                    "resolved_routes_synapse": True,
                },
                "provider_policy": {},
                "provider_status": {"canonical_dns_provider_available": True},
                "missing": [],
                "paths": {
                    "hosts": str(bundle_dir / "synapse.hosts"),
                    "corefile": str(bundle_dir / "Corefile"),
                    "service": str(bundle_dir / "coredns-synapse.service"),
                    "resolved_dropin": str(bundle_dir / "cynic-synapse-dns.conf"),
                },
                "mesh_status": {},
            }
            with patch(
                "organs.fleet.src.core.cli_runtime._cynic_node_json",
                return_value=cynic_node_payload,
            ):
                with patch.object(sys, "argv", status_argv), redirect_stdout(stdout):
                    status = cli.main()

        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["schema"], "cynic.organ_fleet.dns_activation_status.v1")
        self.assertEqual(
            payload["surface_contract"]["schema"],
            "cynic.organ_fleet.dns_activation_status.v1",
        )
        self.assertEqual(status, 0)

    def test_cli_dns_activation_status_falls_back_to_python_when_cynic_node_unavailable(self):
        with tempfile.TemporaryDirectory() as tmp:
            bundle_dir = Path(tmp) / "dns-bundle"
            bundle_argv = [
                "fleet",
                "dns",
                "bundle",
                "--registry",
                "infra/registry.json",
                "--output-dir",
                str(bundle_dir),
            ]
            with patch(
                "organs.fleet.src.core.cli_runtime._cynic_node_json",
                return_value=None,
            ):
                with patch.object(sys, "argv", bundle_argv), redirect_stdout(io.StringIO()):
                    cli.main()

            status_argv = [
                "fleet",
                "dns",
                "activation-status",
                "--hosts-path",
                str(bundle_dir / "synapse.hosts"),
                "--corefile-path",
                str(bundle_dir / "Corefile"),
                "--service-path",
                str(bundle_dir / "coredns-synapse.service"),
                "--resolved-dropin-path",
                str(bundle_dir / "cynic-synapse-dns.conf"),
            ]
            stdout = io.StringIO()
            with patch(
                "organs.fleet.src.core.cli_runtime._cynic_node_json",
                return_value=None,
            ):
                with patch.object(sys, "argv", status_argv), redirect_stdout(stdout):
                    status = cli.main()

        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["schema"], "cynic.organ_fleet.dns_activation_status.v1")
        if payload["checks"]["canonical_dns_provider_available"]:
            self.assertEqual(status, 0)
        else:
            self.assertEqual(status, 1)
            self.assertIn("canonical_dns_provider_available", payload["missing"])

    def test_cli_emits_mesh_status_with_fleet_surface_contract(self):
        argv = ["fleet", "mesh", "status", "--registry", "infra/registry.json"]
        stdout = io.StringIO()
        with patch.object(sys, "argv", argv), redirect_stdout(stdout):
            status = cli.main()

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 0)
        self.assertEqual(payload["schema"], "cynic.infrastructure_mcp.network_mesh_status.v1")
        self.assertEqual(
            payload["surface_contract"]["command"],
            "cynic mesh status",
        )

    def test_cli_maps_runtime_observation_active_to_mesh_active(self):
        argv = ["fleet", "mesh", "status", "--registry", "infra/registry.json", "--observe-runtime"]
        stdout = io.StringIO()
        with patch(
            "organs.fleet.src.core.cli_runtime._cynic_node_observe_payload",
            return_value=None,
        ):
            with patch(
                "organs.fleet.src.core.cli_runtime.observe_network_runtime",
                return_value={
                    "schema": "cynic.network_mesh.runtime_observation.v1",
                    "source": "organ-fleet local observe_network_runtime",
                    "status": "mesh_active",
                    "severity": "clear",
                    "blockers": [],
                    "missing": [],
                    "drift": [],
                    "blocking": [],
                },
            ):
                with patch.object(sys, "argv", argv), redirect_stdout(stdout):
                    status = cli.main()

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 0)
        self.assertEqual(payload["runtime_status"], "mesh_active")
        self.assertEqual(
            payload["runtime_observation"]["schema"],
            "cynic.network_mesh.runtime_observation.v1",
        )

    def test_cli_prefers_cynic_node_runtime_projection_when_available(self):
        argv = ["fleet", "mesh", "status", "--registry", "infra/registry.json", "--observe-runtime"]
        stdout = io.StringIO()
        with patch(
            "organs.fleet.src.core.cli_runtime._cynic_node_observe_payload",
            return_value={
                "status": "provider_fallback_active",
                "blockers": ["synapse_route_missing", "wireguard_mesh_absent"],
                "network_mesh_status": {"runtime_status": "provider_fallback_active"},
                "runtime_observation": {
                    "schema": "cynic.network_mesh.runtime_observation.v1",
                    "source": "cynic-node observe",
                    "status": "provider_fallback_active",
                    "severity": "warning",
                    "blockers": ["synapse_route_missing", "wireguard_mesh_absent"],
                    "missing": ["synapse_route_missing", "wireguard_mesh_absent"],
                    "drift": [
                        {"kind": "runtime_blocker", "missing": "synapse_route_missing"},
                        {"kind": "runtime_blocker", "missing": "wireguard_mesh_absent"},
                    ],
                    "blocking": [],
                    "wireguard_transport": {"state": "route_drift"},
                    "dns_projection": {
                        "state": "active",
                        "checks": {
                            "critical_hosts_resolve": True,
                        },
                    },
                },
            },
        ):
            with patch(
                "organs.fleet.src.core.cli_runtime.observe_network_runtime",
            ) as local_observe:
                with patch.object(sys, "argv", argv), redirect_stdout(stdout):
                    status = cli.main()

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 1)
        self.assertEqual(payload["runtime_status"], "provider_fallback_active")
        local_observe.assert_not_called()
        self.assertEqual(payload["runtime_observation"]["source"], "cynic-node observe")
        self.assertEqual(payload["runtime_observation"]["severity"], "warning")
        self.assertEqual(
            payload["runtime_observation"]["missing"],
            ["synapse_route_missing", "wireguard_mesh_absent"],
        )
        self.assertEqual(
            [item["missing"] for item in payload["drift"]],
            ["synapse_route_missing", "wireguard_mesh_absent"],
        )
        self.assertEqual(
            payload["runtime_observation"]["schema"],
            "cynic.network_mesh.runtime_observation.v1",
        )
        self.assertEqual(payload["dns_status"], "ok")

    def test_mesh_status_uses_runtime_dns_projection_when_not_resolving_dns(self):
        payload = mesh_status_payload(
            registry_ref="infra/registry.json",
            records=[],
            resolve_dns=False,
            include_records=False,
            registry_policy={"canonical_service_plane": "synapse"},
            runtime_observation={
                "schema": "cynic.network_mesh.runtime_observation.v1",
                "status": "provider_fallback_active",
                "dns_projection": {
                    "checks": {
                        "critical_hosts_resolve": False,
                    }
                },
            },
        )
        self.assertEqual(payload["dns_status"], "violation")

    def test_cli_emits_provider_fallback_hosts_with_fleet_surface_contract(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "fallback"
            argv = [
                "fleet",
                "fallback-hosts",
                "--registry",
                "infra/registry.json",
                "--output-dir",
                str(out),
            ]
            stdout = io.StringIO()
            cynic_node_payload = {
                "schema": "cynic.organ_fleet.provider_fallback_hosts.v1",
                "status": "fallback_available",
                "canonical": False,
                "mutates_system": False,
                "registry_ref": "infra/registry.json",
                "hosts_path": str(out / "provider-fallback.hosts"),
                "record_count": 1,
                "records": [
                    {
                        "node": "forge",
                        "role": "build_runner",
                        "provider_plane": "proxmox_guest",
                        "ip": "10.10.10.4",
                        "names": ["forge", "forge.synapse.cynic"],
                        "canonical": False,
                    }
                ],
                "warning": "Provider fallback DNS is temporary evidence plumbing, not the Synapse service contract.",
                "expires_on": "network_mesh_status.runtime_status == mesh_active",
            }
            with patch(
                "organs.fleet.src.core.cli_runtime._cynic_node_json",
                return_value=cynic_node_payload,
            ):
                with patch.object(sys, "argv", argv), redirect_stdout(stdout):
                    status = cli.main()

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 0)
        self.assertEqual(payload["schema"], "cynic.organ_fleet.provider_fallback_hosts.v1")
        self.assertEqual(
            payload["surface_contract"]["schema"],
            "cynic.organ_fleet.provider_fallback_hosts.v1",
        )

    def test_cli_provider_fallback_hosts_falls_back_to_python_when_cynic_node_unavailable(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "fallback"
            argv = [
                "fleet",
                "fallback-hosts",
                "--registry",
                "infra/registry.json",
                "--output-dir",
                str(out),
            ]
            stdout = io.StringIO()
            with patch(
                "organs.fleet.src.core.cli_runtime._cynic_node_json",
                return_value=None,
            ):
                with patch.object(sys, "argv", argv), redirect_stdout(stdout):
                    status = cli.main()
            payload = json.loads(stdout.getvalue())
            self.assertEqual(status, 0)
            self.assertEqual(payload["schema"], "cynic.organ_fleet.provider_fallback_hosts.v1")
            self.assertTrue((out / "provider-fallback.hosts").exists())

    def test_cli_emits_provider_fallback_status_with_fleet_surface_contract(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "fallback"
            hosts_argv = [
                "fleet",
                "fallback-hosts",
                "--registry",
                "infra/registry.json",
                "--output-dir",
                str(out),
            ]
            with patch.object(sys, "argv", hosts_argv), redirect_stdout(io.StringIO()):
                cli.main()

            status_argv = [
                "fleet",
                "fallback-status",
                "--registry",
                "infra/registry.json",
                "--hosts-path",
                str(out / "provider-fallback.hosts"),
            ]
            stdout = io.StringIO()
            cynic_node_payload = {
                "schema": "cynic.organ_fleet.provider_fallback_status.v1",
                "status": "active",
                "canonical": False,
                "registry_ref": "infra/registry.json",
                "hosts_path": str(out / "provider-fallback.hosts"),
                "checks": {
                    "hosts_file_exists": True,
                    "hosts_entries_present": True,
                    "route_active": True,
                    "route_persistent": True,
                    "connectivity_checked": False,
                    "network_connectivity_ok": None,
                },
                "missing": [],
                "hosts_missing": [],
                "route": {
                    "active": True,
                    "persistent": True,
                    "observed_route": "10.10.10.0/24 via 192.168.0.37",
                    "observed_nmcli_routes": "10.10.10.0/24 192.168.0.37",
                    "ip_route_error": None,
                    "nmcli_error": None,
                },
                "connectivity": [],
                "warning": "Provider fallback is a resurrection path only while Synapse/WireGuard remains inactive.",
            }
            with patch(
                "organs.fleet.src.core.cli_runtime._cynic_node_json",
                return_value=cynic_node_payload,
            ):
                with patch.object(sys, "argv", status_argv), redirect_stdout(stdout):
                    status = cli.main()

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 0)
        self.assertEqual(payload["schema"], "cynic.organ_fleet.provider_fallback_status.v1")
        self.assertEqual(payload["checks"]["route_active"], True)

    def test_cli_provider_fallback_status_falls_back_to_python_when_cynic_node_unavailable(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "fallback"
            hosts_argv = [
                "fleet",
                "fallback-hosts",
                "--registry",
                "infra/registry.json",
                "--output-dir",
                str(out),
            ]
            with patch.object(sys, "argv", hosts_argv), redirect_stdout(io.StringIO()):
                cli.main()

            status_argv = [
                "fleet",
                "fallback-status",
                "--registry",
                "infra/registry.json",
                "--hosts-path",
                str(out / "provider-fallback.hosts"),
            ]
            stdout = io.StringIO()
            with patch(
                "organs.fleet.src.core.cli_runtime._cynic_node_json",
                return_value=None,
            ):
                with patch(
                    "organs.fleet.src.core.cli_runtime.provider_route_status",
                    return_value={
                    "cidr": "10.10.10.0/24",
                    "gateway": "192.168.0.37",
                    "active": True,
                    "persistent": True,
                    "observed_route": "10.10.10.0/24 via 192.168.0.37",
                    "observed_nmcli_routes": "10.10.10.0/24 192.168.0.37",
                        "errors": {},
                    },
                ):
                    with patch.object(sys, "argv", status_argv), redirect_stdout(stdout):
                        status = cli.main()

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 0)
        self.assertEqual(payload["schema"], "cynic.organ_fleet.provider_fallback_status.v1")
        self.assertEqual(
            payload["surface_contract"]["schema"],
            "cynic.organ_fleet.provider_fallback_status.v1",
        )


if __name__ == "__main__":
    unittest.main()
