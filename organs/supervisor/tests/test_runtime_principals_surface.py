#!/usr/bin/env python3
import unittest

from pathlib import Path

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "src" / "entrypoints" / "runtime_principals_views_surface.py"


class RuntimePrincipalsSurfaceTests(unittest.TestCase):
    def test_surface_module_exposes_registry(self):
        module = load_module_from_path("runtime_principals_views_surface", MODULE_PATH)

        registry = module.INFRA_SURFACE_REGISTRY.manifest()
        self.assertEqual("cynic.surface_registry.v1", registry["schema"])
        self.assertEqual(4, registry["report_count"])
        self.assertEqual(
            ["cynic bridge status", "cynic fleet status", "cynic mesh status", "cynic storage status"],
            module.INFRA_SURFACE_REGISTRY.report_names(),
        )
        self.assertEqual("cynic storage status", registry["reports"][3]["command"])

    def test_surface_reports_carry_contract_manifests(self):
        module = load_module_from_path("runtime_principals_views_surface", MODULE_PATH)
        globals_ref = module.storage_primary_status.__globals__
        original_run = globals_ref["_run_capture"]

        def fake_run(*_args, **_kwargs):
            return {
                "status": "ok",
                "exit_code": 0,
                "stdout": '{"schema":"cynic.storage_primary_activation.status.v1","status":"active"}',
                "stderr": "",
            }

        globals_ref["_run_capture"] = fake_run
        try:
            report = module.storage_primary_status([])
        finally:
            globals_ref["_run_capture"] = original_run

        self.assertEqual("cynic.storage_primary_activation.status.v1", report["surface_contract"]["schema"])
        self.assertEqual("cynic storage status", report["surface_contract"]["command"])


if __name__ == "__main__":
    unittest.main()
