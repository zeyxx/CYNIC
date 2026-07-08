import json
import unittest
from unittest import mock

from organs.forge.src.adapters.kernel_adapter import (
    KernelRestClient,
    build_lifecycle_profile_payloads,
    build_lifecycle_task_payload,
    lifecycle_profile_intents,
    validate_lifecycle_task_payload,
)
from organs.forge.src.tests.fixtures import registry_fixture


class LifecycleTaskDispatchTests(unittest.TestCase):
    def test_lifecycle_profile_intents_come_from_registry(self):
        registry = registry_fixture()

        self.assertEqual(
            lifecycle_profile_intents("gate-2", registry),
            ["build-and-test", "integration-test"],
        )
        self.assertEqual(lifecycle_profile_intents("bootstrap", registry), ["bootstrap-build-runner"])
        self.assertEqual(lifecycle_profile_intents("deploy", registry), ["deploy-kernel"])
        self.assertEqual(
            lifecycle_profile_intents("mcp-reload", registry),
            ["build-and-reload-mcp"],
        )

    def test_lifecycle_profile_payloads_builds_and_validates_each_intent(self):
        result = build_lifecycle_profile_payloads(
            profile="gate-2",
            domain="profile-dry-run",
            agent_id="cortex-profile-dry-run",
            registry=registry_fixture(),
        )

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["intents"], ["build-and-test", "integration-test"])
        self.assertEqual(len(result["payloads"]), 2)
        self.assertEqual(len(result["validations"]), 2)
        self.assertEqual(
            [validation["intent"] for validation in result["validations"]],
            ["build-and-test", "integration-test"],
        )
        self.assertTrue(all(payload["domain"] == "profile-dry-run" for payload in result["payloads"]))

    def test_lifecycle_intent_can_scope_targets_for_mcp_reload(self):
        registry = registry_fixture()

        result = build_lifecycle_profile_payloads(
            profile="mcp-reload",
            domain="organ-forge",
            agent_id="organ-forge-request",
            registry=registry,
        )

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["intents"], ["build-and-reload-mcp"])
        payload = result["payloads"][0]
        self.assertEqual(payload["domain"], "organ-forge")
        content = json.loads(payload["content"])
        self.assertEqual(content["intent"], "build-and-reload-mcp")
        self.assertEqual(content["request_profile"], "mcp-reload")
        self.assertEqual(
            content["mcp_freshness_controller"]["dispatch_profile"],
            "mcp-reload",
        )
        self.assertIn(
            "infra/agent-exposure.json",
            content["mcp_freshness_controller"]["trigger_sources"],
        )
        self.assertEqual(
            "cynic.lifecycle.runtime_artifact_manifest.v1",
            content["runtime_artifact_manifest"]["schema"],
        )
        self.assertIn(
            "cynic-mcp",
            {
                artifact["name"]
                for artifact in content["runtime_artifact_manifest"]["artifacts"]
            },
        )
        self.assertEqual(
            content["targets"],
            ["crates/cynic-mcp", ".cortex/mcp/run_cynic_mcp.py"],
        )
        self.assertEqual(
            content["coord_targets"],
            [
                "crates/cynic-mcp",
                ".cortex/mcp/run_cynic_mcp.py",
                ".codex/config.toml",
            ],
        )
        self.assertFalse(content["deploy"])
        self.assertEqual(result["validations"][0]["coord_targets"], content["coord_targets"])

    def test_lifecycle_bootstrap_profile_marks_bootstrap_phase(self):
        registry = registry_fixture()

        result = build_lifecycle_profile_payloads(
            profile="bootstrap",
            domain="organ-forge",
            agent_id="organ-forge-bootstrap",
            registry=registry,
        )

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["intents"], ["bootstrap-build-runner"])
        payload = result["payloads"][0]
        content = json.loads(payload["content"])
        self.assertEqual(content["intent"], "bootstrap-build-runner")
        self.assertTrue(content["bootstrap_phase"])
        self.assertFalse(content["deploy"])
        self.assertEqual(content["targets"], ["organs/forge", "infra/registry.json"])
        self.assertEqual(content["coord_targets"], ["organs/forge", "infra/registry.json"])
        self.assertEqual(
            content["operator_interface_proposal"]["public_projection"]["risk_label"],
            "bootstrap",
        )
        self.assertEqual(result["validations"][0]["intent"], "bootstrap-build-runner")

    def test_build_lifecycle_task_payload_resolves_dynamic_targets(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )

        self.assertEqual(payload["kind"], "hermes")
        self.assertEqual(payload["domain"], "supply-chain-e2e-test")
        self.assertEqual(payload["agent_id"], "cortex-e2e-supply-chain")
        content = json.loads(payload["content"])
        self.assertEqual(content["lifecycle"], "kernel_release")
        self.assertEqual(content["intent"], "deploy-kernel")
        self.assertEqual(content["orchestrator_organ"], "organ-forge")
        self.assertEqual(content["build_target"], "cynic-vm-anvil")
        self.assertEqual(content["build_runner_role"], "build_runner")
        self.assertEqual(content["deploy_target"], "cynic-vm-core")
        self.assertEqual(content["deploy_target_role"], "kernel_primary")
        self.assertEqual(content["required_node_status"], "active")
        self.assertEqual(content["fleet_requirements"]["provider"], "synapse")
        self.assertEqual(
            content["activation_requirements"]["build_runner"]["services"],
            ["organ-forge", "cargo-build", "gate-2"],
        )
        self.assertEqual(content["runtime_paths"]["repo_root"], "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}")
        self.assertEqual(content["runtime_path_proofs"]["repo_root"]["kind"], "git_worktree")
        self.assertEqual(
            content["activation_evidence_contract"]["patch_field"],
            "activation_evidence_patch",
        )
        self.assertTrue(content["deploy"])
        self.assertIn("infra/registry.json", content["targets"])
        self.assertEqual(content["coord_targets"], content["targets"])
        self.assertEqual(content["coord_targets"], registry_fixture()["lifecycle"]["kernel_release"]["repo_targets"])
        proposal = content["operator_interface_proposal"]
        self.assertEqual(proposal["schema"], "cynic.agent_proposal.v1")
        self.assertEqual(proposal["visibility"], "operator_private")
        self.assertEqual(proposal["operator_review"]["state"], "pending")
        self.assertFalse(proposal["public_projection"]["eligible"])
        self.assertEqual(proposal["public_projection"]["redaction_status"], "needs_review")
        self.assertEqual(proposal["targets"], content["targets"])
        self.assertEqual(proposal["coord_targets"], content["coord_targets"])

    def test_validate_lifecycle_task_payload_accepts_registry_payload(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )

        result = validate_lifecycle_task_payload(payload, registry_fixture())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["intent"], "deploy-kernel")
        self.assertEqual(
            result["coord_targets"],
            registry_fixture()["lifecycle"]["kernel_release"]["repo_targets"],
        )

    def test_validate_lifecycle_task_payload_rejects_missing_coord_targets(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )
        content = json.loads(payload["content"])
        del content["coord_targets"]
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "content.coord_targets"):
            validate_lifecycle_task_payload(payload, registry_fixture())

    def test_validate_lifecycle_task_payload_rejects_missing_operator_proposal(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )
        content = json.loads(payload["content"])
        del content["operator_interface_proposal"]
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "operator_interface_proposal"):
            validate_lifecycle_task_payload(payload, registry_fixture())

    def test_validate_lifecycle_task_payload_rejects_runtime_path_drift(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )
        content = json.loads(payload["content"])
        content["runtime_paths"]["repo_root"] = "%h/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC"
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "content.runtime_paths"):
            validate_lifecycle_task_payload(payload, registry_fixture())

    def test_validate_lifecycle_task_payload_rejects_runtime_path_proof_drift(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )
        content = json.loads(payload["content"])
        content["runtime_path_proofs"]["repo_root"]["marker"] = "README.md"
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "content.runtime_path_proofs"):
            validate_lifecycle_task_payload(payload, registry_fixture())

    def test_validate_lifecycle_task_payload_rejects_activation_evidence_contract_drift(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )
        content = json.loads(payload["content"])
        content["activation_evidence_contract"]["ingestion_mode"] = "blind_merge"
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "content.activation_evidence_contract"):
            validate_lifecycle_task_payload(payload, registry_fixture())

    def test_validate_lifecycle_task_payload_rejects_runtime_artifact_manifest_drift(self):
        payload = build_lifecycle_task_payload(
            intent="deploy-kernel",
            domain="supply-chain-e2e-test",
            agent_id="cortex-e2e-supply-chain",
            registry=registry_fixture(),
        )
        content = json.loads(payload["content"])
        content["runtime_artifact_manifest"]["artifacts"] = []
        payload["content"] = json.dumps(content, separators=(",", ":"))

        with self.assertRaisesRegex(RuntimeError, "content.runtime_artifact_manifest"):
            validate_lifecycle_task_payload(payload, registry_fixture())

    def test_dispatch_validates_payload_before_posting_agent_task(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.registry = registry
        client.lifecycle = lifecycle

        invalid_content = {
            "lifecycle": "kernel_release",
            "intent": "deploy-kernel",
            "objective": "build kernel release on registry build runner and deploy to kernel primary",
            "orchestrator_organ": "organ-forge",
            "domain": "organ-forge",
            "build_target": "cynic-vm-anvil",
            "build_runner_role": "build_runner",
            "deploy_target": "cynic-vm-core",
            "deploy_target_role": "kernel_primary",
            "required_node_status": "active",
            "activation_requirements": lifecycle["activation_requirements"],
            "fleet_requirements": lifecycle["fleet_requirements"],
            "requires_e2e": True,
            "targets": lifecycle["repo_targets"],
            "deploy": True,
        }

        with mock.patch.object(
            client,
            "_content_for_intent",
            return_value=invalid_content,
        ), mock.patch.object(client, "_post_agent_task") as post_agent_task:
            with self.assertRaisesRegex(RuntimeError, "content.coord_targets"):
                __import__("asyncio").run(
                    client.dispatch_task_and_stream("deploy-kernel", "").__anext__()
                )

        post_agent_task.assert_not_called()

    def test_dispatch_requires_agent_task_read_back_visibility(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.registry = registry
        client.lifecycle = lifecycle

        with mock.patch.object(
            client,
            "_post_agent_task",
            return_value={"task_id": "agent_tasks:missing"},
        ), mock.patch.object(
            client,
            "_list_agent_tasks",
            side_effect=[
                {"tasks": [], "count": 0},
                {"tasks": [], "count": 0},
            ],
        ):
            with self.assertRaisesRegex(RuntimeError, "read-back incomplete"):
                __import__("asyncio").run(
                    client.dispatch_task_and_stream("deploy-kernel", "").__anext__()
                )

    def test_dispatch_accepts_agent_task_pending_read_back_visibility(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.registry = registry
        client.lifecycle = lifecycle

        with mock.patch.object(
            client,
            "_post_agent_task",
            return_value={"task_id": "agent_tasks:pending"},
        ), mock.patch.object(
            client,
            "_list_agent_tasks",
            side_effect=[
                {"tasks": [{"id": "agent_tasks:pending"}], "count": 1},
                {"tasks": [], "count": 0},
            ],
        ):
            result = __import__("asyncio").run(
                client.dispatch_task_and_stream("deploy-kernel", "").__anext__()
            )

        self.assertEqual(result, '{"task_id":"agent_tasks:pending"}\n')

    def test_dispatch_accepts_agent_task_completed_read_back_visibility(self):
        registry = registry_fixture()
        lifecycle = registry["lifecycle"]["kernel_release"]
        client = KernelRestClient.__new__(KernelRestClient)
        client.registry = registry
        client.lifecycle = lifecycle

        with mock.patch.object(
            client,
            "_post_agent_task",
            return_value={"task_id": "agent_tasks:completed"},
        ), mock.patch.object(
            client,
            "_list_agent_tasks",
            side_effect=[
                {"tasks": [], "count": 0},
                {"tasks": [{"id": "agent_tasks:completed"}], "count": 1},
            ],
        ):
            result = __import__("asyncio").run(
                client.dispatch_task_and_stream("deploy-kernel", "").__anext__()
            )

        self.assertEqual(result, '{"task_id":"agent_tasks:completed"}\n')

    def test_list_agent_tasks_url_encodes_query_values(self):
        client = KernelRestClient.__new__(KernelRestClient)
        client.base_url = "http://kernel.local"
        client.api_key = "test-key"
        captured = {}

        class Response:
            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

            def read(self):
                return b'{"tasks":[],"count":0}'

        def fake_urlopen(req, timeout=10):
            captured["url"] = req.full_url
            captured["timeout"] = timeout
            return Response()

        with mock.patch("organs.forge.src.adapters.kernel_task_client.request.urlopen", fake_urlopen):
            result = client._list_agent_tasks(
                kind="hermes",
                domain="gate 2/request&deploy",
                completed=True,
                limit=20,
            )

        self.assertEqual(result, {"tasks": [], "count": 0})
        self.assertEqual(captured["timeout"], 10)
        self.assertEqual(
            captured["url"],
            "http://kernel.local/agent-tasks/completed?kind=hermes&domain=gate+2%2Frequest%26deploy&limit=20",
        )
