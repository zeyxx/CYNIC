import unittest
from pathlib import Path

try:
    from ._test_utils import load_module_from_path
    from .contract_authority_fixture import validate_complete_contract_authority_fixture
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path
    from contract_authority_fixture import validate_complete_contract_authority_fixture


MODULE_PATH = Path(__file__).resolve().parent.parent / "controllers" / "validate_repo_topology.py"


def load_module():
    return load_module_from_path("validate_repo_topology", MODULE_PATH)


class ContractAuthorityTopologyTests(unittest.TestCase):
    def test_validates_contract_authorities_and_mcp_paths(self):
        findings = validate_complete_contract_authority_fixture(load_module())

        self.assertEqual([], [item for item in findings if item.severity == "critical"])
