from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from organs.supervisor.src.core.organ_convergence import build_convergence_report


class OrganConvergenceTests(unittest.TestCase):
    def test_groups_transport_surfaces_for_absorption(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            organs = root / "organs"
            organs.mkdir()
            for name in ["contracts", "twitter", "slack", "solana", "jupyter"]:
                organ = organs / name
                (organ / "src" / "core").mkdir(parents=True)
                (organ / "src" / "adapters").mkdir(parents=True)
                (organ / "src" / "entrypoints").mkdir(parents=True)
                (organ / "MANIFEST.yaml").write_text(
                    f"id: organ-{name}\ndomain: {name}\nstatus: active\n",
                    encoding="utf-8",
                )

            report = build_convergence_report(root)

        self.assertEqual(report["schema"], "cynic.organ_convergence.v1")
        self.assertIn("twitter", report["families"]["communications"])
        self.assertIn("slack", report["families"]["communications"])
        self.assertIn("solana", report["families"]["domain_libraries"])
        self.assertTrue(
            any(item["organ"] == "organs/twitter/" for item in report["actions"]["absorb_or_merge"])
        )
        self.assertTrue(
            any(item["organ"] == "organs/jupyter/" for item in report["actions"]["move_out_of_organs"])
        )


if __name__ == "__main__":
    unittest.main()
