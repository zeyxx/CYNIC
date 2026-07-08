from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from organs.supervisor.src.core.organ_crate_boundary_audit import build_organ_crate_boundary_audit


class OrganCrateBoundaryAuditTests(unittest.TestCase):
    def test_prioritizes_supervisor_rust_absorption_and_organ_moves(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organs = root / "organs"
            organs.mkdir()
            for name in ["supervisor", "solana", "percolator", "twitter", "contracts"]:
                organ = organs / name
                (organ / "src" / "core").mkdir(parents=True)
                (organ / "src" / "adapters").mkdir(parents=True)
                (organ / "src" / "entrypoints").mkdir(parents=True)
                (organ / "MANIFEST.yaml").write_text(
                    f"id: organ-{name}\ndomain: {name}\nstatus: active\n",
                    encoding="utf-8",
                )

            supervisor_core = organs / "supervisor" / "src" / "core"
            (supervisor_core / "surface_contracts.py").write_text(
                "def validate():\n    schema = 'cynic.surface_contracts.v1'\n    return []\n",
                encoding="utf-8",
            )
            (root / "crates" / "not-rust").mkdir(parents=True)

            report = build_organ_crate_boundary_audit(root)

        self.assertEqual(report["schema"], "cynic.organ_crate_boundary_audit.v1")
        actions = report["actions"]
        self.assertTrue(any(item["target"] == "organs/percolator/" for item in actions))
        self.assertTrue(any(item["destination"] == "crates/cynic-mcp" for item in actions))
        self.assertTrue(any(item["target"] == "crates/not-rust" for item in actions))
        self.assertTrue(any(item["target"] == "organs/percolator/" for item in report["first_wave"]))


if __name__ == "__main__":
    unittest.main()
