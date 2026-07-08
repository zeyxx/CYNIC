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


def surface_status_transition_policy(*statuses):
    return {
        status: {
            "owner": "organ-supervisor",
            "rule": f"test {status} rule",
            "exit_condition": f"test {status} exit",
        }
        for status in statuses
    }


class RepoTopologySurfaceValidationPart2Tests(unittest.TestCase):
    def test_rejects_data_control_file_missing_inventory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "data" / "forged").mkdir(parents=True)
            inventory_path = contracts / "data-control-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.data_control_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["classified"],
                        "required_evidence_for_open_membrane": "classified data control file",
                        "watched_globs": ["data/**/*.json"],
                        "surfaces": [],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.DATA_CONTROL_SURFACE_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.DATA_CONTROL_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: ["data/forged/secret.json"]
                findings = module.validate_data_control_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.DATA_CONTROL_SURFACE_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn("data_control_surface_missing_inventory", [item.kind for item in findings])

    def test_allows_classified_data_control_file(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            data_file = root / "data" / "forged" / "state" / "secret.json"
            data_file.parent.mkdir(parents=True)
            data_file.write_text('{"k": "v"}', encoding="utf-8")
            inventory_path = contracts / "data-control-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.data_control_surface_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["classified"],
                        "status_transition_policy": surface_status_transition_policy("classified"),
                        "required_evidence_for_open_membrane": "classified data control file",
                        "watched_globs": ["data/**/*.json"],
                        "surfaces": [
                            {
                                "path": "data/forged/state/secret.json",
                                "status": "classified",
                                "owner": "organ-forge",
                                "classification": "data_control",
                                "canonical_target": "data",
                                "expiry_condition": "rotation",
                                "evidence": ["organs/forged/state/secret.json"],
                                "next_action": "review",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.DATA_CONTROL_SURFACE_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.DATA_CONTROL_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: ["data/forged/state/secret.json"]
                findings = module.validate_data_control_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.DATA_CONTROL_SURFACE_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])


    def test_rejects_root_diagnostic_without_inventory_entry(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "debug_kernel.py").write_text("print('probe')\n", encoding="utf-8")
            inventory_path = contracts / "diagnostic-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.diagnostic_surface_inventory.v1",
                        "status": "draft",
                        "watched_root_globs": ["debug_*.py"],
                        "allowed_statuses": ["local_diagnostic"],
                        "surfaces": [],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.DIAGNOSTIC_SURFACE_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.DIAGNOSTIC_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: ["debug_kernel.py"]
                findings = module.validate_diagnostic_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.DIAGNOSTIC_SURFACE_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn("diagnostic_surface_missing_inventory", [item.kind for item in findings])

    def test_rejects_unknown_diagnostic_surface_status(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "debug_kernel.py").write_text("print('probe')\n", encoding="utf-8")
            inventory_path = contracts / "diagnostic-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.diagnostic_surface_inventory.v1",
                        "status": "draft",
                        "watched_root_globs": ["debug_*.py"],
                        "allowed_statuses": ["local_diagnostic"],
                        "status_transition_policy": surface_status_transition_policy("local_diagnostic"),
                        "surfaces": [
                            {
                                "path": "debug_kernel.py",
                                "status": "improvised",
                                "owner": "organ-forge",
                                "classification": "kernel_probe",
                                "replacement": "typed test",
                                "expiry_condition": "typed test exists",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.DIAGNOSTIC_SURFACE_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.DIAGNOSTIC_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: ["debug_kernel.py"]
                findings = module.validate_diagnostic_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.DIAGNOSTIC_SURFACE_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn("diagnostic_surface_inventory_status_unknown", [item.kind for item in findings])

    def test_allows_classified_root_diagnostic_surface(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "debug_kernel.py").write_text("print('probe')\n", encoding="utf-8")
            inventory_path = contracts / "diagnostic-surface-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.diagnostic_surface_inventory.v1",
                        "status": "draft",
                        "watched_root_globs": ["debug_*.py"],
                        "allowed_statuses": ["local_diagnostic"],
                        "status_transition_policy": surface_status_transition_policy("local_diagnostic"),
                        "surfaces": [
                            {
                                "path": "debug_kernel.py",
                                "status": "local_diagnostic",
                                "owner": "organ-forge",
                                "classification": "kernel_probe",
                                "replacement": "typed test",
                                "expiry_condition": "typed test exists",
                            }
                        ],
                        "totals": {
                            "surfaces": 1,
                            "by_status": {"local_diagnostic": 1},
                        },
                    }
                ),
                encoding="utf-8",
            )
            old_values = (
                module.WORKSPACE_ROOT,
                module.DIAGNOSTIC_SURFACE_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.DIAGNOSTIC_SURFACE_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: ["debug_kernel.py"]
                findings = module.validate_diagnostic_surface_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.DIAGNOSTIC_SURFACE_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])
