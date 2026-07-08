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


def organ_maturity_transition_policy(*statuses):
    return {
        status: {
            "owner": "organ-supervisor",
            "rule": f"test {status} rule",
            "promotes_to": "canonical",
            "exit_condition": f"test {status} exit",
        }
        for status in statuses
    }


def organ_maturity_order():
    return [
        "burn_candidate",
        "legacy_blocked",
        "quarantined",
        "embryonic",
        "transitional",
        "canonical_partial",
        "canonical",
    ]


def write_python_organ_authority_fixture(
    root: Path,
    *,
    inventory_organs: list[dict[str, object]],
    vital_groups: list[dict[str, object]] | None = None,
) -> Path:
    contracts = root / "axioms"
    contracts.mkdir(exist_ok=True)
    (root / "organs" / "fleet" / "src" / "core").mkdir(parents=True, exist_ok=True)
    (root / "organs" / "browser" / "src" / "core").mkdir(parents=True, exist_ok=True)
    (root / "organs" / "fleet" / "src" / "core" / "network.py").write_text("VALUE = 1\n", encoding="utf-8")
    (root / "organs" / "browser" / "src" / "core" / "page.py").write_text("VALUE = 1\n", encoding="utf-8")
    (contracts / "rust-crate-roles-v1.json").write_text(
        json.dumps(
            {
                "schema": "cynic.rust_crate_roles.v1",
                "crates": [
                    {"path": "crates/cynic-node"},
                    {"path": "crates/cynic-mcp"},
                    {"path": "crates/cynic-kernel"},
                ],
            }
        ),
        encoding="utf-8",
    )
    (contracts / "python-vital-function-placement-v1.json").write_text(
        json.dumps(
            {
                "schema": "cynic.python_vital_function_placement.v1",
                "function_groups": vital_groups
                if vital_groups is not None
                else [{"id": "fleet_network_contracts", "organ": "organ-fleet"}],
            }
        ),
        encoding="utf-8",
    )
    inventory_path = contracts / "python-organ-authority-inventory-v1.json"
    inventory_path.write_text(
        json.dumps(
            {
                "schema": "cynic.python_organ_authority_inventory.v1",
                "allowed_classifications": [
                    "python_membrane",
                    "python_transitional_vital",
                    "python_legacy_duplication",
                ],
                "organs": inventory_organs,
                "totals": {
                    "organs": len(inventory_organs),
                    "by_classification": {
                        "python_membrane": sum(
                            1 for item in inventory_organs if item["classification"] == "python_membrane"
                        ),
                        "python_transitional_vital": sum(
                            1 for item in inventory_organs if item["classification"] == "python_transitional_vital"
                        ),
                        "python_legacy_duplication": sum(
                            1 for item in inventory_organs if item["classification"] == "python_legacy_duplication"
                        ),
                    },
                },
            }
        ),
        encoding="utf-8",
    )
    return inventory_path


class RootAndMaturityTopologyTests(unittest.TestCase):
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

    def test_rejects_ignored_top_level_root_visible_to_git(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / ".git").mkdir()
            (root / "axioms").mkdir()
            (root / "package").mkdir()
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
                        "top_level_root_policy": {"ignored_roots": ["package"]},
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.CONTRACT_PATH, module.git_ls_untracked)
            try:
                module.WORKSPACE_ROOT = root
                module.CONTRACT_PATH = contract_path
                module.git_ls_untracked = lambda *args: ["package/package.json"]
                findings = module.validate_topology_contract()
            finally:
                module.WORKSPACE_ROOT, module.CONTRACT_PATH, module.git_ls_untracked = old_values

        self.assertIn("topology_ignored_root_visible_untracked", [item.kind for item in findings])

    def test_rejects_missing_organ_maturity_policy(self):
        module = load_module()
        findings = module.validate_organ_maturity_policy({})
        self.assertEqual("organ_maturity_policy_missing", findings[0].kind)

    def test_rejects_unknown_organ_maturity_status(self):
        module = load_module()
        findings = module.validate_organ_maturity_policy(
            {
                "organ_maturity_policy": {
                    "allowed_statuses": ["canonical", "embryonic"],
                    "status_meanings": {"canonical": "ok", "embryonic": "seed"},
                    "minimum_exception_fields": ["path"],
                    "shape_exception_fields": ["path"],
                    "rule": "known statuses only",
                },
                "organ_boundary_exceptions": [{"path": "organs/ghost/", "status": "improvised"}],
            }
        )
        self.assertEqual("organ_maturity_status_unknown", findings[0].kind)

    def test_allows_declared_embryonic_organ_maturity_status(self):
        module = load_module()
        findings = module.validate_organ_maturity_policy(
            {
                "organ_maturity_policy": {
                    "allowed_statuses": ["canonical", "embryonic"],
                    "status_meanings": {"canonical": "ok", "embryonic": "seed"},
                    "minimum_exception_fields": ["path"],
                    "shape_exception_fields": ["path"],
                    "rule": "known statuses only",
                },
                "organ_boundary_exceptions": [{"path": "organs/ghost/", "status": "embryonic"}],
            }
        )
        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_missing_organ_maturity_inventory_entry(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "ghost").mkdir(parents=True)
            inventory_path = contracts / "organ-maturity-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organ_maturity_inventory.v1",
                        "status": "draft",
                        "maturity_order": organ_maturity_order(),
                        "organs": [],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.ORGAN_MATURITY_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_organ_maturity_inventory_contract({"organ_maturity_policy": {"allowed_statuses": ["canonical"]}})
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked = old_values

        self.assertIn("organ_missing_maturity_inventory", [item.kind for item in findings])

    def test_allows_complete_organ_maturity_inventory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "ghost").mkdir(parents=True)
            inventory_path = contracts / "organ-maturity-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organ_maturity_inventory.v1",
                        "status": "draft",
                        "maturity_order": organ_maturity_order(),
                        "maturity_transition_policy": organ_maturity_transition_policy("canonical_partial"),
                        "organs": [
                            {"path": "organs/ghost/", "maturity": "canonical", "evidence": ["MANIFEST.yaml"], "next_action": "extend through boundary"}
                        ],
                        "totals": {"organs": 1, "by_maturity": {"canonical": 1}},
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.ORGAN_MATURITY_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_organ_maturity_inventory_contract({"organ_maturity_policy": {"allowed_statuses": ["canonical"]}})
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_out_of_order_organ_maturity_inventory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "ghost").mkdir(parents=True)
            inventory_path = contracts / "organ-maturity-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organ_maturity_inventory.v1",
                        "status": "draft",
                        "maturity_order": [
                            "canonical",
                            "canonical_partial",
                            "transitional",
                            "embryonic",
                            "quarantined",
                            "legacy_blocked",
                            "burn_candidate",
                        ],
                        "maturity_transition_policy": organ_maturity_transition_policy("canonical_partial"),
                        "organs": [
                            {"path": "organs/ghost/", "maturity": "canonical", "evidence": ["MANIFEST.yaml"], "next_action": "extend through boundary"}
                        ],
                        "totals": {"organs": 1, "by_maturity": {"canonical": 1}},
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.ORGAN_MATURITY_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_organ_maturity_inventory_contract({"organ_maturity_policy": {"allowed_statuses": ["canonical"]}})
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked = old_values

        self.assertIn("organ_maturity_inventory_order_invalid_sequence", [item.kind for item in findings])

    def test_rejects_canonical_organ_with_untracked_files(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            organ = root / "organs" / "ghost"
            organ.mkdir(parents=True)
            (organ / "MANIFEST.yaml").write_text("id: ghost\n", encoding="utf-8")
            (organ / "local.py").write_text("VALUE = 1\n", encoding="utf-8")
            inventory_path = contracts / "organ-maturity-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organ_maturity_inventory.v1",
                        "status": "draft",
                        "maturity_order": organ_maturity_order(),
                        "maturity_transition_policy": organ_maturity_transition_policy("canonical_partial"),
                        "organs": [{"path": "organs/ghost/", "maturity": "canonical", "evidence": ["MANIFEST.yaml"], "next_action": "extend through boundary"}],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.ORGAN_MATURITY_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: ["organs/ghost/local.py"]
                findings = module.validate_organ_maturity_inventory_contract({"organ_maturity_policy": {"allowed_statuses": ["canonical", "canonical_partial"]}})
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked = old_values

        self.assertIn("canonical_organ_has_untracked_files", [item.kind for item in findings])

    def test_rejects_untracked_organ_without_maturity_evidence(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            organ = root / "organs" / "ghost"
            organ.mkdir(parents=True)
            inventory_path = contracts / "organ-maturity-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organ_maturity_inventory.v1",
                        "status": "draft",
                        "maturity_order": organ_maturity_order(),
                        "organs": [{"path": "organs/ghost/", "maturity": "canonical_partial", "evidence": ["MANIFEST.yaml"], "next_action": "track or quarantine local files"}],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.ORGAN_MATURITY_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: ["organs/ghost/local.py"]
                findings = module.validate_organ_maturity_inventory_contract({"organ_maturity_policy": {"allowed_statuses": ["canonical_partial"]}})
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked = old_values

        self.assertIn("organ_untracked_body_missing_maturity_evidence", [item.kind for item in findings])

    def test_allows_untracked_organ_with_maturity_evidence(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            organ = root / "organs" / "ghost"
            organ.mkdir(parents=True)
            inventory_path = contracts / "organ-maturity-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organ_maturity_inventory.v1",
                        "status": "draft",
                        "maturity_order": organ_maturity_order(),
                        "organs": [{"path": "organs/ghost/", "maturity": "canonical_partial", "evidence": ["MANIFEST.yaml", "untracked organ body files"], "next_action": "track or quarantine local files"}],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.ORGAN_MATURITY_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: ["organs/ghost/local.py"]
                findings = module.validate_organ_maturity_inventory_contract({"organ_maturity_policy": {"allowed_statuses": ["canonical_partial"]}})
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked = old_values

        self.assertEqual([], [item for item in findings if item.kind == "organ_untracked_body_missing_maturity_evidence"])

    def test_rejects_stale_shape_exception_evidence(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            organ = root / "organs" / "ghost"
            organ.mkdir(parents=True)
            inventory_path = contracts / "organ-maturity-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organ_maturity_inventory.v1",
                        "status": "draft",
                        "maturity_order": organ_maturity_order(),
                        "organs": [
                            {
                                "path": "organs/ghost/",
                                "maturity": "canonical_partial",
                                "evidence": ["MANIFEST.yaml", "organ_shape_exceptions"],
                                "next_action": "remove stale exception evidence",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.ORGAN_MATURITY_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_organ_maturity_inventory_contract(
                    {
                        "organ_maturity_policy": {"allowed_statuses": ["canonical_partial"]},
                        "organ_shape_exceptions": [],
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked = old_values

        self.assertIn("organ_maturity_inventory_stale_shape_exception_evidence", [item.kind for item in findings])

    def test_rejects_non_terminal_organ_maturity_without_transition_policy(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "ghost").mkdir(parents=True)
            inventory_path = contracts / "organ-maturity-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organ_maturity_inventory.v1",
                        "status": "draft",
                        "maturity_order": organ_maturity_order(),
                        "organs": [{"path": "organs/ghost/", "maturity": "canonical_partial", "evidence": ["MANIFEST.yaml"], "next_action": "track or quarantine local files"}],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.ORGAN_MATURITY_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_organ_maturity_inventory_contract({"organ_maturity_policy": {"allowed_statuses": ["canonical_partial"]}})
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked = old_values

        self.assertIn("organ_maturity_inventory_transition_policy_missing", [item.kind for item in findings])

    def test_rejects_unknown_organ_maturity_inventory_status(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contracts = root / "axioms"
            contracts.mkdir()
            (root / "organs" / "ghost").mkdir(parents=True)
            inventory_path = contracts / "organ-maturity-inventory-v1.json"
            inventory_path.write_text(
                json.dumps(
                    {
                        "schema": "cynic.organ_maturity_inventory.v1",
                        "status": "draft",
                        "maturity_order": organ_maturity_order(),
                        "organs": [{"path": "organs/ghost/", "maturity": "improvised", "evidence": ["MANIFEST.yaml"], "next_action": "unknown"}],
                    }
                ),
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.ORGAN_MATURITY_INVENTORY_PATH = inventory_path
                module.git_ls_untracked = lambda *args: []
                findings = module.validate_organ_maturity_inventory_contract({"organ_maturity_policy": {"allowed_statuses": ["canonical"]}})
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.ORGAN_MATURITY_INVENTORY_PATH, module.git_ls_untracked = old_values

        self.assertIn("organ_maturity_inventory_status_unknown", [item.kind for item in findings])

    def test_allows_valid_python_organ_authority_inventory(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            inventory_path = write_python_organ_authority_fixture(
                root,
                inventory_organs=[
                    {
                        "path": "organs/fleet/",
                        "classification": "python_transitional_vital",
                        "rust_authority_targets": ["crates/cynic-node"],
                        "reason": "fleet network authority is converging into cynic-node",
                    },
                    {
                        "path": "organs/browser/",
                        "classification": "python_membrane",
                        "rust_authority_targets": [],
                        "reason": "browser remains an adapter membrane",
                    },
                ],
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH = inventory_path
                findings = module.validate_python_organ_authority_inventory_contract()
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_python_vital_organ_misclassified_as_membrane(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            inventory_path = write_python_organ_authority_fixture(
                root,
                inventory_organs=[
                    {
                        "path": "organs/fleet/",
                        "classification": "python_membrane",
                        "rust_authority_targets": ["crates/cynic-node"],
                        "reason": "bad classification for a vital organ",
                    },
                    {
                        "path": "organs/browser/",
                        "classification": "python_membrane",
                        "rust_authority_targets": [],
                        "reason": "browser remains an adapter membrane",
                    },
                ],
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH = inventory_path
                findings = module.validate_python_organ_authority_inventory_contract()
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH = old_values

        self.assertIn("python_organ_authority_vital_organ_misclassified", [item.kind for item in findings])

    def test_rejects_unknown_python_organ_rust_authority_target(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            inventory_path = write_python_organ_authority_fixture(
                root,
                inventory_organs=[
                    {
                        "path": "organs/fleet/",
                        "classification": "python_transitional_vital",
                        "rust_authority_targets": ["crates/cynic-ghost"],
                        "reason": "unknown target should be rejected",
                    },
                    {
                        "path": "organs/browser/",
                        "classification": "python_membrane",
                        "rust_authority_targets": [],
                        "reason": "browser remains an adapter membrane",
                    },
                ],
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH = inventory_path
                findings = module.validate_python_organ_authority_inventory_contract()
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH = old_values

        self.assertIn("python_organ_authority_target_unknown", [item.kind for item in findings])

    def test_rejects_python_organ_authority_total_mismatch(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            inventory_path = write_python_organ_authority_fixture(
                root,
                inventory_organs=[
                    {
                        "path": "organs/fleet/",
                        "classification": "python_transitional_vital",
                        "rust_authority_targets": ["crates/cynic-node"],
                        "reason": "fleet network authority is converging into cynic-node",
                    },
                    {
                        "path": "organs/browser/",
                        "classification": "python_membrane",
                        "rust_authority_targets": [],
                        "reason": "browser remains an adapter membrane",
                    },
                ],
            )
            payload = json.loads(inventory_path.read_text(encoding="utf-8"))
            payload["totals"]["organs"] = 99
            inventory_path.write_text(json.dumps(payload), encoding="utf-8")
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH = inventory_path
                findings = module.validate_python_organ_authority_inventory_contract()
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH = old_values

        self.assertIn("python_organ_authority_total_mismatch", [item.kind for item in findings])

    def test_rejects_active_logic_inside_python_legacy_duplication_organ(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            inventory_path = write_python_organ_authority_fixture(
                root,
                inventory_organs=[
                    {
                        "path": "organs/fleet/",
                        "classification": "python_transitional_vital",
                        "rust_authority_targets": ["crates/cynic-node"],
                        "reason": "fleet network authority is converging into cynic-node",
                    },
                    {
                        "path": "organs/browser/",
                        "classification": "python_legacy_duplication",
                        "rust_authority_targets": ["crates/cynic-node"],
                        "reason": "legacy organ should not keep active logic",
                    },
                ],
            )
            (root / "organs" / "browser" / "src" / "controllers").mkdir(parents=True, exist_ok=True)
            (root / "organs" / "browser" / "src" / "controllers" / "mesh_agent.py").write_text(
                "VALUE = 1\n",
                encoding="utf-8",
            )
            old_values = (module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH)
            try:
                module.WORKSPACE_ROOT = root
                module.ORGANS_ROOT = root / "organs"
                module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH = inventory_path
                findings = module.validate_python_organ_authority_inventory_contract()
            finally:
                module.WORKSPACE_ROOT, module.ORGANS_ROOT, module.PYTHON_ORGAN_AUTHORITY_INVENTORY_PATH = old_values

        self.assertIn("python_legacy_duplication_active_logic_present", [item.kind for item in findings])


if __name__ == "__main__":
    unittest.main()
