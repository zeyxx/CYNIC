from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from organs.supervisor.src.core.odysseum_boundary_audit import build_odysseum_boundary_audit


class OdysseumBoundaryAuditTests(unittest.TestCase):
    def test_maps_public_scope_inflation_and_percolator_mix(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "docs/foundation").mkdir(parents=True)
            (root / "docs/internal").mkdir(parents=True)
            (root / "packages/odysseum-ui/src").mkdir(parents=True)
            (root / "organs/percolator/runbooks").mkdir(parents=True)
            (root / "axioms").mkdir(parents=True)

            (root / "docs/foundation/ODYSSEUM_MANIFEST.md").write_text(
                "Odysseum is a narrow standard.\n",
                encoding="utf-8",
            )
            (root / "axioms/odysseum-manifest-v1.json").write_text(
                '{"purpose":"Stocks on Solana and Options on Solana"}',
                encoding="utf-8",
            )
            (root / "packages/odysseum-ui/src/Home.tsx").write_text(
                "export const label = 'Options on Solana';\n",
                encoding="utf-8",
            )
            (root / "organs/percolator/runbooks/pumpswap_pitch_metrics_exact.sql").write_text(
                "-- ODYSSEUM ARR B2B\n",
                encoding="utf-8",
            )

            report = build_odysseum_boundary_audit(root)

        self.assertEqual(report["schema"], "cynic.odysseum_boundary_audit.v1")
        kinds = {finding["kind"] for finding in report["findings"]}
        self.assertIn("axiom_exceeds_manifest_authority", kinds)
        self.assertIn("odysseum_scope_inflation", kinds)
        self.assertIn("percolator_odysseum_mix", kinds)


if __name__ == "__main__":
    unittest.main()
