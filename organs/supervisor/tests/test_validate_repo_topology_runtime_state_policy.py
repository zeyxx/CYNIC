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

class RuntimeStateAndEvidenceTopologyTests(unittest.TestCase):
    def test_rejects_tracked_runtime_state_without_exception(self):
        module = load_module()
        old_git_ls_files = module.git_ls_files
        try:
            module.git_ls_files = lambda *args: ["organs/forge/state/state.json"]
            findings = module.validate_runtime_state_commit_exceptions(
                {"runtime_state_commit_exceptions": []}
            )
        finally:
            module.git_ls_files = old_git_ls_files

        self.assertEqual("tracked_runtime_state_without_exception", findings[0].kind)

    def test_allows_tracked_runtime_state_with_complete_exception(self):
        module = load_module()
        old_git_ls_files = module.git_ls_files
        try:
            module.git_ls_files = lambda *args: ["organs/forge/state/state.json"]
            findings = module.validate_runtime_state_commit_exceptions(
                {
                    "runtime_state_commit_exceptions": [
                        {
                            "paths": ["organs/forge/state/state.json"],
                            "status": "quarantined",
                            "owner": "organ-forge",
                            "commit_policy": "legacy_tracked_only_no_new_files",
                            "replacement": "Kernel mempool",
                            "expiry_condition": "state file removed",
                        }
                    ]
                }
            )
        finally:
            module.git_ls_files = old_git_ls_files

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_visible_local_data_runtime_output(self):
        module = load_module()
        old_git_ls_untracked = module.git_ls_untracked
        try:
            module.git_ls_untracked = lambda *args: ["data/token_profiles.jsonl"]
            findings = module.validate_local_data_runtime_outputs_ignored()
        finally:
            module.git_ls_untracked = old_git_ls_untracked

        self.assertEqual("local_data_runtime_output_visible", findings[0].kind)

    def test_allows_visible_emergence_data_output(self):
        module = load_module()
        old_git_ls_untracked = module.git_ls_untracked
        try:
            module.git_ls_untracked = lambda *args: [
                "data/emergence/superteam-browser/2026-06-24/MANIFEST.json"
            ]
            findings = module.validate_local_data_runtime_outputs_ignored()
        finally:
            module.git_ls_untracked = old_git_ls_untracked

        self.assertEqual([], findings)

    def test_rejects_emergence_capsule_without_manifest(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            capsule = root / "data" / "emergence" / "probe" / "2026-06-24"
            capsule.mkdir(parents=True)
            (capsule / "analysis.json").write_text("{}", encoding="utf-8")
            evidence_contract = {
                "schema": "cynic.evidence_capsule.v1",
                "status": "normative",
                "root": "data/emergence/",
                "capsule_depth": 2,
                "manifest_name": "MANIFEST.json",
                "manifest_required_fields": ["id", "created_at", "root", "purpose", "status", "sensitivity_policy"],
                "document_inventory_requirements": {
                    "theme": "data_and_evidence",
                    "status": "evidence",
                    "canonical_target": "<capsule>/MANIFEST.json",
                },
            }
            (contracts / "evidence-capsule-v1.json").write_text(json.dumps(evidence_contract), encoding="utf-8")
            (contracts / "document-theme-inventory-v1.json").write_text(
                json.dumps({"schema": "cynic.document_theme_inventory.v1", "documents": []}),
                encoding="utf-8",
            )

            old_values = (
                module.WORKSPACE_ROOT,
                module.EVIDENCE_CAPSULE_CONTRACT_PATH,
                module.DOCUMENT_THEME_INVENTORY_PATH,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.EVIDENCE_CAPSULE_CONTRACT_PATH = contracts / "evidence-capsule-v1.json"
                module.DOCUMENT_THEME_INVENTORY_PATH = contracts / "document-theme-inventory-v1.json"
                findings = module.validate_evidence_capsule_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.EVIDENCE_CAPSULE_CONTRACT_PATH,
                    module.DOCUMENT_THEME_INVENTORY_PATH,
                ) = old_values

        self.assertIn("evidence_capsule_manifest_missing", [item.kind for item in findings])

    def test_rejects_emergence_capsule_without_document_inventory_entry(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            capsule = root / "data" / "emergence" / "probe" / "2026-06-24"
            capsule.mkdir(parents=True)
            manifest = {
                "id": "probe-2026-06-24",
                "created_at": "2026-06-24T00:00:00Z",
                "root": "data/emergence/probe/2026-06-24",
                "purpose": "test capsule",
                "status": "complete_v1",
                "sensitivity_policy": {},
            }
            (capsule / "MANIFEST.json").write_text(json.dumps(manifest), encoding="utf-8")
            evidence_contract = {
                "schema": "cynic.evidence_capsule.v1",
                "status": "normative",
                "root": "data/emergence/",
                "capsule_depth": 2,
                "manifest_name": "MANIFEST.json",
                "manifest_required_fields": ["id", "created_at", "root", "purpose", "status", "sensitivity_policy"],
                "document_inventory_requirements": {
                    "theme": "data_and_evidence",
                    "status": "evidence",
                    "canonical_target": "<capsule>/MANIFEST.json",
                },
            }
            (contracts / "evidence-capsule-v1.json").write_text(json.dumps(evidence_contract), encoding="utf-8")
            (contracts / "document-theme-inventory-v1.json").write_text(
                json.dumps({"schema": "cynic.document_theme_inventory.v1", "documents": []}),
                encoding="utf-8",
            )

            old_values = (
                module.WORKSPACE_ROOT,
                module.EVIDENCE_CAPSULE_CONTRACT_PATH,
                module.DOCUMENT_THEME_INVENTORY_PATH,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.EVIDENCE_CAPSULE_CONTRACT_PATH = contracts / "evidence-capsule-v1.json"
                module.DOCUMENT_THEME_INVENTORY_PATH = contracts / "document-theme-inventory-v1.json"
                findings = module.validate_evidence_capsule_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.EVIDENCE_CAPSULE_CONTRACT_PATH,
                    module.DOCUMENT_THEME_INVENTORY_PATH,
                ) = old_values

        self.assertIn("evidence_capsule_missing_document_inventory", [item.kind for item in findings])

    def test_allows_manifested_emergence_capsule_with_document_inventory_entry(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            capsule = root / "data" / "emergence" / "probe" / "2026-06-24"
            capsule.mkdir(parents=True)
            manifest = {
                "id": "probe-2026-06-24",
                "created_at": "2026-06-24T00:00:00Z",
                "root": "data/emergence/probe/2026-06-24",
                "purpose": "test capsule",
                "status": "complete_v1",
                "sensitivity_policy": {},
            }
            (capsule / "MANIFEST.json").write_text(json.dumps(manifest), encoding="utf-8")
            evidence_contract = {
                "schema": "cynic.evidence_capsule.v1",
                "status": "normative",
                "root": "data/emergence/",
                "capsule_depth": 2,
                "manifest_name": "MANIFEST.json",
                "manifest_required_fields": ["id", "created_at", "root", "purpose", "status", "sensitivity_policy"],
                "document_inventory_requirements": {
                    "theme": "data_and_evidence",
                    "status": "evidence",
                    "canonical_target": "<capsule>/MANIFEST.json",
                },
            }
            (contracts / "evidence-capsule-v1.json").write_text(json.dumps(evidence_contract), encoding="utf-8")
            inventory = {
                "schema": "cynic.document_theme_inventory.v1",
                "documents": [
                    {
                        "path": "data/emergence/probe/2026-06-24/",
                        "theme": "data_and_evidence",
                        "status": "evidence",
                        "canonical_target": "data/emergence/probe/2026-06-24/MANIFEST.json",
                    }
                ],
            }
            (contracts / "document-theme-inventory-v1.json").write_text(json.dumps(inventory), encoding="utf-8")

            old_values = (
                module.WORKSPACE_ROOT,
                module.EVIDENCE_CAPSULE_CONTRACT_PATH,
                module.DOCUMENT_THEME_INVENTORY_PATH,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.EVIDENCE_CAPSULE_CONTRACT_PATH = contracts / "evidence-capsule-v1.json"
                module.DOCUMENT_THEME_INVENTORY_PATH = contracts / "document-theme-inventory-v1.json"
                findings = module.validate_evidence_capsule_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.EVIDENCE_CAPSULE_CONTRACT_PATH,
                    module.DOCUMENT_THEME_INVENTORY_PATH,
                ) = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

