#!/usr/bin/env python3
import tempfile
import unittest

from pathlib import Path

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


ADAPTER_PATH = Path(__file__).resolve().parent.parent / "adapters" / "hermes_fleet_model.py"
CLI_PATH = Path(__file__).resolve().parent.parent / "entrypoints" / "cli.py"


BACKENDS_TOML = """
[backend.qwen36-27b-gpu]
base_url = "http://cynic-host-gpu:8080/v1"
model = "Qwen3.6-27B-IQ3_XXS.gguf"
sovereign = true
supports_tool_use = true
latency_class = "background"

[backend.qwen36-27b-gpu.remediation]
node = "cynic-host-gpu"

[llama-server]
binary = "llama-server"
openai_compat = true
"""


class HermesFleetModelTests(unittest.TestCase):
    def test_fleet_model_state_aligned_when_hermes_matches_backend(self):
        module = load_module_from_path("hermes_fleet_model_adapter_aligned", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "backends.toml"
            path.write_text(BACKENDS_TOML, encoding="utf-8")
            state = module.evaluate_hermes_fleet_model_state(
                backends_toml=path,
                status_text="""
◆ Environment
  Model:        Qwen3.6-27B-IQ3_XXS.gguf
  Provider:     custom:qwen36-27b-gpu
""",
                observed_at="2026-07-03T00:00:00Z",
            ).to_dict()

        self.assertEqual("cynic.hermes_fleet_model_state.v1", state["schema"])
        self.assertEqual("aligned", state["status"])
        self.assertEqual([], state["violations"])
        self.assertEqual("cynic-host-gpu", state["expected_backend"]["fleet_node"])
        self.assertTrue(state["expected_backend"]["sovereign"])

    def test_fleet_model_state_reports_external_provider_mismatch(self):
        module = load_module_from_path("hermes_fleet_model_adapter_mismatch", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "backends.toml"
            path.write_text(BACKENDS_TOML, encoding="utf-8")
            state = module.evaluate_hermes_fleet_model_state(
                backends_toml=path,
                status_text="""
◆ Environment
  Model:        (not set)
  Provider:     Google AI Studio
""",
                observed_at="2026-07-03T00:00:00Z",
            ).to_dict()

        violation_kinds = {item["kind"] for item in state["violations"]}
        self.assertEqual("mismatch", state["status"])
        self.assertIn("provider_mismatch", violation_kinds)
        self.assertIn("model_mismatch", violation_kinds)
        self.assertEqual("qwen36-27b-gpu", state["expected_backend"]["backend_id"])

    def test_cli_manifest_exposes_hermes_fleet_model_state(self):
        module = load_module_from_path("contracts_cli_hermes_fleet_model", CLI_PATH)
        manifest = module.surface_manifest()
        schemas = {item["schema"] for item in manifest["registry"]["reports"]}

        self.assertIn("cynic.hermes_fleet_model_state.v1", schemas)


if __name__ == "__main__":
    unittest.main()
