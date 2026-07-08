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


class OrganShapeTopologyTests(unittest.TestCase):
    def test_rejects_organ_without_manifest_or_declared_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organs = root / "organs"
            (organs / "ghost").mkdir(parents=True)
            contract = {"organ_boundary_exceptions": []}
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = organs
                findings = module.validate_organ_boundaries(contract)
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT = old_values

        self.assertEqual("organ_without_manifest", findings[0].kind)

    def test_allows_organ_without_manifest_when_exception_is_complete(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organs = root / "organs"
            (organs / "ghost").mkdir(parents=True)
            contract = {
                "organ_boundary_exceptions": [
                    {
                        "path": "organs/ghost/",
                        "status": "transitional",
                        "owner": "cortex",
                        "replacement": "manifest-backed organ",
                        "expiry_condition": "owner declares the boundary",
                    }
                ]
            }
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = organs
                findings = module.validate_organ_boundaries(contract)
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_organ_manifest_without_identity_status_or_description(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organ = root / "organs" / "partial"
            organ.mkdir(parents=True)
            (organ / "MANIFEST.yaml").write_text("tier: 2\n", encoding="utf-8")
            contract = {
                "organ_shape": {
                    "manifest_policy": {
                        "identity_fields_any": ["id", "name"],
                        "required_fields": ["status", "description"],
                        "allowed_statuses": ["active", "planned"],
                        "status_case": "case_insensitive",
                    }
                }
            }
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                findings = module.validate_organ_manifest_policy(contract)
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT = old_values

        self.assertEqual(
            [
                "organ_manifest_identity_missing",
                "organ_manifest_required_field_missing",
                "organ_manifest_required_field_missing",
            ],
            [item.kind for item in findings],
        )

    def test_rejects_organ_manifest_unknown_status(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organ = root / "organs" / "partial"
            organ.mkdir(parents=True)
            (organ / "MANIFEST.yaml").write_text(
                "name: partial\nstatus: improvised\ndescription: test\n",
                encoding="utf-8",
            )
            contract = {
                "organ_shape": {
                    "manifest_policy": {
                        "identity_fields_any": ["id", "name"],
                        "required_fields": ["status", "description"],
                        "allowed_statuses": ["active", "planned"],
                        "status_case": "case_insensitive",
                    }
                }
            }
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                findings = module.validate_organ_manifest_policy(contract)
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT = old_values

        self.assertEqual("organ_manifest_status_unknown", findings[0].kind)

    def test_allows_organ_manifest_with_name_or_id_and_known_status(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organ = root / "organs" / "partial"
            organ.mkdir(parents=True)
            (organ / "MANIFEST.yaml").write_text(
                "id: partial\nstatus: ACTIVE\ndescription: test\n",
                encoding="utf-8",
            )
            contract = {
                "organ_shape": {
                    "manifest_policy": {
                        "identity_fields_any": ["id", "name"],
                        "required_fields": ["status", "description"],
                        "allowed_statuses": ["active", "planned"],
                        "status_case": "case_insensitive",
                    }
                }
            }
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                findings = module.validate_organ_manifest_policy(contract)
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_organ_manifest_state_path_outside_relative_membrane(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organ = root / "organs" / "fleet"
            organ.mkdir(parents=True)
            (organ / "MANIFEST.yaml").write_text(
                "name: fleet\nstatus: planned\ndescription: test\nstate_paths:\n  - /tmp/escape.json\n",
                encoding="utf-8",
            )
            contract = {
                "organ_shape": {
                    "state_policy": {
                        "manifest_state_paths": {
                            "field": "state_paths",
                            "allowed_prefixes": ["state/", "desired/"],
                            "require_relative_paths": True,
                        }
                    }
                }
            }
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                findings = module.validate_organ_manifest_state_paths(contract)
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT = old_values

        self.assertEqual("organ_manifest_state_path_not_relative", findings[0].kind)

    def test_allows_organ_manifest_state_paths_with_declared_membranes(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organ = root / "organs" / "fleet"
            (organ / "state").mkdir(parents=True)
            (organ / "desired").mkdir(parents=True)
            (organ / "state" / "SSOT.json").write_text("{}", encoding="utf-8")
            (organ / "desired" / "state.json").write_text("{}", encoding="utf-8")
            (organ / "MANIFEST.yaml").write_text(
                """
name: fleet
status: planned
description: test
state_paths:
  - state/SSOT.json
  - desired/state.json
""",
                encoding="utf-8",
            )
            contract = {
                "organ_shape": {
                    "state_policy": {
                        "manifest_state_paths": {
                            "field": "state_paths",
                            "allowed_prefixes": ["state/", "desired/"],
                            "require_relative_paths": True,
                        }
                    }
                }
            }
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                findings = module.validate_organ_manifest_state_paths(contract)
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_manifest_script_reference_missing_or_untracked_without_status(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organ = root / "organs" / "partial"
            organ.mkdir(parents=True)
            (organ / "MANIFEST.yaml").write_text(
                """
id: partial
status: ACTIVE
description: test
scripts:
  - name: src/entrypoints/cli.py
  - name: local_probe.py
""",
                encoding="utf-8",
            )
            (organ / "local_probe.py").write_text("print('local')\n", encoding="utf-8")
            old_values = (
                module.WORKSPACE_ROOT,
                module.ORGANS_ROOT,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.git_ls_files = lambda *args: ["organs/partial/MANIFEST.yaml"]
                module.git_ls_untracked = lambda *args: ["organs/partial/local_probe.py"]
                findings = module.validate_manifest_script_references()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.ORGANS_ROOT,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual(
            ["manifest_script_reference_missing", "manifest_untracked_script_status_missing"],
            [item.kind for item in findings],
        )

    def test_allows_manifest_untracked_script_reference_with_explicit_status(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organ = root / "organs" / "partial"
            organ.mkdir(parents=True)
            (organ / "MANIFEST.yaml").write_text(
                """
id: partial
status: ACTIVE
description: test
scripts:
  - name: local_probe.py
    integration_status: transition_debt
    owner: organ-partial
    expiry_condition: promote or remove before activation
""",
                encoding="utf-8",
            )
            (organ / "local_probe.py").write_text("print('local')\n", encoding="utf-8")
            old_values = (
                module.WORKSPACE_ROOT,
                module.ORGANS_ROOT,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.git_ls_files = lambda *args: ["organs/partial/MANIFEST.yaml"]
                module.git_ls_untracked = lambda *args: ["organs/partial/local_probe.py"]
                findings = module.validate_manifest_script_references()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.ORGANS_ROOT,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual([], findings)

    def test_allows_manifest_script_reference_tracked_directory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organ = root / "organs" / "partial"
            (organ / "src" / "pipeline").mkdir(parents=True)
            (organ / "src" / "pipeline" / "stage.py").write_text("VALUE = 1\n", encoding="utf-8")
            (organ / "MANIFEST.yaml").write_text(
                """
id: partial
status: ACTIVE
description: test
scripts:
  - name: src/pipeline/
""",
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.ORGANS_ROOT,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.git_ls_files = lambda *args: [
                    "organs/partial/MANIFEST.yaml",
                    "organs/partial/src/pipeline/stage.py",
                ]
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_manifest_script_references()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.ORGANS_ROOT,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual([], findings)

    def test_rejects_python_organ_shape_without_declared_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organ = root / "organs" / "partial"
            (organ / "src" / "core").mkdir(parents=True)
            (organ / "MANIFEST.yaml").write_text("name: partial\n", encoding="utf-8")
            (organ / "src" / "core" / "model.py").write_text("VALUE = 1\n", encoding="utf-8")
            contract = {
                "organ_shape": {
                    "python_executable_required_paths": ["src/core", "src/adapters", "src/entrypoints"]
                },
                "organ_shape_exceptions": [],
            }
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                findings = module.validate_python_organ_shape(contract)
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT = old_values

        self.assertEqual("python_organ_shape_incomplete", findings[0].kind)

    def test_rejects_stale_python_organ_shape_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organ = root / "organs" / "partial"
            (organ / "src" / "core").mkdir(parents=True)
            (organ / "src" / "adapters").mkdir()
            (organ / "MANIFEST.yaml").write_text("name: partial\n", encoding="utf-8")
            (organ / "src" / "core" / "model.py").write_text("VALUE = 1\n", encoding="utf-8")
            contract = {
                "organ_shape": {
                    "python_executable_required_paths": ["src/core", "src/adapters", "src/entrypoints"]
                },
                "organ_shape_exceptions": [
                    {
                        "path": "organs/partial/",
                        "status": "transitional",
                        "owner": "cortex",
                        "missing_paths": ["src/adapters", "src/entrypoints"],
                        "replacement": "complete shape",
                        "expiry_condition": "all paths exist",
                    }
                ],
            }
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                findings = module.validate_python_organ_shape(contract)
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT = old_values

        self.assertEqual("organ_shape_exception_stale", findings[0].kind)


if __name__ == "__main__":
    unittest.main()
