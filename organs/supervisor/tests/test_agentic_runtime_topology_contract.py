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


MODULE_PATH = Path(__file__).resolve().parent.parent / "src" / "core" / "agentic_runtime_topology_contract.py"


def load_module():
    return load_module_from_path("agentic_runtime_topology_contract", MODULE_PATH)


def minimal_contract() -> dict:
    return {
        "schema": "cynic.agentic_runtime_topology.v1",
        "authorities": {
            "human_readable_policy": "docs/foundation/CYNIC_AGENTIC_RUNTIME_ROLES.md",
            "repo_topology_contract": "axioms/repo-topology-v1.json",
        },
        "agentic_roles": [
            {
                "id": "executor",
                "class": "bounded_work_execution",
                "allowed_responsibilities": ["task_execution"],
                "forbidden_responsibilities": ["policy_authorship"],
            }
        ],
        "runtime_classes": [
            {
                "id": "execution-runtime",
                "class": "shared_runtime_framework",
                "allowed_responsibilities": ["execution_loop"],
                "forbidden_responsibilities": ["domain_logic"],
            }
        ],
        "minimum_runtime_set": ["execution-runtime"],
        "canonical_runtime_root_rules": {"forbidden_shared_runtime_shapes": ["services/**/organs/**"]},
        "current_mapping_pressure": [
            {
                "path": "services/runtime-core/",
                "declared_primary_runtime_class": "execution-runtime",
                "declared_roles": ["executor"],
            }
        ],
    }


class AgenticRuntimeTopologyContractTests(unittest.TestCase):
    def test_accepts_minimal_valid_contract(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "docs" / "foundation").mkdir(parents=True)
            (root / "services" / "runtime-core").mkdir(parents=True)
            (root / "docs" / "foundation" / "CYNIC_AGENTIC_RUNTIME_ROLES.md").write_text("policy\n", encoding="utf-8")
            (root / "axioms" / "repo-topology-v1.json").write_text("{}", encoding="utf-8")
            contract_path = root / "axioms" / "agentic-runtime-topology-v1.json"
            contract_path.write_text(json.dumps(minimal_contract()), encoding="utf-8")

            old_root = module.host.WORKSPACE_ROOT
            old_contract = module.CONTRACT_PATH
            try:
                module.host.WORKSPACE_ROOT = root
                module.CONTRACT_PATH = contract_path
                findings = module.validate_agentic_runtime_topology_contract()
            finally:
                module.host.WORKSPACE_ROOT = old_root
                module.CONTRACT_PATH = old_contract

        self.assertEqual([], findings)

    def test_detects_forbidden_services_organs_shape(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "docs" / "foundation").mkdir(parents=True)
            (root / "services" / "cynic-python" / "organs" / "core").mkdir(parents=True)
            (root / "docs" / "foundation" / "CYNIC_AGENTIC_RUNTIME_ROLES.md").write_text("policy\n", encoding="utf-8")
            (root / "axioms" / "repo-topology-v1.json").write_text("{}", encoding="utf-8")
            contract = minimal_contract()
            contract["current_mapping_pressure"][0]["path"] = "services/ouroboros-runtime/src/ouroboros_runtime/core/"
            contract_path = root / "axioms" / "agentic-runtime-topology-v1.json"
            contract_path.write_text(json.dumps(contract), encoding="utf-8")

            old_root = module.host.WORKSPACE_ROOT
            old_contract = module.CONTRACT_PATH
            try:
                module.host.WORKSPACE_ROOT = root
                module.CONTRACT_PATH = contract_path
                findings = module.validate_agentic_runtime_topology_contract()
            finally:
                module.host.WORKSPACE_ROOT = old_root
                module.CONTRACT_PATH = old_contract

        self.assertIn("forbidden_shared_runtime_shape_present", [item.kind for item in findings])


if __name__ == "__main__":
    unittest.main()
