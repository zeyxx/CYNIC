import json
import os
import tempfile
import unittest
from pathlib import Path
from io import StringIO
from unittest import mock

from organs.forge.src.entrypoints import lifecycle_cli as cli
from organs.forge.src.adapters.kernel_adapter import (
    build_lifecycle_runtime_paths,
    verify_lifecycle_runtime_paths,
)
from organs.forge.src.tests.fixtures import registry_fixture


def materialize_git_worktree(repo_root: Path) -> None:
    git_dir = repo_root / ".git"
    (git_dir / "objects").mkdir(parents=True)
    (git_dir / "refs").mkdir()
    (git_dir / "HEAD").write_text("ref: refs/heads/main\n")
    (git_dir / "config").write_text("[core]\n\trepositoryformatversion = 0\n")


class LifecycleRuntimePathTests(unittest.TestCase):
    def test_lifecycle_runtime_paths_are_registry_derived(self):
        payload = build_lifecycle_runtime_paths(registry_fixture())

        self.assertEqual(payload["status"], "blocked")
        self.assertTrue(payload["blocked"])
        self.assertEqual(
            payload["paths"],
            [
                {
                    "node": "cynic-vm-anvil",
                    "role": "build_runner",
                    "path": "repo_root",
                    "value": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}",
                    "proof": {
                        "kind": "git_worktree",
                        "marker": ".git",
                        "sentinels": ["AGENTS.md", "infra/registry.json"],
                    },
                    "required_status": "active",
                    "current_status": "planned",
                    "materialized_by": "lifecycle_activation",
                    "source": "infra/registry.json:lifecycle.kernel_release.runtime_paths",
                },
                {
                    "node": "cynic-vm-core",
                    "role": "kernel_primary",
                    "path": "repo_root",
                    "value": "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}",
                    "proof": {
                        "kind": "git_worktree",
                        "marker": ".git",
                        "sentinels": ["AGENTS.md", "infra/registry.json"],
                    },
                    "required_status": "active",
                    "current_status": "planned",
                    "materialized_by": "lifecycle_activation",
                    "source": "infra/registry.json:lifecycle.kernel_release.runtime_paths",
                },
            ],
        )

    def test_lifecycle_runtime_paths_cli_returns_blocked_status(self):
        with mock.patch(
            "organs.forge.src.entrypoints.lifecycle_diagnostic_cli.build_lifecycle_runtime_paths",
            return_value={
                "status": "blocked",
                "lifecycle": "kernel_release",
                "paths": [{"path": "repo_root"}],
                "blocked": True,
                "blockers": [],
            },
        ) as runtime_paths, mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = cli.run_lifecycle_runtime_paths()

        self.assertEqual(status, 1)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["paths"][0]["path"], "repo_root")
        runtime_paths.assert_called_once_with()

    def test_verify_lifecycle_runtime_paths_accepts_materialized_git_worktree(self):
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/CYNIC"
            materialize_git_worktree(repo_root)
            (repo_root / "AGENTS.md").write_text("# agents\n")
            (repo_root / "infra").mkdir()
            (repo_root / "infra/registry.json").write_text("{}\n")

            payload = verify_lifecycle_runtime_paths(
                "cynic-vm-anvil",
                registry_fixture(),
                home=home,
            )

        self.assertEqual(payload["status"], "ok")
        self.assertTrue(payload["checks"][0]["ok"])
        self.assertTrue(payload["checks"][0]["marker_exists"])
        self.assertTrue(all(payload["checks"][0]["git_probe"].values()))
        evidence = payload["activation_evidence_patch"]["activation_evidence"]
        self.assertEqual(
            payload["activation_evidence_patch"]["node"],
            "cynic-vm-anvil",
        )
        self.assertTrue(evidence["runtime_paths"]["repo_root"]["verified"])
        self.assertTrue(all(evidence["runtime_paths"]["repo_root"]["git_probe"].values()))
        self.assertEqual(
            evidence["runtime_paths"]["repo_root"]["value"],
            "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}",
        )
        self.assertTrue(evidence["runtime_paths"]["repo_root"]["sentinels_verified"])

    def test_verify_lifecycle_runtime_paths_expands_project_slug(self):
        registry = registry_fixture()
        registry["lifecycle"]["kernel_release"]["runtime_paths"]["repo_root"] = (
            "%h/.cynic/repos/${CYNIC_PROJECT_SLUG}"
        )
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/KYON"
            materialize_git_worktree(repo_root)
            (repo_root / "AGENTS.md").write_text("# agents\n")
            (repo_root / "infra").mkdir()
            (repo_root / "infra/registry.json").write_text("{}\n")

            with mock.patch.dict(os.environ, {"CYNIC_PROJECT_SLUG": "KYON"}):
                payload = verify_lifecycle_runtime_paths(
                    "cynic-vm-anvil",
                    registry=registry,
                    home=home,
                )

        evidence = payload["activation_evidence_patch"]["activation_evidence"]
        self.assertTrue(evidence["runtime_paths"]["repo_root"]["verified"])
        self.assertEqual(
            evidence["runtime_paths"]["repo_root"]["expanded"],
            str(repo_root),
        )

    def test_verify_lifecycle_runtime_paths_blocks_missing_sentinel(self):
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/CYNIC"
            materialize_git_worktree(repo_root)
            (repo_root / "AGENTS.md").write_text("# agents\n")

            payload = verify_lifecycle_runtime_paths(
                "cynic-vm-core",
                registry_fixture(),
                home=home,
            )

        self.assertEqual(payload["status"], "blocked")
        self.assertFalse(payload["checks"][0]["ok"])
        missing = [
            item for item in payload["checks"][0]["sentinels"] if not item["exists"]
        ]
        self.assertEqual(missing[0]["path"], "infra/registry.json")
        evidence = payload["activation_evidence_patch"]["activation_evidence"]
        self.assertFalse(evidence["runtime_paths"]["repo_root"]["verified"])
        self.assertFalse(evidence["runtime_paths"]["repo_root"]["sentinels_verified"])

    def test_verify_lifecycle_runtime_paths_blocks_incomplete_git_materialization(self):
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/CYNIC"
            (repo_root / ".git").mkdir(parents=True)
            (repo_root / "AGENTS.md").write_text("# agents\n")
            (repo_root / "infra").mkdir()
            (repo_root / "infra/registry.json").write_text("{}\n")

            payload = verify_lifecycle_runtime_paths(
                "cynic-vm-core",
                registry_fixture(),
                home=home,
            )

        self.assertEqual(payload["status"], "blocked")
        self.assertFalse(payload["checks"][0]["ok"])
        self.assertFalse(payload["checks"][0]["git_probe"]["git_head"])
        self.assertFalse(payload["checks"][0]["git_probe"]["git_config"])
        self.assertFalse(payload["checks"][0]["git_probe"]["git_objects"])
