import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).resolve().parents[1] / "controllers" / "validate_repo_topology.py"


def load_module():
    spec = importlib.util.spec_from_file_location("validate_repo_topology", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class RepoTopologyValidationTests(unittest.TestCase):
    def test_validates_contract_authorities_and_mcp_paths(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (root / "docs" / "foundation").mkdir(parents=True)
            (root / "AGENTS.md").write_text("# agents\n", encoding="utf-8")
            (root / "docs" / "foundation" / "CYNIC_REPO_TOPOLOGY_POLICY.md").write_text(
                "# policy\n",
                encoding="utf-8",
            )
            (root / "docs" / "foundation" / "CYNIC_ORGANISM_POLICY_MATRIX.md").write_text(
                "# matrix\n",
                encoding="utf-8",
            )
            (root / "docs" / "foundation" / "CYNIC_THEME_TOPOLOGY_POLICY.md").write_text(
                "# theme policy\n",
                encoding="utf-8",
            )
            (root / "crates" / "cynic-mcp" / "src").mkdir(parents=True)
            (root / "crates" / "cynic-mcp" / "src" / "proxy.rs").write_text(
                'let root = std::env::var("CYNIC_PROJECT_ROOT")?;\n',
                encoding="utf-8",
            )
            (root / "infra").mkdir()
            (root / "organs").mkdir()
            (root / "services").mkdir()
            (root / "scripts").mkdir()
            (root / ".cortex").mkdir()
            (root / ".cynic").mkdir()
            for path in ["docs/architecture", "crates/cynic-kernel"]:
                (root / path).mkdir(parents=True)
            contract = {
                "schema": "cynic.repo_topology.v1",
                "status": "normative",
                "authorities": {
                    "human_readable_policy": "docs/foundation/CYNIC_REPO_TOPOLOGY_POLICY.md",
                    "policy_matrix": "docs/foundation/CYNIC_ORGANISM_POLICY_MATRIX.md",
                    "agent_entrypoint": "AGENTS.md",
                },
                "root_classes": [
                    {
                        "path": "docs/foundation/",
                        "class": "policy",
                        "owns": ["policy"],
                        "must_not_own": ["runtime_state"],
                    },
                    {
                        "path": "axioms/",
                        "class": "semantic_contract",
                        "owns": ["contracts"],
                        "must_not_own": ["runtime_state"],
                    },
                    {
                        "path": "infra/",
                        "class": "organism_infrastructure",
                        "owns": ["registry"],
                        "must_not_own": ["organ_domain_logic"],
                    },
                    {
                        "path": "crates/cynic-mcp/",
                        "class": "portable_mcp_surface",
                        "owns": ["mcp_tool_contract_projection"],
                        "must_not_own": ["project_specific_hardcoded_paths"],
                    },
                ],
            }
            (contracts / "repo-topology-v1.json").write_text(json.dumps(contract), encoding="utf-8")
            theme_contract = {
                "schema": "cynic.organism_theme_topology.v1",
                "status": "normative",
                "authorities": {
                    "human_readable_policy": "docs/foundation/CYNIC_THEME_TOPOLOGY_POLICY.md",
                    "repo_topology_contract": "axioms/repo-topology-v1.json",
                },
                "themes": [
                    {
                        "id": "runtime",
                        "class": "runtime_topology",
                        "owns": ["topology"],
                        "canonical_sources": ["axioms/repo-topology-v1.json"],
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
            (contracts / "organism-theme-topology-v1.json").write_text(
                json.dumps(theme_contract),
                encoding="utf-8",
            )
            document_inventory = {
                "schema": "cynic.document_theme_inventory.v1",
                "status": "draft",
                "allowed_statuses": ["draft"],
                "documents": [],
            }
            (contracts / "document-theme-inventory-v1.json").write_text(
                json.dumps(document_inventory),
                encoding="utf-8",
            )

            old_values = (
                module.WORKSPACE_ROOT,
                module.CONTRACT_PATH,
                module.THEME_CONTRACT_PATH,
                module.DOCUMENT_THEME_INVENTORY_PATH,
                module.CONTRACTS_ROOT,
                module.MCP_SRC_ROOT,
                module.ORGANS_ROOT,
                module.git_ls_files,
                module.git_ls_untracked,
            )
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACT_PATH = contracts / "repo-topology-v1.json"
                module.THEME_CONTRACT_PATH = contracts / "organism-theme-topology-v1.json"
                module.DOCUMENT_THEME_INVENTORY_PATH = contracts / "document-theme-inventory-v1.json"
                module.CONTRACTS_ROOT = contracts
                module.MCP_SRC_ROOT = root / "crates" / "cynic-mcp" / "src"
                module.ORGANS_ROOT = root / "organs"
                module.git_ls_files = lambda *args: []
                module.git_ls_untracked = lambda *args: []
                findings = module.validate()
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.CONTRACT_PATH,
                    module.THEME_CONTRACT_PATH,
                    module.DOCUMENT_THEME_INVENTORY_PATH,
                    module.CONTRACTS_ROOT,
                    module.MCP_SRC_ROOT,
                    module.ORGANS_ROOT,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

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

    def test_rejects_root_class_without_responsibility_metadata(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "AGENTS.md").write_text("# agents\n", encoding="utf-8")
            contract_path = root / "axioms" / "repo-topology-v1.json"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.repo_topology.v1",
                        "status": "normative",
                        "authorities": {},
                        "root_classes": [{"path": "axioms/"}],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.CONTRACT_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACT_PATH = contract_path
                findings = module.validate_topology_contract()
            finally:
                module.WORKSPACE_ROOT, module.CONTRACT_PATH = old_values

        self.assertEqual(
            ["root_class_metadata_missing", "root_class_metadata_missing", "root_class_metadata_missing"],
            [item.kind for item in findings if item.kind == "root_class_metadata_missing"],
        )

    def test_rejects_unclassified_top_level_root_when_policy_enabled(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir()
            (root / "mystery").mkdir()
            contract_path = root / "axioms" / "repo-topology-v1.json"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.repo_topology.v1",
                        "status": "normative",
                        "authorities": {},
                        "root_classes": [
                            {
                                "path": "axioms/",
                                "class": "semantic_contract",
                                "owns": ["contracts"],
                                "must_not_own": ["runtime_state"],
                            }
                        ],
                        "top_level_root_policy": {"ignored_roots": []},
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.CONTRACT_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACT_PATH = contract_path
                findings = module.validate_topology_contract()
            finally:
                module.WORKSPACE_ROOT, module.CONTRACT_PATH = old_values

        self.assertIn("top_level_root_unclassified", [item.kind for item in findings])

    def test_allows_declared_or_ignored_top_level_roots(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir()
            (root / "target").mkdir()
            contract_path = root / "axioms" / "repo-topology-v1.json"
            contract_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.repo_topology.v1",
                        "status": "normative",
                        "authorities": {},
                        "root_classes": [
                            {
                                "path": "axioms/",
                                "class": "semantic_contract",
                                "owns": ["contracts"],
                                "must_not_own": ["runtime_state"],
                            }
                        ],
                        "top_level_root_policy": {"ignored_roots": ["target"]},
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.CONTRACT_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACT_PATH = contract_path
                findings = module.validate_topology_contract()
            finally:
                module.WORKSPACE_ROOT, module.CONTRACT_PATH = old_values

        self.assertEqual([], [item for item in findings if item.kind == "top_level_root_unclassified"])

    def test_rejects_hardcoded_project_path_in_mcp(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            mcp_src = root / "crates" / "cynic-mcp" / "src"
            mcp_src.mkdir(parents=True)
            (mcp_src / "proxy.rs").write_text(
                'let path = "/home/user/Bureau/example";\n',
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.MCP_SRC_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.MCP_SRC_ROOT = mcp_src
                findings = module.validate_mcp_no_absolute_project_paths()
            finally:
                module.WORKSPACE_ROOT, module.MCP_SRC_ROOT = old_values

        self.assertEqual("hardcoded_project_path_in_mcp", findings[0].kind)

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

    def test_rejects_provider_mutation_script_without_exception(self):
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
                    {"provider_mutation_script_exceptions": []}
                )
            finally:
                (
                    module.WORKSPACE_ROOT,
                    module.git_ls_files,
                    module.git_ls_untracked,
                ) = old_values

        self.assertEqual("provider_mutation_script_without_exception", findings[0].kind)

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

    def test_rejects_json_contract_without_schema_or_status(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (contracts / "bad.json").write_text(json.dumps({"name": "bad"}), encoding="utf-8")
            contract = {
                "contract_file_policy": {
                    "json_required_top_level_fields": ["schema", "status"],
                    "allowed_statuses": ["normative"],
                    "schema_prefix": "cynic.",
                }
            }
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(contract)
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertEqual(
            ["contract_required_field_missing", "contract_required_field_missing"],
            [item.kind for item in findings],
        )

    def test_rejects_non_json_file_in_contracts_without_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (contracts / "note.md").write_text("not a machine contract\n", encoding="utf-8")
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {"contract_file_policy": {"allowed_extensions": [".json"]}}
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertEqual("contract_file_extension_forbidden", findings[0].kind)

    def test_allows_non_json_file_in_contracts_with_explicit_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (contracts / "README.md").write_text("operator index\n", encoding="utf-8")
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {
                        "contract_file_policy": {
                            "allowed_extensions": [".json"],
                            "extension_exceptions": ["axioms/README.md"],
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_undeclared_contract_subdirectory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            (contracts / "misc").mkdir(parents=True)
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {"contract_file_policy": {"allowed_subdirectories": []}}
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertEqual("contract_subdirectory_undeclared", findings[0].kind)

    def test_allows_declared_contract_subdirectory_with_metadata(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            (contracts / "consumer-profiles").mkdir(parents=True)
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {
                        "contract_file_policy": {
                            "allowed_subdirectories": [
                                {
                                    "path": "axioms/consumer-profiles/",
                                    "class": "consumer_profile",
                                    "owner": "portable_coordination_contract",
                                    "reason": "external repo bindings",
                                }
                            ]
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_json_contract_with_unknown_status_or_schema_prefix(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (contracts / "bad.json").write_text(
                json.dumps({"schema": "external.bad.v1", "status": "loose"}),
                encoding="utf-8",
            )
            contract = {
                "contract_file_policy": {
                    "json_required_top_level_fields": ["schema", "status"],
                    "allowed_statuses": ["normative"],
                    "schema_prefix": "cynic.",
                }
            }
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(contract)
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertEqual(
            ["contract_schema_prefix_invalid", "contract_status_unknown"],
            sorted(item.kind for item in findings),
        )

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
                                "status": "legacy_blocked",
                                "owner": "organ-fleet",
                                "replacement": "fleet_node_exec",
                                "guard": "requires explicit env flag",
                                "expiry_condition": "canonical MCP covers it",
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

    def test_rejects_active_reference_to_legacy_hermes_x_tree(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            active_script = root / "scripts" / "active.sh"
            active_script.parent.mkdir(parents=True)
            active_script.write_text("python3 scripts/hermes-x/core/search_executor.py\n", encoding="utf-8")
            legacy_script = root / "scripts" / "hermes-x" / "core" / "search_executor.py"
            legacy_script.parent.mkdir(parents=True)
            legacy_script.write_text("# scripts/hermes-x compatibility shim\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_no_active_legacy_hermes_x_references()
            finally:
                module.WORKSPACE_ROOT = old_root

        kinds = [item.kind for item in findings]
        self.assertIn("active_legacy_hermes_x_reference", kinds)
        self.assertGreaterEqual(len(kinds), 1)

    def test_rejects_legacy_hermes_x_runtime_file(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "scripts" / "hermes-x"
            shim = root / "core" / "runtime.py"
            shim.parent.mkdir(parents=True)
            shim.write_text(
                """\
#!/usr/bin/env python3
def main():
    return True
""",
                encoding="utf-8",
            )

            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = Path(tmp)
                findings = module.validate_legacy_hermes_x_wrapper_contract()
            finally:
                module.WORKSPACE_ROOT = old_root

        kinds = [item.kind for item in findings]
        self.assertIn("legacy_hermes_x_runtime_implementation", kinds)

    def test_allows_legacy_hermes_x_compat_shim(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "scripts" / "hermes-x"
            shim = root / "core" / "search_executor.py"
            shim.parent.mkdir(parents=True)
            shim.write_text(
                """\
#!/usr/bin/env python3
from organs.twitter.src.adapters.search_executor import *  # noqa: F401,F403
""",
                encoding="utf-8",
            )

            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = Path(tmp)
                findings = module.validate_legacy_hermes_x_wrapper_contract()
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual([], [item.kind for item in findings if item.severity == "critical"])

    def test_flags_legacy_hermes_x_systemd_service_name(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "infra" / "systemd"
            legacy_service = root / "hermes-x-ingest.service"
            legacy_service.parent.mkdir(parents=True)
            legacy_service.write_text(
                "[Service]\nType=simple\nExecStart=/bin/true\n",
                encoding="utf-8",
            )

            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = Path(tmp)
                findings = module.validate_legacy_hermes_x_systemd_migration()
            finally:
                module.WORKSPACE_ROOT = old_root

        kinds = [item.kind for item in findings]
        self.assertIn("legacy_hermes_x_systemd_filename", kinds)

    def test_flags_legacy_hermes_x_systemd_dependency(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "infra" / "systemd"
            service = root / "some.service"
            service.parent.mkdir(parents=True)
            service.write_text(
                "[Unit]\nAfter=hermes-x-ingest.service\nWants=hermes-browser@%i.service\n",
                encoding="utf-8",
            )

            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = Path(tmp)
                findings = module.validate_legacy_hermes_x_systemd_migration()
            finally:
                module.WORKSPACE_ROOT = old_root

        kinds = [item.kind for item in findings]
        self.assertIn("legacy_hermes_x_systemd_dependency", kinds)

    def test_rejects_hardcoded_home_path_in_deployment_service(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "infra" / "systemd"
            service = root / "bad-projection.service"
            service.parent.mkdir(parents=True)
            service.write_text(
                "[Service]\nType=simple\nWorkingDirectory=%h/Bureau/SOLANA/asdf-forge/zeyxx/GASdf\n",
                encoding="utf-8",
            )

            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = Path(tmp)
                findings = module.validate_no_hardcoded_deployment_projection_paths()
            finally:
                module.WORKSPACE_ROOT = old_root

        kinds = [item.kind for item in findings]
        self.assertIn("hardcoded_deployment_projection_path", kinds)

    def test_allows_rendered_repo_root_in_deployment_service(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "infra" / "systemd"
            service = root / "good-projection.service"
            service.parent.mkdir(parents=True)
            service.write_text(
                "[Service]\nType=simple\nWorkingDirectory=@CYNIC_PROJECT_ROOT@/organs/gasdf\n",
                encoding="utf-8",
            )

            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = Path(tmp)
                findings = module.validate_no_hardcoded_deployment_projection_paths()
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual([], [item.kind for item in findings if item.kind == "hardcoded_deployment_projection_path"])


if __name__ == "__main__":
    unittest.main()
