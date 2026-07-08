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


    def test_rejects_canonical_document_outside_canonical_doc_zone(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            internal = root / "docs" / "internal"
            internal.mkdir(parents=True)
            (internal / "legacy.md").write_text("# legacy\n", encoding="utf-8")
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "themes": [{"id": "odysseum_futardio_legacy"}],
                        "doc_zones": [
                            {
                                "path": "docs/foundation/",
                                "class": "normative_policy",
                                "may_own": ["canon"],
                                "must_not_own": ["runtime"],
                                "canonicality": "canonical_when_indexed",
                            },
                            {
                                "path": "docs/internal/",
                                "class": "private_draft_and_context",
                                "may_own": ["historical context"],
                                "must_not_own": ["legal truth"],
                                "canonicality": "noncanonical_evidence",
                            },
                        ],
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
                        "allowed_statuses": ["canonical", "evidence"],
                        "documents": [
                            {
                                "path": "docs/internal/legacy.md",
                                "theme": "odysseum_futardio_legacy",
                                "status": "canonical",
                                "canonical_target": "docs/internal/legacy.md",
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
                module.git_ls_files = lambda *args: ["docs/internal/legacy.md"]
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
            "document_theme_inventory_canonical_outside_canonical_zone",
            [item.kind for item in findings],
        )

    def test_allows_canonical_document_inside_canonical_doc_zone(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            foundation = root / "docs" / "foundation"
            foundation.mkdir(parents=True)
            (foundation / "CANON.md").write_text("# canon\n", encoding="utf-8")
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "themes": [{"id": "cynic_protocol_framework"}],
                        "doc_zones": [
                            {
                                "path": "docs/foundation/",
                                "class": "normative_policy",
                                "may_own": ["canon"],
                                "must_not_own": ["runtime"],
                                "canonicality": "canonical_when_indexed",
                            }
                        ],
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
                        "allowed_statuses": ["canonical"],
                        "documents": [
                            {
                                "path": "docs/foundation/CANON.md",
                                "theme": "cynic_protocol_framework",
                                "status": "canonical",
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
                module.git_ls_files = lambda *args: ["docs/foundation/CANON.md"]
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

        self.assertEqual(
            [],
            [
                item
                for item in findings
                if item.kind == "document_theme_inventory_canonical_outside_canonical_zone"
            ],
        )

    def test_rejects_architecture_markdown_missing_theme_inventory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            architecture = root / "docs" / "architecture"
            architecture.mkdir(parents=True)
            (architecture / "NEW_MAP.md").write_text("# map\n", encoding="utf-8")
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "themes": [{"id": "organism_runtime_topology"}],
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
                module.git_ls_files = lambda *args: ["docs/architecture/NEW_MAP.md"]
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
            "sensitive_markdown_missing_theme_inventory",
            [item.kind for item in findings],
        )

    def test_rejects_emergence_markdown_missing_theme_inventory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            emergence = root / "data" / "emergence" / "probe"
            emergence.mkdir(parents=True)
            (emergence / "draft.md").write_text("# draft\n", encoding="utf-8")
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "themes": [{"id": "data_and_evidence"}],
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
                        "allowed_statuses": ["evidence"],
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
                module.git_ls_untracked = lambda *args: ["data/emergence/probe/draft.md"]
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

    def test_allows_emergence_markdown_with_directory_inventory_entry(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            emergence = root / "data" / "emergence" / "probe"
            emergence.mkdir(parents=True)
            (emergence / "MANIFEST.json").write_text("{}", encoding="utf-8")
            (emergence / "draft.md").write_text("# draft\n", encoding="utf-8")
            theme_contract_path = contracts / "organism-theme-topology-v1.json"
            theme_contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organism_theme_topology.v1",
                        "status": "normative",
                        "themes": [{"id": "data_and_evidence"}],
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
                        "allowed_statuses": ["evidence"],
                        "documents": [
                            {
                                "path": "data/emergence/probe/",
                                "theme": "data_and_evidence",
                                "status": "evidence",
                                "canonical_target": "data/emergence/probe/MANIFEST.json",
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
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: ["data/emergence/probe/draft.md"]
                findings = module.validate_document_theme_inventory_contract()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.THEME_CONTRACT_PATH,
                    module.DOCUMENT_THEME_INVENTORY_PATH,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_allows_sensitive_markdown_with_theme_inventory_entry(self):
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
                        "status_transition_policy": {
                            "draft": {
                                "owner": "organ-supervisor",
                                "rule": "test draft rule",
                                "exit_condition": "test exit",
                            }
                        },
                        "documents": [
                            {
                                "path": "docs/internal/draft.md",
                                "theme": "cynic_protocol_framework",
                                "status": "draft",
                                "canonical_target": "docs/foundation/TALARIA_CYNIC_NAMING.md",
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

        self.assertEqual([], [item for item in findings if item.severity == "critical"])


if __name__ == "__main__":
    unittest.main()
