import json
import tempfile
import unittest
from pathlib import Path

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


class GitMembraneClosurePlanTests(unittest.TestCase):
    def test_git_membrane_closure_preflight_blocks_when_git_not_writable(self):
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
                module.git_metadata_writable,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH = plan_path
                module.git_ls_untracked = lambda *args: ["organs/one/local.py"]
                module.git_metadata_writable = lambda: False
                preflight = module.git_membrane_closure_preflight()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                    module.git_metadata_writable,
                ) = old_values

        self.assertFalse(preflight["ok"])
        self.assertFalse(preflight["git_writable"])
        self.assertEqual(1, preflight["untracked_planned"])
        self.assertEqual(0, preflight["untracked_unplanned_count"])
        self.assertIn(".git is not writable in this environment", preflight["blockers"])



    def test_rejects_git_membrane_closure_unknown_disposition(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "ghost").mkdir(parents=True)
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    git_closure_plan_base(
                        waves=[
                            {
                                "wave": 1,
                                "theme": "ghost",
                                "disposition": "improvise",
                                "targets": [{"path": "organs/ghost/", "classification": "organ_body", "reason": "test"}],
                            }
                        ],
                    )
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
                module.git_ls_untracked = lambda *args: ["organs/ghost/local.py"]
                findings = module.validate_git_membrane_closure_plan_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn("git_membrane_closure_disposition_unknown", [item.kind for item in findings])

    def test_rejects_git_membrane_closure_wave_order_invalid(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            for name in ("one", "two"):
                (root / "organs" / name).mkdir(parents=True)
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    git_closure_plan_base(
                        waves=[
                            {
                                "wave": 2,
                                "theme": "two",
                                "disposition": "track",
                                "targets": [{"path": "organs/two/", "classification": "organ_body", "reason": "test"}],
                            },
                            {
                                "wave": 1,
                                "theme": "one",
                                "disposition": "track",
                                "targets": [{"path": "organs/one/", "classification": "organ_body", "reason": "test"}],
                            },
                        ],
                    )
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
                module.git_ls_untracked = lambda *args: ["organs/one/local.py", "organs/two/local.py"]
                findings = module.validate_git_membrane_closure_plan_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn("git_membrane_closure_wave_order_invalid", [item.kind for item in findings])

    def test_rejects_git_membrane_closure_missing_target(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    git_closure_plan_base(
                        waves=[
                            {
                                "wave": 1,
                                "theme": "missing",
                                "disposition": "track",
                                "targets": [{"path": "organs/missing/", "classification": "organ_body", "reason": "test"}],
                            }
                        ],
                    )
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
                findings = module.validate_git_membrane_closure_plan_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn("git_membrane_closure_target_missing", [item.kind for item in findings])

    def test_allows_git_membrane_closure_plan_targeting_open_directory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "ghost").mkdir(parents=True)
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    git_closure_plan_base(
                        waves=[
                            {
                                "wave": 1,
                                "theme": "ghost",
                                "disposition": "track",
                                "targets": [{"path": "organs/ghost/", "classification": "organ_body", "reason": "test"}],
                            }
                        ],
                    )
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
                module.git_ls_untracked = lambda *args: ["organs/ghost/local.py"]
                findings = module.validate_git_membrane_closure_plan_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_untracked_file_missing_from_git_membrane_closure_plan(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "planned").mkdir(parents=True)
            (root / "organs" / "unplanned").mkdir(parents=True)
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    git_closure_plan_base(
                        waves=[
                            {
                                "wave": 1,
                                "theme": "planned",
                                "disposition": "track",
                                "targets": [{"path": "organs/planned/", "classification": "organ_body", "reason": "test"}],
                            }
                        ],
                    )
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
                    "organs/planned/local.py",
                    "organs/unplanned/local.py",
                ]
                findings = module.validate_git_membrane_closure_plan_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn(
            "git_membrane_closure_untracked_not_planned",
            [item.kind for item in findings],
        )

    def test_rejects_git_membrane_closure_plan_missing_execution_requirements(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.git_membrane_closure_plan.v1",
                        "status": "draft",
                        "allowed_dispositions": ["track"],
                        "post_closure_proof": {
                            "required_commands": ["make git-membrane-closure-check w=<wave>"],
                            "required_evidence": ["selected wave is closed"],
                        },
                        "waves": [
                            {
                                "wave": 1,
                                "theme": "ghost",
                                "disposition": "track",
                                "targets": [{"path": "organs/ghost/", "classification": "organ_body", "reason": "test"}],
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
                module.git_ls_untracked = lambda *args: ["organs/ghost/local.py"]
                findings = module.validate_git_membrane_closure_plan_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn(
            "git_membrane_closure_execution_requirements_missing",
            [item.kind for item in findings],
        )

    def test_rejects_git_membrane_closure_plan_missing_post_closure_proof(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            plan_path = contracts / "git-membrane-closure-plan-v1.json"
            plan_path.write_text(
                json.dumps(
                    {
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
                        "waves": [
                            {
                                "wave": 1,
                                "theme": "ghost",
                                "disposition": "track",
                                "targets": [{"path": "organs/ghost/", "classification": "organ_body", "reason": "test"}],
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
                module.git_ls_untracked = lambda *args: ["organs/ghost/local.py"]
                findings = module.validate_git_membrane_closure_plan_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn(
            "git_membrane_closure_proof_missing",
            [item.kind for item in findings],
        )

    def test_git_membrane_closure_preflight_passes_when_writable_and_planned(self):
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
                module.git_metadata_writable,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.GIT_MEMBRANE_CLOSURE_PLAN_PATH = plan_path
                module.git_ls_untracked = lambda *args: ["organs/one/local.py"]
                module.git_metadata_writable = lambda: True
                preflight = module.git_membrane_closure_preflight()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.GIT_MEMBRANE_CLOSURE_PLAN_PATH,
                    module.git_ls_untracked,
                    module.git_metadata_writable,
                ) = old_values

        self.assertTrue(preflight["ok"])
        self.assertTrue(preflight["git_writable"])
        self.assertEqual({"wave": 1, "theme": "one", "untracked_count": 1}, preflight["next_open_wave"])
        self.assertEqual([], preflight["blockers"])


if __name__ == "__main__":
    unittest.main()
