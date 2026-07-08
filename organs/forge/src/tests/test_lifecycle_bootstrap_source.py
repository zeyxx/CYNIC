import json
import subprocess
import tempfile
import unittest
import os
os.environ.pop("CYNIC_FORGEJO_BOOTSTRAP_REF", None)
os.environ.pop("CYNIC_FORGEJO_BOOTSTRAP_REMOTE", None)

from pathlib import Path
from unittest import mock

from organs.forge.src.core.lifecycle_bootstrap_source import probe_bootstrap_source
from organs.forge.src.tests.fixtures import registry_fixture


def _git(cwd: Path, *args: str) -> None:
    env = {
        "GIT_AUTHOR_NAME": "Test",
        "GIT_AUTHOR_EMAIL": "test@example.com",
        "GIT_COMMITTER_NAME": "Test",
        "GIT_COMMITTER_EMAIL": "test@example.com",
    }
    subprocess.run(["git", *args], cwd=cwd, env=env, check=True, capture_output=True, text=True)


def _write_remote_repo(root: Path, registry_payload: dict, branch: str) -> str:
    _git(root, "init", "-b", branch)
    (root / "AGENTS.md").write_text("# agents\n", encoding="utf-8")
    (root / "infra").mkdir()
    (root / "infra" / "registry.json").write_text(json.dumps(registry_payload) + "\n", encoding="utf-8")
    _git(root, "add", "AGENTS.md", "infra/registry.json")
    _git(root, "commit", "-m", "seed")
    return root.resolve().as_uri()


class LifecycleBootstrapSourceTests(unittest.TestCase):
    def test_probe_bootstrap_source_is_ok_when_remote_source_is_not_required(self):
        payload = probe_bootstrap_source(registry=registry_fixture(), env={})

        self.assertEqual("ok", payload["status"])
        self.assertEqual("bootstrap_source_not_required", payload["reason"])
        self.assertEqual([], payload["remote_attempts"])

    def test_probe_bootstrap_source_reports_ready_remote_contract(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_materialization"] = "git_fetch"
        with tempfile.TemporaryDirectory() as remote_dir:
            remote_url = _write_remote_repo(Path(remote_dir), registry_fixture(), "main")
            registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_source"] = {
                "remote": remote_url,
                "ref": "main",
                "ref_env": "CYNIC_FORGEJO_BOOTSTRAP_REF",
                "transport": "file",
                "host_role": "forge_primary",
            }

            payload = probe_bootstrap_source(registry=registry, env={})

        self.assertEqual("ok", payload["status"])
        self.assertEqual("bootstrap_source_ready", payload["reason"])
        self.assertEqual("CYNIC_FORGEJO_BOOTSTRAP_REMOTE", payload["env_contract"]["fallback_remote_env"])
        self.assertTrue(payload["checkout"]["registry_contract"]["ok"])
        self.assertTrue(all(payload["checkout"]["sentinels"].values()))

    def test_probe_bootstrap_source_reports_missing_fallback_env(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_materialization"] = "git_fetch"
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_source"] = {
            "remote": "ssh://git@forgejo.synapse.cynic/jeanterre13/CYNIC.git",
            "ref": "main",
            "transport": "ssh",
            "host_role": "forge_primary",
            "fallback_remote_env": "CYNIC_FORGEJO_BOOTSTRAP_REMOTE",
        }

        with mock.patch(
            "organs.forge.src.core.lifecycle_bootstrap_source._suggest_local_forgejo_remote",
            return_value={
                "source": "git remote forgejo",
                "remote": "http://jeanterre13:token@localhost:18300/jeanterre13/CYNIC.git",
                "remote_redacted": "http://localhost:18300/jeanterre13/CYNIC.git",
                "command": "git remote get-url forgejo",
            },
        ):
            payload = probe_bootstrap_source(registry=registry, env={})

        self.assertEqual("blocked", payload["status"])
        self.assertEqual("bootstrap_source_fallback_env_missing", payload["reason"])
        self.assertFalse(payload["env_contract"]["fallback_remote_present"])
        self.assertEqual("CYNIC_FORGEJO_BOOTSTRAP_REMOTE", payload["env_contract"]["fallback_remote_env"])
        self.assertEqual(["~/.cynic-env", "~/.config/cynic/env"], payload["runtime_env_files"])
        self.assertEqual("git remote get-url forgejo", payload["fallback_remote_suggestion"]["command"])
        self.assertEqual(
            "http://localhost:18300/jeanterre13/CYNIC.git",
            payload["fallback_remote_suggestion"]["remote_redacted"],
        )
        self.assertIn("git remote get-url forgejo", payload["operator_action"])

    def test_probe_bootstrap_source_reports_stale_remote_contract(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_materialization"] = "git_fetch"
        stale_remote_registry = registry_fixture()
        stale_remote_registry["lifecycle"]["kernel_release"].pop("build_policy", None)
        with tempfile.TemporaryDirectory() as remote_dir:
            remote_url = _write_remote_repo(Path(remote_dir), stale_remote_registry, "main")
            registry["lifecycle"]["kernel_release"]["build_policy"]["bootstrap"]["repo_source"] = {
                "remote": remote_url,
                "ref": "main",
                "transport": "file",
                "host_role": "forge_primary",
            }

            payload = probe_bootstrap_source(registry=registry, env={})

        self.assertEqual("blocked", payload["status"])
        self.assertEqual("bootstrap_source_contract_stale", payload["reason"])
        self.assertIn(
            "lifecycle.kernel_release.build_policy",
            payload["checkout"]["registry_contract"]["missing"],
        )


if __name__ == "__main__":
    unittest.main()
