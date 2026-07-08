import tempfile
import unittest
from pathlib import Path

from organs.supervisor.src.core import infra_consumption_contract as module


class InfraConsumptionContractTests(unittest.TestCase):
    def test_gate_0_enforces_infra_consumption_contract(self):
        makefile = (module.PROJECT_ROOT / "Makefile").read_text(encoding="utf-8")
        gate_0 = makefile.split(".PHONY: gate-1", 1)[0]

        self.assertIn("lint-infra-consumption-contract", gate_0)

    def test_storage_check_does_not_document_local_surrealdb_endpoint(self):
        makefile = (module.PROJECT_ROOT / "Makefile").read_text(encoding="utf-8")
        check_storage = makefile.split(".PHONY: check-storage", 1)[1].split(".PHONY:", 1)[0]

        self.assertNotIn("requires :8000", check_storage)
        self.assertNotIn("localhost:8000", check_storage)

    def test_contract_is_normative_and_declares_signs_of_life(self):
        findings = module.validate_infra_consumption_contract()
        critical = [item for item in findings if item.severity == "critical"]

        self.assertEqual([], critical)
        signs = module.organism_signs_of_life_contract()
        self.assertEqual("cynic organism", signs["human_command"])
        self.assertIn("local cortex validation", signs["must_report"])
        self.assertIn("storage readiness", signs["must_report"])

    def test_declared_cortex_forbidden_rules_are_covered_by_patterns(self):
        contract = module.load_contract()
        patterns = module._compiled_forbidden_patterns(contract)

        for forbidden in contract["native_lifecycle"]["forbidden_on_cortex"]:
            self.assertTrue(
                any(pattern.search(forbidden) for _, pattern in patterns),
                f"{forbidden!r} is not covered by forbidden_patterns",
            )

    def test_rejects_uncovered_forbidden_on_cortex_rule(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contract_path = root / "infra-consumption-v1.json"
            scanned = root / "Makefile"
            scanned.write_text("", encoding="utf-8")
            contract_path.write_text(
                """
{
  "schema": "cynic.infra_consumption.v1",
  "status": "normative",
  "capability_roles": ["build_runner"],
  "required_ports": ["LifecycleDispatchPort"],
  "native_lifecycle": {"forbidden_on_cortex": ["rsync target/release/cynic-kernel"]},
  "service_placement": {"surrealdb": {"role": "storage_primary"}},
  "scan_scope": ["Makefile"],
  "forbidden_patterns": [
    {
      "id": "cortex_local_mcp_build",
      "pattern": "cargo build -p cynic-mcp",
      "severity": "critical",
      "detail": "MCP builds must be lifecycle-dispatched."
    }
  ]
}
""",
                encoding="utf-8",
            )
            old_root = module.PROJECT_ROOT
            try:
                module.PROJECT_ROOT = root
                findings = module.validate_infra_consumption_contract(contract_path)
            finally:
                module.PROJECT_ROOT = old_root

        self.assertIn(
            "infra_consumption_forbidden_on_cortex_uncovered",
            [item.kind for item in findings],
        )

    def test_rejects_forbidden_cortex_local_mcp_build(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contract_path = root / "infra-consumption-v1.json"
            scanned = root / "wrapper.sh"
            scanned.write_text("cargo build -p cynic-mcp --release\n", encoding="utf-8")
            contract_path.write_text(
                """
{
  "schema": "cynic.infra_consumption.v1",
  "status": "normative",
  "capability_roles": ["build_runner"],
  "required_ports": ["LifecycleDispatchPort"],
  "native_lifecycle": {"forbidden_on_cortex": ["cargo build -p cynic-mcp"]},
  "service_placement": {"surrealdb": {"role": "storage_primary"}},
  "scan_scope": ["wrapper.sh"],
  "forbidden_patterns": [
    {
      "id": "cortex_local_mcp_build",
      "pattern": "cargo build -p cynic-mcp",
      "severity": "critical",
      "detail": "MCP builds must be lifecycle-dispatched."
    }
  ]
}
""",
                encoding="utf-8",
            )
            old_root = module.PROJECT_ROOT
            try:
                module.PROJECT_ROOT = root
                findings = module.validate_infra_consumption_contract(contract_path)
            finally:
                module.PROJECT_ROOT = old_root

        self.assertIn("infra_consumption_forbidden_cortex_local_mcp_build", [item.kind for item in findings])

    def test_rejects_cortex_local_surrealdb_endpoint(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contract_path = root / "infra-consumption-v1.json"
            scanned = root / "Makefile"
            scanned.write_text("backup:\n\tsurreal export --endpoint http://localhost:8000\n", encoding="utf-8")
            contract_path.write_text(
                """
{
  "schema": "cynic.infra_consumption.v1",
  "status": "normative",
  "capability_roles": ["storage_primary"],
  "required_ports": ["StoragePrimaryPort"],
  "native_lifecycle": {"forbidden_on_cortex": ["surreal --endpoint http://localhost:8000"]},
  "service_placement": {"surrealdb": {"role": "storage_primary"}},
  "scan_scope": ["Makefile"],
  "forbidden_patterns": [
    {
      "id": "local_surrealdb_endpoint",
      "pattern": "surreal (import|sql|export|is-ready) --endpoint http://localhost:8000",
      "severity": "critical",
      "detail": "SurrealDB must be consumed through storage_primary/Synapse or Kernel coordination API."
    }
  ]
}
""",
                encoding="utf-8",
            )
            old_root = module.PROJECT_ROOT
            try:
                module.PROJECT_ROOT = root
                findings = module.validate_infra_consumption_contract(contract_path)
            finally:
                module.PROJECT_ROOT = old_root

        self.assertIn("infra_consumption_forbidden_local_surrealdb_endpoint", [item.kind for item in findings])


if __name__ == "__main__":
    unittest.main()
