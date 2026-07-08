import json
import os
import tempfile
import unittest
from pathlib import Path
from io import StringIO
from unittest import mock

from organs.forge.src.entrypoints import lifecycle_cli as cli
from organs.forge.src.adapters.kernel_adapter import (
    activate_lifecycle_registry_node,
    apply_lifecycle_activation_evidence_patch,
    build_lifecycle_service_evidence_patch,
    persist_lifecycle_activation_evidence_patch,
    persist_lifecycle_registry_node_activation,
    verify_lifecycle_runtime_paths,
)
from organs.forge.src.tests.fixtures import (
    registry_fixture,
    registry_with_ready_anvil,
)


def materialize_git_worktree(repo_root: Path) -> None:
    git_dir = repo_root / ".git"
    (git_dir / "objects").mkdir(parents=True)
    (git_dir / "refs").mkdir()
    (git_dir / "HEAD").write_text("ref: refs/heads/main\n")
    (git_dir / "config").write_text("[core]\n\trepositoryformatversion = 0\n")


class LifecycleActivationEvidenceTests(unittest.TestCase):
    def test_apply_lifecycle_activation_evidence_patch_merges_runtime_proof(self):
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/CYNIC"
            materialize_git_worktree(repo_root)
            (repo_root / "AGENTS.md").write_text("# agents\n")
            (repo_root / "infra").mkdir()
            (repo_root / "infra/registry.json").write_text("{}\n")

            verification = verify_lifecycle_runtime_paths(
                "cynic-vm-anvil",
                registry_fixture(),
                home=home,
            )

        result = apply_lifecycle_activation_evidence_patch(
            verification["activation_evidence_patch"],
            registry_fixture(),
        )

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["node"], "cynic-vm-anvil")
        self.assertEqual(result["accepted_runtime_paths"], ["repo_root"])
        build_node = next(
            node
            for node in result["registry"]["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertTrue(
            build_node["activation_evidence"]["runtime_paths"]["repo_root"]["verified"]
        )

    def test_apply_lifecycle_activation_evidence_patch_merges_service_proof(self):
        patch = {
            "node": "cynic-vm-anvil",
            "activation_evidence": {
                "services": {
                    "organ-forge": {
                        "declared": True,
                        "active": True,
                        "checked_at": "2026-06-15T00:00:00Z",
                    }
                }
            },
        }

        result = apply_lifecycle_activation_evidence_patch(patch, registry_fixture())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["accepted_runtime_paths"], [])
        self.assertEqual(result["accepted_services"], ["organ-forge"])
        build_node = next(
            node
            for node in result["registry"]["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertTrue(
            build_node["activation_evidence"]["services"]["organ-forge"]["active"]
        )

    def test_persist_lifecycle_activation_evidence_patch_writes_validated_merge(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            registry_path = Path(tmpdir) / "registry.json"
            registry_path.write_text(json.dumps(registry_fixture()) + "\n")
            patch = {
                "node": "cynic-vm-anvil",
                "activation_evidence": {
                    "services": {
                        "organ-forge": {
                            "declared": True,
                            "active": True,
                            "checked_at": "2026-06-15T00:00:00Z",
                        }
                    }
                },
            }

            result = persist_lifecycle_activation_evidence_patch(
                patch,
                registry_path=registry_path,
            )
            persisted = json.loads(registry_path.read_text())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["accepted_services"], ["organ-forge"])
        build_node = next(
            node
            for node in persisted["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertTrue(
            build_node["activation_evidence"]["services"]["organ-forge"]["active"]
        )

    def test_activate_lifecycle_registry_node_rejects_incomplete_evidence(self):
        with self.assertRaisesRegex(RuntimeError, "activation evidence is incomplete"):
            activate_lifecycle_registry_node("cynic-vm-anvil", registry_fixture())

    def test_activate_lifecycle_registry_node_requires_lifecycle_node(self):
        registry = registry_with_ready_anvil()
        registry["nodes"].append(
            {
                "name": "cynic-vm-gpu",
                "role": "dog_runner",
                "status": "planned",
            }
        )

        with self.assertRaisesRegex(RuntimeError, "not part of lifecycle"):
            activate_lifecycle_registry_node("cynic-vm-gpu", registry)

    def test_activate_lifecycle_registry_node_marks_ready_node_active(self):
        result = activate_lifecycle_registry_node(
            "cynic-vm-anvil",
            registry_with_ready_anvil(),
        )

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["from_status"], "planned")
        self.assertEqual(result["to_status"], "active")
        build_node = next(
            node
            for node in result["registry"]["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertEqual(build_node["status"], "active")
        self.assertEqual(build_node["status_source"], "lifecycle_node_activation")
        self.assertIn("status_updated_at", build_node)

    def test_persist_lifecycle_registry_node_activation_writes_status(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            registry_path = Path(tmpdir) / "registry.json"
            registry_path.write_text(json.dumps(registry_with_ready_anvil()) + "\n")

            result = persist_lifecycle_registry_node_activation(
                "cynic-vm-anvil",
                registry_path=registry_path,
            )
            persisted = json.loads(registry_path.read_text())

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["to_status"], "active")
        build_node = next(
            node
            for node in persisted["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertEqual(build_node["status"], "active")

    def test_lifecycle_service_evidence_patch_is_registry_derived(self):
        payload = build_lifecycle_service_evidence_patch(
            "cynic-vm-anvil",
            registry_fixture(),
        )

        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["node"], "cynic-vm-anvil")
        self.assertEqual(
            [check["service"] for check in payload["checks"]],
            ["organ-forge", "cargo-build", "gate-2"],
        )
        self.assertTrue(all(check["declared"] for check in payload["checks"]))
        self.assertFalse(any(check["active"] for check in payload["checks"]))
        services = payload["activation_evidence_patch"]["activation_evidence"]["services"]
        self.assertEqual(sorted(services), ["cargo-build", "gate-2", "organ-forge"])
        self.assertFalse(services["organ-forge"]["active"])

    def test_lifecycle_service_evidence_patch_accepts_external_service_states(self):
        payload = build_lifecycle_service_evidence_patch(
            "cynic-vm-anvil",
            registry_fixture(),
            service_states={
                "organ-forge": {
                    "active": True,
                    "checked_at": "2026-06-15T00:00:00Z",
                    "source": "unit-test",
                },
                "cargo-build": {
                    "active": True,
                    "checked_at": "2026-06-15T00:00:00Z",
                    "source": "unit-test",
                },
                "gate-2": {
                    "active": True,
                    "checked_at": "2026-06-15T00:00:00Z",
                    "source": "unit-test",
                },
            },
        )

        self.assertEqual(payload["status"], "ok")
        services = payload["activation_evidence_patch"]["activation_evidence"]["services"]
        self.assertTrue(all(item["active"] for item in services.values()))

    def test_lifecycle_service_evidence_patch_accepts_service_states_json_env(self):
        with mock.patch.dict(
            os.environ,
            {
                "CYNIC_LIFECYCLE_SERVICE_STATES_JSON": json.dumps(
                    {
                        "organ-forge": {
                            "active": True,
                            "checked_at": "2026-06-15T00:00:00Z",
                            "source": "unit-test-env",
                        },
                        "cargo-build": {
                            "active": True,
                            "checked_at": "2026-06-15T00:00:00Z",
                            "source": "unit-test-env",
                        },
                        "gate-2": {
                            "active": True,
                            "checked_at": "2026-06-15T00:00:00Z",
                            "source": "unit-test-env",
                        },
                    }
                )
            },
        ):
            payload = build_lifecycle_service_evidence_patch(
                "cynic-vm-anvil",
                registry_fixture(),
            )
        self.assertEqual(payload["status"], "ok")
        services = payload["activation_evidence_patch"]["activation_evidence"]["services"]
        self.assertEqual(len(services), 3)
        self.assertTrue(all(item["active"] for item in services.values()))

    def test_lifecycle_service_evidence_patch_rejects_invalid_service_states_env(self):
        with mock.patch.dict(os.environ, {"CYNIC_SERVICE_STATES": "bad-json"}, clear=False):
            with self.assertRaisesRegex(RuntimeError, "must be a JSON object"):
                build_lifecycle_service_evidence_patch(
                    "cynic-vm-anvil",
                    registry_fixture(),
                )

    def test_lifecycle_service_evidence_patch_cli_reads_service_states_arg(self):
        service_states = {
            "organ-forge": {
                "active": True,
                "checked_at": "2026-06-15T00:00:00Z",
                "source": "unit-test-cli",
            },
            "cargo-build": {
                "active": True,
                "checked_at": "2026-06-15T00:00:00Z",
                "source": "unit-test-cli",
            },
            "gate-2": {
                "active": True,
                "checked_at": "2026-06-15T00:00:00Z",
                "source": "unit-test-cli",
            },
        }
        with mock.patch(
            "organs.forge.src.entrypoints.lifecycle_diagnostic_cli.build_lifecycle_service_evidence_patch",
            return_value={
                "status": "ok",
                "node": "cynic-vm-anvil",
                "checks": [
                    {"service": "organ-forge", "active": True},
                    {"service": "cargo-build", "active": True},
                    {"service": "gate-2", "active": True},
                ],
                "activation_evidence_patch": {
                    "node": "cynic-vm-anvil",
                    "activation_evidence": {"services": {}},
                },
            },
        ) as service_patch, mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = cli.run_lifecycle_service_evidence_patch(
                "cynic-vm-anvil",
                service_states=service_states,
            )

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["node"], "cynic-vm-anvil")
        service_patch.assert_called_once_with(
            "cynic-vm-anvil",
            service_states=service_states,
        )

    def test_lifecycle_service_evidence_patch_cli_returns_blocked_status(self):
        with mock.patch(
            "organs.forge.src.entrypoints.lifecycle_diagnostic_cli.build_lifecycle_service_evidence_patch",
            return_value={
                "status": "blocked",
                "node": "cynic-vm-anvil",
                "checks": [{"service": "organ-forge", "active": False}],
                "activation_evidence_patch": {
                    "node": "cynic-vm-anvil",
                    "activation_evidence": {"services": {}},
                },
            },
        ) as service_patch, mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = cli.run_lifecycle_service_evidence_patch("cynic-vm-anvil")

        self.assertEqual(status, 1)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["node"], "cynic-vm-anvil")
        service_patch.assert_called_once_with(
            "cynic-vm-anvil",
            service_states=None,
        )

    def test_apply_lifecycle_activation_evidence_patch_rejects_unknown_service(self):
        patch = {
            "node": "cynic-vm-anvil",
            "activation_evidence": {
                "services": {
                    "cynic-kernel": {
                        "declared": True,
                        "active": True,
                        "checked_at": "2026-06-15T00:00:00Z",
                    }
                }
            },
        }

        with self.assertRaisesRegex(RuntimeError, "not required by node role"):
            apply_lifecycle_activation_evidence_patch(patch, registry_fixture())

    def test_apply_lifecycle_activation_evidence_patch_accepts_activation_evidence_patch_wrapper(self):
        fixture = registry_fixture()
        wrapped = build_lifecycle_service_evidence_patch(
            "cynic-vm-anvil",
            fixture,
            service_states={
                "organ-forge": {"active": True, "checked_at": "2026-06-15T00:00:00Z"},
                "cargo-build": {"active": True, "checked_at": "2026-06-15T00:00:00Z"},
                "gate-2": {"active": True, "checked_at": "2026-06-15T00:00:00Z"},
            },
        )["activation_evidence_patch"]
        result = apply_lifecycle_activation_evidence_patch(
            {"activation_evidence_patch": wrapped},
            fixture,
        )

        self.assertEqual(result["status"], "ok")
        self.assertEqual(result["node"], "cynic-vm-anvil")
        self.assertEqual(result["accepted_services"], ["cargo-build", "gate-2", "organ-forge"])

    def test_apply_lifecycle_activation_evidence_patch_rejects_runtime_path_drift(self):
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/CYNIC"
            materialize_git_worktree(repo_root)
            (repo_root / "AGENTS.md").write_text("# agents\n")
            (repo_root / "infra").mkdir()
            (repo_root / "infra/registry.json").write_text("{}\n")

            verification = verify_lifecycle_runtime_paths(
                "cynic-vm-anvil",
                registry_fixture(),
                home=home,
            )
        patch = verification["activation_evidence_patch"]
        patch["activation_evidence"]["runtime_paths"]["repo_root"]["value"] = "/tmp/CYNIC"

        with self.assertRaisesRegex(RuntimeError, "value does not match registry"):
            apply_lifecycle_activation_evidence_patch(patch, registry_fixture())

    def test_apply_lifecycle_activation_evidence_patch_rejects_incoherent_verified_flag(self):
        verification = verify_lifecycle_runtime_paths(
            "cynic-vm-core",
            registry_fixture(),
            home="/tmp/nonexistent-cynic-home",
        )
        patch = verification["activation_evidence_patch"]
        patch["activation_evidence"]["runtime_paths"]["repo_root"]["verified"] = True

        with self.assertRaisesRegex(RuntimeError, "cannot be verified without marker"):
            apply_lifecycle_activation_evidence_patch(patch, registry_fixture())

    def test_lifecycle_validate_activation_evidence_patch_cli_accepts_valid_patch(self):
        with tempfile.TemporaryDirectory() as home:
            repo_root = Path(home) / ".cynic/repos/CYNIC"
            materialize_git_worktree(repo_root)
            (repo_root / "AGENTS.md").write_text("# agents\n")
            (repo_root / "infra").mkdir()
            (repo_root / "infra/registry.json").write_text("{}\n")

            verification = verify_lifecycle_runtime_paths(
                "cynic-vm-anvil",
                registry_fixture(),
                home=home,
            )

        raw_patch = json.dumps(verification["activation_evidence_patch"])
        with mock.patch("sys.stdout", new_callable=StringIO) as stdout:
            status = cli.run_lifecycle_validate_activation_evidence_patch(raw_patch)

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["accepted_runtime_paths"], ["repo_root"])
        self.assertEqual(payload["accepted_services"], [])

    def test_lifecycle_apply_activation_evidence_patch_cli_persists_patch(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            registry_path = Path(tmpdir) / "registry.json"
            registry_path.write_text(json.dumps(registry_fixture()) + "\n")
            patch = {
                "node": "cynic-vm-anvil",
                "activation_evidence": {
                    "services": {
                        "organ-forge": {
                            "declared": True,
                            "active": True,
                            "checked_at": "2026-06-15T00:00:00Z",
                        }
                    }
                },
            }

            with mock.patch("sys.stdout", new_callable=StringIO) as stdout:
                status = cli.run_lifecycle_apply_activation_evidence_patch(
                    json.dumps(patch),
                    registry_path=str(registry_path),
                )
            persisted = json.loads(registry_path.read_text())

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["accepted_services"], ["organ-forge"])
        build_node = next(
            node
            for node in persisted["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertTrue(
            build_node["activation_evidence"]["services"]["organ-forge"]["active"]
        )

    def test_lifecycle_activate_node_cli_persists_status(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            registry_path = Path(tmpdir) / "registry.json"
            registry_path.write_text(json.dumps(registry_with_ready_anvil()) + "\n")

            with mock.patch("sys.stdout", new_callable=StringIO) as stdout:
                status = cli.run_lifecycle_activate_node(
                    "cynic-vm-anvil",
                    registry_path=str(registry_path),
                )
            persisted = json.loads(registry_path.read_text())

        self.assertEqual(status, 0)
        payload = json.loads(stdout.getvalue())
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["node"], "cynic-vm-anvil")
        build_node = next(
            node
            for node in persisted["nodes"]
            if node["name"] == "cynic-vm-anvil"
        )
        self.assertEqual(build_node["status"], "active")
