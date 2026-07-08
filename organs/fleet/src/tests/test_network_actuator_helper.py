import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import Mock

from organs.contracts.src.project_root import resolve_repo_root
from organs.fleet.src.core.network_actuator import NetworkActuatorPolicy


MODULE_PATH = (
    resolve_repo_root(Path(__file__).resolve()) / "organs" / "fleet" / "src" / "entrypoints" / "network_actuator_helper.py"
)

POLICY = {
    "schema": "cynic.network_actuator.policy.v1",
    "helper_path": "/usr/local/libexec/cynic-network-actuator",
    "sudoers_subject": "cynic",
    "max_timeout_seconds": 60,
    "allowed_profiles": {
        "synapse-route": {
            "command_prefixes": [["ip", "route", "replace"], ["ip", "route", "show"]],
        }
    },
    "forbidden_tokens": ["sh", "bash", "-c", "|", "&&", ";"],
}


def load_module():
    spec = importlib.util.spec_from_file_location("network_actuator_helper", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class NetworkActuatorHelperTests(unittest.TestCase):
    def test_dry_run_returns_commands_without_executing(self):
        module = load_module()
        runner = Mock()

        result = module.run_request(
            {
                "profile": "synapse-route",
                "commands": [["ip", "route", "show", "10.20.0.0/24"]],
                "agent_id": "cortex-test",
            },
            policy=NetworkActuatorPolicy(POLICY),
            dry_run=True,
            runner=runner,
        )

        self.assertEqual(result["status"], "dry_run")
        self.assertEqual(result["commands"], [["ip", "route", "show", "10.20.0.0/24"]])
        runner.assert_not_called()

    def test_rejects_non_root_when_not_dry_run(self):
        module = load_module()

        result = module.run_request(
            {
                "profile": "synapse-route",
                "commands": [["ip", "route", "replace", "10.20.0.0/24", "dev", "wg0"]],
                "agent_id": "cortex-test",
                "apply": True,
            },
            policy=NetworkActuatorPolicy(POLICY),
            dry_run=False,
            geteuid=lambda: 1000,
        )

        self.assertEqual(result["status"], "rejected")
        self.assertIn("helper_must_run_as_root", result["violations"])

    def test_executes_commands_as_argv_without_shell(self):
        module = load_module()
        completed = subprocess.CompletedProcess(
            args=["ip", "route", "show", "10.20.0.0/24"],
            returncode=0,
            stdout="10.20.0.0/24 dev wg0\n",
            stderr="",
        )
        runner = Mock(return_value=completed)

        result = module.run_request(
            {
                "profile": "synapse-route",
                "commands": [["ip", "route", "show", "10.20.0.0/24"]],
                "agent_id": "cortex-test",
                "apply": True,
            },
            policy=NetworkActuatorPolicy(POLICY),
            dry_run=False,
            runner=runner,
            geteuid=lambda: 0,
        )

        self.assertEqual(result["status"], "completed")
        runner.assert_called_once()
        args, kwargs = runner.call_args
        self.assertEqual(args[0], ["ip", "route", "show", "10.20.0.0/24"])
        self.assertFalse(kwargs["check"])
        self.assertTrue(kwargs["capture_output"])

    def test_rejects_disallowed_profile_before_execution(self):
        module = load_module()
        runner = Mock()

        result = module.run_request(
            {
                "profile": "wireguard-mesh",
                "commands": [["wg", "show"]],
                "agent_id": "cortex-test",
                "apply": True,
            },
            policy=NetworkActuatorPolicy(POLICY),
            dry_run=False,
            runner=runner,
            geteuid=lambda: 0,
        )

        self.assertEqual(result["status"], "rejected")
        self.assertIn("profile_not_allowed", result["violations"])
        runner.assert_not_called()

    def test_helper_response_schema_is_stable(self):
        module = load_module()
        payload = module.response("dry_run")
        self.assertEqual(payload["schema"], "cynic.network_actuator.response.v1")
        self.assertEqual(payload["status"], "dry_run")


if __name__ == "__main__":
    unittest.main()
