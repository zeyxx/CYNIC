import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from organs.contracts.src.project_root import resolve_repo_root
from organs.keep.src.core.storage_activation import StoragePrimaryActivation


ROOT = resolve_repo_root(Path(__file__).resolve())

REGISTRY = {
    "nodes": [
        {
            "name": "cynic-lxc-surrealdb",
            "aliases": ["organ-surrealdb"],
            "network": {"synapse": "10.20.0.3"},
            "services": [{"name": "surrealdb", "port": 8000}],
        }
    ]
}


class StorageActivationTests(unittest.TestCase):
    def test_manifest_uses_synapse_endpoint_not_lan(self):
        manifest = StoragePrimaryActivation(REGISTRY).manifest()

        self.assertEqual(
            manifest["target"]["endpoint"],
            "http://organ-surrealdb.synapse.cynic:8000",
        )
        self.assertEqual("cynic.storage_primary_activation.bundle.v1", manifest["surface_contract"]["schema"])
        self.assertEqual("cynic storage plan", manifest["surface_contract"]["command"])
        self.assertNotIn("192.168", json.dumps(manifest["target"]))
        self.assertNotIn("192.168", json.dumps(manifest["install_targets"]))
        self.assertIn("lxc_exec_bridge.activation_status=active", manifest["requires"])

    def test_bundle_contains_unit_env_schema_and_manifest(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            schema = root / "schema.surql"
            schema.write_text("-- schema\n", encoding="utf-8")
            out = root / "bundle"

            manifest = StoragePrimaryActivation(REGISTRY).write_bundle(out, schema)

            self.assertFalse(manifest["mutates"])
            self.assertTrue((out / "surrealdb.service").exists())
            self.assertTrue((out / "surrealdb.env.template").exists())
            self.assertTrue((out / "schema.surql").exists())
            self.assertTrue((out / "manifest.json").exists())
            self.assertNotIn("root/root", (out / "README.activation.md").read_text())

    def test_status_blocks_without_runtime_evidence(self):
        status = StoragePrimaryActivation(REGISTRY).activation_status()

        self.assertEqual(status["status"], "blocked")
        self.assertEqual("cynic.storage_primary_activation.status.v1", status["surface_contract"]["schema"])
        self.assertIn("bridge_active", status["missing"])
        self.assertIn("fleet_observes_node_running", status["missing"])
        self.assertIn("mesh_reaches_storage", status["missing"])
        self.assertIn("service_health_healthy", status["missing"])
        requirements = {item["check"]: item for item in status["proof_requirements"]}
        self.assertEqual("missing", requirements["bridge_active"]["status"])
        self.assertEqual("cynic bridge status --json", requirements["bridge_active"]["command"])
        self.assertEqual("organ-fleet", requirements["fleet_observes_node_running"]["resolver_organ"])
        self.assertIn("organ-surrealdb.synapse.cynic", requirements["mesh_reaches_storage"]["expected"])
        self.assertEqual("cynic-node", requirements["mesh_reaches_storage"]["resolver_organ"])
        self.assertEqual("organ-keep", requirements["bundle_complete"]["resolver_organ"])
        self.assertIn("Synapse", requirements["service_health_healthy"]["expected"])

    def test_status_active_with_bridge_fleet_bundle_and_health(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            schema = root / "schema.surql"
            schema.write_text("-- schema\n", encoding="utf-8")
            out = root / "bundle"
            activation = StoragePrimaryActivation(REGISTRY)
            activation.write_bundle(out, schema)
            status = activation.activation_status(
                bundle_dir=out,
                bridge_status={"status": "active"},
                fleet_report={
                    "effective": [
                        {
                            "name": "cynic-lxc-surrealdb",
                            "observed_status": "running",
                            "effective_status": "active",
                        }
                    ]
                },
                mesh_status={
                    "policy_status": "ok",
                    "dns_status": "ok",
                    "runtime_status": "not_observed_without_wireguard_agent",
                    "dns_checks": [
                        {
                            "checks": [
                                {
                                    "name": "organ-surrealdb.synapse.cynic",
                                    "expected_ip": "10.20.0.3",
                                    "resolved_ips": ["10.20.0.3"],
                                    "status": "resolved_expected_ip",
                                }
                            ]
                        }
                    ],
                },
                health={"healthy": True},
            )

        self.assertEqual(status["status"], "active")
        self.assertEqual("cynic.storage_primary_activation.status.v1", status["surface_contract"]["schema"])
        self.assertEqual([], status["missing"])
        self.assertTrue(all(item["status"] == "satisfied" for item in status["proof_requirements"]))

    def test_status_blocks_when_mesh_dns_not_checked(self):
        status = StoragePrimaryActivation(REGISTRY).activation_status(
            mesh_status={
                "policy_status": "ok",
                "dns_status": "not_checked",
                "runtime_status": "not_observed_without_wireguard_agent",
                "dns_checks": [],
            }
        )

        self.assertEqual(status["status"], "blocked")
        self.assertIn("mesh_reaches_storage", status["missing"])

    def test_cli_status_accepts_evidence_stdin(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            registry_path = root / "registry.json"
            schema = root / "schema.surql"
            bundle = root / "bundle"
            registry_path.write_text(json.dumps(REGISTRY), encoding="utf-8")
            schema.write_text("-- schema\n", encoding="utf-8")
            activation = StoragePrimaryActivation(REGISTRY)
            activation.write_bundle(bundle, schema)
            evidence = {
                "bridge_status": {"status": "active"},
                "fleet_report": {
                    "effective": [
                        {
                            "name": "cynic-lxc-surrealdb",
                            "observed_status": "running",
                            "effective_status": "active",
                        }
                    ]
                },
                "mesh_status": {
                    "policy_status": "ok",
                    "dns_status": "ok",
                    "dns_checks": [
                        {
                            "checks": [
                                {
                                    "name": "organ-surrealdb.synapse.cynic",
                                    "expected_ip": "10.20.0.3",
                                    "status": "resolved_expected_ip",
                                }
                            ]
                        }
                    ],
                },
                "health": {"healthy": True},
            }

            result = subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "organs/keep/src/entrypoints/cli.py"),
                    "storage-primary",
                    "status",
                    "--registry",
                    str(registry_path),
                    "--bundle-dir",
                    str(bundle),
                    "--evidence-stdin",
                ],
                check=False,
                input=json.dumps(evidence),
                text=True,
                capture_output=True,
            )

        self.assertEqual(result.returncode, 0, result.stderr)
        payload = json.loads(result.stdout)
        self.assertEqual(payload["status"], "active")
        self.assertEqual(payload["missing"], [])

    def test_cli_status_builds_fleet_report_from_proxmox_list_text(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            registry_path = root / "registry.json"
            schema = root / "schema.surql"
            bundle = root / "bundle"
            proxmox_text = root / "proxmox.txt"
            registry_path.write_text(
                json.dumps(
                    {
                        "nodes": [
                            {
                                "name": "cynic-lxc-surrealdb",
                                "role": "storage_primary",
                                "status": "planned",
                                "aliases": ["organ-surrealdb"],
                                "network": {"synapse": "10.20.0.3"},
                                "services": [{"name": "surrealdb", "port": 8000}],
                                "vms": [
                                    {
                                        "name": "cynic-lxc-surrealdb",
                                        "id": "lxc/103",
                                    }
                                ],
                            }
                        ],
                        "organs": [{"name": "organ-keep"}],
                    }
                ),
                encoding="utf-8",
            )
            schema.write_text("-- schema\n", encoding="utf-8")
            StoragePrimaryActivation(REGISTRY).write_bundle(bundle, schema)
            proxmox_text.write_text(
                """=== Node: pve ===
Containers (LXC):
- ID: 103, Name: cynic-lxc-surrealdb, Status: running, Mem: 2048MB
""",
                encoding="utf-8",
            )

            result = subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "organs/keep/src/entrypoints/cli.py"),
                    "storage-primary",
                    "status",
                    "--registry",
                    str(registry_path),
                    "--bundle-dir",
                    str(bundle),
                    "--proxmox-list-text",
                    str(proxmox_text),
                ],
                check=False,
                text=True,
                capture_output=True,
            )

        self.assertEqual(result.returncode, 1, result.stderr)
        payload = json.loads(result.stdout)
        self.assertTrue(payload["checks"]["fleet_observes_node_running"])
        self.assertNotIn("fleet_observes_node_running", payload["missing"])


if __name__ == "__main__":
    unittest.main()
