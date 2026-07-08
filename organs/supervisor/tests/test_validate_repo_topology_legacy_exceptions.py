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

class LegacyExceptionTopologyTests(unittest.TestCase):
    def test_allows_provider_mutation_script_with_complete_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = root / "scripts" / "danger.sh"
            script.parent.mkdir()
            script.write_text("pct create 103 template\n", encoding="utf-8")
            old_values = (
                module.WORKSPACE_ROOT,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.git_ls_files = lambda *args: ["scripts/danger.sh"]
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_provider_mutation_script_exceptions(
                    {
                        "provider_mutation_script_exceptions": [
                            {
                                "path": "scripts/danger.sh",
                                "status": "transitional",
                                "owner": "organ-supervisor",
                                "classification": "provider_mutation",
                                "replacement": "MCP/coord",
                                "guard": "requires explicit env flag",
                                "expiry_condition": "all mutations use MCP or organ helpers",
                            }
                        ]
                    }
                )
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_transitional_path_without_boundary_metadata(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "scripts").mkdir()
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_current_transitional_paths(
                    {
                        "current_transitional_paths": [
                            {
                                "path": "scripts/",
                                "owner": "cortex",
                                "replacement": "organ entrypoint",
                                "expiry_condition": "all scripts migrated",
                            }
                        ]
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual(
            ["transitional_path_metadata_missing", "transitional_path_metadata_missing"],
            [item.kind for item in findings],
        )

    def test_allows_transitional_path_with_complete_boundary_metadata(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "scripts").mkdir()
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_current_transitional_paths(
                    {
                        "current_transitional_paths": [
                            {
                                "path": "scripts/",
                                "owner": "cortex",
                                "allowed_file_kinds": ["diagnostic"],
                                "forbidden_concerns": ["canonical_lifecycle"],
                                "replacement": "organ entrypoint",
                                "expiry_condition": "all scripts migrated",
                            }
                        ]
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_file_under_closed_legacy_path(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = root / "scripts" / "new-entry.sh"
            script.parent.mkdir()
            script.write_text("#!/bin/sh\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_closed_legacy_paths(
                    {
                        "closed_legacy_paths": [
                            {
                                "path": "scripts/",
                                "owner": "cortex",
                                "status": "legacy_blocked",
                                "replacement": "organ entrypoint",
                                "expiry_condition": "scripts deletion committed",
                            }
                        ]
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual("closed_legacy_path_contains_file", findings[0].kind)

    def test_allows_empty_closed_legacy_path(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "scripts").mkdir()
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_closed_legacy_paths(
                    {
                        "closed_legacy_paths": [
                            {
                                "path": "scripts/",
                                "owner": "cortex",
                                "status": "legacy_blocked",
                                "replacement": "organ entrypoint",
                                "expiry_condition": "scripts deletion committed",
                            }
                        ]
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_untracked_transitional_wrapper_without_file_exception(self):
        module = load_module()
        old_git_ls_untracked = module.git_ls_untracked
        try:
            module.git_ls_untracked = lambda *args: ["scripts/new-wrapper.sh"]
            findings = module.validate_transitional_wrapper_file_exceptions(
                {"transitional_wrapper_file_exceptions": []}
            )
        finally:
            module.git_ls_untracked = old_git_ls_untracked

        self.assertEqual("unclassified_new_transitional_wrapper", findings[0].kind)

    def test_allows_untracked_transitional_wrapper_with_file_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = root / "scripts" / "new-wrapper.sh"
            script.parent.mkdir()
            script.write_text("#!/bin/sh\n", encoding="utf-8")
            old_values = (module.WORKSPACE_ROOT, module.git_ls_untracked)
            try:
                module.WORKSPACE_ROOT = root
                module.git_ls_untracked = lambda *args: ["scripts/new-wrapper.sh"]
                findings = module.validate_transitional_wrapper_file_exceptions(
                    {
                        "transitional_wrapper_file_exceptions": [
                            {
                                "path": "scripts/new-wrapper.sh",
                                "status": "transitional",
                                "owner": "organ-supervisor",
                                "classification": "diagnostic",
                                "replacement": "organ-supervisor entrypoint",
                                "expiry_condition": "entrypoint exists",
                            }
                        ]
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.git_ls_untracked = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_manifest_under_scripts_without_misplaced_organ_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "scripts" / "hidden-organ" / "MANIFEST.yaml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text("id: hidden\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_misplaced_organ_exceptions(
                    {"misplaced_organ_exceptions": []}
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual("organ_manifest_under_transitional_scripts", findings[0].kind)

    def test_allows_manifest_under_scripts_with_misplaced_organ_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = root / "scripts" / "hidden-organ" / "MANIFEST.yaml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text("id: hidden\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_misplaced_organ_exceptions(
                    {
                        "misplaced_organ_exceptions": [
                            {
                                "path": "scripts/hidden-organ/",
                                "status": "legacy_blocked",
                                "owner": "organ-twitter",
                                "target": "organs/twitter/",
                                "replacement": "move into organ-twitter",
                                "expiry_condition": "manifest removed from scripts",
                            }
                        ]
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual([], [item for item in findings if item.severity == "critical"])


if __name__ == "__main__":
    unittest.main()
