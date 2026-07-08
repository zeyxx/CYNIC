from pathlib import Path
import unittest

from organs.forge.src.core.lifecycle_activation import build_lifecycle_activation_plan
from organs.forge.src.core.lifecycle_bootstrap_executor import (
    BuildRunnerBootstrapExecutor,
    build_bootstrap_commands,
)
from organs.forge.src.tests.fixtures import registry_fixture


def build_runner_bootstrap_contract():
    plan = build_lifecycle_activation_plan(registry_fixture())
    action = next(
        item
        for item in plan["actions"]
        if item["action"] == "materialize_runtime_path"
        and item["role"] == "build_runner"
    )
    return action["bootstrap_contract"]


def build_runner_bootstrap_contract_git_fetch():
    registry = registry_fixture()
    registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_materialization"] = "git_fetch"
    registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_source"] = {
        "remote": "ssh://git@forgejo.synapse.cynic/CYNIC/CYNIC.git",
        "ref": "main",
        "ref_env": "CYNIC_FORGEJO_BOOTSTRAP_REF",
        "transport": "ssh",
        "host_role": "forge_primary",
        "fallback_remote_env": "CYNIC_FORGEJO_BOOTSTRAP_REMOTE",
        "fallback_transport": "local_tunnel",
    }
    plan = build_lifecycle_activation_plan(registry)
    action = next(
        item
        for item in plan["actions"]
        if item["action"] == "materialize_runtime_path"
        and item["role"] == "build_runner"
    )
    return action["bootstrap_contract"]


class LifecycleBootstrapExecutorTests(unittest.TestCase):
    def test_bootstrap_contract_builds_fleet_node_exec_requests(self):
        contract = build_runner_bootstrap_contract()
        result = BuildRunnerBootstrapExecutor().execute(contract)

        self.assertEqual(result["status"], "requires_transport")
        self.assertEqual(result["transport"], "fleet_node_exec")
        self.assertEqual(result["transport_command_env"], "CYNIC_FLEET_NODE_EXEC_COMMAND")
        self.assertEqual(
            result["transport_command_default"],
            "python3 organs/forge/src/entrypoints/fleet_node_exec_wrapper.py",
        )
        self.assertEqual(result["node"], "cynic-vm-anvil")
        self.assertGreaterEqual(len(result["requests"]), 8)
        self.assertTrue(
            all(request["tool"] == "fleet_node_exec" for request in result["requests"])
        )
        self.assertTrue(
            all(request["node"] == "cynic-vm-anvil" for request in result["requests"])
        )
        commands = [request["command"] for request in result["requests"]]
        
        expected_dest = Path.home() / ".cynic/repos/CYNIC"
        repo_materialize_command = next(
            command for command in commands if f'dest="{expected_dest}"' in command
        )
        
        self.assertIn('src=${CYNIC_PROJECT_ROOT:-}', repo_materialize_command)
        self.assertIn('cp -a "$src"/. "$tmp"/', repo_materialize_command)
        self.assertIn("dpkg-query -W ca-certificates musl-tools", commands)
        repo_materialized_command = next(
            command for command in commands if ".cynic/repos/CYNIC/.git" in command
        )
        self.assertIn(".git/HEAD", repo_materialized_command)
        self.assertIn(".git/config", repo_materialized_command)
        self.assertIn(".git/objects", repo_materialized_command)
        repo_registry_contract_command = next(
            command for command in commands if "remote lifecycle registry contract missing:" in command
        )
        self.assertIn("lifecycle.kernel_release.build_policy", repo_registry_contract_command)
        self.assertIn("orchestrator_organ", repo_registry_contract_command)
        self.assertIn("remote lifecycle registry contract missing:", repo_registry_contract_command)
        
        expected_build_cmd = f"cd {expected_dest} && cargo +1.88.0 build -p cynic-node --release --target x86_64-unknown-linux-musl"
        self.assertIn(expected_build_cmd, commands)
        self.assertTrue(any("grep -E 'static|static-pie'" in command for command in commands))

    def test_bootstrap_contract_builds_git_fetch_materialization_requests(self):
        contract = build_runner_bootstrap_contract_git_fetch()
        result = BuildRunnerBootstrapExecutor().execute(contract)

        self.assertEqual(result["status"], "requires_transport")
        commands = [request["command"] for request in result["requests"]]
        repo_materialize_command = next(
            command for command in commands if 'git clone --origin origin --branch "$ref"' in command
        )
        self.assertIn('remote="ssh://git@forgejo.synapse.cynic/CYNIC/CYNIC.git"', repo_materialize_command)
        self.assertIn('ref_default="main"', repo_materialize_command)
        self.assertIn('ref_env="CYNIC_FORGEJO_BOOTSTRAP_REF"', repo_materialize_command)
        self.assertIn('eval "ref=\\${CYNIC_FORGEJO_BOOTSTRAP_REF:-$ref_default}"', repo_materialize_command)
        self.assertIn('fallback_env="CYNIC_FORGEJO_BOOTSTRAP_REMOTE"', repo_materialize_command)
        self.assertIn('fallback_remote="${CYNIC_FORGEJO_BOOTSTRAP_REMOTE:-}"', repo_materialize_command)
        self.assertIn('git ls-remote "$selected_remote" HEAD', repo_materialize_command)
        self.assertIn('git fetch --prune origin "$ref"', repo_materialize_command)
        self.assertIn('git reset --hard "origin/$ref"', repo_materialize_command)

    def test_bootstrap_executor_uses_injected_transport(self):
        contract = build_runner_bootstrap_contract()
        calls = []

        class FakeTransport:
            def execute(self, *, node, command, timeout_seconds):
                calls.append(
                    {
                        "node": node,
                        "command": command,
                        "timeout_seconds": timeout_seconds,
                    }
                )
                return {"status": "ok"}

        result = BuildRunnerBootstrapExecutor(FakeTransport()).execute(contract)

        self.assertEqual(result["status"], "ok")
        self.assertEqual(len(calls), len(build_bootstrap_commands(contract)))
        self.assertEqual(calls[0]["node"], "cynic-vm-anvil")

    def test_bootstrap_executor_stops_on_failed_stage(self):
        contract = build_runner_bootstrap_contract()

        class FailingTransport:
            def execute(self, *, node, command, timeout_seconds):
                if "rustup target list" in command:
                    return {"status": "blocked", "stderr": "missing musl target"}
                return {"status": "ok"}

        result = BuildRunnerBootstrapExecutor(FailingTransport()).execute(contract)

        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["failed_stage"], "rust_musl_target_present")

    def test_bootstrap_contract_rejects_wrong_schema(self):
        with self.assertRaisesRegex(RuntimeError, "unsupported bootstrap contract schema"):
            BuildRunnerBootstrapExecutor().execute({"schema": "wrong"})


if __name__ == "__main__":
    unittest.main()
