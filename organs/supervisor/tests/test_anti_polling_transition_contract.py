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


MODULE_PATH = Path(__file__).resolve().parent.parent / "src" / "core" / "anti_polling_transition_contract.py"


def load_module():
    return load_module_from_path("anti_polling_transition_contract", MODULE_PATH)


def minimal_contract() -> dict:
    return {
        "schema": "cynic.anti_polling_transition.v1",
        "authorities": {
            "human_readable_policy": "docs/foundation/CYNIC_ANTI_POLLING_POLICY.md",
            "runtime_roles": "docs/foundation/CYNIC_AGENTIC_RUNTIME_ROLES.md",
        },
        "polling_markers": ["poll_interval"],
        "scan_roots": ["services/runtime/src/"],
        "transitional_polling_surfaces": [
            {
                "path": "services/runtime/src/runtime.py",
                "current_mechanism": "interval loop",
                "target_mechanism": "subscription",
                "status": "transitional",
            }
        ],
    }


class AntiPollingTransitionContractTests(unittest.TestCase):
    def test_accepts_minimal_valid_contract(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "docs" / "foundation").mkdir(parents=True)
            (root / "services" / "runtime" / "src").mkdir(parents=True)
            (root / "docs" / "foundation" / "CYNIC_ANTI_POLLING_POLICY.md").write_text("policy\n", encoding="utf-8")
            (root / "docs" / "foundation" / "CYNIC_AGENTIC_RUNTIME_ROLES.md").write_text("roles\n", encoding="utf-8")
            (root / "services" / "runtime" / "src" / "runtime.py").write_text("poll_interval = 1\n", encoding="utf-8")
            contract_path = root / "axioms" / "anti-polling-transition-v1.json"
            contract_path.write_text(json.dumps(minimal_contract()), encoding="utf-8")

            old_root = module.host.WORKSPACE_ROOT
            old_contract = module.CONTRACT_PATH
            try:
                module.host.WORKSPACE_ROOT = root
                module.CONTRACT_PATH = contract_path
                findings = module.validate_anti_polling_transition_contract()
            finally:
                module.host.WORKSPACE_ROOT = old_root
                module.CONTRACT_PATH = old_contract

        self.assertEqual([], findings)

    def test_detects_undeclared_polling_surface(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "axioms").mkdir(parents=True)
            (root / "docs" / "foundation").mkdir(parents=True)
            (root / "services" / "runtime" / "src").mkdir(parents=True)
            (root / "docs" / "foundation" / "CYNIC_ANTI_POLLING_POLICY.md").write_text("policy\n", encoding="utf-8")
            (root / "docs" / "foundation" / "CYNIC_AGENTIC_RUNTIME_ROLES.md").write_text("roles\n", encoding="utf-8")
            (root / "services" / "runtime" / "src" / "runtime.py").write_text("poll_interval = 1\n", encoding="utf-8")
            (root / "services" / "runtime" / "src" / "rogue.py").write_text("poll_interval = 2\n", encoding="utf-8")
            contract_path = root / "axioms" / "anti-polling-transition-v1.json"
            contract_path.write_text(json.dumps(minimal_contract()), encoding="utf-8")

            old_root = module.host.WORKSPACE_ROOT
            old_contract = module.CONTRACT_PATH
            try:
                module.host.WORKSPACE_ROOT = root
                module.CONTRACT_PATH = contract_path
                findings = module.validate_anti_polling_transition_contract()
            finally:
                module.host.WORKSPACE_ROOT = old_root
                module.CONTRACT_PATH = old_contract

        self.assertIn("undeclared_polling_surface", [item.kind for item in findings])
