#!/usr/bin/env python3
import unittest

from pathlib import Path

try:
    from ._test_utils import load_module_from_path, resolve_project_root
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path, resolve_project_root


ROOT = resolve_project_root(Path(__file__).resolve())
SHARED_MODULE_PATH = ROOT / "organs" / "contracts" / "src" / "surface_contracts.py"
SHIM_MODULE_PATH = Path(__file__).resolve().parent.parent / "src" / "core" / "surface_contracts.py"


class SurfaceContractsTests(unittest.TestCase):
    def test_supervisor_shim_reexports_shared_contracts(self):
        shared = load_module_from_path("shared_surface_contracts", SHARED_MODULE_PATH)
        shim = load_module_from_path("supervisor_surface_contracts", SHIM_MODULE_PATH)

        self.assertEqual(shared.ReportSurfaceSpec("cynic.example.report.v1", "cynic example").as_manifest(), shim.ReportSurfaceSpec("cynic.example.report.v1", "cynic example").as_manifest())
        self.assertEqual(shared.CommandSurfaceSpec("status", lambda _args: 0).as_manifest(), shim.CommandSurfaceSpec("status", lambda _args: 0).as_manifest())
        self.assertEqual(shared.SurfaceRegistry(reports=(shared.ReportSurfaceSpec("cynic.example.report.v1", "cynic example"),)).manifest(), shim.SurfaceRegistry(reports=(shim.ReportSurfaceSpec("cynic.example.report.v1", "cynic example"),)).manifest())
        self.assertEqual(shared.attach_surface_contract({"status": "ok"}, shared.ReportSurfaceSpec("cynic.example.report.v1", "cynic example")), shim.attach_surface_contract({"status": "ok"}, shim.ReportSurfaceSpec("cynic.example.report.v1", "cynic example")))


if __name__ == "__main__":
    unittest.main()
