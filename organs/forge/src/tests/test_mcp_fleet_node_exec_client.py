import io
import json
import subprocess
import unittest
from unittest import mock

from organs.forge.src.adapters.mcp_fleet_node_exec_client import McpFleetNodeExecClient


class FakeMcpProcess:
    def __init__(self, lines: list[dict | str], *, stderr: str = ""):
        self.stdin = io.StringIO()
        encoded_lines = [
            line if isinstance(line, str) else json.dumps(line)
            for line in lines
        ]
        self.stdout = io.StringIO("\n".join(encoded_lines) + "\n")
        self.stderr = io.StringIO(stderr)
        self.terminated = False

    def terminate(self):
        self.terminated = True

    def wait(self, timeout=None):
        return 0

    def kill(self):
        self.terminated = True


class McpFleetNodeExecClientTests(unittest.TestCase):
    def test_client_calls_fleet_node_exec_tool(self):
        process = FakeMcpProcess(
            [
                {"jsonrpc": "2.0", "id": 1, "result": {"protocolVersion": "2024-11-05"}},
                {
                    "jsonrpc": "2.0",
                    "id": 2,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": json.dumps(
                                    {
                                        "schema": "cynic.infrastructure_mcp.fleet_node_exec.v1",
                                        "status": "ok",
                                        "node": "cynic-vm-anvil",
                                    }
                                ),
                            }
                        ]
                    },
                },
            ]
        )

        def fake_popen(*args, **kwargs):
            return process

        client = McpFleetNodeExecClient(["python3", ".cortex/mcp/run_cynic_mcp.py"], ".", fake_popen)
        with mock.patch(
            "organs.forge.src.adapters.mcp_fleet_node_exec_client.select.select",
            return_value=([process.stdout], [], []),
        ):
            result = client.call_fleet_node_exec(
                {
                    "node": "cynic-vm-anvil",
                    "command": "hostname",
                    "timeout_seconds": 30,
                }
            )

        self.assertTrue(result["ok"])
        self.assertEqual(result["status"], "ok")
        sent = [json.loads(line) for line in process.stdin.getvalue().splitlines()]
        self.assertEqual(sent[0]["method"], "initialize")
        self.assertEqual(sent[1]["method"], "notifications/initialized")
        self.assertEqual(sent[2]["method"], "tools/call")
        self.assertEqual(sent[2]["params"]["name"], "fleet_node_exec")
        self.assertEqual(sent[2]["params"]["arguments"]["node"], "cynic-vm-anvil")
        self.assertEqual(sent[2]["params"]["arguments"]["command"], "hostname")

    def test_client_blocks_on_mcp_error(self):
        process = FakeMcpProcess(
            [
                {"jsonrpc": "2.0", "id": 1, "result": {"protocolVersion": "2024-11-05"}},
                {"jsonrpc": "2.0", "id": 2, "error": {"code": -32000, "message": "denied"}},
            ],
            stderr="tool-error-line\n",
        )
        client = McpFleetNodeExecClient(["mcp"], ".", lambda *args, **kwargs: process)
        with mock.patch(
            "organs.forge.src.adapters.mcp_fleet_node_exec_client.select.select",
            side_effect=[
                ([process.stdout], [], []),
                ([process.stdout], [], []),
                ([process.stderr], [], []),
                ([], [], []),
            ],
        ):
            result = client.call_fleet_node_exec(
                {"node": "cynic-vm-anvil", "command": "hostname"}
            )

        self.assertFalse(result["ok"])
        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["error"], "mcp_tool_error")
        self.assertIn("tool-error-line", result["stderr"])

    def test_client_requires_node_and_command(self):
        client = McpFleetNodeExecClient(["mcp"], ".", lambda *args, **kwargs: None)

        with self.assertRaisesRegex(RuntimeError, "node missing"):
            client.call_fleet_node_exec({"command": "hostname"})
        with self.assertRaisesRegex(RuntimeError, "command missing"):
            client.call_fleet_node_exec({"node": "cynic-vm-anvil"})


if __name__ == "__main__":
    unittest.main()
