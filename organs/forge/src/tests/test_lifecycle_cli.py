import json
from io import StringIO
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from organs.forge.src.entrypoints import (
    cli,
    lifecycle_activation_cli,
    lifecycle_cli,
    lifecycle_diagnostic_cli,
)
from organs.forge.src.tests.fixtures import PROJECT_ROOT


class LifecycleCliTests(unittest.TestCase):
    def test_kernel_url_cli_uses_shared_normalizer(self):
        with mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = lifecycle_cli.run_kernel_url("http://100.64.0.1:3030/")

        self.assertEqual(status, 0)
        self.assertEqual(stdout.getvalue().strip(), "http://100.64.0.1:3030")

    def test_lifecycle_error_context_includes_blockers(self):
        diagnostic = mock.Mock()
        diagnostic.lifecycle_context.return_value = {
            "lifecycle_blockers": [{"category": "fleet", "code": "node:fleet_missing"}],
            "activation_evidence_gap_details": [],
            "status_nodes_ready": False,
            "fleet_nodes_ready": False,
            "lifecycle_nodes_ready": False,
        }

        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "KernelRestClient",
            return_value=diagnostic,
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_bootstrap_source",
            return_value={
                "status": "blocked",
                "reason": "bootstrap_source_contract_stale",
                "selected_remote": "http://localhost:18300/jeanterre13/CYNIC.git",
                "selected_ref": "main",
            },
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_consumer_runtime",
            return_value={
                "status": "blocked",
                "runtime_user": "cynic",
                "runtime_user_active": False,
                "cargo_available": False,
                "problems": ["runtime_user_not_active", "cargo_missing"],
                "operator_action": "restore runtime consumer",
            },
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_network_runtime",
            return_value={
                "enabled": True,
                "status": "blocked",
                "blockers": [
                    {
                        "category": "network",
                        "kind": "runtime_blocker",
                        "code": "network_runtime:canonical_dns_projection_drift",
                        "blocker": "canonical_dns_projection_drift",
                    }
                ],
            },
        ):
            context = lifecycle_diagnostic_cli.lifecycle_error_context()

        self.assertEqual(context["diagnostic_status"], "ok")
        self.assertEqual(context["lifecycle_blockers"][0]["category"], "fleet")
        self.assertEqual(context["lifecycle_blockers"][1]["category"], "bootstrap_source")
        self.assertEqual(context["lifecycle_blockers"][2]["category"], "network")
        self.assertEqual(
            context["bootstrap_source"]["reason"],
            "bootstrap_source_contract_stale",
        )
        self.assertEqual(context["runtime_probe"]["problems"], ["runtime_user_not_active", "cargo_missing"])
        self.assertEqual(context["network_probe"]["status"], "blocked")
        self.assertFalse(context["lifecycle_nodes_ready"])
        diagnostic.inspect_fleet.assert_called_once_with()

    def test_lifecycle_check_surfaces_bootstrap_source_in_output(self):
        diagnostic = mock.Mock()
        diagnostic.lifecycle_context.return_value = {
            "lifecycle": "kernel_release",
            "lifecycle_blockers": [],
            "status_nodes_ready": True,
            "fleet_nodes_ready": True,
            "lifecycle_nodes_ready": True,
        }

        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "KernelRestClient",
            return_value=diagnostic,
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_bootstrap_source",
            return_value={
                "status": "blocked",
                "reason": "bootstrap_source_unreachable",
                "selected_remote": None,
                "selected_ref": "main",
            },
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_consumer_runtime",
            return_value={"status": "ok"},
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_network_runtime",
            return_value={"enabled": True, "status": "ok", "blockers": []},
        ), mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = lifecycle_diagnostic_cli.run_lifecycle_check(strict=False)

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertFalse(payload["lifecycle_nodes_ready"])
        self.assertEqual(payload["lifecycle_blockers"][0]["category"], "bootstrap_source")
        self.assertEqual(payload["bootstrap_source"]["reason"], "bootstrap_source_unreachable")
        diagnostic.inspect_fleet.assert_called_once_with()

    def test_lifecycle_mcp_freshness_cli_emits_dispatch_plan(self):
        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "lifecycle_mcp_freshness_payload",
            return_value={
                "status": "ok",
                "action": "dispatch_lifecycle_profile",
                "dispatch": {"profile": "mcp-reload"},
            },
        ), mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = lifecycle_diagnostic_cli.run_lifecycle_mcp_freshness()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["dispatch"]["profile"], "mcp-reload")

    def test_lifecycle_dispatch_error_payload_classifies_missing_auth(self):
        with mock.patch(
            "organs.forge.src.entrypoints.lifecycle_diagnostic_cli.lifecycle_error_context",
            return_value={"diagnostic_status": "ok", "lifecycle_blockers": []},
        ):
            payload = lifecycle_cli.lifecycle_dispatch_error_payload(
                RuntimeError("CYNIC_API_KEY is required for organ-forge kernel dispatch")
            )

        self.assertEqual(payload["status"], "error")
        self.assertEqual(payload["error_kind"], "missing_kernel_auth")
        self.assertIn("organ-forge environment", payload["operator_action"])
        self.assertFalse(payload["local_execution_policy"]["builds_on_operator_machine"])

    def test_lifecycle_dispatch_error_payload_classifies_sandbox_network_denial(self):
        with mock.patch(
            "organs.forge.src.entrypoints.lifecycle_diagnostic_cli.lifecycle_error_context",
            return_value={"diagnostic_status": "ok", "lifecycle_blockers": []},
        ):
            payload = lifecycle_cli.lifecycle_dispatch_error_payload(
                RuntimeError("agent task dispatch failed: <urlopen error [Errno 1] Operation not permitted>")
            )

        self.assertEqual(payload["error_kind"], "sandbox_network_denied")
        self.assertIn("MCP", payload["operator_action"])

    def test_delegate_profile_json_emits_single_aggregate_object(self):
        calls = []

        async def fake_delegate_intent(repo_path, intent, emit_logs=True, strict_lifecycle=True):
            calls.append({"intent": intent, "strict_lifecycle": strict_lifecycle})
            return {"intent": intent, "status": "ok", "result": f"{intent}:queued"}

        with mock.patch.object(
            cli,
            "lifecycle_profile_intents",
            return_value=["build-and-test", "integration-test"],
        ), mock.patch.object(cli, "delegate_intent", side_effect=fake_delegate_intent), mock.patch(
            "sys.stdout",
            new_callable=StringIO,
        ) as stdout:
            status = __import__("asyncio").run(
                cli.run_delegate_profile("/repo", "gate-2", json_mode=True)
            )

        lines = [line for line in stdout.getvalue().splitlines() if line]
        self.assertEqual(status, 0)
        self.assertEqual(len(lines), 1)
        payload = json.loads(lines[0])
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["profile"], "gate-2")
        self.assertEqual(len(payload["results"]), 2)
        self.assertEqual(
            calls,
            [
                {"intent": "build-and-test", "strict_lifecycle": True},
                {"intent": "integration-test", "strict_lifecycle": True},
            ],
        )

    def test_delegate_profile_uses_non_strict_lifecycle_for_bootstrap(self):
        calls = []

        async def fake_delegate_intent(repo_path, intent, emit_logs=True, strict_lifecycle=True):
            calls.append({"intent": intent, "strict_lifecycle": strict_lifecycle})
            return {"intent": intent, "status": "ok", "result": f"{intent}:queued"}

        with mock.patch.object(
            cli,
            "lifecycle_profile_intents",
            return_value=["bootstrap-build-runner"],
        ), mock.patch.object(cli, "delegate_intent", side_effect=fake_delegate_intent), mock.patch(
            "sys.stdout",
            new_callable=StringIO,
        ) as stdout:
            status = __import__("asyncio").run(
                cli.run_delegate_profile("/repo", "bootstrap", json_mode=True)
            )

        payload = json.loads(stdout.getvalue())
        self.assertEqual(status, 0)
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["profile"], "bootstrap")
        self.assertEqual(calls, [{"intent": "bootstrap-build-runner", "strict_lifecycle": False}])

    def test_lifecycle_blockers_payload_is_concise(self):
        activation_summary = [
            {
                "action": "restore_fleet_probe",
                "node": None,
                "role": None,
                "code": None,
                "targets": ["organs/fleet"],
                "coord_targets": ["organs/fleet"],
                "consumer": "organ-forge-hermes-agent",
                "priority": 98,
            }
        ]
        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "build_lifecycle_activation_requests",
            return_value={
                "status": "blocked",
                "activation_schema_version": "activation.v1",
                "lifecycle": "kernel_release",
                "build_target": "cynic-vm-anvil",
                "deploy_target": "cynic-vm-core",
                "status_nodes_ready": False,
                "fleet_nodes_ready": False,
                "lifecycle_nodes_ready": False,
                "blockers": [{"category": "fleet", "code": "node:fleet_missing"}],
                "requests": [{"kind": "hermes"}],
                "request_summary": activation_summary,
            },
        ) as activation_requests, mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_bootstrap_source",
            return_value={
                "status": "blocked",
                "reason": "bootstrap_source_contract_stale",
                "selected_remote": "http://localhost:18300/jeanterre13/CYNIC.git",
                "selected_ref": "main",
            },
        ) as bootstrap_probe, mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_consumer_runtime",
            return_value={
                "status": "blocked",
                "runtime_user": "cynic",
                "runtime_user_active": False,
                "cargo_available": False,
                "problems": ["runtime_user_not_active", "cargo_missing"],
                "operator_action": "restore runtime consumer",
            },
        ) as runtime_probe, mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_network_runtime",
            return_value={
                "enabled": True,
                "status": "blocked",
                "blockers": [
                    {"category": "network", "code": "network_runtime:canonical_dns_projection_drift"},
                    {
                        "category": "network",
                        "kind": "remote_exec_transport",
                        "code": "network_remote_exec_transport:mcp_initialize_failed",
                        "actual": "mcp_initialize_failed",
                        "required": "ok",
                    },
                ],
            },
        ) as network_probe:
            payload = lifecycle_diagnostic_cli.lifecycle_blockers_payload()

        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["activation_schema_version"], "activation.v1")
        self.assertEqual(payload["lifecycle"], "kernel_release")
        self.assertEqual(payload["build_target"], "cynic-vm-anvil")
        self.assertEqual(payload["blockers"][0]["category"], "fleet")
        self.assertEqual(payload["blockers"][1]["category"], "bootstrap_source")
        self.assertEqual(payload["blockers"][1]["code"], "bootstrap_source_contract_stale")
        self.assertEqual(payload["blockers"][2]["category"], "network")
        self.assertEqual(payload["blockers"][3]["code"], "network_remote_exec_transport:mcp_initialize_failed")
        self.assertEqual(payload["runtime_probe"]["problems"], ["runtime_user_not_active", "cargo_missing"])
        self.assertEqual(
            payload["bootstrap_source"]["selected_remote"],
            "http://localhost:18300/jeanterre13/CYNIC.git",
        )
        self.assertEqual(payload["activation_request_count"], 1)
        self.assertEqual(payload["activation_request_summary"], activation_summary)
        self.assertIn("lifecycle-activation-dispatch", payload["activation_dispatch_command"])
        activation_requests.assert_called_once_with(
            inspect_fleet=True,
            include_network_probe=True,
        )
        bootstrap_probe.assert_called_once_with()
        runtime_probe.assert_called_once_with()
        network_probe.assert_called_once_with()

    def test_lifecycle_root_install_sequence_payload_is_minimal_and_ordered(self):
        root_install_sequence = [
            {
                "order": 1,
                "action": "install_host_exec_bundle",
                "dependency": "required_before_remote_network_repair",
                "manual_root_install_command": "install host-exec",
            },
            {
                "order": 2,
                "action": "install_network_actuator_bundle",
                "dependency": "requires_host_exec_transport",
                "manual_root_install_command": "install network-actuator",
            },
        ]
        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "build_lifecycle_activation_requests",
            return_value={
                "status": "blocked",
                "activation_schema_version": "activation.v1",
                "lifecycle": "kernel_release",
                "build_target": "cynic-vm-anvil",
                "deploy_target": "cynic-vm-core",
                "blockers": [{"category": "network", "code": "network_host_exec:missing_activation"}],
                "root_install_sequence": root_install_sequence,
                "requests": [],
            },
        ) as activation_requests, mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_bootstrap_source",
            return_value={"status": "blocked", "reason": "bootstrap_source_unreachable"},
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_consumer_runtime",
            return_value={"status": "blocked", "runtime_user_active": False},
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_network_runtime",
            return_value={"enabled": True, "status": "blocked", "blockers": []},
        ):
            payload = lifecycle_diagnostic_cli.lifecycle_root_install_sequence_payload()

        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["root_install_count"], 2)
        self.assertEqual(payload["root_install_sequence"], root_install_sequence)
        self.assertEqual(
            [step["action"] for step in payload["root_install_sequence"]],
            ["install_host_exec_bundle", "install_network_actuator_bundle"],
        )
        self.assertEqual(payload["network_probe_status"], "blocked")
        self.assertIn("network_host_exec:missing_activation", payload["blocker_codes"])
        self.assertIn("lifecycle-root-install-sequence", payload["source_command"])
        activation_requests.assert_called_once_with(
            inspect_fleet=True,
            include_network_probe=True,
        )

    def test_organism_gap_map_payload_groups_observed_and_structural_gaps(self):
        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "build_lifecycle_activation_requests",
            return_value={
                "status": "blocked",
                "activation_schema_version": "activation.v1",
                "lifecycle": "kernel_release",
                "build_target": "cynic-vm-anvil",
                "deploy_target": "cynic-vm-core",
                "blockers": [
                    {
                        "category": "network",
                        "kind": "host_exec_activation",
                        "code": "network_host_exec:missing_activation",
                        "hostname": "ubuntu",
                        "resolver_organ": "organ-fleet",
                    },
                    {
                        "category": "network",
                        "kind": "runtime_blocker",
                        "code": "network_runtime:wireguard_mesh_absent",
                        "hostname": "ubuntu",
                        "resolver_organ": "cynic-node",
                    },
                ],
                "root_install_sequence": [{"action": "install_host_exec_bundle"}],
                "requests": [],
            },
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_bootstrap_source",
            return_value={"status": "blocked", "reason": "bootstrap_source_unreachable"},
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_consumer_runtime",
            return_value={"status": "blocked", "runtime_user_active": False},
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_network_runtime",
            return_value={"enabled": True, "status": "blocked", "blockers": []},
        ), mock.patch(
            "organs.forge.src.core.lifecycle_diagnostic_payloads._registry_network_policy_summary",
            return_value={
                "version": "network-policy.v1",
                "owner": "organ-wireguard",
                "owner_status": "planned",
                "canonical_service_plane": "synapse",
                "canonical_plane_contract": {
                    "cidr": "10.20.0.0/24",
                    "dns_zone": "synapse.cynic",
                    "canonical": True,
                },
            },
        ), mock.patch(
            "organs.forge.src.core.lifecycle_diagnostic_payloads._ownership_drift_reference_map",
            return_value={
                "reference_count": 5,
                "surface_class_counts": {
                    "desired_state_owner": 1,
                    "registry_reference": 1,
                    "contract_reference": 1,
                    "mcp_runtime_reference": 1,
                    "legacy_organ_implementation": 1,
                },
                "references": [
                    {
                        "path": "infra/registry.json",
                        "line": 1254,
                        "surface_class": "desired_state_owner",
                        "reference": '"owner": "organ-wireguard",',
                    },
                    {
                        "path": "crates/cynic-mcp/src/routers/local.rs",
                        "line": 352,
                        "surface_class": "mcp_runtime_reference",
                        "reference": 'description = "Render ... organ-wireguard"',
                    },
                    {
                        "path": "infra/registry.json",
                        "line": 807,
                        "surface_class": "registry_reference",
                        "reference": '"name": "organ-wireguard",',
                    },
                    {
                        "path": "axioms/infrastructure-mcp-v1.json",
                        "line": 298,
                        "surface_class": "contract_reference",
                        "reference": '"organ": "organ-wireguard",',
                    },
                    {
                        "path": "organs/wireguard/MANIFEST.yaml",
                        "line": 1,
                        "surface_class": "legacy_organ_implementation",
                        "reference": "name: organ-wireguard",
                    },
                ],
            },
        ), mock.patch(
            "organs.forge.src.core.lifecycle_diagnostic_payloads._load_registry",
            return_value={
                "nodes": [
                    {"name": "organ-wireguard", "role": "legacy_synapse_mesh_implementation", "status": "planned"}
                ],
                "organs": [
                    {"name": "organ-wireguard", "role": "legacy_synapse_mesh_implementation", "status": "planned"}
                ],
            },
        ), mock.patch(
            "organs.forge.src.core.lifecycle_diagnostic_payloads._load_infrastructure_mcp_contract",
            return_value={
                "operation_families": {
                    "mesh": {
                        "operations": {
                            "network.mesh.status": {
                                "organ_projection": {"organ": "organ-fleet"}
                            },
                            "network.mesh.dns.activation_plan": {
                                "organ_projection": {"organ": "organ-fleet"}
                            },
                            "network.mesh.provider_fallback_status": {
                                "organ_projection": {"organ": "organ-fleet"}
                            },
                        }
                    }
                }
            },
        ):
            payload = lifecycle_diagnostic_cli.organism_gap_map_payload()

        self.assertEqual(payload["schema"], "cynic.organism_gap_map.v1")
        self.assertEqual(payload["observed_gap_count"], 3)
        host_exec_gap = next(
            item for item in payload["observed_gaps"] if item["code"] == "network_host_exec:missing_activation"
        )
        runtime_gap = next(
            item for item in payload["observed_gaps"] if item["code"] == "network_runtime:wireguard_mesh_absent"
        )
        self.assertEqual(host_exec_gap["family"], "root_transport_install")
        self.assertEqual(host_exec_gap["depends_on"], "install_host_exec_bundle")
        self.assertEqual(host_exec_gap["blocked_by"], ["install_host_exec_bundle"])
        self.assertTrue(host_exec_gap["done_when"])
        self.assertEqual(runtime_gap["family"], "network_runtime")
        self.assertEqual(runtime_gap["blocked_by"], ["network_actuator:missing_activation"])
        self.assertTrue(runtime_gap["done_when"])
        self.assertEqual(payload["gap_family_summary"][0]["family"], "root_transport_install")
        self.assertEqual(payload["gap_family_summary"][0]["count"], 1)
        self.assertIn("organ-fleet", payload["gap_family_summary"][0]["resolver_organs"])
        self.assertEqual(payload["gap_family_summary"][1]["family"], "network_runtime")
        self.assertEqual(payload["gap_family_summary"][1]["highest_severity"], "critical")
        self.assertEqual(payload["structural_gap_count"], 4)
        self.assertEqual(
            [item["id"] for item in payload["structural_unobserved_gaps"]],
            [
                "root_network_policy_missing",
                "network_effectuator_missing",
                "deprecated_network_owner_in_desired_state",
                "post_repair_evidence_not_enforced",
            ],
        )
        self.assertEqual(payload["gap_coverage_summary"]["observed_family_count"], 3)
        self.assertEqual(payload["gap_coverage_summary"]["structural_family_count"], 4)
        self.assertIn(
            "deprecated_network_owner_in_desired_state",
            payload["gap_coverage_summary"]["critical_structural_gaps"],
        )
        self.assertEqual(payload["registry_network_policy"]["owner"], "organ-wireguard")
        self.assertEqual(payload["registry_network_policy"]["owner_status"], "planned")
        self.assertEqual(payload["ownership_drift_map"]["reference_count"], 5)
        self.assertEqual(payload["ownership_drift_map"]["surface_class_counts"]["desired_state_owner"], 1)
        self.assertEqual(payload["ownership_drift_map"]["references"][0]["path"], "infra/registry.json")
        self.assertEqual(payload["active_ownership_blockers"]["active_reference_count"], 4)
        self.assertEqual(payload["active_ownership_blockers"]["lane_counts"]["desired_state_migration"], 2)
        self.assertEqual(payload["active_ownership_blockers"]["lane_counts"]["mcp_contract_migration"], 2)
        self.assertEqual(
            payload["active_ownership_blockers"]["references"][0]["lane"],
            "desired_state_migration",
        )
        self.assertEqual(payload["prioritized_mutation_targets"][0]["path"], "infra/registry.json")
        self.assertEqual(payload["prioritized_mutation_targets"][0]["priority_score"], 200)
        self.assertEqual(payload["prioritized_mutation_targets"][1]["path"], "axioms/infrastructure-mcp-v1.json")
        self.assertEqual(
            payload["prioritized_mutation_targets"][1]["lanes"],
            ["mcp_contract_migration"],
        )
        self.assertEqual(payload["registry_owner_transition_preview"]["target_file"], "infra/registry.json")
        self.assertEqual(payload["registry_owner_transition_preview"]["mutation_count"], 3)
        self.assertEqual(
            payload["registry_owner_transition_preview"]["mutations"][0]["json_pointer"],
            "/network_policy/owner",
        )
        self.assertEqual(
            payload["registry_owner_transition_preview"]["mutations"][0]["proposed_value"],
            "cynic-node",
        )
        self.assertEqual(
            payload["infrastructure_mcp_transition_preview"]["target_file"],
            "axioms/infrastructure-mcp-v1.json",
        )
        self.assertEqual(payload["infrastructure_mcp_transition_preview"]["mutation_count"], 0)
        self.assertEqual(payload["combined_transition_plan_preview"]["phase_count"], 3)
        self.assertEqual(
            payload["combined_transition_plan_preview"]["phases"][0]["name"],
            "desired_state_owner_cutover",
        )
        self.assertEqual(
            payload["combined_transition_plan_preview"]["phases"][1]["name"],
            "mcp_contract_stabilized",
        )
        self.assertEqual(
            payload["combined_transition_plan_preview"]["phases"][2]["target_files"],
            [],
        )
        self.assertEqual(
            payload["owner_transition_recommendation"]["recommended_semantic_owner"],
            "cynic-node",
        )
        self.assertEqual(
            payload["owner_transition_recommendation"]["recommended_execution_surface"],
            "network-actuator",
        )
        self.assertIn(
            "organ-fleet",
            payload["owner_transition_recommendation"]["recommended_supporting_organs"],
        )
        self.assertEqual(
            payload["owner_transition_recommendation"]["primary_mutation_targets"][:2],
            ["infra/registry.json", "axioms/infrastructure-mcp-v1.json"],
        )
        self.assertEqual(payload["ownership_summary"]["desired_state_owner"], "organ-wireguard")
        self.assertIn("organ-fleet", payload["ownership_summary"]["observed_resolver_organs"])
        self.assertEqual(payload["ownership_summary"]["ownership_gap_count"], 2)
        self.assertEqual(
            payload["ownership_summary"]["ownership_gaps"][0]["kind"],
            "desired_owner_not_in_runtime_resolution",
        )
        self.assertEqual(payload["root_install_sequence"], [{"action": "install_host_exec_bundle"}])
        self.assertEqual(payload["critical_path"][0]["code"], "network_host_exec:missing_activation")
        self.assertEqual(payload["repair_batches"][0]["batch"], "root_transport_foundation")
        self.assertTrue(payload["repair_batches"][0]["can_run_now"])
        self.assertFalse(payload["repair_batches"][0]["blocked_on_external_authority"])
        self.assertEqual(payload["next_repair_batch"]["batch"], "root_transport_foundation")
        self.assertEqual(payload["network_gap_ledger"]["schema"], "cynic.network_infra_gaps.v1")
        self.assertIn("organism-gap-map", payload["source_command"])

    def test_organism_gap_map_closes_deprecated_owner_gap_after_cutover(self):
        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "build_lifecycle_activation_requests",
            return_value={
                "status": "blocked",
                "activation_schema_version": "activation.v1",
                "lifecycle": "kernel_release",
                "build_target": "cynic-vm-anvil",
                "deploy_target": "cynic-vm-core",
                "blockers": [
                    {
                        "category": "network",
                        "kind": "host_exec_activation",
                        "code": "network_host_exec:missing_activation",
                        "hostname": "ubuntu",
                        "resolver_organ": "organ-fleet",
                    },
                    {
                        "category": "network",
                        "kind": "runtime_blocker",
                        "code": "network_runtime:wireguard_mesh_absent",
                        "hostname": "ubuntu",
                        "resolver_organ": "cynic-node",
                    },
                ],
                "root_install_sequence": [{"action": "install_host_exec_bundle"}],
                "requests": [],
            },
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_bootstrap_source",
            return_value={"status": "blocked", "reason": "bootstrap_source_unreachable"},
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_consumer_runtime",
            return_value={"status": "blocked", "runtime_user_active": False},
        ), mock.patch.object(
            lifecycle_diagnostic_cli,
            "probe_local_network_runtime",
            return_value={"enabled": True, "status": "blocked", "blockers": []},
        ), mock.patch(
            "organs.forge.src.core.lifecycle_diagnostic_payloads._registry_network_policy_summary",
            return_value={
                "version": "network-policy.v1",
                "owner": "cynic-node",
                "owner_status": "declared",
                "canonical_service_plane": "synapse",
                "canonical_plane_contract": {
                    "cidr": "10.20.0.0/24",
                    "dns_zone": "synapse.cynic",
                    "canonical": True,
                },
            },
        ), mock.patch(
            "organs.forge.src.core.lifecycle_diagnostic_payloads._ownership_drift_reference_map",
            return_value={
                "reference_count": 2,
                "surface_class_counts": {
                    "diagnostic_runtime_reference": 1,
                    "legacy_organ_implementation": 1,
                },
                "references": [
                    {
                        "path": "organs/forge/src/core/lifecycle_diagnostic_payloads.py",
                        "line": 800,
                        "surface_class": "diagnostic_runtime_reference",
                        "reference": 'reason = "semantic ownership must leave organ-wireguard"',
                    },
                    {
                        "path": "organs/wireguard/MANIFEST.yaml",
                        "line": 1,
                        "surface_class": "legacy_organ_implementation",
                        "reference": "name: organ-wireguard",
                    },
                ],
            },
        ), mock.patch(
            "organs.forge.src.core.lifecycle_diagnostic_payloads._load_registry",
            return_value={"nodes": [], "organs": []},
        ), mock.patch(
            "organs.forge.src.core.lifecycle_diagnostic_payloads._load_infrastructure_mcp_contract",
            return_value={
                "operation_families": {
                    "mesh": {
                        "operations": {
                            "network.mesh.status": {
                                "organ_projection": {"organ": "organ-fleet"}
                            },
                            "network.mesh.dns.activation_plan": {
                                "organ_projection": {"organ": "organ-fleet"}
                            },
                            "network.mesh.provider_fallback_status": {
                                "organ_projection": {"organ": "organ-fleet"}
                            },
                        }
                    }
                }
            },
        ):
            payload = lifecycle_diagnostic_cli.organism_gap_map_payload()

        structural_ids = [item["id"] for item in payload["structural_unobserved_gaps"]]
        self.assertNotIn("deprecated_network_owner_in_desired_state", structural_ids)
        self.assertNotIn(
            "deprecated_network_owner_in_desired_state",
            payload["gap_coverage_summary"]["critical_structural_gaps"],
        )
        self.assertEqual(payload["active_ownership_blockers"]["active_reference_count"], 1)
        self.assertEqual(
            payload["active_ownership_blockers"]["lane_counts"],
            {"diagnostic_surface_migration": 1},
        )
        self.assertEqual(payload["registry_owner_transition_preview"]["mutation_count"], 0)
        self.assertEqual(payload["infrastructure_mcp_transition_preview"]["mutation_count"], 0)
        self.assertEqual(
            payload["combined_transition_plan_preview"]["phases"][0]["name"],
            "desired_state_owner_stabilized",
        )
        self.assertEqual(
            payload["combined_transition_plan_preview"]["phases"][1]["name"],
            "mcp_contract_stabilized",
        )
        self.assertEqual(payload["ownership_summary"]["desired_state_owner"], "cynic-node")
        self.assertEqual(payload["ownership_summary"]["ownership_gap_count"], 0)
        self.assertIn(
            "legacy cleanup and regression prevention",
            payload["owner_transition_recommendation"]["transition_reason"],
        )

    def test_organism_gap_map_cli_prints_json(self):
        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "organism_gap_map_payload",
            return_value={
                "status": "blocked",
                "schema": "cynic.organism_gap_map.v1",
                "observed_gap_count": 1,
                "observed_gaps": [{"code": "network_host_exec:missing_activation"}],
            },
        ), mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = lifecycle_diagnostic_cli.run_organism_gap_map()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["observed_gap_count"], 1)
        self.assertEqual(payload["observed_gaps"][0]["code"], "network_host_exec:missing_activation")

    def test_ownership_drift_reference_map_scans_only_authority_surfaces(self):
        from organs.forge.src.core import lifecycle_diagnostic_payloads as payloads

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "infra").mkdir(parents=True)
            (root / "axioms").mkdir(parents=True)
            (root / "crates" / "cynic-mcp" / "src").mkdir(parents=True)
            (root / "organs" / "wireguard").mkdir(parents=True)
            (root / "organs" / "forge" / "src" / "core").mkdir(parents=True)

            (root / "infra" / "registry.json").write_text(
                '{\n  "network_policy": {\n    "owner": "organ-wireguard"\n  }\n}\n',
                encoding="utf-8",
            )
            (root / "axioms" / "infrastructure-mcp-v1.json").write_text(
                '{"operation_families":{"mesh":{"operations":{"network.mesh.status":{"organ_projection":{"organ":"organ-wireguard"}}}}}}\n',
                encoding="utf-8",
            )
            (root / "crates" / "cynic-mcp" / "src" / "proxy.rs").write_text(
                '// organ-wireguard legacy router note\n',
                encoding="utf-8",
            )
            (root / "organs" / "wireguard" / "MANIFEST.yaml").write_text(
                'name: organ-wireguard\n',
                encoding="utf-8",
            )
            (root / "organs" / "forge" / "src" / "core" / "lifecycle_diagnostic_payloads.py").write_text(
                'reason = "semantic ownership must leave organ-wireguard"\n',
                encoding="utf-8",
            )

            with mock.patch.object(payloads, "PROJECT_ROOT", root):
                report = payloads._ownership_drift_reference_map()

        self.assertEqual(report["reference_count"], 3)
        self.assertEqual(
            sorted(item["path"] for item in report["references"]),
            [
                "axioms/infrastructure-mcp-v1.json",
                "crates/cynic-mcp/src/proxy.rs",
                "infra/registry.json",
            ],
        )
        self.assertEqual(report["surface_class_counts"]["desired_state_owner"], 1)
        self.assertEqual(report["surface_class_counts"]["contract_reference"], 1)
        self.assertEqual(report["surface_class_counts"]["mcp_runtime_reference"], 1)

    def test_organism_next_repair_batch_payload_selects_first_runnable_batch(self):
        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "organism_gap_map_payload",
            return_value={
                "status": "blocked",
                "activation_schema_version": "activation.v1",
                "lifecycle": "kernel_release",
                "build_target": "cynic-vm-anvil",
                "deploy_target": "cynic-vm-core",
                "observed_gap_count": 3,
                "repair_batches": [
                    {
                        "batch": "root_transport_foundation",
                        "order": 1,
                        "gaps": ["network_host_exec:missing_activation"],
                        "can_run_now": True,
                        "blocked_on_external_authority": False,
                        "reason": "local root required",
                    },
                    {
                        "batch": "network_runtime_repair",
                        "order": 3,
                        "gaps": ["network_runtime:wireguard_mesh_absent"],
                        "can_run_now": False,
                        "blocked_on_external_authority": True,
                        "reason": "needs active actuator",
                    },
                ],
            },
        ):
            payload = lifecycle_diagnostic_cli.organism_next_repair_batch_payload()

        self.assertEqual(payload["schema"], "cynic.organism_next_repair_batch.v1")
        self.assertTrue(payload["next_batch_ready"])
        self.assertEqual(payload["next_repair_batch"]["batch"], "root_transport_foundation")
        self.assertEqual(payload["remaining_blocked_batches"][0]["batch"], "network_runtime_repair")
        self.assertIn("organism-next-repair-batch", payload["source_command"])

    def test_organism_next_repair_batch_cli_prints_json(self):
        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "organism_next_repair_batch_payload",
            return_value={
                "status": "blocked",
                "schema": "cynic.organism_next_repair_batch.v1",
                "next_batch_ready": True,
                "next_repair_batch": {"batch": "root_transport_foundation"},
            },
        ), mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = lifecycle_diagnostic_cli.run_organism_next_repair_batch()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertTrue(payload["next_batch_ready"])
        self.assertEqual(payload["next_repair_batch"]["batch"], "root_transport_foundation")

    def test_main_routes_organism_gap_map(self):
        with mock.patch(
            "sys.argv",
            ["forge", "organism-gap-map"],
        ), mock.patch.object(
            cli._cli_runtime,
            "run_organism_gap_map",
            return_value=0,
        ) as run_command:
            status = cli.main()

        self.assertEqual(status, 0)
        run_command.assert_called_once_with()

    def test_main_routes_organism_next_repair_batch(self):
        with mock.patch(
            "sys.argv",
            ["forge", "organism-next-repair-batch"],
        ), mock.patch.object(
            cli._cli_runtime,
            "run_organism_next_repair_batch",
            return_value=0,
        ) as run_command:
            status = cli.main()

        self.assertEqual(status, 0)
        run_command.assert_called_once_with()

    def test_lifecycle_root_install_sequence_cli_prints_json(self):
        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "lifecycle_root_install_sequence_payload",
            return_value={
                "status": "blocked",
                "root_install_count": 1,
                "root_install_sequence": [{"action": "install_host_exec_bundle"}],
            },
        ), mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = lifecycle_diagnostic_cli.run_lifecycle_root_install_sequence()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["root_install_count"], 1)
        self.assertEqual(payload["root_install_sequence"][0]["action"], "install_host_exec_bundle")

    def test_main_routes_lifecycle_root_install_sequence(self):
        with mock.patch(
            "sys.argv",
            ["forge", "lifecycle-root-install-sequence"],
        ), mock.patch.object(
            cli._cli_runtime,
            "run_lifecycle_root_install_sequence",
            return_value=0,
        ) as run_command:
            status = cli.main()

        self.assertEqual(status, 0)
        run_command.assert_called_once_with()

    def test_lifecycle_root_install_script_contains_ordered_commands(self):
        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "lifecycle_root_install_sequence_payload",
            return_value={
                "lifecycle": "kernel_release",
                "build_target": "cynic-vm-anvil",
                "deploy_target": "cynic-vm-core",
                "root_install_sequence": [
                    {
                        "order": 1,
                        "action": "install_host_exec_bundle",
                        "hostname": "ubuntu",
                        "dependency": "required_before_remote_network_repair",
                        "required_commands": ["install", "visudo"],
                        "required_bundle_files": [
                            "/tmp/cynic-host-exec/cynic-host-exec",
                            "/tmp/cynic-host-exec/manifest.json",
                        ],
                        "manual_root_install_command": "install host-exec",
                        "postflight_commands": ["check host-exec"],
                    },
                    {
                        "order": 2,
                        "action": "install_network_actuator_bundle",
                        "hostname": "ubuntu",
                        "dependency": "requires_host_exec_transport",
                        "manual_root_install_command": "install network-actuator",
                        "postflight_commands": ["check network-actuator"],
                    },
                ],
            },
        ):
            script = lifecycle_diagnostic_cli.lifecycle_root_install_script()

        self.assertTrue(script.startswith("#!/usr/bin/env bash\nset -euo pipefail\n"))
        self.assertIn('echo >&2 "error: this script must run as root"', script)
        self.assertIn("# Step 1: install_host_exec_bundle on ubuntu", script)
        self.assertIn("echo '==> install_host_exec_bundle (ubuntu)'", script)
        self.assertIn("command -v install >/dev/null", script)
        self.assertIn("command -v visudo >/dev/null", script)
        self.assertIn("[[ -f '/tmp/cynic-host-exec/cynic-host-exec' ]]", script)
        self.assertIn("install host-exec", script)
        self.assertIn("check host-exec", script)
        self.assertIn("# Step 2: install_network_actuator_bundle on ubuntu", script)
        self.assertIn("echo '==> install_network_actuator_bundle (ubuntu)'", script)
        self.assertIn("install network-actuator", script)
        self.assertIn("check network-actuator", script)

    def test_lifecycle_root_install_script_cli_prints_shell(self):
        with mock.patch.object(
            lifecycle_diagnostic_cli,
            "lifecycle_root_install_script",
            return_value="#!/usr/bin/env bash\nset -euo pipefail\n",
        ), mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = lifecycle_diagnostic_cli.run_lifecycle_root_install_script()

        self.assertEqual(status, 0)
        self.assertTrue(stdout.getvalue().startswith("#!/usr/bin/env bash"))

    def test_main_routes_lifecycle_root_install_script(self):
        with mock.patch(
            "sys.argv",
            ["forge", "lifecycle-root-install-script"],
        ), mock.patch.object(
            cli._cli_runtime,
            "run_lifecycle_root_install_script",
            return_value=0,
        ) as run_command:
            status = cli.main()

        self.assertEqual(status, 0)
        run_command.assert_called_once_with()

    def test_lifecycle_activation_plan_cli_returns_blocked_status(self):
        with mock.patch(
            "organs.forge.src.entrypoints.lifecycle_activation_cli.build_lifecycle_activation_plan",
            return_value={
                "status": "blocked",
                "lifecycle": "kernel_release",
                "actions": [{"action": "activate_registry_node"}],
            },
        ) as activation_plan, mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = lifecycle_cli.run_lifecycle_activation_plan()

        self.assertEqual(status, 1)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["actions"][0]["action"], "activate_registry_node")
        activation_plan.assert_called_once_with(
            inspect_fleet=True,
            include_network_probe=True,
        )

    def test_lifecycle_activation_dispatch_cli_returns_ok_status(self):
        with mock.patch(
            "organs.forge.src.entrypoints.lifecycle_activation_cli.dispatch_lifecycle_activation_requests",
            return_value={
                "status": "ok",
                "plan_status": "blocked",
                "requests": [{"kind": "hermes"}],
                "responses": [{"id": "task-1"}],
                "dispatched_count": 1,
            },
        ) as dispatch_requests, mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = lifecycle_cli.run_lifecycle_activation_dispatch()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["dispatched_count"], 1)
        dispatch_requests.assert_called_once_with(
            inspect_fleet=True,
            include_network_probe=True,
        )

    def test_lifecycle_activation_dispatch_cli_returns_structured_error(self):
        with mock.patch(
            "organs.forge.src.entrypoints.lifecycle_activation_cli.dispatch_lifecycle_activation_requests",
            side_effect=RuntimeError("CYNIC_API_KEY is required for organ-forge kernel dispatch"),
        ), mock.patch(
            "organs.forge.src.entrypoints.lifecycle_diagnostic_cli.lifecycle_error_context",
            return_value={"diagnostic_status": "ok", "lifecycle_blockers": []},
        ), mock.patch("sys.stderr", new_callable=StringIO) as stderr:
            status = lifecycle_cli.run_lifecycle_activation_dispatch()

        self.assertEqual(status, 1)
        payload = json.loads(stderr.getvalue())
        self.assertEqual(payload["error_kind"], "missing_kernel_auth")
        self.assertFalse(payload["local_execution_policy"]["builds_on_operator_machine"])

    def test_lifecycle_blockers_docs_expose_activation_dry_run_contract(self):
        agents = (PROJECT_ROOT / "AGENTS.md").read_text()
        codex = (PROJECT_ROOT / "CODEX.md").read_text()

        self.assertIn("activation_request_summary", agents)
        self.assertIn('activation_schema_version == "activation.v1"', agents)
        self.assertIn("lifecycle-activation-dispatch", agents)
        self.assertIn("This command is non-mutating", agents)
        self.assertIn("dry-run activation action summaries", codex)
        self.assertIn('activation_schema_version == "activation.v1"', codex)
        self.assertIn("make lifecycle-blockers", codex)
