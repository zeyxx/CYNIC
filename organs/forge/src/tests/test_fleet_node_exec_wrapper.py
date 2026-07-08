import json
import unittest
from unittest import mock

from organs.forge.src.entrypoints import fleet_node_exec_wrapper


class FleetNodeExecWrapperTests(unittest.TestCase):
    def test_wrapper_reads_json_and_calls_mcp_client(self):
        calls = []

        class FakeClient:
            def call_fleet_node_exec(self, request):
                calls.append(request)
                return {"status": "ok", "ok": True, "node": request["node"]}

        with mock.patch.object(
            fleet_node_exec_wrapper.McpFleetNodeExecClient,
            "from_env",
            return_value=FakeClient(),
        ):
            result = fleet_node_exec_wrapper.run(
                json.dumps(
                    {
                        "tool": "fleet_node_exec",
                        "node": "cynic-vm-anvil",
                        "command": "hostname",
                        "timeout_seconds": 30,
                    }
                )
            )

        self.assertEqual(result["status"], "ok")
        self.assertEqual(calls[0]["node"], "cynic-vm-anvil")
        self.assertEqual(calls[0]["command"], "hostname")

    def test_wrapper_rejects_non_object_json(self):
        with self.assertRaisesRegex(RuntimeError, "JSON object"):
            fleet_node_exec_wrapper.run("[]")


if __name__ == "__main__":
    unittest.main()
