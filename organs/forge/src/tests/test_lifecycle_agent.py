import json
import logging
from pathlib import Path
import subprocess
import unittest
from unittest import mock

from organs.forge.controllers.lifecycle_agent import LifecycleAgentController
from organs.forge.src.adapters.kernel_adapter import build_lifecycle_activation_requests
from organs.forge.src.core.lifecycle_consumer_status import build_lifecycle_consumer_status_report
from organs.forge.src.core.lifecycle_consumer_status import probe_local_consumer_runtime
from organs.forge.src.core.entities import AgentTask
from organs.forge.src.tests.fixtures import registry_fixture


class LifecycleAgentTests(unittest.TestCase):
    def test_lifecycle_consumer_status_reports_unclaimed_pending_tasks(self):
        report = build_lifecycle_consumer_status_report(
            pending_payload={
                "tasks": [
                    {
                        "id": "agent_tasks:mcp",
                        "age_seconds": 42,
                        "claimed_by": None,
                        "content": json.dumps(
                            {
                                "intent": "build-and-reload-mcp",
                                "request_profile": "mcp-reload",
                            }
                        ),
                    }
                ]
            },
            completed_payload={"tasks": []},
            kind="hermes",
            domain="organ-forge",
        )

        self.assertEqual("degraded", report["status"])
        self.assertEqual("not_claiming", report["consumer_state"])
        self.assertEqual(1, report["unclaimed_count"])
        self.assertEqual(["agent_tasks:mcp"], report["mcp_reload_pending_task_ids"])

    def test_lifecycle_consumer_status_surfaces_missing_runtime_even_without_pending_tasks(self):
        report = build_lifecycle_consumer_status_report(
            pending_payload={"tasks": []},
            completed_payload={"tasks": []},
            kind="hermes",
            domain="organ-forge",
            runtime_probe={
                "status": "blocked",
                "problems": ["runtime_user_not_active", "cargo_missing"],
                "operator_action": "restore runtime",
            },
        )

        self.assertEqual("degraded", report["status"])
        self.assertEqual("runtime_unavailable", report["consumer_state"])
        self.assertEqual("restore runtime", report["next_action"])
        self.assertEqual(["runtime_user_not_active", "cargo_missing"], report["runtime_probe"]["problems"])

    def test_lifecycle_consumer_status_accepts_manual_daemon_as_runtime_presence(self):
        report = build_lifecycle_consumer_status_report(
            pending_payload={"tasks": []},
            completed_payload={"tasks": []},
            kind="hermes",
            domain="organ-forge",
            runtime_probe={
                "status": "ok",
                "runtime_user_active": False,
                "cargo_available": True,
                "daemon_probe": {
                    "running": True,
                    "matches": [
                        f"1234 /usr/bin/python3 {Path.home()}/.cynic/repos/CYNIC/organs/forge/src/entrypoints/cli.py daemon"
                    ],
                },
                "problems": [],
            },
        )

        self.assertEqual("ok", report["status"])
        self.assertEqual("idle", report["consumer_state"])

    def test_probe_local_consumer_runtime_reports_missing_user_session_and_cargo(self):
        loginctl = subprocess.CompletedProcess(
            ["loginctl", "list-users"],
            0,
            stdout=" UID USER LINGER STATE\n1000 user yes active\n",
            stderr="",
        )

        with mock.patch(
            "organs.forge.src.core.lifecycle_consumer_status.subprocess.run",
            return_value=loginctl,
        ), mock.patch(
            "organs.forge.src.core.lifecycle_consumer_status._resolve_runtime_cargo",
            return_value=None,
        ), mock.patch(
            "organs.forge.src.core.lifecycle_consumer_status._probe_manual_consumer_process",
            return_value={"running": False, "matches": []},
        ), mock.patch(
            "pathlib.Path.exists",
            autospec=True,
            side_effect=lambda path: str(path) in {
                str(Path.home() / ".config/cynic/env"),
                str(Path.home() / ".config/systemd/user/hermes-agent-executor.service"),
            },
        ), mock.patch(
            "os.access", return_value=False):
            probe = probe_local_consumer_runtime()

        self.assertEqual("blocked", probe["status"])
        self.assertFalse(probe["runtime_user_active"])
        self.assertFalse(probe["cargo_available"])
        self.assertIn("runtime_user_not_active", probe["problems"])
        self.assertIn("cargo_missing", probe["problems"])

    def test_probe_local_consumer_runtime_accepts_manual_daemon_when_loginctl_is_inactive(self):
        loginctl = subprocess.CompletedProcess(
            ["loginctl", "list-users"],
            0,
            stdout=" UID USER LINGER STATE\n1000 user yes active\n",
            stderr="",
        )

        with mock.patch(
            "organs.forge.src.core.lifecycle_consumer_status.subprocess.run",
            return_value=loginctl,
        ), mock.patch(
            "organs.forge.src.core.lifecycle_consumer_status._resolve_runtime_cargo",
            return_value=str(Path.home() / ".cargo/bin/cargo"),
        ), mock.patch(
            "organs.forge.src.core.lifecycle_consumer_status._probe_manual_consumer_process",
            return_value={
                "running": True,
                "matches": [
                    f"1234 /usr/bin/python3 {Path.home()}/.cynic/repos/CYNIC/organs/forge/src/entrypoints/cli.py daemon"
                ],
            },
        ), mock.patch(
            "pathlib.Path.exists",
            autospec=True,
            return_value=True,
        ):
            probe = probe_local_consumer_runtime()

        self.assertEqual("ok", probe["status"])
        self.assertFalse(probe["runtime_user_active"])
        self.assertTrue(probe["cargo_available"])
        self.assertEqual([], probe["problems"])
        self.assertTrue(probe["daemon_probe"]["running"])

    def test_lifecycle_agent_fetch_pending_tasks_uses_bounded_poll_limit(self):
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        controller.kind = "hermes"
        controller.domain = "organ-forge"
        controller.poll_domains = ["organ-forge"]
        controller.poll_limit = 3
        calls = []

        class FakeKernelTasks:
            def list_agent_tasks(self, **kwargs):
                calls.append(kwargs)
                return {"tasks": []}

        controller.kernel_client = type(
            "FakeKernelClient",
            (),
            {"kernel_tasks": FakeKernelTasks()},
        )()

        self.assertEqual([], controller._fetch_pending_tasks())
        self.assertEqual(
            calls,
            [
                {
                    "kind": "hermes",
                    "domain": "organ-forge",
                    "completed": False,
                    "limit": 3,
                }
            ],
        )

    def test_lifecycle_agent_fetches_extra_lifecycle_domains(self):
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        controller.kind = "hermes"
        controller.domain = "organ-forge"
        controller.poll_domains = ["organ-forge", "mcp-runtime-request"]
        controller.poll_limit = 3
        calls = []

        class FakeKernelTasks:
            def list_agent_tasks(self, **kwargs):
                calls.append(kwargs)
                return {"tasks": []}

        controller.kernel_client = type(
            "FakeKernelClient",
            (),
            {"kernel_tasks": FakeKernelTasks()},
        )()

        self.assertEqual([], controller._fetch_pending_tasks())
        self.assertEqual(
            [call["domain"] for call in calls],
            ["organ-forge", "mcp-runtime-request"],
        )

    def test_lifecycle_agent_submits_result_to_task_domain(self):
        task = AgentTask.from_dict(
            {
                "id": "agent_tasks:mcp",
                "kind": "hermes",
                "domain": "mcp-runtime-request",
                "agent_id": "organ-forge-cli",
                "content": json.dumps(
                    {
                        "lifecycle": "kernel_release",
                        "intent": "build-and-reload-mcp",
                        "request_profile": "mcp-reload",
                    }
                ),
                "status": "pending",
            }
        )
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        submitted = {}
        controller.intent_executor = mock.Mock(
            execute=mock.Mock(return_value={"status": "ok", "intent": "build-and-reload-mcp"})
        )
        controller._submit_result = lambda task_id, success, result_data, domain=None: submitted.update(
            {"task_id": task_id, "success": success, "result": result_data, "domain": domain}
        )

        __import__("asyncio").run(controller.execute_task(task))

        self.assertTrue(submitted["success"])
        self.assertEqual("mcp-runtime-request", submitted["domain"])

    def test_lifecycle_agent_spools_result_when_kernel_submit_fails(self):
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        controller.kind = "hermes"
        controller.domain = "organ-forge"
        spooled = []

        class FailingKernelTasks:
            def submit_agent_task_result(self, *args, **kwargs):
                raise RuntimeError("kernel down")

        class FakeMemoryStore:
            def enqueue_outbox(self, **kwargs):
                spooled.append(kwargs)

        controller.kernel_client = type(
            "FakeKernelClient",
            (),
            {"kernel_tasks": FailingKernelTasks()},
        )()
        controller.memory_store = FakeMemoryStore()

        self.assertFalse(
            controller._submit_result(
                "agent_tasks:abc",
                False,
                {"status": "failed"},
            )
        )
        self.assertEqual("agent_task_result_submit_failed", spooled[0]["event_type"])
        self.assertEqual("kernel:/agent-tasks/result", spooled[0]["destination"])
        self.assertEqual("agent_tasks:abc", spooled[0]["payload"]["task_id"])
        self.assertEqual("organ-forge", spooled[0]["payload"]["domain"])
        self.assertEqual("kernel down", spooled[0]["payload"]["error"])

    def test_lifecycle_agent_recognizes_activation_task_contract(self):
        request_payload = build_lifecycle_activation_requests(registry_fixture())["requests"][0]
        task = AgentTask.from_dict(
            {
                "id": "agent_tasks:activation",
                "kind": "hermes",
                "domain": "organ-forge",
                "agent_id": "organ-forge-cli",
                "content": request_payload["content"],
                "status": "pending",
            }
        )
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        submitted = {}

        def fake_submit(task_id, success, result_data, **kwargs):
            submitted["task_id"] = task_id
            submitted["success"] = success
            submitted["result"] = result_data

        controller._submit_result = fake_submit

        with self.assertLogs(
            "organs.forge.controllers.lifecycle_agent",
            level="INFO",
        ) as logs:
            __import__("asyncio").run(controller.execute_task(task))

        self.assertEqual(submitted["task_id"], "agent_tasks:activation")
        self.assertFalse(submitted["success"])
        self.assertEqual(
            submitted["result"]["error_kind"],
            "activation_action_not_implemented",
        )
        self.assertEqual(submitted["result"]["activation_schema_version"], "activation.v1")
        self.assertRegex(submitted["result"]["correlation_id"], r"^activation:[0-9a-f]{16}$")
        self.assertNotEqual(submitted["result"].get("error"), "Missing intent in task content")
        self.assertTrue(
            any("has no executor handler yet" in message for message in logs.output)
        )
        self.assertFalse(any(record.levelno >= logging.WARNING for record in logs.records))

    def test_lifecycle_agent_rejects_activation_schema_mismatch(self):
        request_payload = build_lifecycle_activation_requests(registry_fixture())["requests"][0]
        content = json.loads(request_payload["content"])
        content["activation_schema_version"] = "activation.v0"
        task = AgentTask.from_dict(
            {
                "id": "agent_tasks:activation",
                "kind": "hermes",
                "domain": "organ-forge",
                "agent_id": "organ-forge-cli",
                "content": json.dumps(content, separators=(",", ":")),
                "status": "pending",
            }
        )
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        submitted = {}
        controller._submit_result = lambda task_id, success, result_data, **kwargs: submitted.update(
            {"task_id": task_id, "success": success, "result": result_data}
        )

        __import__("asyncio").run(controller.execute_task(task))

        self.assertFalse(submitted["success"])
        self.assertEqual(
            submitted["result"]["error_kind"],
            "activation_schema_version_mismatch",
        )

    def test_lifecycle_agent_executes_intent_without_redispatching(self):
        task = AgentTask.from_dict(
            {
                "id": "agent_tasks:intent",
                "kind": "hermes",
                "domain": "organ-forge",
                "agent_id": "organ-forge-cli",
                "content": json.dumps(
                    {
                        "lifecycle": "kernel_release",
                        "intent": "build-and-reload-mcp",
                        "request_profile": "mcp-reload",
                        "deploy_target": "cynic-vm-v2",
                        "coord_targets": ["crates/cynic-mcp"],
                    }
                ),
                "status": "pending",
            }
        )
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        submitted = {}
        controller._delegate_intent = mock.AsyncMock()
        controller.kernel_client = type(
            "FakeKernelClient",
            (),
            {
                "build_target": "cynic-vm-anvil",
                "build_node": {"name": "cynic-vm-anvil", "aliases": ["forge", "ubuntu"]},
            },
        )()
        controller.intent_executor = mock.Mock(
            execute=mock.Mock(
                return_value={
                    "status": "ok",
                    "intent": "build-and-reload-mcp",
                    "request_profile": "mcp-reload",
                }
            )
        )
        controller._submit_result = lambda task_id, success, result_data, **kwargs: submitted.update(
            {"task_id": task_id, "success": success, "result": result_data}
        )

        __import__("asyncio").run(controller.execute_task(task))

        controller._delegate_intent.assert_not_called()
        controller.intent_executor.execute.assert_called_once_with(task)
        self.assertTrue(submitted["success"])
        self.assertEqual(submitted["result"]["request_profile"], "mcp-reload")

    def test_lifecycle_agent_blocks_intent_on_non_build_runner_host(self):
        task = AgentTask.from_dict(
            {
                "id": "agent_tasks:intent",
                "kind": "hermes",
                "domain": "organ-forge",
                "agent_id": "organ-forge-cli",
                "content": json.dumps(
                    {
                        "lifecycle": "kernel_release",
                        "intent": "build-and-reload-mcp",
                        "request_profile": "mcp-reload",
                        "build_target": "cynic-vm-anvil",
                    }
                ),
                "status": "pending",
            }
        )
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        submitted = {}
        controller.kernel_client = type(
            "FakeKernelClient",
            (),
            {
                "build_target": "cynic-vm-anvil",
                "build_node": {"name": "cynic-vm-anvil", "aliases": ["forge"]},
            },
        )()
        controller.intent_executor = mock.Mock()
        controller._submit_result = lambda task_id, success, result_data, **kwargs: submitted.update(
            {"task_id": task_id, "success": success, "result": result_data}
        )

        with mock.patch.object(
            controller,
            "_runtime_host_candidates",
            return_value=["ubuntu", "ubuntu.local"],
        ):
            __import__("asyncio").run(controller.execute_task(task))

        controller.intent_executor.execute.assert_not_called()
        self.assertFalse(submitted["success"])
        self.assertEqual(
            submitted["result"]["error_kind"],
            "lifecycle_execution_host_mismatch",
        )
        self.assertEqual(submitted["result"]["expected_build_target"], "cynic-vm-anvil")
        self.assertEqual(submitted["result"]["observed_execution_hosts"], ["ubuntu", "ubuntu.local"])

    def test_lifecycle_agent_handles_runtime_path_activation_when_verified(self):
        request_payload = next(
            item
            for item in build_lifecycle_activation_requests(registry_fixture())["requests"]
            if json.loads(item["content"])["action"]["action"] == "materialize_runtime_path"
        )
        task = AgentTask.from_dict(
            {
                "id": "agent_tasks:activation",
                "kind": "hermes",
                "domain": "organ-forge",
                "agent_id": "organ-forge-cli",
                "content": request_payload["content"],
                "status": "pending",
            }
        )
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        submitted = {}
        verification = {
            "status": "ok",
            "node": task.content.action["node"],
            "activation_evidence_patch": {
                "node": task.content.action["node"],
                "activation_evidence": {"runtime_paths": {"repo_root": {"verified": True}}},
            },
        }
        controller._verify_runtime_paths = lambda node_name: verification
        controller._persist_activation_evidence_patch = lambda patch: {
            "status": "ok",
            "registry_path": "/srv/cynic/.cynic/repos/CYNIC/infra/registry.json",
            "node": patch["node"],
            "accepted_runtime_paths": ["repo_root"],
            "accepted_services": [],
        }
        controller._submit_result = lambda task_id, success, result_data, **kwargs: submitted.update(
            {"task_id": task_id, "success": success, "result": result_data}
        )

        __import__("asyncio").run(controller.execute_task(task))

        self.assertTrue(submitted["success"])
        self.assertEqual(submitted["result"]["status"], "ok")
        self.assertEqual(submitted["result"]["action"], "materialize_runtime_path")
        self.assertEqual(
            submitted["result"]["persistence"]["registry_path"],
            "/srv/cynic/.cynic/repos/CYNIC/infra/registry.json",
        )
        self.assertEqual(submitted["result"]["persistence"]["accepted_runtime_paths"], ["repo_root"])

    def test_lifecycle_agent_requires_registry_path_after_runtime_persistence(self):
        request_payload = next(
            item
            for item in build_lifecycle_activation_requests(registry_fixture())["requests"]
            if json.loads(item["content"])["action"]["action"] == "materialize_runtime_path"
        )
        task = AgentTask.from_dict(
            {
                "id": "agent_tasks:activation",
                "kind": "hermes",
                "domain": "organ-forge",
                "agent_id": "organ-forge-cli",
                "content": request_payload["content"],
                "status": "pending",
            }
        )
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        submitted = {}
        controller._verify_runtime_paths = lambda node_name: {
            "status": "ok",
            "node": node_name,
            "activation_evidence_patch": {
                "node": node_name,
                "activation_evidence": {"runtime_paths": {"repo_root": {"verified": True}}},
            },
        }
        controller._persist_activation_evidence_patch = lambda patch: {
            "status": "ok",
            "accepted_runtime_paths": ["repo_root"],
            "accepted_services": [],
        }
        controller._submit_result = lambda task_id, success, result_data, **kwargs: submitted.update(
            {"task_id": task_id, "success": success, "result": result_data}
        )

        __import__("asyncio").run(controller.execute_task(task))

        self.assertFalse(submitted["success"])
        self.assertEqual(
            submitted["result"]["error_kind"],
            "activation_persistence_missing_registry_path",
        )

    def test_lifecycle_agent_requests_bootstrap_transport_when_runtime_path_unverified(self):
        request_payload = next(
            item
            for item in build_lifecycle_activation_requests(registry_fixture())["requests"]
            if json.loads(item["content"])["action"]["action"] == "materialize_runtime_path"
        )
        task = AgentTask.from_dict(
            {
                "id": "agent_tasks:activation",
                "kind": "hermes",
                "domain": "organ-forge",
                "agent_id": "organ-forge-cli",
                "content": request_payload["content"],
                "status": "pending",
            }
        )
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        submitted = {}
        controller._verify_runtime_paths = lambda node_name: {
            "status": "blocked",
            "node": node_name,
            "checks": [{"ok": False}],
        }
        controller._persist_activation_evidence_patch = mock.Mock()
        controller._submit_result = lambda task_id, success, result_data, **kwargs: submitted.update(
            {"task_id": task_id, "success": success, "result": result_data}
        )

        __import__("asyncio").run(controller.execute_task(task))

        self.assertFalse(submitted["success"])
        self.assertEqual(submitted["result"]["error_kind"], "bootstrap_transport_required")
        self.assertEqual(
            submitted["result"]["bootstrap_execution"]["transport"],
            "fleet_node_exec",
        )
        self.assertEqual(
            submitted["result"]["bootstrap_execution"]["requests"][0]["node"],
            "cynic-vm-anvil",
        )
        controller._persist_activation_evidence_patch.assert_not_called()

    def test_lifecycle_agent_bootstraps_then_persists_when_transport_succeeds(self):
        request_payload = next(
            item
            for item in build_lifecycle_activation_requests(registry_fixture())["requests"]
            if json.loads(item["content"])["action"]["action"] == "materialize_runtime_path"
        )
        task = AgentTask.from_dict(
            {
                "id": "agent_tasks:activation",
                "kind": "hermes",
                "domain": "organ-forge",
                "agent_id": "organ-forge-cli",
                "content": request_payload["content"],
                "status": "pending",
            }
        )
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        submitted = {}
        verification_calls = []

        def fake_verify(node_name):
            verification_calls.append(node_name)
            if len(verification_calls) == 1:
                return {
                    "status": "blocked",
                    "node": node_name,
                    "checks": [{"ok": False}],
                }
            return {
                "status": "ok",
                "node": node_name,
                "activation_evidence_patch": {
                    "node": node_name,
                    "activation_evidence": {
                        "runtime_paths": {"repo_root": {"verified": True}}
                    },
                },
            }

        controller._verify_runtime_paths = fake_verify
        controller._execute_bootstrap_contract = lambda contract: {
            "status": "ok",
            "node": contract["node"],
            "results": [{"stage": "repo_materialized", "ok": True}],
        }
        controller._persist_activation_evidence_patch = lambda patch: {
            "status": "ok",
            "registry_path": "/srv/cynic/.cynic/repos/CYNIC/infra/registry.json",
            "node": patch["node"],
            "accepted_runtime_paths": ["repo_root"],
            "accepted_services": [],
        }
        controller._submit_result = lambda task_id, success, result_data, **kwargs: submitted.update(
            {"task_id": task_id, "success": success, "result": result_data}
        )

        __import__("asyncio").run(controller.execute_task(task))

        self.assertTrue(submitted["success"])
        self.assertEqual(verification_calls, ["cynic-vm-anvil", "cynic-vm-anvil"])
        self.assertEqual(submitted["result"]["bootstrap_execution"]["status"], "ok")

    def test_lifecycle_agent_persists_activation_evidence_in_daemon_repo(self):
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        controller.repo_path = "/srv/cynic/CYNIC"
        patch = {"node": "cynic-vm-anvil", "activation_evidence": {"runtime_paths": {}}}

        with mock.patch(
            "organs.forge.controllers.lifecycle_agent.persist_lifecycle_activation_evidence_patch",
            return_value={"status": "ok"},
        ) as persist:
            result = controller._persist_activation_evidence_patch(patch)

        self.assertEqual(result["status"], "ok")
        persist.assert_called_once_with(
            patch,
            registry_path=Path("/srv/cynic/CYNIC") / "infra" / "registry.json",
        )

    def test_lifecycle_agent_derives_runtime_home_from_daemon_repo_path(self):
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        controller.repo_path = "/srv/cynic/.cynic/repos/CYNIC"

        self.assertEqual(controller._runtime_home(), "/srv/cynic")

    def test_lifecycle_agent_rejects_ambiguous_runtime_home(self):
        controller = LifecycleAgentController.__new__(LifecycleAgentController)
        controller.repo_path = "/srv/cynic/CYNIC"

        with self.assertRaisesRegex(RuntimeError, "repo_path must resolve"):
            controller._runtime_home()
