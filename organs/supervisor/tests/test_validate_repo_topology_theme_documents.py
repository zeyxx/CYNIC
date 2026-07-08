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


class ThemeDocumentTopologyValidationTests(unittest.TestCase):
    def test_rejects_theme_topology_missing_canonical_source(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "authorities": {},
                        "themes": [
                            {
                                "id": "runtime",
                                "class": "runtime_topology",
                                "owns": ["topology"],
                                "canonical_sources": ["axioms/missing.json"],
                                "allowed_projection_paths": ["docs/architecture/"],
                                "must_not_own": ["legal_truth"],
                                "drift_rule": "contracts first",
                            }
                        ],
                        "doc_zones": [
                            {
                                "path": "axioms/",
                                "class": "machine_contract",
                                "may_own": ["contracts"],
                                "must_not_own": ["drafts"],
                                "canonicality": "canonical",
                            }
                        ],
                        "promotion_rule": {
                            "from": ["docs/internal/"],
                            "to": ["docs/foundation/"],
                            "requires": ["declared theme"],
                        },
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.THEME_CONTRACT_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.THEME_CONTRACT_PATH = theme_contract_path
                findings = module.validate_theme_topology_contract()
            finally:
                module.WORKSPACE_ROOT, module.THEME_CONTRACT_PATH = old_values

        self.assertIn(
            "theme_topology_canonical_source_missing",
            [item.kind for item in findings],
        )

    def test_rejects_incomplete_theme_metadata(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "authorities": {},
                        "themes": [{"id": "runtime"}],
                        "doc_zones": [],
                        "promotion_rule": {},
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.THEME_CONTRACT_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.THEME_CONTRACT_PATH = theme_contract_path
                findings = module.validate_theme_topology_contract()
            finally:
                module.WORKSPACE_ROOT, module.THEME_CONTRACT_PATH = old_values

        kinds = [item.kind for item in findings]
        self.assertIn("theme_topology_theme_metadata_missing", kinds)
        self.assertIn("theme_topology_doc_zones_missing", kinds)
        self.assertIn("theme_topology_promotion_rule_incomplete", kinds)

    def test_rejects_sensitive_markdown_missing_theme_inventory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            internal = root / "docs" / "internal"
            internal.mkdir(parents=True)
            (internal / "draft.md").write_text("# draft\n", encoding="utf-8")
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "themes": [{"id": "cynic_protocol_framework"}],
                    }
                ),
                encoding="utf-8",
            )
            inventory_path = contracts / "document-theme-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.document_theme_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["draft"],
                        "documents": [],
                    }
                ),
                encoding="utf-8",
            )

            old_values = (
                module.WORKSPACE_ROOT,
                module.THEME_CONTRACT_PATH,
                module.DOCUMENT_THEME_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.THEME_CONTRACT_PATH = theme_contract_path
                module.DOCUMENT_THEME_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: ["docs/internal/draft.md"]
                findings = module.validate_document_theme_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.THEME_CONTRACT_PATH,
                    module.DOCUMENT_THEME_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn(
            "sensitive_markdown_missing_theme_inventory",
            [item.kind for item in findings],
        )

    def test_rejects_foundation_markdown_missing_theme_inventory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            foundation = root / "docs" / "foundation"
            foundation.mkdir(parents=True)
            (foundation / "NEW_CANON.md").write_text("# canon\n", encoding="utf-8")
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "themes": [{"id": "cynic_protocol_framework"}],
                    }
                ),
                encoding="utf-8",
            )
            inventory_path = contracts / "document-theme-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.document_theme_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["draft"],
                        "documents": [],
                    }
                ),
                encoding="utf-8",
            )

            old_values = (
                module.WORKSPACE_ROOT,
                module.THEME_CONTRACT_PATH,
                module.DOCUMENT_THEME_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.THEME_CONTRACT_PATH = theme_contract_path
                module.DOCUMENT_THEME_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: ["docs/foundation/NEW_CANON.md"]
                findings = module.validate_document_theme_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.THEME_CONTRACT_PATH,
                    module.DOCUMENT_THEME_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn(
            "sensitive_markdown_missing_theme_inventory",
            [item.kind for item in findings],
        )

    def test_ignores_deleted_tracked_sensitive_markdown(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "themes": [{"id": "cynic_protocol_framework"}],
                    }
                ),
                encoding="utf-8",
            )
            inventory_path = contracts / "document-theme-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.document_theme_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["draft"],
                        "documents": [],
                    }
                ),
                encoding="utf-8",
            )

            old_values = (
                module.WORKSPACE_ROOT,
                module.THEME_CONTRACT_PATH,
                module.DOCUMENT_THEME_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.THEME_CONTRACT_PATH = theme_contract_path
                module.DOCUMENT_THEME_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: ["packages/cynic-ui/README.md"]
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_document_theme_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.THEME_CONTRACT_PATH,
                    module.DOCUMENT_THEME_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertNotIn(
            "sensitive_markdown_missing_theme_inventory",
            [item.kind for item in findings],
        )

    def test_ignores_deleted_tracked_inventory_entry(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "themes": [{"id": "cynic_protocol_framework"}],
                    }
                ),
                encoding="utf-8",
            )
            inventory_path = contracts / "document-theme-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.document_theme_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["projection"],
                        "documents": [
                            {
                                "path": "docs/internal/deleted.md",
                                "theme": "cynic_protocol_framework",
                                "status": "projection",
                                "canonical_target": "docs/foundation/CYNIC.md",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            old_values = (
                module.WORKSPACE_ROOT,
                module.THEME_CONTRACT_PATH,
                module.DOCUMENT_THEME_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.THEME_CONTRACT_PATH = theme_contract_path
                module.DOCUMENT_THEME_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: ["docs/internal/deleted.md"]
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_document_theme_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.THEME_CONTRACT_PATH,
                    module.DOCUMENT_THEME_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertNotIn(
            "document_theme_inventory_path_missing",
            [item.kind for item in findings],
        )

    def test_rejects_transient_document_status_without_transition_policy(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            internal = root / "docs" / "internal"
            internal.mkdir(parents=True)
            (internal / "draft.md").write_text("# draft\n", encoding="utf-8")
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "themes": [{"id": "cynic_protocol_framework"}],
                    }
                ),
                encoding="utf-8",
            )
            inventory_path = contracts / "document-theme-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.document_theme_inventory.v1",
                        "status": "draft",
                        "allowed_statuses": ["draft"],
                        "documents": [
                            {
                                "path": "docs/internal/draft.md",
                                "theme": "cynic_protocol_framework",
                                "status": "draft",
                                "canonical_target": "docs/foundation/CANON.md",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )

            old_values = (
                module.WORKSPACE_ROOT,
                module.THEME_CONTRACT_PATH,
                module.DOCUMENT_THEME_INVENTORY_PATH,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.THEME_CONTRACT_PATH = theme_contract_path
                module.DOCUMENT_THEME_INVENTORY_PATH = inventory_path
                module.git_ls_files = lambda *args: ["docs/internal/draft.md"]
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_document_theme_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.THEME_CONTRACT_PATH,
                    module.DOCUMENT_THEME_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertIn(
            "document_theme_inventory_status_transition_missing",
            [item.kind for item in findings],
        )



if __name__ == "__main__":
    unittest.main()
