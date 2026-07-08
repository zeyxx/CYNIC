import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

from organs.fleet.src.adapters.host_exec_adapter import HostExecPolicyFile
from organs.fleet.src.core.host_exec import HostExecPolicy, HostExecRequest
from organs.fleet.src.entrypoints import cli


POLICY = {
    "schema": "cynic.host_exec.policy.v1",
    "helper_path": "/usr/local/libexec/cynic-host-exec",
    "sudoers_subject": "cynic",
    "max_timeout_seconds": 60,
    "allowed_purposes": {
        "read_only": {
            "allowed_roles": ["cortex_host", "inference_node", "hypervisor"],
            "argv_prefixes": [["hostname"], ["systemctl", "is-active"]],
        },
        "service_control": {
            "allowed_roles": ["cortex_host"],
            "argv_prefixes": [["systemctl", "restart"], ["systemctl", "status"]],
            "requires_declared_service": True,
        },
    },
    "forbidden_tokens": ["sh", "bash", "-c", "|", "&&", ";"],
}


class HostExecTests(unittest.TestCase):
    def test_read_only_allows_hostname_for_cortex_host(self):
        policy = HostExecPolicy(POLICY)
        result = policy.validate_request(
            HostExecRequest(
                node="cynic-host-cortex",
                role="cortex_host",
                purpose="read_only",
                argv=("hostname",),
                agent_id="cortex-test",
            )
        )

        self.assertTrue(result.ok)
        self.assertEqual((), result.violations)

    def test_rejects_role_outside_allowed_purpose(self):
        policy = HostExecPolicy(POLICY)
        result = policy.validate_request(
            HostExecRequest(
                node="cynic-host-proxmox",
                role="hypervisor",
                purpose="service_control",
                argv=("systemctl", "restart", "proxmox.service"),
                agent_id="cortex-test",
            ),
            declared_services={"proxmox"},
        )

        self.assertFalse(result.ok)
        self.assertIn("role_not_allowed_for_purpose", result.violations)

    def test_service_control_requires_declared_service(self):
        policy = HostExecPolicy(POLICY)
        result = policy.validate_request(
            HostExecRequest(
                node="cynic-host-cortex",
                role="cortex_host",
                purpose="service_control",
                argv=("systemctl", "restart", "cynic-node.service"),
                agent_id="cortex-test",
            ),
            declared_services={"surrealdb"},
        )

        self.assertFalse(result.ok)
        self.assertIn("service_not_declared", result.violations)

    def test_rendered_sudoers_allows_helper_not_systemctl(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "policy.json"
            path.write_text(json.dumps(POLICY), encoding="utf-8")

            sudoers = HostExecPolicyFile(path).render_sudoers()

        self.assertIn("/usr/local/libexec/cynic-host-exec", sudoers)
        self.assertNotIn("/usr/bin/systemctl", sudoers)
        self.assertNotIn("/bin/sh", sudoers)

    def test_install_bundle_writes_helper_policy_sudoers_and_manifest(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / "policy.json"
            helper = root / "helper.py"
            bundle = root / "bundle"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            helper.write_text("#!/usr/bin/env python3\nprint('ok')\n", encoding="utf-8")

            manifest = HostExecPolicyFile(path).write_install_bundle(helper, bundle)

            self.assertTrue((bundle / "cynic-host-exec").exists())
            self.assertTrue((bundle / "host-exec.policy.json").exists())
            self.assertTrue((bundle / "cynic-host-exec.sudoers").exists())
            self.assertTrue((bundle / "manifest.json").exists())
            self.assertEqual(manifest["bundle_files"]["helper"], str(bundle / "cynic-host-exec"))

    def test_activation_plan_references_bundle_and_install_targets(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / "policy.json"
            helper = root / "helper.py"
            bundle = root / "bundle"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            helper.write_text("#!/usr/bin/env python3\nprint('ok')\n", encoding="utf-8")
            policy_file = HostExecPolicyFile(path)
            policy_file.write_install_bundle(helper, bundle)

            plan = policy_file.activation_plan(helper, bundle_dir=bundle)

        self.assertEqual(plan["schema"], "cynic.host_exec.activation_plan.v1")
        self.assertTrue(plan["bundle_expected_complete"])
        self.assertIn("/tmp/cynic-host-exec", plan["preflight"][0])
        self.assertEqual(
            plan["install_commands"][0],
            "install -d -o root -g root -m 0755 /usr/local/libexec",
        )
        self.assertIn("/usr/local/libexec/cynic-host-exec", plan["install_commands"][3])
        self.assertIn("activation-status reports status=active", plan["success_criteria"])

    def test_install_plan_prepares_parent_directories_before_install(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / "policy.json"
            helper = root / "helper.py"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            helper.write_text("#!/usr/bin/env python3\nprint('ok')\n", encoding="utf-8")

            plan = HostExecPolicyFile(path).install_plan(helper)

        self.assertEqual(
            plan["prepare_directories"],
            [
                "install -d -o root -g root -m 0755 /usr/local/libexec",
                "install -d -o root -g root -m 0755 /etc/cynic",
                "install -d -o root -g root -m 0755 /etc/sudoers.d",
            ],
        )
        self.assertEqual(plan["activation_checks"][:3], plan["prepare_directories"])

    def test_activation_status_accepts_minimal_safe_install(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / "policy.json"
            helper = root / "helper.py"
            bundle = root / "bundle"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            helper.write_text("#!/usr/bin/env python3\nprint('ok')\n", encoding="utf-8")
            policy_file = HostExecPolicyFile(path)
            policy_file.write_install_bundle(helper, bundle)
            snapshot = {
                "helper": {"exists": True, "mode": "0755", "executable": True},
                "policy": {"exists": True, "mode": "0644"},
                "sudoers": {
                    "exists": True,
                    "mode": "0440",
                    "content": policy_file.render_sudoers(),
                },
            }
            status = policy_file.activation_status(snapshot, bundle_dir=bundle)

        self.assertEqual(status["status"], "active")
        self.assertEqual([], status["missing"])

    def test_cli_validates_policy(self):
        argv = [
            "fleet",
            "host-exec",
            "validate-policy",
            "--json",
            "--policy",
            "organs/fleet/policies/host-exec.policy.json",
        ]
        stdout = io.StringIO()
        with patch.object(sys, "argv", argv), redirect_stdout(stdout):
            status = cli.main()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["schema"], "cynic.host_exec.policy_validation.v1")

    def test_cli_emits_host_exec_activation_plan(self):
        stdout = io.StringIO()
        with patch.object(
            sys,
            "argv",
            [
                "fleet",
                "host-exec",
                "activation-plan",
                "--bundle-dir",
                "/tmp/cynic-host-exec",
            ],
        ), redirect_stdout(stdout):
            status = cli.main()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["schema"], "cynic.host_exec.activation_plan.v1")

    def test_cli_host_exec_activation_status_reports_bundle_evidence(self):
        stdout = io.StringIO()
        with patch.object(
            sys,
            "argv",
            [
                "fleet",
                "host-exec",
                "activation-status",
                "--bundle-dir",
                "/tmp/cynic-host-exec",
            ],
        ), redirect_stdout(stdout):
            status = cli.main()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertIn("bundle_complete", payload)


if __name__ == "__main__":
    unittest.main()
