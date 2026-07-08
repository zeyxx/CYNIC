import json
import tempfile
import unittest
from pathlib import Path
from ._test_utils import resolve_project_root


AXIOM_PATH = resolve_project_root(Path(__file__).resolve()) / "axioms" / "multiscale-organism-property-map-v1.json"


class MultiscaleOrganismPropertyMapTests(unittest.TestCase):
    def load_contract(self):
        return json.loads(AXIOM_PATH.read_text(encoding="utf-8"))

    def test_contract_is_well_formed(self):
        contract = self.load_contract()

        self.assertEqual("cynic.multiscale_organism_property_map.v1", contract["schema"])
        self.assertEqual("draft", contract["status"])
        self.assertEqual("axioms/meta-layer-contract-v1.json", contract["meta_contract"])
        self.assertIn("planck_limit", contract["scale_order"])
        self.assertIn("identity", contract["property_nucleus"])
        self.assertGreaterEqual(len(contract["evidence_sources"]), 1)

    def test_each_scale_has_empirical_projection(self):
        contract = self.load_contract()

        for scale in contract["scales"]:
            with self.subTest(scale=scale["name"]):
                representation = scale["representation"]
                self.assertIn("projection_kind", scale)
                if scale["name"] == "organism":
                    self.assertIsNone(scale["parent_scale"])
                else:
                    self.assertIn("parent_scale", scale)
                self.assertIn("identity", representation)
                self.assertIn("boundary", representation)
                self.assertIn("state", representation)
                self.assertIn("energy", representation)
                self.assertIn("flux", representation)
                self.assertIn("memory", representation)
                self.assertIn("observation", representation)
                self.assertIn("action", representation)
                self.assertIn("dependency", representation)
                self.assertIn("invariant", representation)
                self.assertIn("failure_mode", representation)
                self.assertGreaterEqual(len(scale["evidence"]), 1)

    def test_planck_limit_is_description_only(self):
        contract = self.load_contract()
        limit = next(scale for scale in contract["scales"] if scale["name"] == "planck_limit")

        self.assertEqual("description_limit_only", contract["validity_rule"]["limit_semantics"])
        self.assertEqual("none", limit["representation"]["action"])
        self.assertEqual("physical_system", limit["parent_scale"])
        self.assertIn("category error", limit["representation"]["failure_mode"])

    def test_rejects_scale_with_unknown_parent(self):
        from organs.supervisor.src.core.multiscale_organism_property_map import (
            MultiscaleOrganismPropertyMapService,
        )
        from organs.supervisor.src.core.repo_topology_host_validation import Finding

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            contract_path = root / "axioms" / "multiscale-organism-property-map-v1.json"
            contract_path.parent.mkdir(parents=True)
            contract = self.load_contract()
            contract["scales"][1]["parent_scale"] = "imaginary"
            contract_path.write_text(json.dumps(contract), encoding="utf-8")

            service = MultiscaleOrganismPropertyMapService(
                workspace_root=root,
                contract_path=contract_path,
                load_json=lambda path: json.loads(path.read_text(encoding="utf-8")),
                relative=lambda path: str(Path(path).relative_to(root)),
                finding=Finding,
            )
            kinds = [item.kind for item in service.validate()]

        self.assertIn("multiscale_organism_property_map_parent_scale_unknown", kinds)


if __name__ == "__main__":
    unittest.main()
