import json
import os
import subprocess
import tempfile
import unittest
import os
os.environ.pop("CYNIC_FORGEJO_BOOTSTRAP_REF", None)
os.environ.pop("CYNIC_FORGEJO_BOOTSTRAP_REMOTE", None)

from pathlib import Path
from unittest import mock

from organs.forge.src.core.entities import AgentTask
from organs.forge.src.core.lifecycle_intent_executor import LifecycleIntentExecutor


def _task(intent="build-and-reload-mcp"):
    return AgentTask.from_dict(
        {
            "id": "agent_tasks:executor",
            "kind": "hermes",
            "domain": "organ-forge",
            "agent_id": "test",
            "status": "pending",
            "content": json.dumps(
                {
                    "lifecycle": "kernel_release",
                    "intent": intent,
                    "request_profile": "mcp-reload",
                    "deploy_target": "cynic-vm-v2",
                    "coord_targets": ["crates/cynic-mcp"],
                }
            ),
        }
    )


def _task_with_build_target(intent="build-and-reload-mcp", build_target="cynic-vm-anvil"):
    return AgentTask.from_dict(
        {
            "id": "agent_tasks:executor",
            "kind": "hermes",
            "domain": "organ-forge",
            "agent_id": "test",
            "status": "pending",
            "content": json.dumps(
                {
                    "lifecycle": "kernel_release",
                    "intent": intent,
                    "request_profile": "mcp-reload",
                    "deploy_target": "cynic-vm-v2",
                    "coord_targets": ["crates/cynic-mcp"],
                    "build_target": build_target,
                }
            ),
        }
    )


def _classification_task():
    return AgentTask.from_dict(
        {
            "id": "agent_tasks:classify",
            "kind": "hermes",
            "domain": "organ-forge",
            "agent_id": "test",
            "status": "pending",
            "content": json.dumps(
                {
                    "lifecycle": "kernel_release",
                    "intent": "classify-release-matter-for-sovereign-forgejo-proposal",
                    "request_profile": "release-matter-classification",
                    "deploy_target": "cynic-vm-v2",
                    "coord_targets": ["organs/forge"],
                }
            ),
        }
    )


class LifecycleIntentExecutorTests(unittest.TestCase):
    def test_release_matter_classification_groups_dirty_paths_for_forgejo_proposal(self):
        calls = []

        def runner(command, **kwargs):
            calls.append(command)
            if command[:2] == ["git", "rev-parse"] and command[-1] == "HEAD":
                return subprocess.CompletedProcess(command, 0, stdout="abc123\n", stderr="")
            if command == ["git", "branch", "--show-current"]:
                return subprocess.CompletedProcess(command, 0, stdout="main\n", stderr="")
            if command[:2] == ["git", "rev-parse"]:
                return subprocess.CompletedProcess(command, 0, stdout="origin/main\n", stderr="")
            if command == ["git", "remote", "-v"]:
                return subprocess.CompletedProcess(
                    command,
                    0,
                    stdout="origin\tssh://git@10.20.0.10/cynic/CYNIC.git (fetch)\n",
                    stderr="",
                )
            if command == ["git", "status", "--porcelain"]:
                return subprocess.CompletedProcess(
                    command,
                    0,
                    stdout=(
                        " M crates/cynic-mcp/src/proxy.rs\n"
                        "?? organs/forge/src/core/organ_memory.py\n"
                        " M Makefile\n"
                        "?? apps/kyon-mobile/README.md\n"
                    ),
                    stderr="",
                )
            return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

        result = LifecycleIntentExecutor(Path("/repo"), runner=runner).execute(_classification_task())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["dirty_count"], 4)
        groups = {group["name"]: group for group in result["groups"]}
        self.assertIn("mcp_runtime", groups)
        self.assertIn("organ_forge_lifecycle", groups)
        self.assertIn("command_surfaces", groups)
        self.assertIn("application_product_drift", groups)
        self.assertFalse(result["proposal_plan"]["cortex_commit_allowed"])
        self.assertFalse(result["proposal_plan"]["destructive_sync_allowed"])
        self.assertEqual(
            result["proposal_plan"]["recommended_actor"],
            "organ-forge-hermes-agent",
        )

    def test_release_matter_classification_blocks_without_forgejo_remote(self):
        def runner(command, **kwargs):
            if command == ["git", "status", "--porcelain"]:
                return subprocess.CompletedProcess(command, 0, stdout=" M Makefile\n", stderr="")
            if command == ["git", "remote", "-v"]:
                return subprocess.CompletedProcess(command, 0, stdout="origin\thttps://github.com/x/y (fetch)\n", stderr="")
            if command[0] == "git":
                return subprocess.CompletedProcess(command, 0, stdout="ok\n", stderr="")
            return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

        result = LifecycleIntentExecutor(Path("/repo"), runner=runner).execute(_classification_task())

        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["error_kind"], "forgejo_remote_not_detected")

    def test_mcp_reload_runs_build_between_freshness_checks(self):
        calls = []
        with tempfile.TemporaryDirectory() as tmpdir:
            repo = Path(tmpdir)
            binary = repo / "target" / "release" / "cynic-mcp"
            binary.parent.mkdir(parents=True)
            binary.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            binary.chmod(0o755)

            def runner(command, **kwargs):
                calls.append({"command": command, "kwargs": kwargs})
                if command[:2] == ["git", "rev-parse"] and command[-1] == "HEAD":
                    return subprocess.CompletedProcess(command, 0, stdout="abc123\n", stderr="")
                if command == ["git", "branch", "--show-current"]:
                    return subprocess.CompletedProcess(command, 0, stdout="main\n", stderr="")
                if command[:2] == ["git", "rev-parse"]:
                    return subprocess.CompletedProcess(command, 0, stdout="origin/main\n", stderr="")
                if command == ["git", "remote", "-v"]:
                    return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                if command == ["git", "status", "--porcelain"]:
                    return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                if command == ["cargo", "build", "-p", "cynic-mcp", "--release"]:
                    return subprocess.CompletedProcess(command, 0, stdout="built", stderr="")
                payload = {
                    "schema": "cynic.mcp_launcher_check.v1",
                    "status": "fresh",
                    "canonical_binary": str(binary),
                }
                return subprocess.CompletedProcess(command, 0, stdout=json.dumps(payload), stderr="")

            result = LifecycleIntentExecutor(repo, runner=runner).execute(_task())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(
            [call["command"] for call in calls[:5]],
            [
                ["git", "rev-parse", "HEAD"],
                ["git", "branch", "--show-current"],
                ["git", "rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
                ["git", "remote", "-v"],
                ["git", "status", "--porcelain"],
            ],
        )
        self.assertEqual(
            [call["command"] for call in calls[5:]],
            [
                ["python3", ".cortex/mcp/run_cynic_mcp.py"],
                ["cargo", "build", "-p", "cynic-mcp", "--release"],
                ["python3", ".cortex/mcp/run_cynic_mcp.py"],
            ],
        )
        self.assertEqual(calls[0]["kwargs"]["cwd"], str(repo))
        self.assertEqual(calls[5]["kwargs"]["env"]["CYNIC_MCP_CHECK_ONLY"], "1")
        self.assertEqual(result["runtime_dna"]["status"], "ok")
        self.assertEqual(result["execution_context"]["repo_path"], str(repo.resolve()))
        self.assertTrue(result["build_triggered"])
        self.assertEqual(result["read_back"]["status"], "fresh")
        self.assertEqual(result["artifact"]["status"], "ok")
        self.assertEqual(result["artifact"]["path"], str(binary))
        self.assertEqual(result["artifacts"], [str(binary)])
        self.assertEqual(len(result["artifact"]["sha256"]), 64)
        self.assertEqual(result["expected_artifact_path"], str(binary))
        self.assertEqual(
            calls[6]["kwargs"]["env"]["CARGO_TARGET_DIR"],
            str(repo / "target"),
        )
        self.assertEqual(
            calls[6]["kwargs"]["env"]["CYNIC_PROJECT_ROOT"],
            str(repo),
        )
        self.assertEqual(result["build_profile"], "release")

    def test_mcp_artifact_materialization_proof_can_succeed_without_rebuild(self):
        task = _task("mcp-artifact-materialization-proof")
        calls = []
        with tempfile.TemporaryDirectory() as tmpdir:
            repo = Path(tmpdir)
            binary = repo / "target" / "release" / "cynic-mcp"
            binary.parent.mkdir(parents=True)
            binary.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            binary.chmod(0o755)

            def runner(command, **kwargs):
                calls.append({"command": command, "kwargs": kwargs})
                if command[:2] == ["git", "rev-parse"] and command[-1] == "HEAD":
                    return subprocess.CompletedProcess(command, 0, stdout="abc123\n", stderr="")
                if command == ["git", "branch", "--show-current"]:
                    return subprocess.CompletedProcess(command, 0, stdout="main\n", stderr="")
                if command[:2] == ["git", "rev-parse"]:
                    return subprocess.CompletedProcess(command, 0, stdout="origin/main\n", stderr="")
                if command == ["git", "remote", "-v"]:
                    return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                if command == ["git", "status", "--porcelain"]:
                    return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                payload = {
                    "schema": "cynic.mcp_launcher_check.v1",
                    "status": "fresh",
                    "canonical_binary": str(binary),
                }
                return subprocess.CompletedProcess(command, 0, stdout=json.dumps(payload), stderr="")

            result = LifecycleIntentExecutor(repo, runner=runner).execute(task)

        self.assertEqual(result["status"], "ok")
        self.assertFalse(result["build_triggered"])
        self.assertEqual([call["command"] for call in calls[5:]], [["python3", ".cortex/mcp/run_cynic_mcp.py"]])
        self.assertEqual(result["read_back"]["status"], "fresh")
        self.assertEqual(result["artifact"]["path"], str(binary))
        self.assertEqual(result["build_profile"], "release")

    def test_mcp_reload_builds_debug_when_launcher_uses_local_debug_repair_binary(self):
        calls = []
        with tempfile.TemporaryDirectory() as tmpdir:
            repo = Path(tmpdir)
            binary = repo / "target" / "debug" / "cynic-mcp"
            binary.parent.mkdir(parents=True)
            binary.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            binary.chmod(0o755)

            def runner(command, **kwargs):
                calls.append({"command": command, "kwargs": kwargs})
                if command[:2] == ["git", "rev-parse"] and command[-1] == "HEAD":
                    return subprocess.CompletedProcess(command, 0, stdout="abc123\n", stderr="")
                if command == ["git", "branch", "--show-current"]:
                    return subprocess.CompletedProcess(command, 0, stdout="main\n", stderr="")
                if command[:2] == ["git", "rev-parse"]:
                    return subprocess.CompletedProcess(command, 0, stdout="origin/main\n", stderr="")
                if command == ["git", "remote", "-v"]:
                    return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                if command == ["git", "status", "--porcelain"]:
                    return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                if command == ["cargo", "build", "-p", "cynic-mcp"]:
                    return subprocess.CompletedProcess(command, 0, stdout="built", stderr="")
                payload = {
                    "schema": "cynic.mcp_launcher_check.v1",
                    "status": "stale" if len([c for c in calls if c["command"][0] == "python3"]) == 1 else "fresh",
                    "canonical_binary": str(binary),
                    "repair_local_debug_binary": True,
                }
                return subprocess.CompletedProcess(command, 78 if payload["status"] == "stale" else 0, stdout=json.dumps(payload), stderr="")

            result = LifecycleIntentExecutor(repo, runner=runner).execute(_task())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["build_profile"], "debug_repair")
        self.assertEqual(result["expected_artifact_path"], str(binary))
        self.assertEqual(
            [call["command"] for call in calls[5:]],
            [
                ["python3", ".cortex/mcp/run_cynic_mcp.py"],
                ["cargo", "build", "-p", "cynic-mcp"],
                ["python3", ".cortex/mcp/run_cynic_mcp.py"],
            ],
        )

    def test_mcp_reload_blocks_when_runtime_dna_is_degraded(self):
        calls = []

        def runner(command, **kwargs):
            calls.append(command)
            if command == ["git", "status", "--porcelain"]:
                return subprocess.CompletedProcess(command, 0, stdout=" M Makefile\n", stderr="")
            if command[0] == "git":
                return subprocess.CompletedProcess(command, 0, stdout="ok\n", stderr="")
            return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

        result = LifecycleIntentExecutor(Path("/repo"), runner=runner).execute(_task())

        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["error_kind"], "runtime_dna_not_clean_for_mcp_reload")
        self.assertFalse(result["build_triggered"])
        self.assertEqual(result["steps"], [])
        self.assertEqual(calls[:5], [
            ["git", "rev-parse", "HEAD"],
            ["git", "branch", "--show-current"],
            ["git", "rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
            ["git", "remote", "-v"],
            ["git", "status", "--porcelain"],
        ])

    def test_mcp_reload_blocks_when_runtime_branch_misses_bootstrap_ref(self):
        calls = []

        def runner(command, **kwargs):
            calls.append(command)
            if command[:2] == ["git", "rev-parse"] and command[-1] == "HEAD":
                return subprocess.CompletedProcess(command, 0, stdout="abc123\n", stderr="")
            if command == ["git", "branch", "--show-current"]:
                return subprocess.CompletedProcess(command, 0, stdout="main\n", stderr="")
            if command[:2] == ["git", "rev-parse"]:
                return subprocess.CompletedProcess(command, 0, stdout="origin/main\n", stderr="")
            if command == ["git", "remote", "-v"]:
                return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
            if command == ["git", "status", "--porcelain"]:
                return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
            return subprocess.CompletedProcess(command, 0, stdout="", stderr="")

        with mock.patch.dict(os.environ, {"CYNIC_FORGEJO_BOOTSTRAP_REF": "bootstrap/2026-07-03-runtime-snapshot"}):
            result = LifecycleIntentExecutor(Path("/repo"), runner=runner).execute(_task())

        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["error_kind"], "runtime_bootstrap_ref_mismatch")
        self.assertEqual(result["expected_bootstrap_ref"], "bootstrap/2026-07-03-runtime-snapshot")
        self.assertEqual(result["observed_branch"], "main")
        self.assertEqual(result["observed_upstream"], "origin/main")
        self.assertFalse(result["build_triggered"])
        self.assertEqual(result["steps"], [])

    def test_mcp_reload_fails_when_post_readback_is_not_fresh(self):
        responses = [
            subprocess.CompletedProcess([], 0, stdout='{"status":"stale"}', stderr=""),
            subprocess.CompletedProcess([], 0, stdout="built", stderr=""),
            subprocess.CompletedProcess([], 0, stdout='{"status":"stale"}', stderr=""),
        ]

        def runner(command, **kwargs):
            if command[0] == "git":
                if command == ["git", "status", "--porcelain"]:
                    return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                return subprocess.CompletedProcess(command, 0, stdout="ok\n", stderr="")
            return responses.pop(0)

        result = LifecycleIntentExecutor(Path("/repo"), runner=runner).execute(_task())

        self.assertEqual(result["status"], "failed")
        self.assertEqual(result["error_kind"], "mcp_freshness_readback_failed")
        self.assertEqual(result["read_back"]["status"], "stale")

    def test_mcp_artifact_materialization_proof_does_not_attempt_build_when_stale(self):
        task = _task("mcp-artifact-materialization-proof")
        calls = []

        def runner(command, **kwargs):
            calls.append({"command": command, "kwargs": kwargs})
            if command[0] == "git":
                if command == ["git", "status", "--porcelain"]:
                    return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                return subprocess.CompletedProcess(command, 0, stdout="ok\n", stderr="")
            return subprocess.CompletedProcess(
                command,
                0,
                stdout=json.dumps({"status": "stale", "canonical_binary": "/tmp/cynic-kernel-mcp"}),
                stderr="",
            )

        result = LifecycleIntentExecutor(Path("/repo"), runner=runner).execute(task)

        self.assertEqual(result["status"], "failed")
        self.assertFalse(result["build_triggered"])
        self.assertEqual(result["error_kind"], "mcp_freshness_readback_failed")
        self.assertEqual([call["command"] for call in calls[5:]], [["python3", ".cortex/mcp/run_cynic_mcp.py"]])
        self.assertEqual(result["steps"][0]["status"], "ok")
        self.assertEqual(result["steps"][0]["returncode"], 0)

    def test_mcp_reload_fails_when_reported_binary_is_missing(self):
        missing_path = "/tmp/cynic-mcp-does-not-exist"
        responses = [
            subprocess.CompletedProcess([], 0, stdout='{"status":"stale"}', stderr=""),
            subprocess.CompletedProcess([], 0, stdout="built", stderr=""),
            subprocess.CompletedProcess(
                [],
                0,
                stdout=json.dumps(
                    {
                        "status": "fresh",
                        "canonical_binary": missing_path,
                    }
                ),
                stderr="",
            ),
        ]

        def runner(command, **kwargs):
            if command[0] == "git":
                if command == ["git", "status", "--porcelain"]:
                    return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                return subprocess.CompletedProcess(command, 0, stdout="ok\n", stderr="")
            return responses.pop(0)

        result = LifecycleIntentExecutor(Path("/repo"), runner=runner).execute(_task())

        self.assertEqual(result["status"], "failed")
        self.assertEqual(result["error_kind"], "mcp_binary_proof_failed")
        self.assertEqual(result["artifact"]["status"], "missing")
        self.assertEqual(result["artifact"]["path"], missing_path)

    def test_mcp_reload_fails_when_fresh_binary_is_outside_runtime_repo_target(self):
        with tempfile.TemporaryDirectory() as tmp_home:
            outside_binary = Path(tmp_home) / "bin" / "cynic-kernel-mcp"
            outside_binary.parent.mkdir(parents=True, exist_ok=True)
            outside_binary.write_text("#!/bin/sh\n", encoding="utf-8")
            outside_binary.chmod(0o755)
            responses = [
                subprocess.CompletedProcess([], 0, stdout='{"status":"stale"}', stderr=""),
                subprocess.CompletedProcess([], 0, stdout="built", stderr=""),
                subprocess.CompletedProcess(
                    [],
                    0,
                    stdout=json.dumps(
                        {
                            "status": "fresh",
                            "canonical_binary": str(outside_binary),
                        }
                    ),
                    stderr="",
                ),
            ]

            def runner(command, **kwargs):
                if command[0] == "git":
                    if command == ["git", "status", "--porcelain"]:
                        return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                    return subprocess.CompletedProcess(command, 0, stdout="ok\n", stderr="")
                return responses.pop(0)

            with mock.patch.dict(os.environ, {"HOME": tmp_home}):
                result = LifecycleIntentExecutor(Path("/repo"), runner=runner).execute(_task())

        self.assertEqual(result["status"], "failed")
        self.assertEqual(result["error_kind"], "mcp_binary_projection_mismatch")
        self.assertEqual(result["observed_artifact_path"], str(outside_binary))
        self.assertEqual(result["expected_artifact_path"], "/repo/target/release/cynic-mcp")

    def test_missing_command_is_structured_step_failure(self):
        def runner(command, **kwargs):
            if command[0] == "git":
                if command == ["git", "status", "--porcelain"]:
                    return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                return subprocess.CompletedProcess(command, 0, stdout="ok\n", stderr="")
            if command == ["cargo", "build", "-p", "cynic-mcp", "--release"]:
                raise FileNotFoundError("cargo")
            return subprocess.CompletedProcess(command, 0, stdout='{"status":"fresh"}', stderr="")

        with mock.patch(
            "organs.forge.src.core.lifecycle_intent_executor._resolve_cargo_binary",
            return_value=None,
        ):
            result = LifecycleIntentExecutor("/repo", runner=runner).execute(_task())

        self.assertEqual(result["status"], "failed")
        self.assertEqual(result["error_kind"], "lifecycle_intent_command_failed")
        self.assertEqual(result["steps"][1]["name"], "build_cynic_mcp")
        self.assertEqual(result["steps"][1]["status"], "failed")
        self.assertIn("cargo", result["steps"][1]["stderr"])

    def test_missing_cargo_retries_with_resolved_home_binary(self):
        calls = []
        with tempfile.TemporaryDirectory() as tmpdir:
            repo = Path(tmpdir)
            cargo_home = repo / "home"
            cargo_bin = cargo_home / ".cargo" / "bin" / "cargo"
            cargo_bin.parent.mkdir(parents=True)
            cargo_bin.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            cargo_bin.chmod(0o755)
            binary = repo / "target" / "release" / "cynic-mcp"
            binary.parent.mkdir(parents=True)
            binary.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
            binary.chmod(0o755)

            def runner(command, **kwargs):
                calls.append({"command": command, "kwargs": kwargs})
                if command[0] == "git":
                    if command == ["git", "status", "--porcelain"]:
                        return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                    return subprocess.CompletedProcess(command, 0, stdout="ok\n", stderr="")
                if command == ["cargo", "build", "-p", "cynic-mcp", "--release"]:
                    raise FileNotFoundError("cargo")
                if command == [str(cargo_bin), "build", "-p", "cynic-mcp", "--release"]:
                    return subprocess.CompletedProcess(command, 0, stdout="built", stderr="")
                payload = {
                    "schema": "cynic.mcp_launcher_check.v1",
                    "status": "fresh",
                    "canonical_binary": str(binary),
                }
                return subprocess.CompletedProcess(command, 0, stdout=json.dumps(payload), stderr="")

            with mock.patch.dict(os.environ, {"HOME": str(cargo_home)}, clear=False), mock.patch("shutil.which", return_value=None):
                result = LifecycleIntentExecutor(repo, runner=runner).execute(_task())

        self.assertEqual(result["status"], "ok")
        self.assertTrue(result["steps"][1]["resolved_binary_retry"])
        self.assertEqual(
            result["steps"][1]["command"],
            [str(cargo_bin), "build", "-p", "cynic-mcp", "--release"],
        )

    def test_structured_mcp_check_returncode_78_is_not_treated_as_command_failure(self):
        task = _task("mcp-artifact-materialization-proof")

        def runner(command, **kwargs):
            if command[0] == "git":
                if command == ["git", "status", "--porcelain"]:
                    return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
                return subprocess.CompletedProcess(command, 0, stdout="ok\n", stderr="")
            return subprocess.CompletedProcess(
                command,
                78,
                stdout=json.dumps(
                    {
                        "schema": "cynic.mcp_launcher_check.v1",
                        "status": "stale",
                        "canonical_binary": "/tmp/cynic-kernel-mcp",
                    }
                ),
                stderr="",
            )

        result = LifecycleIntentExecutor(Path("/repo"), runner=runner).execute(task)

        self.assertEqual(result["error_kind"], "mcp_freshness_readback_failed")
        self.assertEqual(result["steps"][0]["status"], "ok")
        self.assertEqual(result["steps"][0]["returncode"], 78)
        self.assertEqual(result["steps"][0]["structured_payload"]["status"], "stale")

    def test_unknown_intent_is_structured_blocker(self):
        result = LifecycleIntentExecutor("/repo").execute(_task("deploy-kernel"))

        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["error_kind"], "lifecycle_intent_executor_not_implemented")
        self.assertEqual(result["intent"], "deploy-kernel")

    def test_execute_blocks_when_task_is_on_non_build_runner_host(self):
        runner = mock.Mock(return_value=subprocess.CompletedProcess([], 0, stdout="", stderr=""))
        task = _task_with_build_target("build-and-reload-mcp")
        executor = LifecycleIntentExecutor(
            "/repo",
            runner=runner,
            build_node={"name": "forge", "aliases": ["ubuntu"]},
            observed_hosts=["alpha", "beta"],
        )

        result = executor.execute(task)

        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["error_kind"], "lifecycle_execution_host_mismatch")
        self.assertEqual(result["intent"], "build-and-reload-mcp")
        self.assertEqual(result["expected_build_target"], "cynic-vm-anvil")
        self.assertEqual(result["registry_build_target"], "cynic-vm-anvil")
        self.assertEqual(result["allowed_execution_hosts"], ["cynic-vm-anvil", "forge", "ubuntu"])
        self.assertEqual(result["observed_execution_hosts"], ["alpha", "beta"])
        runner.assert_not_called()

    def test_execute_executes_intent_when_host_matches_alias(self):
        calls = []
        task = _task_with_build_target("classify-release-matter-for-sovereign-forgejo-proposal")

        def runner(command, **kwargs):
            calls.append(command)
            if command[:2] == ["git", "rev-parse"] and command[-1] == "HEAD":
                return subprocess.CompletedProcess(command, 0, stdout="abc123\n", stderr="")
            if command == ["git", "branch", "--show-current"]:
                return subprocess.CompletedProcess(command, 0, stdout="main\n", stderr="")
            if command[:2] == ["git", "rev-parse"]:
                return subprocess.CompletedProcess(command, 0, stdout="origin/main\n", stderr="")
            if command == ["git", "remote", "-v"]:
                return subprocess.CompletedProcess(
                    command,
                    0,
                    stdout="origin\thttps://10.20.0.10:123/git/cynic/CYNIC.git (fetch)\n",
                    stderr="",
                )
            if command == ["git", "status", "--porcelain"]:
                return subprocess.CompletedProcess(command, 0, stdout="", stderr="")
            return subprocess.CompletedProcess(command, 0, stdout="ok\n", stderr="")

        executor = LifecycleIntentExecutor(
            "/repo",
            runner=runner,
            build_node={"name": "forge", "aliases": ["ubuntu"]},
            observed_hosts=["ubuntu"],
        )
        result = executor.execute(task)

        self.assertEqual(result["status"], "ok")
        self.assertTrue(calls)
        self.assertEqual(calls[0], ["git", "rev-parse", "HEAD"])


if __name__ == "__main__":
    unittest.main()
