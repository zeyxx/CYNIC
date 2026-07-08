#!/usr/bin/env python3
import tempfile
import unittest
import json
import io
from contextlib import redirect_stdout
from unittest import mock
from pathlib import Path

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


ADAPTER_PATH = Path(__file__).resolve().parent.parent / "adapters" / "hermes_execution_dependencies.py"
CLI_PATH = Path(__file__).resolve().parent.parent / "entrypoints" / "cli.py"


class HermesExecutionDependenciesTests(unittest.TestCase):
    def test_blocks_when_backend_and_synapse_dns_are_not_ready(self):
        module = load_module_from_path("hermes_execution_dependencies_adapter", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            backends = Path(tmpdir) / "backends.toml"
            backends.write_text("", encoding="utf-8")
            payload = module.evaluate_hermes_execution_dependencies(
                backends_toml=backends,
                expected_backend_id="qwen36-27b-gpu",
                observed_at="2026-07-03T00:00:00Z",
                backend_readiness_provider=lambda: {
                    "schema": "cynic.hermes_fleet_backend_readiness.v1",
                    "status": "degraded",
                    "backend_id": "qwen36-27b-gpu",
                    "base_url": "http://cynic-host-gpu.synapse.cynic:8080/v1",
                    "expected_model": "Qwen3.6-27B-IQ3_XXS.gguf",
                    "violations": [{"kind": "synapse_mesh_route_unreachable"}],
                    "next_actions": [
                        {
                            "owner": "organ-fleet",
                            "action": "restore_synapse_mesh_route",
                            "command": "python3 organs/fleet/src/entrypoints/cli.py mesh status --resolve-dns --include-records",
                            "done_when": "10.20.0.7:8080 is reachable",
                        }
                    ],
                },
                dns_activation_status_provider=lambda: {
                    "schema": "cynic.organ_fleet.dns_activation_status.v1",
                    "status": "blocked",
                    "missing": [
                        "coredns_binary_available",
                        "canonical_dns_provider_available",
                        "hosts_exists",
                    ],
                    "checks": {"coredns_binary_available": False},
                    "provider_status": {"canonical_available": False},
                },
                executor_liveness_provider=lambda: {
                    "schema": "cynic.hermes_executor_liveness.v1",
                    "status": "not_registered",
                    "service_id": "hermes-agent-executor",
                    "observed_services": 0,
                },
            )

        self.assertEqual("cynic.hermes_execution_dependencies.v1", payload["schema"])
        self.assertEqual("blocked", payload["status"])
        blocker_kinds = {item["kind"] for item in payload["blockers"]}
        self.assertIn("hermes_executor_not_registered", blocker_kinds)
        self.assertIn("fleet_backend_not_ready", blocker_kinds)
        self.assertIn("synapse_dns_activation_not_active", blocker_kinds)
        action_names = {item["action"] for item in payload["next_actions"]}
        self.assertIn("start_or_repair_hermes_agent_executor", action_names)
        self.assertIn("install_canonical_coredns_provider", action_names)
        self.assertIn("install_synapse_dns_activation_bundle", action_names)
        self.assertIn("restore_synapse_mesh_route", action_names)

    def test_cli_manifest_exposes_execution_dependencies(self):
        module = load_module_from_path("contracts_cli_hermes_execution_dependencies", CLI_PATH)
        schemas = {item["schema"] for item in module.surface_manifest()["registry"]["reports"]}

        self.assertIn("cynic.hermes_execution_dependencies.v1", schemas)
        self.assertIn("cynic.hermes_execution_dependency_tasks.v1", schemas)
        self.assertIn("cynic.hermes_execution_dependency_dispatch.v1", schemas)
        self.assertIn("hermes-execution-dependency-tasks", module.USAGE)
        self.assertIn("hermes-execution-dependency-dispatch", module.USAGE)

    def test_materializes_dispatchable_organ_tasks_from_next_actions(self):
        module = load_module_from_path("hermes_execution_dependency_tasks_adapter", ADAPTER_PATH)
        payload = module.materialize_hermes_execution_dependency_tasks(
            dependencies={
                "schema": "cynic.hermes_execution_dependencies.v1",
                "status": "blocked",
                "backend_id": "qwen36-27b-gpu",
                "next_actions": [
                    {
                        "owner": "hermes-agent-executor",
                        "action": "start_or_repair_hermes_agent_executor",
                        "command": "systemctl --user status hermes-agent-executor.service --no-pager",
                        "done_when": "/services lists hermes-agent-executor as registered",
                    },
                    {
                        "owner": "organ-fleet",
                        "action": "install_synapse_dns_activation_bundle",
                        "command": "python3 organs/fleet/src/entrypoints/cli.py dns bundle --registry infra/registry.json --output-dir /tmp/cynic-synapse-dns",
                        "done_when": "dns activation-status reports bundle files present",
                    },
                    {
                        "owner": "organ-fleet",
                        "action": "restore_synapse_mesh_route",
                        "command": "python3 organs/fleet/src/entrypoints/cli.py mesh status --resolve-dns --include-records",
                        "done_when": "10.20.0.7:8080 is reachable",
                    },
                ],
            },
            agent_id="codex-test",
            observed_at="2026-07-03T00:00:00Z",
        )

        self.assertEqual("cynic.hermes_execution_dependency_tasks.v1", payload["schema"])
        self.assertEqual(2, payload["task_count"])
        by_domain = {task["domain"]: task for task in payload["tasks"]}
        task = by_domain["organ-fleet"]
        self.assertEqual("hermes", task["kind"])
        self.assertEqual("codex-test", task["agent_id"])
        content = json.loads(task["content"])
        self.assertEqual("cynic.hermes_dependency_remediation_task.v1", content["schema"])
        self.assertEqual("organ-fleet", content["owner"])
        self.assertEqual(2, len(content["actions"]))
        self.assertIn(
            "python3 organs/contracts/src/entrypoints/cli.py hermes-execution-dependencies --backends-toml backends.toml --registry infra/registry.json --expected-backend qwen36-27b-gpu --timeout-secs 2",
            content["proof_commands"],
        )
        executor_content = json.loads(by_domain["hermes-agent-executor"]["content"])
        self.assertEqual("hermes-agent-executor", executor_content["owner"])

    def test_cli_handler_materializes_dependency_tasks(self):
        module = load_module_from_path("contracts_cli_hermes_dependency_tasks", CLI_PATH)

        with mock.patch.object(
            module,
            "materialize_hermes_execution_dependency_tasks_from_args",
            return_value={
                "schema": "cynic.hermes_execution_dependency_tasks.v1",
                "task_count": 1,
                "tasks": [],
            },
        ):
            with redirect_stdout(io.StringIO()):
                payload = module.main(["hermes-execution-dependency-tasks"])

        self.assertEqual(0, payload)

    def test_dispatch_dependency_tasks_defaults_to_dry_run(self):
        module = load_module_from_path("hermes_execution_dependency_dispatch_adapter", ADAPTER_PATH)
        dispatcher = mock.Mock()
        report = module.dispatch_hermes_execution_dependency_tasks(
            task_report={
                "schema": "cynic.hermes_execution_dependency_tasks.v1",
                "tasks": [
                    {
                        "kind": "hermes",
                        "domain": "organ-fleet",
                        "objective": "restore mesh",
                        "content": "{}",
                    }
                ],
            },
            dispatcher=dispatcher,
            dispatch=False,
            observed_at="2026-07-03T00:00:00Z",
        )

        self.assertEqual("cynic.hermes_execution_dependency_dispatch.v1", report["schema"])
        self.assertFalse(report["mutates"])
        self.assertEqual("dry_run", report["mode"])
        self.assertEqual("dry_run", report["results"][0]["status"])
        dispatcher.dispatch_task.assert_not_called()

    def test_dispatch_dependency_tasks_posts_when_enabled(self):
        module = load_module_from_path("hermes_execution_dependency_dispatch_enabled", ADAPTER_PATH)
        dispatcher = mock.Mock()
        dispatcher.dispatch_task.return_value = True
        task = {
            "kind": "hermes",
            "domain": "organ-fleet",
            "objective": "restore mesh",
            "content": "{}",
        }

        report = module.dispatch_hermes_execution_dependency_tasks(
            task_report={"schema": "cynic.hermes_execution_dependency_tasks.v1", "tasks": [task]},
            dispatcher=dispatcher,
            dispatch=True,
            observed_at="2026-07-03T00:00:00Z",
        )

        self.assertTrue(report["mutates"])
        self.assertEqual("dispatch", report["mode"])
        self.assertEqual("dispatched", report["results"][0]["status"])
        dispatcher.dispatch_task.assert_called_once_with(task)


if __name__ == "__main__":
    unittest.main()
