from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from organs.supervisor.src.core.rust_migration_inventory import build_rust_migration_inventory


class RustMigrationInventoryTests(unittest.TestCase):
    def test_routes_stable_supervisor_contracts_to_existing_crates(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            core = root / "organs" / "supervisor" / "src" / "core"
            controllers = root / "organs" / "supervisor" / "controllers"
            core.mkdir(parents=True)
            controllers.mkdir(parents=True)
            (core / "organ_memory_contract.py").write_text(
                "def validate():\n    schema = 'cynic.organ_memory.v1'\n    return []\n",
                encoding="utf-8",
            )
            (core / "infrastructure_mcp_contract.py").write_text(
                "def validate():\n    schema = 'cynic.infrastructure_mcp.v1'\n    return []\n",
                encoding="utf-8",
            )
            (controllers / "audit_runtime.py").write_text(
                "def main():\n    return 0\n",
                encoding="utf-8",
            )

            report = build_rust_migration_inventory(root)

        self.assertEqual(report["schema"], "cynic.supervisor_rust_migration_inventory.v1")
        self.assertIn(
            "organs/supervisor/src/core/organ_memory_contract.py",
            report["crate_absorption_targets"]["crates/cynic-kernel"],
        )
        self.assertIn(
            "organs/supervisor/src/core/infrastructure_mcp_contract.py",
            report["crate_absorption_targets"]["crates/cynic-mcp"],
        )
        self.assertTrue(
            any(
                item["path"] == "organs/supervisor/controllers/audit_runtime.py"
                for item in report["keep_in_supervisor_python"]
            )
        )


if __name__ == "__main__":
    unittest.main()
