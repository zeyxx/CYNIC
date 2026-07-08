#!/usr/bin/env python3
import tempfile
import unittest
from unittest import mock

from pathlib import Path

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


ORGANISM_MODULE_PATH = Path(__file__).resolve().parent.parent / "src" / "entrypoints" / "runtime_principals_views_organism.py"
FACADE_MODULE_PATH = Path(__file__).resolve().parent.parent / "src" / "entrypoints" / "runtime_principals_views_actions.py"


class RuntimePrincipalsOrganismTests(unittest.TestCase):
    def test_organism_module_exposes_report_surface(self):
        module = load_module_from_path("runtime_principals_views_organism", ORGANISM_MODULE_PATH)
        report = module.organism_actions_report(run_probes=False)

        self.assertEqual("cynic.organism.actions.v1", report["schema"])
        self.assertFalse(report["mutates"])
        self.assertFalse(report["mutates_mempool"])
        self.assertEqual("cynic.organism.actions.v1", report["surface_contract"]["schema"])
        self.assertEqual(0, report["action_count"])
        self.assertEqual("cynic activate", report["dispatch_command"])
        self.assertEqual([], report["actions"])
        self.assertEqual([], report["blockers"])

        text = module.organism_actions_text(report)
        self.assertIn("CYNIC organism actions", text)
        self.assertIn("Pending actions: 0", text)
        registry = module.ORGANISM_SURFACE_REGISTRY.manifest()
        self.assertEqual("cynic.surface_registry.v1", registry["schema"])
        self.assertEqual(3, registry["report_count"])
        self.assertIn("cynic organism", module.ORGANISM_SURFACE_REGISTRY.report_names())
        self.assertIn("cynic converge", module.ORGANISM_SURFACE_REGISTRY.report_names())

    def test_organism_module_exposes_convergence_surface(self):
        module = load_module_from_path("runtime_principals_views_organism", ORGANISM_MODULE_PATH)
        report = module.organism_convergence_report(run_probes=False)

        self.assertEqual("cynic.organism.convergence.v1", report["schema"])
        self.assertFalse(report["mutates"])
        self.assertFalse(report["mutates_mempool"])
        self.assertEqual("cynic.organism.convergence.v1", report["surface_contract"]["schema"])
        self.assertIn("actions", report)
        self.assertIn("dispatchable_requests", report)

        text = module.organism_convergence_text(report)
        self.assertIn("CYNIC organism convergence", text)
        self.assertIn("Machine output: cynic converge --json", text)

    def test_convergence_dispatch_blocks_without_kernel_env(self):
        module = load_module_from_path("runtime_principals_views_organism", ORGANISM_MODULE_PATH)

        with tempfile.TemporaryDirectory() as tmp:
            with mock.patch.dict("os.environ", {"HOME": tmp}, clear=True):
                report = module.organism_convergence_dispatch_report(run_probes=False)

        self.assertEqual("cynic.organism.convergence_dispatch.v1", report["schema"])
        self.assertEqual("blocked", report["status"])
        self.assertFalse(report["mutates_mempool"])
        self.assertIn("CYNIC_REST_ADDR", report["reason"])
        self.assertFalse(report["kernel_environment"]["present"]["CYNIC_REST_ADDR"])

    def test_convergence_dispatch_loads_cynic_env_without_exposing_secret(self):
        module = load_module_from_path("runtime_principals_views_organism", ORGANISM_MODULE_PATH)

        class FakeClient:
            def __init__(self, base_url, api_key):
                self.base_url = base_url
                self.api_key = api_key

            def list_agent_tasks(self, *, kind, domain, completed=False, limit=50):
                return {"tasks": []}

            def post_agent_task(self, payload):
                return {"task_id": "agent_tasks:from-env"}

            def assert_agent_task_visible(self, task_id, *, kind, domain):
                if task_id != "agent_tasks:from-env":
                    raise AssertionError(task_id)

        fake_convergence = {
            "status": "blocked",
            "actions": [],
            "patterns": [],
            "dispatchable_requests": [
                {
                    "kind": "hermes",
                    "domain": "organism-convergence",
                    "agent_id": "organ-supervisor",
                    "consumer": "organ-forge-hermes-agent",
                    "priority": 90,
                    "coord_targets": ["crates/cynic-mcp"],
                    "content": '{"idempotency_key":"organism-convergence:L3:cynic-mcp"}',
                }
            ],
        }

        with tempfile.TemporaryDirectory() as tmp:
            env_file = Path(tmp) / ".cynic-env"
            env_file.write_text(
                "export CYNIC_REST_ADDR=http://kernel:3030\nexport CYNIC_API_KEY=redacted-from-file\n",
                encoding="utf-8",
            )
            with mock.patch.dict("os.environ", {"HOME": tmp}, clear=True):
                globals_ref = module.organism_convergence_dispatch_report.__globals__
                original_report = globals_ref["organism_convergence_report"]
                original_client = globals_ref["KernelTaskClient"]
                globals_ref["organism_convergence_report"] = lambda run_probes=True: fake_convergence
                globals_ref["KernelTaskClient"] = FakeClient
                try:
                    report = module.organism_convergence_dispatch_report(run_probes=False)
                finally:
                    globals_ref["organism_convergence_report"] = original_report
                    globals_ref["KernelTaskClient"] = original_client

        self.assertEqual("ok", report["status"])
        self.assertEqual(["agent_tasks:from-env"], report["task_ids"])
        self.assertEqual(["CYNIC_REST_ADDR", "CYNIC_API_KEY"], report["kernel_environment"]["loaded_keys"])
        self.assertNotIn("redacted-from-file", str(report))

    def test_convergence_dispatch_reports_kernel_read_failure(self):
        module = load_module_from_path("runtime_principals_views_organism", ORGANISM_MODULE_PATH)

        class FailingClient:
            def __init__(self, base_url, api_key):
                self.base_url = base_url
                self.api_key = api_key

            def list_agent_tasks(self, *, kind, domain, completed=False, limit=50):
                raise OSError("kernel unreachable")

        fake_convergence = {
            "status": "blocked",
            "actions": [],
            "patterns": [],
            "dispatchable_requests": [
                {
                    "kind": "hermes",
                    "domain": "organism-convergence",
                    "agent_id": "organ-supervisor",
                    "consumer": "organ-forge-hermes-agent",
                    "priority": 90,
                    "coord_targets": ["crates/cynic-mcp"],
                    "content": '{"idempotency_key":"organism-convergence:L3:cynic-mcp"}',
                }
            ],
        }

        with mock.patch.dict("os.environ", {"CYNIC_REST_ADDR": "http://kernel:3030", "CYNIC_API_KEY": "test"}):
            globals_ref = module.organism_convergence_dispatch_report.__globals__
            original_report = globals_ref["organism_convergence_report"]
            original_client = globals_ref["KernelTaskClient"]
            globals_ref["organism_convergence_report"] = lambda run_probes=True: fake_convergence
            globals_ref["KernelTaskClient"] = FailingClient
            try:
                report = module.organism_convergence_dispatch_report(run_probes=False)
            finally:
                globals_ref["organism_convergence_report"] = original_report
                globals_ref["KernelTaskClient"] = original_client

        self.assertEqual("blocked", report["status"])
        self.assertFalse(report["mutates_mempool"])
        self.assertIn("kernel task read failed", report["reason"])
        self.assertEqual("not_attempted", report["read_back"])

    def test_convergence_dispatch_posts_with_readback(self):
        module = load_module_from_path("runtime_principals_views_organism", ORGANISM_MODULE_PATH)

        class FakeClient:
            def __init__(self, base_url, api_key):
                self.base_url = base_url
                self.api_key = api_key

            def list_agent_tasks(self, *, kind, domain, completed=False, limit=50):
                return {"tasks": []}

            def post_agent_task(self, payload):
                return {"task_id": "agent_tasks:1"}

            def assert_agent_task_visible(self, task_id, *, kind, domain):
                if task_id != "agent_tasks:1":
                    raise AssertionError(task_id)

        fake_convergence = {
            "status": "blocked",
            "actions": [],
            "patterns": [],
            "dispatchable_requests": [
                {
                    "kind": "hermes",
                    "domain": "organism-convergence",
                    "agent_id": "organ-supervisor",
                    "consumer": "organ-forge-hermes-agent",
                    "priority": 90,
                    "coord_targets": ["crates/cynic-mcp"],
                    "content": '{"idempotency_key":"organism-convergence:L3:cynic-mcp"}',
                }
            ],
        }

        with mock.patch.dict("os.environ", {"CYNIC_REST_ADDR": "http://kernel:3030", "CYNIC_API_KEY": "test"}):
            with mock.patch.object(module, "organism_convergence_report", return_value=fake_convergence):
                globals_ref = module.organism_convergence_dispatch_report.__globals__
                original_report = globals_ref["organism_convergence_report"]
                original_client = globals_ref["KernelTaskClient"]
                globals_ref["organism_convergence_report"] = lambda run_probes=True: fake_convergence
                globals_ref["KernelTaskClient"] = FakeClient
                try:
                    report = module.organism_convergence_dispatch_report(run_probes=False)
                finally:
                    globals_ref["organism_convergence_report"] = original_report
                    globals_ref["KernelTaskClient"] = original_client

        self.assertEqual("ok", report["status"])
        self.assertTrue(report["mutates_mempool"])
        self.assertEqual(1, report["dispatched"])
        self.assertEqual(["agent_tasks:1"], report["task_ids"])

    def test_facade_reexports_organism_surface(self):
        facade = load_module_from_path("runtime_principals_views_actions", FACADE_MODULE_PATH)
        organism = load_module_from_path("runtime_principals_views_organism", ORGANISM_MODULE_PATH)

        self.assertEqual(facade.organism_actions_report(run_probes=False), organism.organism_actions_report(run_probes=False))
        self.assertEqual(facade.organism_text(organism.organism_report(run_probes=False)), organism.organism_text(organism.organism_report(run_probes=False)))


if __name__ == "__main__":
    unittest.main()
