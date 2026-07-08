import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "controllers" / "validate_repo_topology.py"


def load_module():
    return load_module_from_path("validate_repo_topology", MODULE_PATH)


def git_closure_plan_base(**overrides):
    plan = {
        "schema": "cynic.git_membrane_closure_plan.v1",
        "status": "draft",
        "allowed_dispositions": ["track"],
        "execution_requirements": {
            "requires_git_metadata_write": True,
            "preflight_command": "make git-membrane-closure-preflight",
            "claim_required": True,
            "executor_authority": "test cortex",
            "forbidden_when": ["test blocker"],
        },
        "post_closure_proof": {
            "required_commands": ["make git-membrane-closure-check w=<wave>"],
            "required_evidence": ["selected wave is closed"],
        },
    }
    plan.update(overrides)
    return plan


class GitMembraneClosureSummaryTests(unittest.TestCase):
    def test_git_matter_report_groups_visible_paths_for_inference(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True, text=True)
            (root / "organs" / "forge" / "src").mkdir(parents=True)
            (root / "crates" / "cynic-kernel" / "src").mkdir(parents=True)
            (root / "organs" / "forge" / "src" / "agent.py").write_text("print('forge')\n", encoding="utf-8")
            (root / "crates" / "cynic-kernel" / "src" / "api.rs").write_text("pub fn api() {}\n", encoding="utf-8")
            subprocess.run(["git", "add", "crates/cynic-kernel/src/api.rs"], cwd=root, check=True, capture_output=True, text=True)

            old_values = (
                module.WORKSPACE_ROOT,
                module.git_metadata_writable,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.git_metadata_writable = lambda: True
                report = module.git_matter_report()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.git_metadata_writable,
                ) = old_values

        self.assertEqual("cynic.git_matter_report.v1", report["schema"])
        self.assertEqual(2, report["visible_path_total"])
        self.assertEqual(1, report["untracked_total"])
        self.assertEqual(1, report["staged_total"])
        groups = {
            (item["family"], item["topological_class"], item["status"]): item
            for item in report["groups"]
        }
        self.assertIn(("organs/forge", "organ_body", "untracked"), groups)
        self.assertIn(("crates/cynic-kernel", "kernel_source", "staged"), groups)
        self.assertEqual(len(report["groups"]), len(report["inference_batches"]))

    def test_git_matter_report_surfaces_git_command_failures(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            old_values = (
                module.WORKSPACE_ROOT,
                module.git_metadata_writable,
            )

            def fail_git(*args, **kwargs):
                raise subprocess.CalledProcessError(128, args[0], stderr="fatal: broken object")

            try:
                module.WORKSPACE_ROOT = root
                module.git_metadata_writable = lambda: True
                with patch("organs.supervisor.src.core.git_membrane_reporting.subprocess.run", fail_git):
                    report = module.git_matter_report()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.git_metadata_writable,
                ) = old_values

        self.assertFalse(report["ok"])
        self.assertEqual(0, report["visible_path_total"])
        self.assertTrue(any("fatal: broken object" in blocker for blocker in report["blockers"]))

    def test_git_membrane_closure_summary_counts_by_wave(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "ghost").mkdir(parents=True)
            (root / "docs").mkdir()
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.git_membrane_closure_plan.v1",
                        "status": "draft",
                        "allowed_dispositions": ["track"],
                        "waves": [
                            {
                                "wave": 1,
                                "theme": "ghost",
                                "disposition": "track",
                                "targets": [{"path": "organs/ghost/", "classification": "organ_body", "reason": "test"}],
                            },
                            {
                                "wave": 2,
                                "theme": "docs",
                                "disposition": "track",
                                "targets": [{"path": "docs/", "classification": "document_projection", "reason": "test"}],
                            },
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH = plan_path
                module.git_ls_untracked = lambda *args: [
                    "organs/ghost/local.py",
                    "docs/projection.md",
                    "unplanned.py",
                ]
                summary = module.git_membrane_closure_summary()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual(3, summary["untracked_total"])
        self.assertEqual(2, summary["untracked_planned"])
        self.assertEqual(["unplanned.py"], summary["untracked_unplanned"])
        self.assertEqual(1, summary["waves"][0]["untracked_count"])
        self.assertEqual(1, summary["waves"][1]["untracked_count"])

    def test_git_membrane_closure_commands_group_pathspecs_by_wave(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "docs" / "space dir").mkdir(parents=True)
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.git_membrane_closure_plan.v1",
                        "status": "draft",
                        "allowed_dispositions": ["track"],
                        "waves": [
                            {
                                "wave": 1,
                                "theme": "docs",
                                "disposition": "track",
                                "targets": [
                                    {"path": "docs/space dir/", "reason": "test"},
                                    {"path": "docs/empty/", "reason": "test"},
                                ],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH = plan_path
                module.git_ls_untracked = lambda *args: ["docs/space dir/file.md"]
                commands = module.git_membrane_closure_commands()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual(["docs/space dir/"], commands["waves"][0]["pathspecs"])
        self.assertEqual("git add -- 'docs/space dir/'", commands["waves"][0]["git_add_command"])
        self.assertEqual(
            "git ls-files --others --exclude-standard -- 'docs/space dir/'",
            commands["waves"][0]["post_add_untracked_check"],
        )
        self.assertEqual("make lint-repo-topology-contract", commands["waves"][0]["post_add_validation"])

    def test_git_membrane_closure_commands_can_filter_one_wave(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "one").mkdir(parents=True)
            (root / "organs" / "two").mkdir(parents=True)
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.git_membrane_closure_plan.v1",
                        "status": "draft",
                        "allowed_dispositions": ["track"],
                        "waves": [
                            {
                                "wave": 1,
                                "theme": "one",
                                "disposition": "track",
                                "targets": [{"path": "organs/one/", "classification": "organ_body", "reason": "test"}],
                            },
                            {
                                "wave": 2,
                                "theme": "two",
                                "disposition": "track",
                                "targets": [{"path": "organs/two/", "classification": "organ_body", "reason": "test"}],
                            },
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH = plan_path
                module.git_ls_untracked = lambda *args: [
                    "organs/one/local.py",
                    "organs/two/local.py",
                ]
                commands = module.git_membrane_closure_commands(selected_wave=2)
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual(2, commands["untracked_planned"])
        self.assertEqual([], commands["untracked_unplanned"])
        self.assertEqual([2], [wave["wave"] for wave in commands["waves"]])
        self.assertEqual(["organs/two/"], commands["waves"][0]["pathspecs"])

    def test_git_membrane_closure_check_fails_for_open_wave(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "open").mkdir(parents=True)
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.git_membrane_closure_plan.v1",
                        "status": "draft",
                        "allowed_dispositions": ["track"],
                        "waves": [
                            {
                                "wave": 1,
                                "theme": "open",
                                "disposition": "track",
                                "targets": [{"path": "organs/open/", "classification": "organ_body", "reason": "test"}],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH = plan_path
                module.git_ls_untracked = lambda *args: ["organs/open/local.py"]
                check = module.git_membrane_closure_check(selected_wave=1)
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertFalse(check["ok"])
        self.assertEqual([{"wave": 1, "theme": "open", "untracked_count": 1}], check["open_waves"])

    def test_git_membrane_closure_check_passes_for_closed_wave(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "closed").mkdir(parents=True)
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.git_membrane_closure_plan.v1",
                        "status": "draft",
                        "allowed_dispositions": ["track"],
                        "waves": [
                            {
                                "wave": 1,
                                "theme": "closed",
                                "disposition": "track",
                                "targets": [{"path": "organs/closed/", "classification": "organ_body", "reason": "test"}],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH = plan_path
                module.git_ls_untracked = lambda *args: []
                check = module.git_membrane_closure_check(selected_wave=1)
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertTrue(check["ok"])
        self.assertEqual([], check["open_waves"])

    def test_git_membrane_closure_check_fails_for_unknown_wave(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "one").mkdir(parents=True)
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.git_membrane_closure_plan.v1",
                        "status": "draft",
                        "allowed_dispositions": ["track"],
                        "waves": [
                            {
                                "wave": 1,
                                "theme": "one",
                                "disposition": "track",
                                "targets": [{"path": "organs/one/", "classification": "organ_body", "reason": "test"}],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH = plan_path
                module.git_ls_untracked = lambda *args: []
                check = module.git_membrane_closure_check(selected_wave=99)
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertFalse(check["ok"])
        self.assertIn("closure wave 99 is not declared", check["issues"])

    def test_git_membrane_closure_summary_reports_next_open_wave(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "one").mkdir(parents=True)
            (root / "organs" / "two").mkdir(parents=True)
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.git_membrane_closure_plan.v1",
                        "status": "draft",
                        "allowed_dispositions": ["track"],
                        "waves": [
                            {
                                "wave": 1,
                                "theme": "one",
                                "disposition": "track",
                                "targets": [{"path": "organs/one/", "classification": "organ_body", "reason": "test"}],
                            },
                            {
                                "wave": 2,
                                "theme": "two",
                                "disposition": "track",
                                "targets": [{"path": "organs/two/", "classification": "organ_body", "reason": "test"}],
                            },
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH = plan_path
                module.git_ls_untracked = lambda *args: ["organs/two/local.py"]
                summary = module.git_membrane_closure_summary()
                commands = module.git_membrane_closure_commands(
                    selected_wave=summary["next_open_wave"]["wave"]
                )
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual({"wave": 2, "theme": "two", "untracked_count": 1}, summary["next_open_wave"])
        self.assertEqual([2], [wave["wave"] for wave in commands["waves"]])

    def test_git_membrane_closure_summary_has_no_next_wave_when_closed(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "one").mkdir(parents=True)
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.git_membrane_closure_plan.v1",
                        "status": "draft",
                        "allowed_dispositions": ["track"],
                        "waves": [
                            {
                                "wave": 1,
                                "theme": "one",
                                "disposition": "track",
                                "targets": [{"path": "organs/one/", "classification": "organ_body", "reason": "test"}],
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH = plan_path
                module.git_ls_untracked = lambda *args: []
                summary = module.git_membrane_closure_summary()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIsNone(summary["next_open_wave"])

