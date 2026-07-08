import json
import subprocess
import unittest
from unittest import mock

from organs.forge.src.adapters.fleet_node_exec_adapter import FleetNodeExecCommandTransport


class FleetNodeExecCommandTransportTests(unittest.TestCase):
    def test_transport_sends_json_request_to_configured_command(self):
        calls = []

        def fake_runner(argv, **kwargs):
            calls.append({"argv": argv, **kwargs})
            return subprocess.CompletedProcess(
                argv,
                0,
                stdout=json.dumps({"status": "ok", "exec": {"exit_code": 0}}),
                stderr="",
            )

        transport = FleetNodeExecCommandTransport(["fleet-node-exec-wrapper"], fake_runner)
        result = transport.execute(
            node="cynic-vm-anvil",
            command="hostname",
            timeout_seconds=30,
        )

        self.assertTrue(result["ok"])
        self.assertEqual(result["status"], "ok")
        request = json.loads(calls[0]["input"])
        self.assertEqual(request["tool"], "fleet_node_exec")
        self.assertEqual(request["node"], "cynic-vm-anvil")
        self.assertEqual(request["command"], "hostname")
        self.assertEqual(calls[0]["timeout"], 35)
        self.assertFalse(calls[0]["check"])

    def test_transport_blocks_on_nonzero_wrapper_exit(self):
        def fake_runner(argv, **kwargs):
            return subprocess.CompletedProcess(argv, 2, stdout="", stderr="no route")

        transport = FleetNodeExecCommandTransport(["fleet-node-exec-wrapper"], fake_runner)
        result = transport.execute(
            node="cynic-vm-anvil",
            command="hostname",
            timeout_seconds=30,
        )

        self.assertFalse(result["ok"])
        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["returncode"], 2)
        self.assertEqual(result["stderr"], "no route")

    def test_transport_blocks_on_non_json_stdout(self):
        def fake_runner(argv, **kwargs):
            return subprocess.CompletedProcess(argv, 0, stdout="not-json", stderr="")

        transport = FleetNodeExecCommandTransport(["fleet-node-exec-wrapper"], fake_runner)
        result = transport.execute(
            node="cynic-vm-anvil",
            command="hostname",
            timeout_seconds=30,
        )

        self.assertFalse(result["ok"])
        self.assertEqual(result["status"], "blocked")
        self.assertIn("non-JSON", result["error"])

    def test_transport_from_env_is_optional_and_split(self):
        with mock.patch.dict(
            "os.environ",
            {"CYNIC_FLEET_NODE_EXEC_COMMAND": "python3 /opt/cynic/fleet-node-exec.py"},
            clear=False,
        ):
            transport = FleetNodeExecCommandTransport.from_env()

        self.assertIsNotNone(transport)
        self.assertEqual(
            transport.argv,
            ["python3", "/opt/cynic/fleet-node-exec.py"],
        )

        with mock.patch.dict("os.environ", {}, clear=True):
            self.assertIsNone(FleetNodeExecCommandTransport.from_env())


if __name__ == "__main__":
    unittest.main()
