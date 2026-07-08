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


ADAPTER_PATH = Path(__file__).resolve().parent.parent / "adapters" / "hermes_fleet_config_projection.py"
CLI_PATH = Path(__file__).resolve().parent.parent / "entrypoints" / "cli.py"


BACKENDS_TOML = """
[backend.qwen36-27b-gpu]
base_url = "http://cynic-host-gpu:8080/v1"
model = "Qwen3.6-27B-IQ3_XXS.gguf"
context_size = 65536
api_key_env = "CYNIC_SOVEREIGN_KEY"
sovereign = true
supports_tool_use = true
"""

BACKENDS_TOML_WITH_PLACEHOLDER = """
[backend.qwen36-27b-gpu]
base_url = "http://<TAILSCALE_HOST_GPU>:8080/v1"
model = "Qwen3.6-27B-IQ3_XXS.gguf"
context_size = 65536
api_key_env = "CYNIC_SOVEREIGN_KEY"
sovereign = true
supports_tool_use = true

[backend.qwen36-27b-gpu.remediation]
node = "cynic-host-gpu"
"""


class HermesFleetConfigProjectionTests(unittest.TestCase):
    def test_projection_renders_named_custom_provider_without_secret_value(self):
        module = load_module_from_path("hermes_fleet_config_projection_adapter", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "backends.toml"
            path.write_text(BACKENDS_TOML, encoding="utf-8")
            payload = module.render_hermes_fleet_config_projection(
                backends_toml=path,
                expected_backend_id="qwen36-27b-gpu",
                observed_at="2026-07-03T00:00:00Z",
            )

        self.assertEqual("cynic.hermes_fleet_config_projection.v1", payload["schema"])
        self.assertFalse(payload["mutates"])
        self.assertEqual("custom:qwen36-27b-gpu", payload["provider_slug"])
        self.assertEqual("CYNIC_SOVEREIGN_KEY", payload["required_secret_env"])
        self.assertNotIn("api_key", payload["config_patch"]["custom_providers"][0])
        self.assertEqual(
            "Qwen3.6-27B-IQ3_XXS.gguf",
            payload["config_patch"]["model"]["default"],
        )
        self.assertIn("provider: custom:qwen36-27b-gpu", payload["yaml_snippet"])

    def test_projection_resolves_placeholder_to_synapse_dns(self):
        module = load_module_from_path("hermes_fleet_config_projection_adapter_registry", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            backends = root / "backends.toml"
            registry = root / "registry.json"
            backends.write_text(BACKENDS_TOML_WITH_PLACEHOLDER, encoding="utf-8")
            registry.write_text(
                """
{
  "nodes": [
    {
      "name": "cynic-host-gpu",
      "aliases": ["cynic-gpu"],
      "network": {"synapse": "10.20.0.7"}
    }
  ]
}
""",
                encoding="utf-8",
            )
            payload = module.render_hermes_fleet_config_projection(
                backends_toml=backends,
                registry_json=registry,
                expected_backend_id="qwen36-27b-gpu",
                observed_at="2026-07-03T00:00:00Z",
            )

        self.assertEqual(
            "http://cynic-host-gpu.synapse.cynic:8080/v1",
            payload["config_patch"]["model"]["base_url"],
        )
        self.assertTrue(payload["endpoint_evidence"]["resolved"])
        self.assertEqual("10.20.0.7", payload["endpoint_evidence"]["resolved_synapse_ip"])

    def test_cli_manifest_exposes_projection(self):
        module = load_module_from_path("contracts_cli_hermes_fleet_projection", CLI_PATH)
        manifest = module.surface_manifest()
        schemas = {item["schema"] for item in manifest["registry"]["reports"]}

        self.assertIn("cynic.hermes_fleet_config_projection.v1", schemas)
        self.assertIn("cynic.hermes_fleet_config_drift.v1", schemas)

    def test_drift_reports_missing_config(self):
        module = load_module_from_path("hermes_fleet_config_projection_drift_missing", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            backends = root / "backends.toml"
            missing = root / "missing.yaml"
            backends.write_text(BACKENDS_TOML, encoding="utf-8")
            payload = module.render_hermes_fleet_config_drift(
                backends_toml=backends,
                config_path=missing,
                expected_backend_id="qwen36-27b-gpu",
                observed_at="2026-07-03T00:00:00Z",
            )

        self.assertEqual("cynic.hermes_fleet_config_drift.v1", payload["schema"])
        self.assertEqual("drift", payload["status"])
        self.assertEqual("missing_config", payload["violations"][0]["kind"])
        self.assertTrue(payload["apply_required"])

    def test_drift_reports_aligned_config(self):
        module = load_module_from_path("hermes_fleet_config_projection_drift_aligned", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            backends = root / "backends.toml"
            config = root / "config.yaml"
            backends.write_text(BACKENDS_TOML, encoding="utf-8")
            config.write_text(
                """
model:
  provider: custom:qwen36-27b-gpu
  default: Qwen3.6-27B-IQ3_XXS.gguf
  base_url: http://cynic-host-gpu:8080/v1
custom_providers:
  - name: qwen36-27b-gpu
    base_url: http://cynic-host-gpu:8080/v1
    model: Qwen3.6-27B-IQ3_XXS.gguf
    key_env: CYNIC_SOVEREIGN_KEY
    api_mode: openai
""",
                encoding="utf-8",
            )
            payload = module.render_hermes_fleet_config_drift(
                backends_toml=backends,
                config_path=config,
                expected_backend_id="qwen36-27b-gpu",
                observed_at="2026-07-03T00:00:00Z",
            )

        self.assertEqual("aligned", payload["status"])
        self.assertEqual([], payload["violations"])
        self.assertFalse(payload["apply_required"])

    def test_drift_reports_external_provider(self):
        module = load_module_from_path("hermes_fleet_config_projection_drift_external", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            backends = root / "backends.toml"
            config = root / "config.yaml"
            backends.write_text(BACKENDS_TOML, encoding="utf-8")
            config.write_text(
                """
model:
  provider: google
  default: gemini
""",
                encoding="utf-8",
            )
            payload = module.render_hermes_fleet_config_drift(
                backends_toml=backends,
                config_path=config,
                expected_backend_id="qwen36-27b-gpu",
                observed_at="2026-07-03T00:00:00Z",
            )

        kinds = {item["kind"] for item in payload["violations"]}
        self.assertEqual("drift", payload["status"])
        self.assertIn("model_provider_mismatch", kinds)
        self.assertIn("missing_custom_provider", kinds)


if __name__ == "__main__":
    unittest.main()
