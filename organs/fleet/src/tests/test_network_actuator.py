import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

from organs.fleet.src.adapters.network_actuator_adapter import NetworkActuatorPolicyFile
from organs.fleet.src.core.network_actuator import NetworkActuatorPolicy, NetworkActuatorRequest
from organs.fleet.src.entrypoints import cli


POLICY = {
    "schema": "cynic.network_actuator.policy.v1",
    "helper_path": "/usr/local/libexec/cynic-network-actuator",
    "sudoers_subject": "cynic",
    "max_timeout_seconds": 60,
    "allowed_profiles": {
        "synapse-route": {
            "command_prefixes": [["ip", "route", "replace"], ["ip", "route", "show"]],
        },
        "wireguard-mesh": {
            "command_prefixes": [["systemctl", "start", "wg-quick@wg0.service"], ["wg", "show"]],
        },
    },
    "forbidden_tokens": ["sh", "bash", "-c", "|", "&&", ";"],
}


class NetworkActuatorTests(unittest.TestCase):
    def test_policy_allows_declared_profile_and_prefixes(self):
        policy = NetworkActuatorPolicy(POLICY)
        result = policy.validate_request(
            NetworkActuatorRequest(
                profile="synapse-route",
                commands=(("ip", "route", "replace", "10.20.0.0/24", "dev", "wg0"),),
                agent_id="cortex-test",
                apply=True,
            )
        )

        self.assertTrue(result.ok)
        self.assertEqual((), result.violations)

    def test_policy_rejects_missing_apply_flag_and_shell_tokens(self):
        policy = NetworkActuatorPolicy(POLICY)
        result = policy.validate_request(
            NetworkActuatorRequest(
                profile="synapse-route",
                commands=(("sh", "-c", "ip route replace 10.20.0.0/24 dev wg0"),),
                agent_id="cortex-test",
                apply=False,
            )
        )

        self.assertFalse(result.ok)
        self.assertIn("apply_flag_required", result.violations)
        self.assertIn("forbidden_token", result.violations)

    def test_rendered_sudoers_allows_helper_not_raw_commands(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "policy.json"
            path.write_text(json.dumps(POLICY), encoding="utf-8")

            sudoers = NetworkActuatorPolicyFile(path).render_sudoers()

        self.assertIn("/usr/local/libexec/cynic-network-actuator", sudoers)
        self.assertNotIn("/usr/sbin/ip", sudoers)
        self.assertNotIn("/bin/sh", sudoers)

    def test_install_bundle_writes_helper_policy_sudoers_and_manifest(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / "policy.json"
            helper = root / "helper.py"
            bundle = root / "bundle"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            helper.write_text("#!/usr/bin/env python3\nprint('ok')\n", encoding="utf-8")

            manifest = NetworkActuatorPolicyFile(path).write_install_bundle(helper, bundle)

            self.assertTrue((bundle / "cynic-network-actuator").exists())
            self.assertTrue((bundle / "network-actuator.policy.json").exists())
            self.assertTrue((bundle / "cynic-network-actuator.sudoers").exists())
            self.assertTrue((bundle / "manifest.json").exists())
            self.assertEqual(manifest["bundle_files"]["helper"], str(bundle / "cynic-network-actuator"))

    def test_activation_plan_contains_preflight_install_postflight_and_denial_checks(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / "policy.json"
            helper = root / "helper.py"
            bundle = root / "bundle"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            helper.write_text("#!/usr/bin/env python3\nprint('ok')\n", encoding="utf-8")
            policy_file = NetworkActuatorPolicyFile(path)
            policy_file.write_install_bundle(helper, bundle)

            plan = policy_file.activation_plan(helper, bundle_dir=bundle)

        self.assertEqual(plan["schema"], "cynic.network_actuator.activation_plan.v1")
        self.assertTrue(plan["bundle_expected_complete"])
        self.assertTrue(plan["preflight"])
        self.assertTrue(plan["install_commands"])
        self.assertEqual(
            plan["install_commands"][0],
            "install -d -o root -g root -m 0755 /usr/local/libexec",
        )
        self.assertTrue(plan["postflight"])
        self.assertTrue(plan["denial_checks"])

    def test_install_plan_prepares_parent_directories_before_install(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / "policy.json"
            helper = root / "helper.py"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            helper.write_text("#!/usr/bin/env python3\nprint('ok')\n", encoding="utf-8")

            plan = NetworkActuatorPolicyFile(path).install_plan(helper)

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
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            policy_file = NetworkActuatorPolicyFile(path)
            bundle = root / "bundle"
            bundle.mkdir()
            for name in [
                "manifest.json",
                "cynic-network-actuator",
                "network-actuator.policy.json",
                "cynic-network-actuator.sudoers",
            ]:
                (bundle / name).write_text("x\n", encoding="utf-8")
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
        self.assertTrue(status["bundle_complete"])
        self.assertEqual([], status["next_actions"])
        self.assertIn("root_authority", status)
        self.assertIn("sudo_noninteractive_available", status["root_authority"])

    def test_activation_status_reports_missing_bundle_when_not_provided(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "policy.json"
            path.write_text(json.dumps(POLICY), encoding="utf-8")
            policy_file = NetworkActuatorPolicyFile(path)
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

        self.assertEqual(status["status"], "missing_activation")
        self.assertIn("bundle_complete", status["missing"])
        self.assertIn("build_network_actuator_bundle", status["next_actions"])
        self.assertIn("root_authority", status)

    def test_cli_emits_network_activation_plan(self):
        argv = [
            "fleet",
            "network-actuator",
            "activation-plan",
            "--bundle-dir",
            "/tmp/cynic-network-actuator",
        ]
        stdout = io.StringIO()
        with patch.object(sys, "argv", argv), redirect_stdout(stdout):
            status = cli.main()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["schema"], "cynic.network_actuator.activation_plan.v1")

    def test_cli_validates_network_policy(self):
        argv = [
            "fleet",
            "network-actuator",
            "validate-policy",
            "--json",
            "--policy",
            "organs/fleet/policies/network-actuator.policy.json",
        ]
        stdout = io.StringIO()
        with patch.object(sys, "argv", argv), redirect_stdout(stdout):
            status = cli.main()

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertTrue(payload["ok"])
        self.assertEqual(payload["schema"], "cynic.network_actuator.policy_validation.v1")


if __name__ == "__main__":
    unittest.main()
