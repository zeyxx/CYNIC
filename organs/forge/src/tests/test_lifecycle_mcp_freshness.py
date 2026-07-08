import unittest
from datetime import datetime, timezone
from unittest import mock

from organs.forge.src.core.lifecycle_mcp_freshness import build_mcp_freshness_plan
from organs.forge.src.entrypoints import lifecycle_diagnostic_cli
from organs.forge.src.tests.fixtures import registry_fixture


class LifecycleMcpFreshnessTests(unittest.TestCase):
    def test_fresh_mcp_surface_requires_no_action(self):
        plan = build_mcp_freshness_plan(
            registry_fixture(),
            {"status": "fresh"},
            now=datetime(2026, 7, 2, 12, 0, tzinfo=timezone.utc),
        )

        self.assertEqual(plan["status"], "ok")
        self.assertEqual(plan["action"], "no_action")
        self.assertIsNone(plan["dispatch"])
        self.assertEqual(plan["profile"], "mcp-reload")
        self.assertEqual("cynic-mcp", plan["artifact"]["name"])
        self.assertEqual("rust_binary", plan["artifact"]["kind"])
        self.assertIn("infra/agent-exposure.json", plan["trigger_sources"])

    def test_stale_mcp_surface_dispatches_mcp_reload_profile(self):
        plan = build_mcp_freshness_plan(registry_fixture(), {"status": "stale"})

        self.assertEqual(plan["status"], "ok")
        self.assertEqual(plan["action"], "dispatch_lifecycle_profile")
        self.assertEqual(plan["dispatch"]["profile"], "mcp-reload")
        self.assertEqual(plan["dispatch"]["tool"], "cynic_lifecycle_dispatch")
        self.assertEqual(plan["dispatch"]["kind"], "hermes")
        self.assertEqual(plan["dispatch"]["domain"], "organ-forge")
        self.assertEqual(plan["dispatch"]["consumer"], "organ-forge-hermes-agent")
        self.assertEqual(plan["debounce_seconds"], 900)

    def test_missing_failed_and_timeout_are_actionable(self):
        for status in ["missing", "failed", "timeout"]:
            with self.subTest(status=status):
                plan = build_mcp_freshness_plan(registry_fixture(), {"status": status})
                self.assertEqual(plan["action"], "dispatch_lifecycle_profile")
                self.assertIn(status, plan["reason"])

    def test_pending_duplicate_debounces_dispatch(self):
        plan = build_mcp_freshness_plan(
            registry_fixture(),
            {"status": "stale"},
            pending_duplicate=True,
        )

        self.assertEqual(plan["action"], "observe_existing_request")
        self.assertIsNone(plan["dispatch"])
        self.assertEqual(plan["debounce_seconds"], 900)

    def test_unknown_status_is_blocked(self):
        plan = build_mcp_freshness_plan(registry_fixture(), {"status": "weird"})

        self.assertEqual(plan["status"], "blocked")
        self.assertEqual(plan["action"], "observe_unknown_status")

    def test_lifecycle_mcp_freshness_payload_maps_stale_launcher_check_to_dispatch(self):
        registry = registry_fixture()
        completed = mock.Mock(
            returncode=0,
            stdout='{"schema":"cynic.mcp_launcher_check.v1","status":"stale","canonical_binary":"/tmp/cynic-mcp","stale_source":"/tmp/src.rs"}\n',
            stderr="",
        )

        with mock.patch.object(
            lifecycle_diagnostic_cli.subprocess,
            "run",
            return_value=completed,
        ):
            payload = lifecycle_diagnostic_cli.lifecycle_mcp_freshness_payload(registry=registry)

        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["action"], "dispatch_lifecycle_profile")
        self.assertEqual(payload["dispatch"]["profile"], "mcp-reload")
        self.assertEqual(payload["check"]["stdout"]["status"], "stale")

    def test_lifecycle_mcp_freshness_payload_maps_timeout_to_dispatch(self):
        registry = registry_fixture()

        with mock.patch.object(
            lifecycle_diagnostic_cli.subprocess,
            "run",
            side_effect=lifecycle_diagnostic_cli.subprocess.TimeoutExpired(["bash"], 30),
        ):
            payload = lifecycle_diagnostic_cli.lifecycle_mcp_freshness_payload(registry=registry)

        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["freshness_status"], "timeout")
        self.assertEqual(payload["action"], "dispatch_lifecycle_profile")


if __name__ == "__main__":
    unittest.main()
