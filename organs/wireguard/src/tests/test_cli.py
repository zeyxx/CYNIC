import tempfile
import unittest
import json
import io
import socket
from pathlib import Path
from contextlib import redirect_stdout
from unittest.mock import patch

from organs.wireguard.src.entrypoints.cli import (
    build_parser,
    dispatch,
    run_dns_activation_status,
    run_dns_bundle,
    run_dns_plan,
    run_provider_fallback_hosts,
    run_provider_fallback_status,
    run_status,
    run_sync_dns,
    run_validate,
)
from organs.wireguard.src.adapters.registry_adapter import RegistryAdapter, RegistryTopologyError
from organs.wireguard.src.adapters.coredns_adapter import CoreDNSAdapter
from organs.wireguard.src.entrypoints.cli import WIREGUARD_SURFACE_REGISTRY


class WireguardCliTests(unittest.TestCase):
    def test_sync_dns_writes_synapse_hosts_from_registry(self):
        with tempfile.TemporaryDirectory() as tmp:
            state_dir = Path(tmp) / "coredns"
            status = run_sync_dns("infra/registry.json", str(state_dir))
            hosts = (state_dir / "synapse.hosts").read_text(encoding="utf-8")

        self.assertEqual(status, 0)
        self.assertIn("cynic-vm-v2.synapse.cynic", hosts)
        self.assertIn("cynic-vm-anvil.synapse.cynic", hosts)
        self.assertIn("organ-surrealdb.synapse.cynic", hosts)

    def test_render_synapse_hosts_is_non_mutating(self):
        nodes = RegistryAdapter("infra/registry.json").fetch_static_topology()

        hosts = CoreDNSAdapter(state_dir="/tmp/not-used").render_synapse_hosts(nodes)

        self.assertIn("organ-surrealdb.synapse.cynic", hosts)
        self.assertIn("10.20.0.3", hosts)

    def test_validate_accepts_current_registry_network_policy(self):
        self.assertEqual(run_validate("infra/registry.json"), 0)

    def test_status_reports_declared_records_without_dns_resolution(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            status = run_status("infra/registry.json", resolve_dns=False, include_records=True)

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 0)
        self.assertEqual(payload["schema"], "cynic.infrastructure_mcp.network_mesh_status.v1")
        self.assertEqual("cynic.infrastructure_mcp.network_mesh_status.v1", payload["surface_contract"]["schema"])
        self.assertEqual(payload["policy_status"], "ok")
        self.assertEqual(payload["dns_status"], "not_checked")
        self.assertIsNone(payload["controller_status"])
        self.assertEqual(payload["runtime_status"], "not_observed")
        self.assertIn("proxmox_guest", payload["provider_fallback_planes"])
        self.assertEqual([], payload["drift"])
        storage_records = [
            item for item in payload["records"] if item["node"] == "cynic-lxc-surrealdb"
        ]
        self.assertEqual(storage_records[0]["ip"], "10.20.0.3")
        self.assertIn("organ-surrealdb.synapse.cynic", storage_records[0]["names"])

    def test_status_reports_dns_resolution_failure(self):
        stdout = io.StringIO()
        with patch("socket.getaddrinfo", side_effect=socket.gaierror("no dns")):
            with redirect_stdout(stdout):
                status = run_status("infra/registry.json", resolve_dns=True, include_records=False)

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 1)
        self.assertEqual(payload["dns_status"], "violation")
        self.assertTrue(
            any(item["kind"] == "synapse_dns_resolution" for item in payload["missing"])
        )

    def test_status_reports_runtime_observation_drift(self):
        runtime_observation = {
            "schema": "cynic.network_mesh.runtime_observation.v1",
            "source": "organ-wireguard local observe_network_runtime",
            "interface": "wg0",
            "synapse_cidr": "10.20.0.0/24",
            "status": "blocked",
            "severity": "critical",
            "blockers": ["wireguard_interface", "wg_quick_systemd_active"],
            "missing": ["wireguard_interface", "wg_quick_systemd_active"],
            "drift": [
                {"kind": "runtime_missing", "missing": "wireguard_interface"},
                {"kind": "runtime_missing", "missing": "wg_quick_systemd_active"},
            ],
            "blocking": [
                {"kind": "runtime_missing", "missing": "wireguard_interface"},
                {"kind": "runtime_missing", "missing": "wg_quick_systemd_active"},
            ],
            "checks": {
                "interface_exists": False,
                "systemd_active": False,
                "wg_binary_available": False,
                "wg_active": False,
                "route_uses_interface": False,
            },
            "observed": {},
            "errors": {},
        }
        stdout = io.StringIO()
        with patch(
            "organs.fleet.src.core.cli_runtime._cynic_node_observe_payload",
            return_value=None,
        ), patch(
            "organs.fleet.src.core.cli_runtime.observe_network_runtime",
            return_value=runtime_observation,
        ):
            with redirect_stdout(stdout):
                status = run_status(
                    "infra/registry.json",
                    resolve_dns=False,
                    include_records=False,
                    observe_runtime=True,
                )

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 1)
        self.assertEqual(payload["runtime_status"], "blocked")
        self.assertEqual(payload["runtime_observation"]["status"], "blocked")
        self.assertEqual(
            payload["runtime_observation"]["schema"],
            "cynic.network_mesh.runtime_observation.v1",
        )
        self.assertTrue(payload["blocking"])
        self.assertTrue(
            any(item.get("missing") == "wireguard_interface" for item in payload["drift"])
        )

    def test_dns_plan_is_non_mutating_activation_contract(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            status = run_dns_plan()

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 0)
        self.assertFalse(payload["mutates"])
        self.assertEqual("cynic.organ_fleet.dns_activation_plan.v1", payload["surface_contract"]["schema"])
        self.assertEqual(payload["target"]["zone"], "synapse.cynic")
        self.assertEqual(payload["provider_policy"]["canonical_provider"], "coredns")
        self.assertEqual(payload["package_preconditions"][0]["binary"], "coredns")
        self.assertIn("curl_pipe_shell", payload["package_preconditions"][0]["forbidden_sources"])
        self.assertIn("unverified upstream CoreDNS binary", payload["forbidden"])
        self.assertIn(
            "automatic fallback to a noncanonical DNS provider without an adapter contract",
            payload["forbidden"],
        )
        self.assertIn("getent hosts organ-surrealdb.synapse.cynic resolves 10.20.0.3", payload["activation_checks"])

    def test_dns_bundle_contains_hosts_corefile_service_and_resolved_dropin(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "bundle"
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                status = run_dns_bundle("infra/registry.json", str(out))

            manifest = json.loads(stdout.getvalue())
            hosts = (out / "synapse.hosts").read_text(encoding="utf-8")
            corefile_exists = (out / "Corefile").exists()
            service_exists = (out / "coredns-synapse.service").exists()
            resolved_dropin_exists = (out / "cynic-synapse-dns.conf").exists()

        self.assertEqual(status, 0)
        self.assertEqual(manifest["bundle_dir"], str(out.resolve()))
        self.assertIn("manifest.json", manifest["bundle_files"])
        self.assertEqual("cynic.organ_fleet.dns_activation_bundle.v1", manifest["surface_contract"]["schema"])
        self.assertIn("organ-surrealdb.synapse.cynic", hosts)
        self.assertTrue(corefile_exists)
        self.assertTrue(service_exists)
        self.assertTrue(resolved_dropin_exists)

    def test_provider_fallback_hosts_are_explicitly_noncanonical(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "fallback"
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                status = run_provider_fallback_hosts("infra/registry.json", str(out))
            manifest = json.loads(stdout.getvalue())
            hosts = (out / "provider-fallback.hosts").read_text(encoding="utf-8")

        self.assertEqual(status, 0)
        self.assertFalse(manifest["canonical"])
        self.assertFalse(manifest["mutates_system"])
        self.assertIn("forgejo.synapse.cynic", hosts)
        self.assertIn("10.10.10.101", hosts)
        self.assertIn("cynic-vm-anvil", hosts)
        self.assertIn("10.10.10.102", hosts)

    def test_provider_fallback_status_accepts_generated_hosts_without_connectivity(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "fallback"
            with redirect_stdout(io.StringIO()):
                run_provider_fallback_hosts("infra/registry.json", str(out))
            stdout = io.StringIO()
            with patch(
                "organs.wireguard.src.adapters.cli_runtime._provider_route_status",
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
                with redirect_stdout(stdout):
                    status = run_provider_fallback_status(
                        "infra/registry.json",
                        str(out / "provider-fallback.hosts"),
                        check_connectivity=False,
                    )

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 0)
        self.assertEqual(payload["status"], "active")
        self.assertFalse(payload["canonical"])
        self.assertTrue(payload["checks"]["route_persistent"])

    def test_dns_activation_status_accepts_complete_bundle_paths_without_resolution(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "bundle"
            with redirect_stdout(io.StringIO()):
                run_dns_bundle("infra/registry.json", str(out))
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                status = run_dns_activation_status(
                    str(out / "synapse.hosts"),
                    str(out / "Corefile"),
                    str(out / "coredns-synapse.service"),
                    str(out / "cynic-synapse-dns.conf"),
                    resolve_dns=False,
                )

        payload = json.loads(stdout.getvalue())
        self.assertTrue(payload["checks"]["hosts_contains_storage_record"])
        self.assertTrue(payload["checks"]["corefile_references_hosts"])
        self.assertTrue(payload["checks"]["service_references_corefile"])
        self.assertTrue(payload["checks"]["resolved_routes_synapse"])
        self.assertEqual(payload["provider_policy"]["canonical_provider"], "coredns")
        self.assertEqual(payload["provider_status"]["canonical_binary"], "coredns")
        self.assertEqual(
            payload["checks"]["canonical_dns_provider_available"],
            payload["provider_status"]["canonical_available"],
        )
        if payload["checks"]["coredns_binary_available"]:
            self.assertEqual(status, 0)
        else:
            self.assertEqual(status, 1)
            self.assertIn("coredns_binary_available", payload["missing"])
            self.assertIn("canonical_dns_provider_available", payload["missing"])
        self.assertEqual("cynic.organ_fleet.dns_activation_status.v1", payload["surface_contract"]["schema"])

    def test_registry_adapter_rejects_duplicate_synapse_ips(self):
        with tempfile.TemporaryDirectory() as tmp:
            registry = Path(tmp) / "registry.json"
            registry.write_text(
                json.dumps(
                    {
                        "network_policy": {
                            "canonical_service_plane": "synapse",
                            "planes": {"synapse": {"cidr": "10.20.0.0/24"}},
                            "reservations": [
                                {"name": "node-a", "synapse": "10.20.0.2"},
                                {"name": "node-b", "synapse": "10.20.0.2"},
                            ],
                        },
                        "nodes": [
                            {"name": "node-a", "network": {"synapse": "10.20.0.2"}},
                            {"name": "node-b", "network": {"synapse": "10.20.0.2"}},
                        ],
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(RegistryTopologyError, "duplicate synapse IP"):
                RegistryAdapter(str(registry)).fetch_static_topology()

    def test_registry_adapter_rejects_synapse_ip_outside_policy_cidr(self):
        with tempfile.TemporaryDirectory() as tmp:
            registry = Path(tmp) / "registry.json"
            registry.write_text(
                json.dumps(
                    {
                        "network_policy": {
                            "canonical_service_plane": "synapse",
                            "planes": {"synapse": {"cidr": "10.20.0.0/24"}},
                            "reservations": [
                                {"name": "node-a", "synapse": "10.30.0.2"},
                            ],
                        },
                        "nodes": [
                            {"name": "node-a", "network": {"synapse": "10.30.0.2"}},
                        ],
                    }
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(RegistryTopologyError, "outside 10.20.0.0/24"):
                RegistryAdapter(str(registry)).fetch_static_topology()

    def test_wireguard_surface_registry_exposes_common_report_contracts(self):
        manifest = WIREGUARD_SURFACE_REGISTRY.manifest()

        self.assertEqual("cynic.surface_registry.v1", manifest["schema"])
        self.assertEqual(6, manifest["report_count"])
        self.assertIn("cynic status", WIREGUARD_SURFACE_REGISTRY.report_names())
        self.assertIn("cynic dns activation-status", WIREGUARD_SURFACE_REGISTRY.report_names())
        self.assertIn("cynic fallback-hosts", WIREGUARD_SURFACE_REGISTRY.report_names())
        self.assertIn("cynic fallback-status", WIREGUARD_SURFACE_REGISTRY.report_names())

    def test_daemon_requires_explicit_breakglass_env(self):
        parser = build_parser()
        args = parser.parse_args(["daemon"])
        stderr = io.StringIO()

        with patch.dict("os.environ", {}, clear=False):
            with patch("sys.stderr", stderr):
                status = dispatch(args, parser)

        self.assertEqual(status, 2)
        self.assertIn("legacy-blocked", stderr.getvalue())

    def test_daemon_runs_when_breakglass_env_is_set(self):
        parser = build_parser()
        args = parser.parse_args(["daemon"])

        with patch.dict("os.environ", {"CYNIC_ALLOW_LEGACY_WIREGUARD_DAEMON": "1"}, clear=False):
            with patch("organs.wireguard.src.entrypoints.cli.run_daemon", return_value=0) as run_daemon_mock:
                status = dispatch(args, parser)

        self.assertEqual(status, 0)
        run_daemon_mock.assert_called_once_with()


if __name__ == "__main__":
    unittest.main()
