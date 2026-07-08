#!/usr/bin/env python3
import tempfile
import unittest
from pathlib import Path
from unittest import mock
from urllib.error import URLError

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


ADAPTER_PATH = Path(__file__).resolve().parent.parent / "adapters" / "hermes_fleet_backend_readiness.py"
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


class Response:
    status = 200

    def __init__(self, body: str):
        self.body = body.encode("utf-8")

    def __enter__(self):
        return self

    def __exit__(self, *args):
        return None

    def read(self, _limit: int):
        return self.body


class HermesFleetBackendReadinessTests(unittest.TestCase):
    def test_ready_when_models_endpoint_lists_expected_model(self):
        module = load_module_from_path("hermes_backend_readiness_ready", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "backends.toml"
            path.write_text(BACKENDS_TOML, encoding="utf-8")
            with mock.patch.object(
                module,
                "urlopen",
                return_value=Response('{"data":[{"id":"Qwen3.6-27B-IQ3_XXS.gguf"}]}'),
            ):
                payload = module.probe_hermes_fleet_backend_readiness(
                    backends_toml=path,
                    expected_backend_id="qwen36-27b-gpu",
                    observed_at="2026-07-03T00:00:00Z",
                )

        self.assertEqual("cynic.hermes_fleet_backend_readiness.v1", payload["schema"])
        self.assertEqual("ready", payload["status"])
        self.assertEqual([], payload["violations"])
        self.assertIn("Qwen3.6-27B-IQ3_XXS.gguf", payload["observed_model_ids"])

    def test_degraded_when_expected_model_missing(self):
        module = load_module_from_path("hermes_backend_readiness_missing_model", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "backends.toml"
            path.write_text(BACKENDS_TOML, encoding="utf-8")
            with mock.patch.object(module, "urlopen", return_value=Response('{"data":[{"id":"other"}]}')):
                payload = module.probe_hermes_fleet_backend_readiness(
                    backends_toml=path,
                    expected_backend_id="qwen36-27b-gpu",
                    observed_at="2026-07-03T00:00:00Z",
                )

        self.assertEqual("degraded", payload["status"])
        self.assertEqual("expected_model_not_listed", payload["violations"][0]["kind"])

    def test_degraded_on_connection_error(self):
        module = load_module_from_path("hermes_backend_readiness_connection_error", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "backends.toml"
            path.write_text(BACKENDS_TOML, encoding="utf-8")
            with mock.patch.object(module, "urlopen", side_effect=URLError("no route")):
                payload = module.probe_hermes_fleet_backend_readiness(
                    backends_toml=path,
                    expected_backend_id="qwen36-27b-gpu",
                    observed_at="2026-07-03T00:00:00Z",
                )

        self.assertEqual("degraded", payload["status"])
        self.assertEqual("connection_error", payload["violations"][0]["kind"])

    def test_synapse_dns_and_route_are_distinct_violations(self):
        module = load_module_from_path("hermes_backend_readiness_synapse_route", ADAPTER_PATH)
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
      "network": {"synapse": "10.20.0.7"}
    }
  ]
}
""",
                encoding="utf-8",
            )
            with mock.patch.object(
                module.socket,
                "getaddrinfo",
                side_effect=OSError("Name or service not known"),
            ), mock.patch.object(module, "urlopen", side_effect=URLError("No route to host")):
                payload = module.probe_hermes_fleet_backend_readiness(
                    backends_toml=backends,
                    registry_json=registry,
                    expected_backend_id="qwen36-27b-gpu",
                    observed_at="2026-07-03T00:00:00Z",
                )

        kinds = {item["kind"] for item in payload["violations"]}
        self.assertEqual("degraded", payload["status"])
        self.assertIn("synapse_dns_unresolved", kinds)
        self.assertIn("synapse_mesh_route_unreachable", kinds)
        self.assertEqual("10.20.0.7", payload["projection"]["endpoint_evidence"]["resolved_synapse_ip"])
        self.assertEqual("connection_error", payload["evidence"]["synapse_ip_probe"]["status"])
        actions = {item["action"]: item for item in payload["next_actions"]}
        self.assertIn("activate_synapse_dns", actions)
        self.assertIn("restore_synapse_mesh_route", actions)
        self.assertEqual(
            "python3 organs/fleet/src/entrypoints/cli.py dns plan",
            actions["activate_synapse_dns"]["command"],
        )

    def test_cli_manifest_exposes_readiness(self):
        module = load_module_from_path("contracts_cli_backend_readiness", CLI_PATH)
        schemas = {item["schema"] for item in module.surface_manifest()["registry"]["reports"]}

        self.assertIn("cynic.hermes_fleet_backend_readiness.v1", schemas)


if __name__ == "__main__":
    unittest.main()
