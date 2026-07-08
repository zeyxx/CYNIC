import unittest

from organs.forge.src.adapters.kernel_adapter import (
    lifecycle_health_status_result,
    validate_lifecycle_policy,
)
from organs.forge.src.adapters.registry_adapter import (
    lifecycle_transition_contract,
    lifecycle_transition_debt,
)
from organs.forge.src.core.lifecycle_status import build_lifecycle_status_report
from organs.forge.src.tests.fixtures import registry_fixture


class LifecycleRegistryPolicyTests(unittest.TestCase):
    def test_validate_lifecycle_policy_accepts_registry_contract(self):
        result = validate_lifecycle_policy(registry_fixture())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["request_profiles"], ["bootstrap", "deploy", "gate-2", "mcp-reload"])
        self.assertEqual(result["repo_root"], "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}")
        self.assertEqual(result["workflow"]["name"], "kernel_release")
        self.assertEqual(
            result["workflow"]["state_layers"]["declared"]["source"],
            "infra/registry.json",
        )
        self.assertEqual(
            result["workflow"]["state_layers"]["verified"]["source"],
            "kernel_readback",
        )

    def test_validate_lifecycle_policy_rejects_missing_host_exec_transport_on_host_native_role(self):
        registry = registry_fixture()
        host_node = next(node for node in registry["nodes"] if node["role"] == "cortex_host")
        host_node.pop("host_exec_transport")

        with self.assertRaisesRegex(RuntimeError, "host_exec_transport missing"):
            validate_lifecycle_policy(registry)

    def test_validate_lifecycle_policy_rejects_executable_host_transport_without_command(self):
        registry = registry_fixture()
        host_node = next(node for node in registry["nodes"] if node["role"] == "cortex_host")
        host_node["host_exec_transport"]["kind"] = "service_agent_surface"

        with self.assertRaisesRegex(
            RuntimeError,
            "host_exec_transport executable kinds require transport_command_env or transport_command_default",
        ):
            validate_lifecycle_policy(registry)

    def test_validate_lifecycle_policy_accepts_executable_host_transport_with_command(self):
        registry = registry_fixture()
        host_node = next(node for node in registry["nodes"] if node["role"] == "cortex_host")
        host_node["host_exec_transport"]["kind"] = "ssh_sudo_helper"
        host_node["host_exec_transport"]["transport_command_default"] = (
            "sudo -n /usr/local/libexec/cynic-host-exec"
        )

        result = validate_lifecycle_policy(registry)

        self.assertEqual(result["status"], "ok")

    def test_lifecycle_transition_contract_reports_layer_sequence(self):
        result = lifecycle_transition_contract(registry_fixture())

        self.assertEqual("ok", result["status"])
        self.assertEqual(
            [
                "desired",
                "declared",
                "observed",
                "effective",
                "applied",
                "verified",
            ],
            result["transition_contract"]["layers"],
        )
        transitions = result["transition_contract"]["transitions"]
        self.assertEqual(("effective", "applied"), (transitions[3]["from"], transitions[3]["to"]))
        self.assertTrue(transitions[3]["read_back_required"])

    def test_lifecycle_transition_debt_reports_ready_activation_gap(self):
        result = lifecycle_transition_debt(
            registry_fixture(),
            activation_report={
                "lifecycle_nodes_ready": True,
                "blockers": [],
                "activation_request_count": 1,
                "activation_request_summary": [
                    {"action": "materialize_runtime_path", "node": "cynic-vm-anvil"}
                ],
            },
        )

        self.assertEqual("debt", result["status"])
        codes = {item["code"] for item in result["debts"]}
        self.assertIn("ready_but_activation_requests_generated", codes)
        self.assertIn("missing_runner_result_readback", codes)
        self.assertIn("missing_completed_task_readback", codes)

    def test_lifecycle_status_abstracts_transition_debt(self):
        result = build_lifecycle_status_report(
            lifecycle_transition_contract(registry_fixture()),
            {
                "status_nodes_ready": True,
                "fleet_nodes_ready": True,
                "lifecycle_nodes_ready": True,
                "blockers": [],
                "activation_request_count": 0,
                "build_target": "cynic-vm-anvil",
                "deploy_target": "cynic-vm-core",
            },
        )

        self.assertEqual("ready_for_dispatch", result["state"])
        self.assertTrue(result["can_dispatch"])
        self.assertEqual("cynic-vm-anvil", result["targets"]["build"])

    def test_lifecycle_status_surfaces_runtime_attention_when_bootstrap_source_is_blocked(self):
        result = build_lifecycle_status_report(
            lifecycle_transition_contract(registry_fixture()),
            {
                "status_nodes_ready": True,
                "fleet_nodes_ready": True,
                "lifecycle_nodes_ready": True,
                "blockers": [
                    {
                        "category": "bootstrap_source",
                        "code": "bootstrap_source_contract_stale",
                    }
                ],
                "activation_request_count": 0,
                "build_target": "cynic-vm-anvil",
                "deploy_target": "cynic-vm-core",
            },
        )

        self.assertEqual("needs_runtime_attention", result["state"])
        self.assertFalse(result["can_dispatch"])
        self.assertIn("inspect_bootstrap_source", result["next_commands"])

    def test_validate_lifecycle_policy_rejects_operator_checkout_runtime_root(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["runtime_paths"][
            "repo_root"
        ] = "/tmp/userhome/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC"

        with self.assertRaises(RuntimeError) as ctx:
            validate_lifecycle_policy(registry)

        self.assertIn("runtime_paths.repo_root", str(ctx.exception))

    def test_validate_lifecycle_policy_rejects_missing_runtime_path_proof(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["runtime_path_proofs"].pop("repo_root")

        with self.assertRaises(RuntimeError) as ctx:
            validate_lifecycle_policy(registry)

        self.assertIn("runtime_path_proofs.repo_root", str(ctx.exception))

    def test_lifecycle_health_status_uses_registry_accepted_status(self):
        self.assertTrue(lifecycle_health_status_result(503, registry_fixture())["accepted"])
        self.assertFalse(lifecycle_health_status_result(500, registry_fixture())["accepted"])

    def test_validate_lifecycle_policy_rejects_unknown_profile_intent(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["request_profiles"]["gate-2"].append(
            "unknown-intent"
        )

        with self.assertRaisesRegex(RuntimeError, "unknown intent unknown-intent"):
            validate_lifecycle_policy(registry)

    def test_validate_lifecycle_policy_accepts_git_fetch_repo_source(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_materialization"] = "git_fetch"
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_source"] = {
            "remote": "ssh://git@forgejo.synapse.cynic/CYNIC/CYNIC.git",
            "ref": "main",
            "ref_env": "CYNIC_FORGEJO_BOOTSTRAP_REF",
            "transport": "ssh",
            "host_role": "forge_primary",
            "fallback_remote_env": "CYNIC_FORGEJO_BOOTSTRAP_REMOTE",
            "fallback_transport": "local_tunnel",
        }

        result = validate_lifecycle_policy(registry)

        self.assertEqual(result["status"], "ok")

    def test_validate_lifecycle_policy_rejects_git_fetch_without_repo_source(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_materialization"] = "git_fetch"
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"].pop("repo_source", None)

        with self.assertRaisesRegex(RuntimeError, "repo_source.remote missing for git_fetch"):
            validate_lifecycle_policy(registry)

    def test_validate_lifecycle_policy_rejects_empty_git_fetch_fallback_remote_env(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_materialization"] = "git_fetch"
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_source"] = {
            "remote": "ssh://git@forgejo.synapse.cynic/CYNIC/CYNIC.git",
            "ref": "main",
            "transport": "ssh",
            "host_role": "forge_primary",
            "fallback_remote_env": "",
        }

        with self.assertRaisesRegex(RuntimeError, "fallback_remote_env must be a non-empty string"):
            validate_lifecycle_policy(registry)

    def test_validate_lifecycle_policy_rejects_empty_git_fetch_ref_env(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_materialization"] = "git_fetch"
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_source"] = {
            "remote": "ssh://git@forgejo.synapse.cynic/CYNIC/CYNIC.git",
            "ref": "main",
            "ref_env": "",
            "transport": "ssh",
            "host_role": "forge_primary",
        }

        with self.assertRaisesRegex(RuntimeError, "ref_env must be a non-empty string"):
            validate_lifecycle_policy(registry)
