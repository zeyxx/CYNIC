import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

from organs.fleet.src.adapters.lxc_exec_bridge_adapter import LxcExecBridgePolicyFile
from organs.fleet.src.core.exec_bridge import LxcExecBridgePolicy, LxcExecRequest
from organs.fleet.src.entrypoints import cli


POLICY = {
    "schema": "cynic.lxc_exec_bridge.policy.v1",
    "helper_path": "/usr/local/libexec/cynic-lxc-exec-bridge",
    "sudoers_subject": "cynic",
    "max_timeout_seconds": 60,
    "allowed_purposes": {
        "read_only": {
            "argv_prefixes": [["hostname"], ["systemctl", "is-active"]],
        },
        "service_control": {
            "argv_prefixes": [["systemctl", "restart"]],
            "requires_declared_service": True,
        },
    },
    "forbidden_tokens": ["sh", "bash", "-c", "|", "&&", ";"],
}


class LxcExecBridgeTests(unittest.TestCase):
    def test_read_only_allows_argv_prefix_without_shell(self):
        policy = LxcExecBridgePolicy(POLICY)
        result = policy.validate_request(
            LxcExecRequest(
                node="pve",
                vmid=103,
                purpose="read_only",
                argv=("hostname",),
                agent_id="cortex-test",
            )
        )

        self.assertTrue(result.ok)
        self.assertEqual((), result.violations)

    def test_rejects_shell_syntax_and_forbidden_tokens(self):
        policy = LxcExecBridgePolicy(POLICY)
        result = policy.validate_request(
            LxcExecRequest(
                node="pve",
                vmid=103,
                purpose="read_only",
                argv=("sh", "-c", "hostname && cat /etc/passwd"),
                agent_id="cortex-test",
            )
        )

        self.assertFalse(result.ok)
        self.assertIn("forbidden_token", result.violations)
        self.assertIn("shell_syntax_forbidden", result.violations)

    def test_service_control_requires_declared_service(self):
        policy = LxcExecBridgePolicy(POLICY)
        result = policy.validate_request(
            LxcExecRequest(
                node="pve",
                vmid=103,
                purpose="service_control",
                argv=("systemctl", "restart", "surrealdb.service"),
                agent_id="cortex-test",
            ),
            declared_services={"cynic-kernel"},
        )

        self.assertFalse(result.ok)
        self.assertIn("service_not_declared", result.violations)

    def test_service_control_accepts_declared_service(self):
        policy = LxcExecBridgePolicy(POLICY)
        result = policy.validate_request(
            LxcExecRequest(
                node="pve",
                vmid=103,
                purpose="service_control",
                argv=("systemctl", "restart", "surrealdb.service"),
                agent_id="cortex-test",
            ),
            declared_services={"surrealdb"},
        )

        self.assertTrue(result.ok)

    def test_rendered_sudoers_allows_helper_not_pct(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "policy.json"
            path.write_text(json.dumps(POLICY), encoding="utf-8")

            sudoers = LxcExecBridgePolicyFile(path).render_sudoers()

        self.assertIn("/usr/local/libexec/cynic-lxc-exec-bridge", sudoers)
        self.assertNotIn("/usr/sbin/pct", sudoers)
        self.assertNotIn("/bin/sh", sudoers)

    def test_install_plan_is_non_mutating_and_targets_helper(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "policy.json"
            helper = Path(tmp) / "helper.py"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            helper.write_text("# helper\n", encoding="utf-8")

            plan = LxcExecBridgePolicyFile(path).install_plan(helper)

        self.assertFalse(plan["mutates"])
        self.assertEqual(plan["helper_target"], "/usr/local/libexec/cynic-lxc-exec-bridge")
        self.assertEqual(plan["sudoers_target"], "/etc/sudoers.d/cynic-lxc-exec-bridge")
        self.assertNotIn("/usr/sbin/pct", plan["sudoers_rule"])

    def test_install_bundle_writes_helper_policy_sudoers_and_manifest(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / "policy.json"
            helper = root / "helper.py"
            bundle = root / "bundle"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            helper.write_text("#!/usr/bin/env python3\nprint('ok')\n", encoding="utf-8")

            manifest = LxcExecBridgePolicyFile(path).write_install_bundle(helper, bundle)

            self.assertTrue((bundle / "cynic-lxc-exec-bridge").exists())
            self.assertTrue((bundle / "lxc-exec-bridge.policy.json").exists())
            self.assertTrue((bundle / "cynic-lxc-exec-bridge.sudoers").exists())
            self.assertTrue((bundle / "manifest.json").exists())
            self.assertEqual(manifest["bundle_files"]["helper"], str(bundle / "cynic-lxc-exec-bridge"))
            self.assertNotIn("/usr/sbin/pct", (bundle / "cynic-lxc-exec-bridge.sudoers").read_text())

    def test_install_bundle_is_idempotent_after_sudoers_mode_is_locked(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / "policy.json"
            helper = root / "helper.py"
            bundle = root / "bundle"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            helper.write_text("#!/usr/bin/env python3\nprint('ok')\n", encoding="utf-8")
            policy_file = LxcExecBridgePolicyFile(path)

            policy_file.write_install_bundle(helper, bundle)
            policy_file.write_install_bundle(helper, bundle)

            sudoers = bundle / "cynic-lxc-exec-bridge.sudoers"
            self.assertEqual("0o440", oct(sudoers.stat().st_mode & 0o777))
            self.assertIn("/usr/local/libexec/cynic-lxc-exec-bridge", sudoers.read_text())

    def test_activation_status_reports_missing_paths(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "policy.json"
            path.write_text(json.dumps(POLICY), encoding="utf-8")

            snapshot = {
                "helper": {"exists": False, "mode": None, "executable": False},
                "policy": {"exists": False, "mode": None},
                "sudoers": {"exists": False, "mode": None, "content": ""},
            }
            status = LxcExecBridgePolicyFile(path).activation_status(snapshot)

        self.assertEqual(status["status"], "missing_activation")
        self.assertIn("helper_exists", status["missing"])
        self.assertIn("sudoers_rule_matches", status["missing"])

    def test_activation_status_accepts_minimal_safe_install(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "policy.json"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            policy_file = LxcExecBridgePolicyFile(path)
            snapshot = {
                "helper": {"exists": True, "mode": "0755", "executable": True},
                "policy": {"exists": True, "mode": "0644"},
                "sudoers": {
                    "exists": True,
                    "mode": "0440",
                    "content": policy_file.render_sudoers(),
                },
            }
            status = policy_file.activation_status(snapshot)

        self.assertEqual(status["status"], "active")
        self.assertEqual([], status["missing"])

    def test_cli_validates_policy(self):
        argv = [
            "fleet",
            "lxc-exec-bridge",
            "validate-policy",
            "--json",
            "--policy",
            "organs/fleet/policies/lxc-exec-bridge.policy.json",
        ]
        stdout = io.StringIO()
        with patch.object(sys, "argv", argv), redirect_stdout(stdout):
            status = cli.main()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["schema"], "cynic.lxc_exec_bridge.policy_validation.v1")


if __name__ == "__main__":
    unittest.main()
