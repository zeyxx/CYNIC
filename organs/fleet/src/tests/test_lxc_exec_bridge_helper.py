import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import Mock

from organs.contracts.src.project_root import resolve_repo_root
from organs.fleet.src.core.exec_bridge import LxcExecBridgePolicy


MODULE_PATH = (
    resolve_repo_root(Path(__file__).resolve()) / "organs" / "fleet" / "src" / "entrypoints" / "lxc_exec_bridge_helper.py"
)

POLICY = {
    "schema": "cynic.lxc_exec_bridge.policy.v1",
    "helper_path": "/usr/local/libexec/cynic-lxc-exec-bridge",
    "sudoers_subject": "cynic",
    "max_timeout_seconds": 60,
    "allowed_purposes": {
        "read_only": {"argv_prefixes": [["hostname"]]},
        "service_control": {
            "argv_prefixes": [["systemctl", "restart"]],
            "requires_declared_service": True,
        },
    },
    "forbidden_tokens": ["sh", "bash", "-c", "|", "&&", ";"],
}


def load_module():
    spec = importlib.util.spec_from_file_location("lxc_exec_bridge_helper", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class LxcExecBridgeHelperTests(unittest.TestCase):
    def test_dry_run_returns_pct_argv_without_executing(self):
        module = load_module()
        runner = Mock()

        result = module.run_bridge(
            {
                "node": "pve",
                "vmid": 103,
                "purpose": "read_only",
                "argv": ["hostname"],
                "agent_id": "cortex-test",
            },
            policy=LxcExecBridgePolicy(POLICY),
            declared_services=set(),
            dry_run=True,
            runner=runner,
        )

        self.assertEqual(result["status"], "dry_run")
        self.assertEqual(result["command"], ["pct", "exec", "103", "--", "hostname"])
        runner.assert_not_called()

    def test_rejects_non_root_when_not_dry_run(self):
        module = load_module()

        result = module.run_bridge(
            {
                "node": "pve",
                "vmid": 103,
                "purpose": "read_only",
                "argv": ["hostname"],
                "agent_id": "cortex-test",
            },
            policy=LxcExecBridgePolicy(POLICY),
            declared_services=set(),
            dry_run=False,
            geteuid=lambda: 1000,
        )

        self.assertEqual(result["status"], "rejected")
        self.assertIn("helper_must_run_as_root", result["violations"])

    def test_executes_pct_as_argv_list_without_shell(self):
        module = load_module()
        completed = subprocess.CompletedProcess(
            args=["pct", "exec", "103", "--", "hostname"],
            returncode=0,
            stdout="storage\n",
            stderr="",
        )
        runner = Mock(return_value=completed)

        result = module.run_bridge(
            {
                "node": "pve",
                "vmid": 103,
                "purpose": "read_only",
                "argv": ["hostname"],
                "agent_id": "cortex-test",
            },
            policy=LxcExecBridgePolicy(POLICY),
            declared_services=set(),
            dry_run=False,
            runner=runner,
            geteuid=lambda: 0,
        )

        self.assertEqual(result["status"], "completed")
        self.assertEqual(result["exit_code"], 0)
        runner.assert_called_once()
        args, kwargs = runner.call_args
        self.assertEqual(args[0], ["pct", "exec", "103", "--", "hostname"])
        self.assertFalse(kwargs["check"])
        self.assertTrue(kwargs["capture_output"])

    def test_rejects_undeclared_service_before_execution(self):
        module = load_module()
        runner = Mock()

        result = module.run_bridge(
            {
                "node": "pve",
                "vmid": 103,
                "purpose": "service_control",
                "argv": ["systemctl", "restart", "surrealdb.service"],
                "agent_id": "cortex-test",
            },
            policy=LxcExecBridgePolicy(POLICY),
            declared_services={"cynic-kernel"},
            dry_run=False,
            runner=runner,
            geteuid=lambda: 0,
        )

        self.assertEqual(result["status"], "rejected")
        self.assertIn("service_not_declared", result["violations"])
        runner.assert_not_called()

    def test_helper_main_dry_run_emits_json(self):
        module = load_module()
        request = json.dumps(
            {
                "node": "pve",
                "vmid": 103,
                "purpose": "read_only",
                "argv": ["hostname"],
                "agent_id": "cortex-test",
            }
        )

        self.assertIn("hostname", request)
        self.assertEqual(module.response("dry_run")["schema"], "cynic.lxc_exec_bridge.response.v1")


if __name__ == "__main__":
    unittest.main()
