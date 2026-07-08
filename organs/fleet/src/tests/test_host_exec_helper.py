import importlib.util
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import Mock

from organs.contracts.src.project_root import resolve_repo_root
from organs.fleet.src.core.host_exec import HostExecPolicy


MODULE_PATH = (
    resolve_repo_root(Path(__file__).resolve()) / "organs" / "fleet" / "src" / "entrypoints" / "host_exec_helper.py"
)

POLICY = {
    "schema": "cynic.host_exec.policy.v1",
    "helper_path": "/usr/local/libexec/cynic-host-exec",
    "sudoers_subject": "cynic",
    "max_timeout_seconds": 60,
    "allowed_purposes": {
        "read_only": {
            "allowed_roles": ["cortex_host", "inference_node", "hypervisor"],
            "argv_prefixes": [["hostname"], ["systemctl", "status"]],
        },
        "service_control": {
            "allowed_roles": ["cortex_host", "inference_node"],
            "argv_prefixes": [["systemctl", "restart"]],
            "requires_declared_service": True,
        },
    },
    "forbidden_tokens": ["sh", "bash", "-c", "|", "&&", ";"],
}


def load_module():
    spec = importlib.util.spec_from_file_location("host_exec_helper", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class HostExecHelperTests(unittest.TestCase):
    def test_dry_run_returns_command_without_executing(self):
        module = load_module()
        runner = Mock()

        result = module.run_request(
            {
                "node": "cynic-host-cortex",
                "role": "cortex_host",
                "purpose": "read_only",
                "command": ["hostname"],
                "agent_id": "cortex-test",
            },
            policy=HostExecPolicy(POLICY),
            declared_services=set(),
            dry_run=True,
            runner=runner,
        )

        self.assertEqual(result["status"], "dry_run")
        self.assertEqual(result["command"], ["hostname"])
        runner.assert_not_called()

    def test_rejects_non_root_when_not_dry_run(self):
        module = load_module()

        result = module.run_request(
            {
                "node": "cynic-host-cortex",
                "role": "cortex_host",
                "purpose": "service_control",
                "command": ["systemctl", "restart", "cynic-node.service"],
                "agent_id": "cortex-test",
            },
            policy=HostExecPolicy(POLICY),
            declared_services={"cynic-node"},
            dry_run=False,
            geteuid=lambda: 1000,
        )

        self.assertEqual(result["status"], "rejected")
        self.assertIn("helper_must_run_as_root", result["violations"])

    def test_executes_commands_as_argv_without_shell(self):
        module = load_module()
        completed = subprocess.CompletedProcess(
            args=["hostname"],
            returncode=0,
            stdout="ubuntu\n",
            stderr="",
        )
        runner = Mock(return_value=completed)

        result = module.run_request(
            {
                "node": "cynic-host-cortex",
                "role": "cortex_host",
                "purpose": "read_only",
                "command": ["hostname"],
                "agent_id": "cortex-test",
            },
            policy=HostExecPolicy(POLICY),
            declared_services=set(),
            dry_run=False,
            runner=runner,
            geteuid=lambda: 0,
        )

        self.assertEqual(result["status"], "completed")
        runner.assert_called_once()
        args, kwargs = runner.call_args
        self.assertEqual(args[0], ["hostname"])
        self.assertFalse(kwargs["check"])
        self.assertTrue(kwargs["capture_output"])

    def test_rejects_undeclared_service_before_execution(self):
        module = load_module()
        runner = Mock()

        result = module.run_request(
            {
                "node": "cynic-host-cortex",
                "role": "cortex_host",
                "purpose": "service_control",
                "command": ["systemctl", "restart", "cynic-node.service"],
                "agent_id": "cortex-test",
            },
            policy=HostExecPolicy(POLICY),
            declared_services={"surrealdb"},
            dry_run=False,
            runner=runner,
            geteuid=lambda: 0,
        )

        self.assertEqual(result["status"], "rejected")
        self.assertIn("service_not_declared", result["violations"])
        runner.assert_not_called()

    def test_helper_response_schema_is_stable(self):
        module = load_module()
        payload = module.response("dry_run")
        self.assertEqual(payload["schema"], "cynic.host_exec.response.v1")
        self.assertEqual(payload["status"], "dry_run")


if __name__ == "__main__":
    unittest.main()
