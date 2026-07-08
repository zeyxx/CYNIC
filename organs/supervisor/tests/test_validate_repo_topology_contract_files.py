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

class ContractFilePolicyTopologyTests(unittest.TestCase):
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

    def test_rejects_legacy_docs_contracts_root(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (root / "docs" / "contracts").mkdir(parents=True)
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {
                        "contract_file_policy": {
                            "legacy_contract_roots_forbidden": [
                                {
                                    "path": "docs/contracts/",
                                    "replacement": "axioms/",
                                    "reason": "legacy namespace",
                                }
                            ]
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertEqual("legacy_contract_root_present", findings[0].kind)

    def test_rejects_root_topology_markdown(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (root / "TOPOLOGY.md").write_text("# topology\n", encoding="utf-8")
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {
                        "contract_file_policy": {
                            "legacy_contract_roots_forbidden": [
                                {
                                    "path": "TOPOLOGY.md",
                                    "replacement": "axioms/repo-topology-v1.json",
                                    "reason": "legacy root topology",
                                }
                            ]
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertEqual("legacy_contract_root_present", findings[0].kind)

    def test_rejects_incomplete_rust_hexagonal_crate_policy(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            crate = root / "crates" / "cynic-node"
            (crate / "src" / "domain").mkdir(parents=True)
            (crate / "src" / "adapters").mkdir(parents=True)
            (crate / "src" / "usecases").mkdir(parents=True)
            (crate / "src" / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
            (crate / "build.rs").write_text("fn main() {}\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_rust_hexagonal_crate_policy(
                    {
                        "rust_hexagonal_crate_policy": {
                            "allowed_crates": [
                                {
                                    "path": "crates/cynic-node/",
                                    "required_paths": [
                                        "src/main.rs",
                                        "src/domain/",
                                        "src/adapters/",
                                        "src/ports/",
                                        "src/usecases/",
                                    ],
                                    "owner": "cynic-node",
                                    "reason": "missing ports on purpose",
                                }
                            ]
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertIn("rust_hexagonal_crate_required_path_missing", [item.kind for item in findings])

    def test_allows_complete_rust_hexagonal_crate_policy(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            crate = root / "crates" / "cynic-node"
            (crate / "src" / "domain").mkdir(parents=True)
            (crate / "src" / "adapters").mkdir(parents=True)
            (crate / "src" / "ports").mkdir(parents=True)
            (crate / "src" / "usecases").mkdir(parents=True)
            (crate / "src" / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
            (crate / "build.rs").write_text("fn main() {}\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_rust_hexagonal_crate_policy(
                    {
                        "rust_hexagonal_crate_policy": {
                            "allowed_crates": [
                                {
                                    "path": "crates/cynic-node/",
                                    "required_paths": [
                                        "src/main.rs",
                                        "src/domain/",
                                        "src/adapters/",
                                        "src/ports/",
                                        "src/usecases/",
                                    ],
                                    "max_rs_file_lines": 10,
                                    "owner": "cynic-node",
                                    "reason": "complete hex crate shape",
                                }
                            ]
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual([], findings)

    def test_rejects_rust_hexagonal_crate_file_that_exceeds_line_budget(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            crate = root / "crates" / "cynic-node"
            (crate / "src" / "domain").mkdir(parents=True)
            (crate / "src" / "adapters").mkdir(parents=True)
            (crate / "src" / "ports").mkdir(parents=True)
            (crate / "src" / "usecases").mkdir(parents=True)
            (crate / "src" / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
            (crate / "src" / "domain" / "heavy.rs").write_text(
                "\n".join(f"pub fn f{i}() {{}}" for i in range(20)) + "\n",
                encoding="utf-8",
            )
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_rust_hexagonal_crate_policy(
                    {
                        "rust_hexagonal_crate_policy": {
                            "allowed_crates": [
                                {
                                    "path": "crates/cynic-node/",
                                    "required_paths": [
                                        "src/main.rs",
                                        "src/domain/",
                                        "src/adapters/",
                                        "src/ports/",
                                        "src/usecases/",
                                    ],
                                    "max_rs_file_lines": 10,
                                    "owner": "cynic-node",
                                    "reason": "enforce split modules",
                                }
                            ]
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertIn("rust_hexagonal_crate_file_too_large", [item.kind for item in findings])

    def test_allows_declared_rust_hexagonal_crate_line_count_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            crate = root / "crates" / "cynic-node"
            (crate / "src" / "domain").mkdir(parents=True)
            (crate / "src" / "adapters").mkdir(parents=True)
            (crate / "src" / "ports").mkdir(parents=True)
            (crate / "src" / "usecases").mkdir(parents=True)
            (crate / "src" / "main.rs").write_text(
                "\n".join(f"fn step_{i}() {{}}" for i in range(20)) + "\n",
                encoding="utf-8",
            )
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_rust_hexagonal_crate_policy(
                    {
                        "rust_hexagonal_crate_policy": {
                            "allowed_crates": [
                                {
                                    "path": "crates/cynic-node/",
                                    "required_paths": [
                                        "src/main.rs",
                                        "src/domain/",
                                        "src/adapters/",
                                        "src/ports/",
                                        "src/usecases/",
                                    ],
                                    "max_rs_file_lines": 10,
                                    "line_count_exceptions": [
                                        {
                                            "path": "src/main.rs",
                                            "max_lines": 30,
                                            "reason": "temporary runtime composition shell",
                                        }
                                    ],
                                    "owner": "cynic-node",
                                    "reason": "allow bounded transition",
                                }
                            ]
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual([], findings)

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

    def test_rejects_root_axiom_contract_missing_from_registry(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (contracts / "axiom-registry-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.axiom_registry.v1",
                        "status": "normative",
                        "functional_layers": [{"id": "semantic_contract"}],
                        "topological_layers": [{"id": "repo"}],
                        "contract_scales": [
                            {
                                "id": "repo",
                                "projection": "fractal_scale",
                                "description": "filesystem contract scale",
                            }
                        ],
                        "contracts": [
                            {
                                "id": "axiom_registry",
                                "path": "axioms/axiom-registry-v1.json",
                                "family": "governance",
                                "functional_layer": "semantic_contract",
                                "topological_layer": "repo",
                                "scale": "repo",
                                "owner": "organ-supervisor",
                                "merge_policy": "extend_existing_family",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            (contracts / "orphan-v1.json").write_text(
                json.dumps({"schema": "cynic.orphan.v1", "status": "draft"}),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {
                        "contract_file_policy": {
                            "registry_contract": "axioms/axiom-registry-v1.json",
                            "json_required_top_level_fields": ["schema", "status"],
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertIn("axiom_contract_unregistered", [item.kind for item in findings])

    def test_allows_root_axiom_contracts_declared_in_registry(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (contracts / "axiom-registry-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.axiom_registry.v1",
                        "status": "normative",
                        "functional_layers": [{"id": "semantic_contract"}],
                        "topological_layers": [{"id": "repo"}],
                        "contract_scales": [
                            {
                                "id": "repo",
                                "projection": "fractal_scale",
                                "description": "filesystem contract scale",
                            }
                        ],
                        "contracts": [
                            {
                                "id": "axiom_registry",
                                "path": "axioms/axiom-registry-v1.json",
                                "family": "governance",
                                "functional_layer": "semantic_contract",
                                "topological_layer": "repo",
                                "scale": "repo",
                                "owner": "organ-supervisor",
                                "merge_policy": "extend_existing_family",
                            },
                            {
                                "id": "declared",
                                "path": "axioms/declared-v1.json",
                                "family": "test",
                                "functional_layer": "semantic_contract",
                                "topological_layer": "repo",
                                "scale": "repo",
                                "owner": "organ-supervisor",
                                "merge_policy": "extend_existing_family",
                            },
                        ],
                    }
                ),
                encoding="utf-8",
            )
            (contracts / "declared-v1.json").write_text(
                json.dumps({"schema": "cynic.declared.v1", "status": "draft"}),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {
                        "contract_file_policy": {
                            "registry_contract": "axioms/axiom-registry-v1.json",
                            "json_required_top_level_fields": ["schema", "status"],
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_topological_layer_with_unknown_axis(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (contracts / "axiom-registry-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.axiom_registry.v1",
                        "status": "normative",
                        "functional_layers": [{"id": "semantic_contract"}],
                        "topology_axes": [
                            {
                                "id": "repo",
                                "root_layer": "repo",
                                "owns": ["filesystem"],
                                "must_not_own": ["runtime"],
                            }
                        ],
                        "topological_layers": [{"id": "floating.layer"}],
                        "contract_scales": [
                            {
                                "id": "repo",
                                "projection": "fractal_scale",
                                "description": "filesystem contract scale",
                            }
                        ],
                        "contracts": [
                            {
                                "id": "axiom_registry",
                                "path": "axioms/axiom-registry-v1.json",
                                "family": "governance",
                                "functional_layer": "semantic_contract",
                                "topological_layer": "floating.layer",
                                "scale": "repo",
                                "owner": "organ-supervisor",
                                "merge_policy": "extend_existing_family",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {"contract_file_policy": {"registry_contract": "axioms/axiom-registry-v1.json"}}
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertIn("axiom_registry_topological_layer_axis_unknown", [item.kind for item in findings])

    def test_rejects_topological_layer_with_unknown_parent(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (contracts / "axiom-registry-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.axiom_registry.v1",
                        "status": "normative",
                        "functional_layers": [{"id": "semantic_contract"}],
                        "topology_axes": [
                            {
                                "id": "repo",
                                "root_layer": "repo",
                                "owns": ["filesystem"],
                                "must_not_own": ["runtime"],
                            }
                        ],
                        "topological_layers": [
                            {"id": "repo", "parent": "organism"},
                            {"id": "repo.docs", "parent": "repo"},
                        ],
                        "contract_scales": [
                            {
                                "id": "repo",
                                "projection": "fractal_scale",
                                "description": "filesystem contract scale",
                            }
                        ],
                        "contracts": [
                            {
                                "id": "axiom_registry",
                                "path": "axioms/axiom-registry-v1.json",
                                "family": "governance",
                                "functional_layer": "semantic_contract",
                                "topological_layer": "repo",
                                "scale": "repo",
                                "owner": "organ-supervisor",
                                "merge_policy": "extend_existing_family",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {"contract_file_policy": {"registry_contract": "axioms/axiom-registry-v1.json"}}
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertIn("axiom_registry_topological_layer_parent_unknown", [item.kind for item in findings])

    def test_rejects_organ_layer_with_unknown_parent_layer(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            organs = root / "organs"
            (organs / "contracts").mkdir(parents=True)
            (organs / "contracts" / "MANIFEST.yaml").write_text(
                "name: contracts\nstatus: canonical\ndescription: test\n",
                encoding="utf-8",
            )
            contracts.mkdir(parents=True)
            (contracts / "organ-layer-topology-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.organ_layer_topology.v1",
                        "status": "normative",
                        "authorities": {
                            "human_readable_policy": "docs/foundation/CYNIC_REPO_TOPOLOGY_POLICY.md",
                            "policy_matrix": "docs/foundation/CYNIC_ORGANISM_POLICY_MATRIX.md",
                            "organ_maturity_inventory": "axioms/organ-maturity-inventory-v1.json",
                        },
                        "symbolic_vocabulary": {
                            "Keter": "source intent",
                            "Binah": "bounded form",
                        },
                        "layer_policy": {
                            "ordering": "ascending_abstraction",
                            "rank_meaning": "test",
                            "required_fields": [
                                "id",
                                "rank",
                                "name",
                                "responsibility",
                                "system_primitives",
                                "symbolic_alignment",
                                "must_not_own",
                                "canonical_examples",
                            ],
                        },
                        "layers": [
                            {
                                "id": "foundational_contract",
                                "rank": 10,
                                "name": "Foundational contract",
                                "responsibility": "test",
                                "system_primitives": ["json schema", "contract validation", "git object"],
                                "symbolic_alignment": ["Keter", "Binah"],
                                "must_not_own": ["runtime_state"],
                                "canonical_examples": ["organs/contracts/"],
                                "parent_layer": None,
                                "projection_kind": "fractal_rank",
                            },
                            {
                                "id": "governance_control_plane",
                                "rank": 20,
                                "name": "Governance control plane",
                                "responsibility": "test",
                                "system_primitives": ["lease", "claim table", "policy evaluator"],
                                "symbolic_alignment": ["Keter", "Binah"],
                                "must_not_own": ["runtime_state"],
                                "canonical_examples": ["organs/contracts/"],
                                "parent_layer": "imaginary",
                                "projection_kind": "fractal_rank",
                            },
                        ],
                        "organs": [
                            {
                                "path": "organs/contracts/",
                                "layer_id": "foundational_contract",
                                "layer_rank": 10,
                                "owner": "organ-contracts",
                                "why": "test",
                                "evidence": ["MANIFEST.yaml"],
                            }
                        ],
                        "totals": {"layers": 2, "organs": 1, "by_layer": {"foundational_contract": 1}},
                    }
                ),
                encoding="utf-8",
            )
            from organs.supervisor.src.core.organ_layer_topology import OrganLayerTopologyService

            service = OrganLayerTopologyService(
                workspace_root=root,
                contract_path=contracts / "organ-layer-topology-v1.json",
                organs_root=organs,
                load_json=module.load_json,
                relative=lambda path: str(Path(path).relative_to(root)),
                finding=module.Finding,
            )

            findings = service.validate()

        self.assertIn("organ_layer_parent_unknown", [item.kind for item in findings])

    def test_rejects_axiom_registry_without_contract_scales(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (contracts / "axiom-registry-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.axiom_registry.v1",
                        "status": "normative",
                        "functional_layers": [{"id": "semantic_contract"}],
                        "topological_layers": [{"id": "repo"}],
                        "contracts": [
                            {
                                "id": "axiom_registry",
                                "path": "axioms/axiom-registry-v1.json",
                                "family": "governance",
                                "functional_layer": "semantic_contract",
                                "topological_layer": "repo",
                                "scale": "repo",
                                "owner": "organ-supervisor",
                                "merge_policy": "extend_existing_family",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {"contract_file_policy": {"registry_contract": "axioms/axiom-registry-v1.json"}}
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertIn("axiom_registry_contract_scales_missing", [item.kind for item in findings])

    def test_rejects_axiom_contract_with_unknown_scale(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir(parents=True)
            (contracts / "axiom-registry-v1.json").write_text(
                json.dumps(
                    {
                        "schema": "cynic.axiom_registry.v1",
                        "status": "normative",
                        "functional_layers": [{"id": "semantic_contract"}],
                        "topological_layers": [{"id": "repo"}],
                        "contract_scales": [
                            {
                                "id": "repo",
                                "projection": "fractal_scale",
                                "description": "filesystem contract scale",
                            }
                        ],
                        "contracts": [
                            {
                                "id": "axiom_registry",
                                "path": "axioms/axiom-registry-v1.json",
                                "family": "governance",
                                "functional_layer": "semantic_contract",
                                "topological_layer": "repo",
                                "scale": "imaginary_scale",
                                "owner": "organ-supervisor",
                                "merge_policy": "extend_existing_family",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.CONTRACTS_ROOT)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACTS_ROOT = contracts
                findings = module.validate_contract_file_policy(
                    {"contract_file_policy": {"registry_contract": "axioms/axiom-registry-v1.json"}}
                )
            finally:
                module.WORKSPACE_ROOT, module.CONTRACTS_ROOT = old_values

        self.assertIn("axiom_registry_contract_scale_unknown", [item.kind for item in findings])
