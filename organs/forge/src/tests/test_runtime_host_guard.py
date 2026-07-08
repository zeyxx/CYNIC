import types
import unittest
from unittest import mock

from organs.forge.src.core import runtime_host_guard


class RuntimeHostGuardTests(unittest.TestCase):
    def test_runtime_host_candidates_deduplicates_and_filters_empty_values(self):
        with mock.patch.object(
            runtime_host_guard.os,
            "uname",
            return_value=types.SimpleNamespace(nodename="forge"),
        ), mock.patch.object(runtime_host_guard.socket, "gethostname", return_value="forge"), mock.patch.object(
            runtime_host_guard.socket,
            "getfqdn",
            return_value="forge.local",
        ):
            hosts = runtime_host_guard.runtime_host_candidates()

        self.assertEqual(hosts, ["forge", "forge.local"])

    def test_build_runner_host_guard_allows_matching_alias(self):
        self.assertIsNone(
            runtime_host_guard.build_runner_host_guard(
                expected_build_target="cynic-vm-anvil",
                build_node={"name": "forge", "aliases": ["ubuntu", "runner"]},
                observed_hosts=["unknown", "runner", "other"],
            )
        )

    def test_build_runner_host_guard_blocks_when_mismatch(self):
        payload = runtime_host_guard.build_runner_host_guard(
            expected_build_target="cynic-vm-anvil",
            build_node={"name": "forge", "aliases": ["ubuntu"]},
            observed_hosts=["alpha", "beta"],
            registry_build_target="cynic-vm-anvil",
            intent="build-and-reload-mcp",
            task_domain="organ-forge",
        )

        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["error_kind"], "lifecycle_execution_host_mismatch")
        self.assertEqual(payload["expected_build_target"], "cynic-vm-anvil")
        self.assertEqual(payload["registry_build_target"], "cynic-vm-anvil")
        self.assertEqual(payload["intent"], "build-and-reload-mcp")
        self.assertEqual(payload["task_domain"], "organ-forge")
        self.assertEqual(payload["allowed_execution_hosts"], ["cynic-vm-anvil", "forge", "ubuntu"])
        self.assertEqual(payload["observed_execution_hosts"], ["alpha", "beta"])
