import json
import os
import subprocess
import unittest
from unittest import mock

from organs.forge.src.adapters.kernel_adapter import KernelRestClient
from organs.forge.src.core.entities import FleetProbe
from organs.forge.src.core.lifecycle_readiness import (
    _run_json_program,
    _run_json_shell,
    inspect_fleet,
    probe_local_network_runtime,
    probe_fleet_node_exec_transport,
)
from organs.forge.src.tests.fixtures import registry_fixture


class LifecycleFleetReadinessTests(unittest.TestCase):
    def test_run_json_shell_returns_structured_timeout_metadata(self):
        with mock.patch(
            "organs.forge.src.core.lifecycle_readiness.subprocess.run",
            side_effect=subprocess.TimeoutExpired(["bash", "-lc", "echo"], timeout=3, output="", stderr=""),
        ):
            payload, metadata = _run_json_shell("echo", timeout=3)

        self.assertIsNone(payload)
        self.assertEqual(metadata["error"], "timeout")
        self.assertEqual(metadata["timeout_seconds"], 3)
        self.assertEqual(metadata["command"], "echo")

    def test_run_json_program_returns_structured_timeout_metadata(self):
        with mock.patch(
            "organs.forge.src.core.lifecycle_readiness.subprocess.run",
            side_effect=subprocess.TimeoutExpired(["python3", "tool.py"], timeout=7, output="", stderr=""),
        ):
            payload, metadata = _run_json_program(["python3", "tool.py"], {"x": 1}, timeout=7)

        self.assertIsNone(payload)
        self.assertEqual(metadata["error"], "timeout")
        self.assertEqual(metadata["timeout_seconds"], 7)
        self.assertEqual(metadata["argv"], ["python3", "tool.py"])

    def test_probe_fleet_node_exec_transport_enriches_mcp_initialize_failure_with_launcher_check(self):
        with mock.patch(
            "organs.forge.src.core.lifecycle_readiness._run_json_program",
            return_value=(
                {
                    "status": "blocked",
                    "ok": False,
                    "error": "mcp_initialize_failed",
                    "transport": "mcp_stdio_fleet_node_exec",
                    "stderr": "stale mcp",
                },
                {"argv": ["python3"], "exit_code": 1, "stdout": "{}", "stderr": ""},
            ),
        ), mock.patch(
            "organs.forge.src.core.lifecycle_readiness._probe_mcp_launcher_freshness",
            return_value={
                "schema": "cynic.mcp_launcher_check.v1",
                "status": "stale",
                "canonical_binary": "/repo/target/debug/cynic-mcp",
                "repair_local_debug_binary": True,
                "stale_source": "/repo/crates/cynic-mcp/src/helpers_split/misc.rs",
            },
        ):
            probe = probe_fleet_node_exec_transport("ubuntu")

        self.assertEqual(probe["error"], "mcp_initialize_failed")
        self.assertEqual(probe["launcher_status"], "stale")
        self.assertTrue(probe["repair_local_debug_binary"])
        self.assertEqual(probe["canonical_binary"], "/repo/target/debug/cynic-mcp")
        self.assertEqual(
            probe["stale_source"],
            "/repo/crates/cynic-mcp/src/helpers_split/misc.rs",
        )

    def test_probe_fleet_node_exec_transport_surfaces_structured_mcp_tool_error(self):
        with mock.patch(
            "organs.forge.src.core.lifecycle_readiness._run_json_program",
            return_value=(
                {
                    "status": "precondition_missing",
                    "ok": False,
                    "missing": "host_exec_transport",
                    "reason": "Registry node is a host-native surface, not a Proxmox guest resource. fleet_node_exec currently covers Proxmox guests and the LXC exec bridge only.",
                    "required_bridge": "declare a canonical host exec transport for cortex/manual hosts before using fleet_node_exec on this node",
                    "declared_role": "cortex_host",
                    "transport": "mcp_stdio_fleet_node_exec",
                },
                {"argv": ["python3"], "exit_code": 1, "stdout": "{}", "stderr": ""},
            ),
        ):
            probe = probe_fleet_node_exec_transport("ubuntu")

        self.assertEqual(probe["status"], "precondition_missing")
        self.assertEqual(probe["missing"], "host_exec_transport")
        self.assertEqual(
            probe["required_bridge"],
            "declare a canonical host exec transport for cortex/manual hosts before using fleet_node_exec on this node",
        )

    def test_probe_local_network_runtime_surfaces_host_exec_activation_gap(self):
        def fake_run_json_shell(command, *, timeout=30):
            if command == "cargo run -q -p cynic-node -- observe --json":
                return (
                    {
                        "hostname": "ubuntu",
                        "severity": "warning",
                        "status": "provider_fallback_active",
                        "network_mesh_status": {"runtime_status": "provider_fallback_active"},
                        "blockers": [],
                    },
                    {"command": command, "exit_code": 0, "stdout": "{}", "stderr": ""},
                )
            if command == "cargo run -q -p cynic-node -- network actuator-status --json":
                return (
                    {
                        "status": "missing_activation",
                        "root_authority": {
                            "command": "sudo -n true",
                            "sudo_noninteractive_available": False,
                        },
                    },
                    {"command": command, "exit_code": 0, "stdout": "{}", "stderr": ""},
                )
            if command == "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-status --bundle-dir /tmp/cynic-host-exec":
                return (
                    {
                        "status": "missing_activation",
                        "missing": ["helper_exists", "sudoers_exists"],
                        "helper_path": "/usr/local/libexec/cynic-host-exec",
                        "sudoers_rule": "cynic ALL=(root) NOPASSWD: /usr/local/libexec/cynic-host-exec",
                        "next_actions": ["build_host_exec_bundle", "install_host_exec_bundle_on_target_node"],
                        "proof_requirements": [{"check": "bundle_complete"}],
                        "bundle_complete": False,
                    },
                    {"command": command, "exit_code": 0, "stdout": "{}", "stderr": ""},
                )
            raise AssertionError(command)

        with mock.patch(
            "organs.forge.src.core.lifecycle_readiness._run_json_shell",
            side_effect=fake_run_json_shell,
        ), mock.patch(
            "organs.forge.src.core.lifecycle_readiness.probe_fleet_node_exec_transport",
            return_value={"status": "transport_failed"},
        ):
            probe = probe_local_network_runtime()

        host_exec_blocker = next(
            item for item in probe["blockers"] if item["kind"] == "host_exec_activation"
        )
        self.assertEqual(host_exec_blocker["code"], "network_host_exec:missing_activation")
        self.assertEqual(host_exec_blocker["helper_path"], "/usr/local/libexec/cynic-host-exec")
        self.assertIn("sudoers_exists", host_exec_blocker["missing"])
        self.assertIn("build_host_exec_bundle", host_exec_blocker["next_actions"])
        self.assertFalse(host_exec_blocker["bundle_complete"])

    def test_probe_local_network_runtime_prefers_canonical_mesh_runtime_projection(self):
        def fake_run_json_shell(command, *, timeout=30):
            if command == "cargo run -q -p cynic-node -- observe --json":
                return (
                    {
                        "hostname": "ubuntu",
                        "severity": "warning",
                        "status": "provider_fallback_active",
                        "network_mesh_status": {"runtime_status": "mesh_active"},
                        "blockers": ["wireguard_mesh_absent"],
                    },
                    {"command": command, "exit_code": 0, "stdout": "{}", "stderr": ""},
                )
            if command == "cargo run -q -p cynic-node -- network actuator-status --json":
                return (
                    {"status": "active"},
                    {"command": command, "exit_code": 0, "stdout": "{}", "stderr": ""},
                )
            if command == "python3 organs/fleet/src/entrypoints/cli.py host-exec activation-status --bundle-dir /tmp/cynic-host-exec":
                return (
                    {"status": "active"},
                    {"command": command, "exit_code": 0, "stdout": "{}", "stderr": ""},
                )
            raise AssertionError(command)

        with mock.patch(
            "organs.forge.src.core.lifecycle_readiness._run_json_shell",
            side_effect=fake_run_json_shell,
        ):
            probe = probe_local_network_runtime()

        runtime_blocker = next(
            item for item in probe["blockers"] if item["kind"] == "runtime_blocker"
        )
        self.assertEqual(runtime_blocker["runtime_status"], "mesh_active")

    def test_lifecycle_blockers_combine_activation_status_and_fleet(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.lifecycle = lifecycle
        client.build_node = registry["nodes"][0]
        client.deploy_node = registry["nodes"][1]
        client.fleet_probe = {
            "status": "ok",
            "missing": ["cynic-vm-anvil", "cynic-vm-core"],
            "matches": {},
        }
        client.fleet_inventory_probe = {"enabled": True, "status": "ok"}

        blockers = client._lifecycle_blockers(
            activation_gap_details=client._activation_evidence_gap_details(),
            required_status="active",
        )
        categories = [blocker["category"] for blocker in blockers]

        self.assertEqual(categories.count("activation"), 10)
        self.assertEqual(categories.count("status"), 2)
        self.assertEqual(categories.count("fleet"), 2)
        self.assertIn("cynic-vm-anvil:status_planned_not_active", [b["code"] for b in blockers])
        self.assertIn("cynic-vm-core:fleet_missing", [b["code"] for b in blockers])

    def test_lifecycle_blockers_include_fleet_probe_failure(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.lifecycle = lifecycle
        client.build_node = registry["nodes"][0]
        client.deploy_node = registry["nodes"][1]
        client.fleet_probe = {"status": "error", "error": "synapse_status_failed"}
        client.fleet_inventory_probe = {"enabled": True, "status": "ok"}

        blockers = client._lifecycle_blockers(
            activation_gap_details=[],
            required_status="active",
        )

        self.assertIn("fleet_probe:error", [blocker["code"] for blocker in blockers])
        fleet_probe = next(blocker for blocker in blockers if blocker["code"] == "fleet_probe:error")
        self.assertEqual(fleet_probe["kind"], "synapse_probe")
        self.assertEqual(fleet_probe["error"], "synapse_status_failed")
        self.assertEqual(fleet_probe["provider"], "synapse")
        self.assertEqual(fleet_probe["resolver_organ"], "organ-fleet")
        self.assertEqual(fleet_probe["resolver_consumer"], "organ-forge-hermes-agent")

    def test_inspect_fleet_reports_missing_synapse_cli_actionably(self):
        with mock.patch(
            "organs.forge.src.core.lifecycle_readiness.subprocess.run",
            side_effect=FileNotFoundError(),
        ):
            probe = inspect_fleet("cynic-vm-anvil", "cynic-vm-v2")

        self.assertEqual(probe.status, "degraded")
        self.assertEqual(probe.error, "synapse_cli_unavailable")
        self.assertEqual(probe.targets, ["cynic-vm-anvil", "cynic-vm-v2"])

    def test_lifecycle_blocker_for_missing_synapse_cli_names_restoring_organ(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.lifecycle = lifecycle
        client.build_node = registry["nodes"][0]
        client.deploy_node = registry["nodes"][1]
        client.fleet_probe = {
            "status": "degraded",
            "error": "synapse_cli_unavailable",
            "targets": ["cynic-vm-anvil", "cynic-vm-v2"],
        }
        client.fleet_inventory_probe = {"enabled": True, "status": "ok"}

        blockers = client._lifecycle_blockers(
            activation_gap_details=[],
            required_status="active",
        )
        blocker = next(item for item in blockers if item["kind"] == "synapse_probe")

        self.assertEqual(blocker["provider"], "synapse")
        self.assertEqual(blocker["resolver_organ"], "organ-fleet")
        self.assertEqual(blocker["resolver_consumer"], "organ-forge-hermes-agent")
        self.assertIn("operator_action", blocker)
        self.assertIn("organs/fleet", blocker["command"])

    def test_lifecycle_blockers_include_fleet_inventory_evidence_gaps(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.lifecycle = lifecycle
        client.build_node = registry["nodes"][0]
        client.deploy_node = registry["nodes"][1]
        client.fleet_probe = {"status": "ok", "missing": [], "matches": {}}
        client.fleet_inventory_probe = {
            "enabled": True,
            "status": "degraded",
            "summary": {
                "provider_evidence_gaps": [
                    "fleet:missing_node:cynic-vm-core",
                    "fleet:unexpected_node:rogue-node",
                ],
                "declaration_missing_nodes": ["cynic-vm-core"],
                "declaration_unexpected_nodes": ["rogue-node"],
            },
        }

        blockers = client._lifecycle_blockers(
            activation_gap_details=[],
            required_status="active",
        )
        codes = [blocker["code"] for blocker in blockers if blocker["category"] == "fleet"]

        self.assertIn("fleet_inventory:degraded", codes)
        self.assertIn("fleet:missing_node:cynic-vm-core", codes)
        self.assertIn("cynic-vm-core:fleet_inventory_missing", codes)
        fleet_inventory = next(
            blocker for blocker in blockers if blocker["code"] == "fleet_inventory:degraded"
        )
        self.assertEqual(fleet_inventory["provider"], "organ-fleet")
        self.assertEqual(fleet_inventory["resolver_organ"], "organ-fleet")
        self.assertIn("organs/fleet", fleet_inventory["command"])

    def test_fleet_inventory_probe_extraction_from_health_payload(self):
        client = KernelRestClient.__new__(KernelRestClient)
        health_payload = {
            "environment": {
                "probes": [
                    {
                        "name": "fleet_inventory",
                        "status": "ok",
                        "duration_ms": 12,
                        "details": {
                            "kind": "fleet_inventory",
                            "data": {
                                "nodes": [{"node_id": "vm-a"}],
                                "summary": {
                                    "declared_nodes": 1,
                                    "observed_nodes": 1,
                                    "matched_nodes": 1,
                                    "missing_nodes": 0,
                                    "unexpected_nodes": 0,
                                    "provider_evidence_gaps": [],
                                },
                            },
                        },
                    }
                ]
            }
        }

        result = client._extract_fleet_inventory_from_health(health_payload)

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["name"], "fleet_inventory")
        self.assertEqual(result["summary"]["declared_nodes"], 1)
        self.assertEqual(result["duration_ms"], 12)
        self.assertEqual(result["nodes"][0]["node_id"], "vm-a")

    def test_fleet_inventory_probe_extraction_from_organ_fleet_report(self):
        report = {
            "summary": {
                "status": "degraded",
                "declared_nodes": 3,
                "observed_resources": 1,
                "critical_anomaly_count": 2,
            },
            "anomalies": [
                {
                    "severity": "critical",
                    "kind": "active_without_runtime_proof",
                    "target": "cynic-vm-anvil",
                },
                {
                    "severity": "critical",
                    "kind": "active_without_runtime_proof",
                    "target": "cynic-vm-v2",
                },
                {
                    "severity": "warning",
                    "kind": "organ_dir_without_registry_state",
                    "target": "organ-rag",
                },
            ],
        }

        result = KernelRestClient._extract_fleet_inventory_from_fleet_report(report)

        self.assertEqual(result["status"], "degraded")
        self.assertEqual(result["name"], "organ_fleet_report")
        self.assertEqual(result["summary"]["declared_nodes"], 3)
        self.assertEqual(result["summary"]["observed_nodes"], 1)
        self.assertEqual(result["summary"]["missing_nodes"], 2)
        self.assertEqual(
            result["summary"]["provider_evidence_gaps"],
            [
                "active_without_runtime_proof:cynic-vm-anvil",
                "active_without_runtime_proof:cynic-vm-v2",
            ],
        )

    def test_lifecycle_context_normalizes_legacy_fleet_inventory_probe_object(self):
        client = KernelRestClient.__new__(KernelRestClient)
        client.registry = registry_fixture()
        client.lifecycle = client.registry["lifecycle"]["kernel_release"]
        client.build_node = client.registry["nodes"][0]
        client.deploy_node = client.registry["nodes"][1]
        client.build_target = client.build_node["name"]
        client.deploy_target = client.deploy_node["name"]
        client.base_url = "http://kernel.local"
        client.kernel_endpoint_source = "test"
        client.kernel_bus_canonical = True
        client.kernel_probe = {"enabled": False, "status": "not_run"}
        client.fleet_probe = FleetProbe.not_run()
        client.fleet_inventory_probe = FleetProbe.not_run()

        context = client.lifecycle_context()

        self.assertIn(context["fleet_inventory_probe"]["status"], {"ok", "degraded", "error"})
        self.assertEqual(context["fleet_inventory_probe"]["name"], "organ_fleet_report")

    def test_lifecycle_context_uses_organ_fleet_report_when_kernel_inventory_not_run(self):
        client = KernelRestClient.__new__(KernelRestClient)
        client.registry = registry_fixture()
        client.lifecycle = client.registry["lifecycle"]["kernel_release"]
        client.build_node = client.registry["nodes"][0]
        client.deploy_node = client.registry["nodes"][1]
        client.build_target = client.build_node["name"]
        client.deploy_target = client.deploy_node["name"]
        client.base_url = "http://kernel.local"
        client.kernel_endpoint_source = "test"
        client.kernel_bus_canonical = True
        client.kernel_probe = {"enabled": False, "status": "not_run"}
        client.fleet_probe = FleetProbe.not_run()
        client.fleet_inventory_probe = FleetProbe.not_run()

        with mock.patch.object(client, "_probe_kernel_bus", side_effect=RuntimeError("offline")), \
            mock.patch.object(client, "_probe_fleet_inventory_report") as fleet_report:
            fleet_report.side_effect = lambda: setattr(
                client,
                "fleet_inventory_probe",
                {"enabled": True, "status": "degraded", "summary": {"missing_nodes": 1}},
            )
            context = client.lifecycle_context()

        self.assertEqual(context["fleet_inventory_probe"]["status"], "degraded")
        fleet_report.assert_called_once_with()

    def test_fleet_inventory_report_probe_accepts_proxmox_evidence_path(self):
        client = KernelRestClient.__new__(KernelRestClient)
        client.fleet_inventory_probe = FleetProbe.not_run().to_dict()
        completed = mock.Mock()
        completed.returncode = 0
        completed.stdout = json.dumps(
            {
                "summary": {
                    "status": "ok",
                    "declared_nodes": 2,
                    "observed_resources": 2,
                    "critical_anomaly_count": 0,
                },
                "anomalies": [],
            }
        )

        with mock.patch.dict(
            os.environ,
            {"CYNIC_FLEET_PROXMOX_LIST_TEXT": "/tmp/proxmox-list.txt"},
        ), mock.patch(
            "organs.forge.src.adapters.fleet_evidence_adapter.subprocess.run",
            return_value=completed,
        ) as run:
            client._probe_fleet_inventory_report()

        command = run.call_args.args[0]
        self.assertIn("--proxmox-list-text", command)
        self.assertIn("/tmp/proxmox-list.txt", command)
        self.assertEqual(client.fleet_inventory_probe["status"], "ok")
