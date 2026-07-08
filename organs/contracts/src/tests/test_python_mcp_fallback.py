import importlib.util
import unittest
from pathlib import Path
from unittest import mock


PROJECT_ROOT = Path(__file__).resolve().parents[4]
MODULE_PATH = PROJECT_ROOT / ".cortex" / "mcp" / "python_rest_mcp.py"


def load_module():
    spec = importlib.util.spec_from_file_location("python_rest_mcp", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class PythonMcpFallbackTests(unittest.TestCase):
    def test_initialize_and_tools_list(self):
        module = load_module()

        initialized = module._handle(
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05"}}
        )
        listed = module._handle({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})

        self.assertEqual("cynic-python-rest-mcp", initialized["result"]["serverInfo"]["name"])
        tool_names = {tool["name"] for tool in listed["result"]["tools"]}
        self.assertIn("cynic_coord_claim", tool_names)
        self.assertIn("cynic_coord_claim_batch", tool_names)
        self.assertIn("cynic_observe", tool_names)
        self.assertIn("cynic_dispatch_agent_task", tool_names)
        self.assertIn("cynic_list_completed_agent_tasks", tool_names)
        self.assertIn("cynic_lifecycle_status", tool_names)
        self.assertIn("cynic_lifecycle_dispatch", tool_names)
        self.assertIn("cynic_health", tool_names)
        for tool in listed["result"]["tools"]:
            self.assertIn("inputSchema", tool)
            self.assertEqual("object", tool["inputSchema"]["type"])

    def test_health_returns_degraded_kernel_body(self):
        module = load_module()

        with mock.patch.object(
            module,
            "kernel_json_request",
            return_value=(503, '{"status":"degraded"}', {"status": "degraded"}),
        ):
            result = module.cynic_health()

        self.assertEqual("degraded", result["status"])
        self.assertEqual(503, result["http_status"])

    def test_unknown_tool_reports_protocol_error(self):
        module = load_module()

        response = module._handle(
            {
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {"name": "missing_tool", "arguments": {}},
            }
        )

        self.assertEqual(-32602, response["error"]["code"])

    def test_lifecycle_dispatch_requires_pending_or_completed_readback(self):
        module = load_module()

        with (
            mock.patch.object(
                module,
                "build_lifecycle_profile_payloads",
                return_value={
                    "payloads": [
                        {
                            "kind": "hermes",
                            "domain": "gate-2-request",
                            "content": '{"intent":"gate-2"}',
                            "agent_id": "cortex-test",
                        }
                    ]
                },
            ),
            mock.patch.object(
                module,
                "_request",
                side_effect=[
                    {"task_id": "task-1"},
                    {"count": 0, "tasks": []},
                    {"count": 0, "tasks": []},
                ],
            ),
        ):
            result = module.cynic_lifecycle_dispatch("gate-2", agent_id="cortex-test")

        self.assertEqual("error", result["status"])
        self.assertEqual(["task-1"], result["missing_dispatched_ids"])

    def test_lifecycle_payloads_default_to_organ_forge_consumer_domain(self):
        module = load_module()

        with mock.patch.object(
            module,
            "build_lifecycle_profile_payloads",
            return_value={"status": "ok", "payloads": []},
        ) as build:
            module.cynic_lifecycle_payloads("mcp-reload", agent_id="cortex-test")

        build.assert_called_once_with(
            profile="mcp-reload",
            domain="organ-forge",
            agent_id="cortex-test",
        )

    def test_lifecycle_status_returns_frictionless_report(self):
        module = load_module()

        with (
            mock.patch.object(
                module,
                "lifecycle_transition_contract",
                return_value={
                    "status": "ok",
                    "lifecycle": "kernel_release",
                    "transition_contract": {},
                },
            ),
            mock.patch.object(
                module,
                "lifecycle_blockers_payload",
                return_value={
                    "status_nodes_ready": True,
                    "fleet_nodes_ready": True,
                    "lifecycle_nodes_ready": True,
                    "blockers": [],
                    "activation_request_count": 0,
                    "build_target": "cynic-vm-anvil",
                    "deploy_target": "cynic-vm-v2",
                },
            ),
        ):
            result = module.cynic_lifecycle_status()

        self.assertEqual("cynic.lifecycle_status.v1", result["schema"])
        self.assertEqual("ready_for_dispatch", result["state"])
        self.assertTrue(result["can_dispatch"])

    def test_lifecycle_dispatch_succeeds_with_completed_readback(self):
        module = load_module()

        with (
            mock.patch.object(
                module,
                "build_lifecycle_profile_payloads",
                return_value={
                    "payloads": [
                        {
                            "kind": "hermes",
                            "domain": "deploy-request",
                            "content": '{"intent":"deploy"}',
                            "agent_id": "cortex-test",
                        }
                    ]
                },
            ),
            mock.patch.object(
                module,
                "_request",
                side_effect=[
                    {"task_id": "task-2"},
                    {"count": 0, "tasks": []},
                    {"count": 1, "tasks": [{"task_id": "task-2"}]},
                ],
            ),
        ):
            result = module.cynic_lifecycle_dispatch("deploy", agent_id="cortex-test")

        self.assertEqual("dispatched", result["status"])
        self.assertEqual(["task-2"], result["visible_dispatched_ids"])
        self.assertEqual([], result["missing_dispatched_ids"])


if __name__ == "__main__":
    unittest.main()
