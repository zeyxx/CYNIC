#!/usr/bin/env python3
import unittest

from pathlib import Path

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


CORE_PATH = Path(__file__).resolve().parent.parent / "core" / "hermes_kanban_graph.py"
CLI_PATH = Path(__file__).resolve().parent.parent / "entrypoints" / "cli.py"


class HermesKanbanGraphTests(unittest.TestCase):
    def test_example_graph_event_has_schema_and_payload_shape(self):
        module = load_module_from_path("hermes_kanban_graph_core", CORE_PATH)
        payload = module.example_hermes_kanban_graph_event()

        self.assertEqual("cynic.hermes_kanban_graph_event.v1", payload["schema"])
        self.assertEqual("kanban_status", payload["event"])
        self.assertEqual("hk-demo", payload["hermes_kanban_task_id"])
        self.assertEqual("in_progress", payload["status"])
        self.assertEqual({"source": "example"}, payload["payload"])

    def test_example_execution_result_has_schema(self):
        module = load_module_from_path("hermes_kanban_graph_core_result", CORE_PATH)
        payload = module.example_hermes_execution_result()

        self.assertEqual("cynic.hermes.execution_result.v1", payload["schema"])
        self.assertEqual("hermes-kanban", payload["executor"])

    def test_cli_manifest_exposes_graph_and_result_contracts(self):
        module = load_module_from_path("contracts_cli_hermes_kanban_graph", CLI_PATH)
        manifest = module.surface_manifest()
        schemas = {item["schema"] for item in manifest["registry"]["reports"]}

        self.assertIn("cynic.hermes_kanban_graph_event.v1", schemas)
        self.assertIn("cynic.hermes.execution_result.v1", schemas)


if __name__ == "__main__":
    unittest.main()
