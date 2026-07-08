from __future__ import annotations

from pathlib import Path
import tempfile
import unittest
from unittest import mock

from organs.supervisor.src.core.organism_stack import organism_stack_report


def _write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class OrganismStackTests(unittest.TestCase):
    def test_stack_reports_all_layers_and_missing_mcp_binary(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _write(root / "infra/registry.json", '{"nodes": [{"name": "n1", "role": "kernel_primary"}]}')
            _write(
                root / "organs/supervisor/MANIFEST.yaml",
                "id: organ-supervisor\nstatus: active\ndomain: soma\n",
            )
            _write(
                root / "organs/forge/MANIFEST.yaml",
                "id: organ-forge\nstatus: active\ndomain: lifecycle\n",
            )
            _write(
                root / ".codex/config.toml",
                'command = "python3"\nargs = ["./.cortex/mcp/run_cynic_mcp.py"]\nargs = ["./.cortex/mcp/run_browser_mcp.py"]\n',
            )

            with mock.patch("organs.contracts.src.adapters.mcp_binary.shutil.which", return_value=None), mock.patch.dict(
                "os.environ",
                {"ORGAN_BROWSER_HUB_URL": "http://127.0.0.1:9"},
            ):
                report = organism_stack_report(root, {"runtime_ready": True})

            self.assertEqual(report["layer_count"], 8)
            self.assertIn("idempotency_key", report["async_primitives"])
            self.assertEqual(
                {layer["id"] for layer in report["layers"]},
                {"L0", "L1", "L2", "L3", "L4", "L5", "L6", "L7"},
            )
            mcp = next(entity for entity in report["entities"] if entity["entity"] == "cynic-mcp")
            self.assertEqual(mcp["health"], "blocked")
            self.assertEqual(mcp["observed"], "no executable cynic-mcp binary found")
            self.assertTrue(any(pattern["pattern"] == "cell_runtime_gap" for pattern in report["patterns"]))
            mcp_action = next(action for action in report["actions"] if action["entity"] == "cynic-mcp")
            self.assertEqual(mcp_action["kind"], "lifecycle_dispatch")
            self.assertTrue(mcp_action["mutates_mempool"])
            self.assertEqual(mcp_action["consumer"], "organ-forge-hermes-agent")
            self.assertIn("crates/cynic-mcp", mcp_action["coord_targets"])
            request = next(item for item in report["dispatchable_requests"] if item["consumer"] == "organ-forge-hermes-agent")
            self.assertEqual(request["domain"], "organism-convergence")
            self.assertIn("organism-convergence:L3:cynic-mcp", request["content"])
            browser_hub = next(entity for entity in report["entities"] if entity["entity"] == "organ-browser-hub")
            self.assertEqual(browser_hub["health"], "degraded")
            self.assertIn("activate_codex_browser_mcp.py --start-services", browser_hub["next_action"])
            browser_cdp = next(entity for entity in report["entities"] if entity["entity"] == "organ-browser-cdp")
            self.assertEqual(browser_cdp["health"], "degraded")
            self.assertIn("cannot observe Chrome CDP", browser_cdp["observed"])

    def test_stack_detects_overlapping_organ_domains(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _write(root / "infra/registry.json", '{"nodes": []}')
            _write(root / ".codex/config.toml", "")
            _write(root / "organs/a/MANIFEST.yaml", "id: organ-a\nstatus: active\ndomain: social\n")
            _write(root / "organs/b/MANIFEST.yaml", "id: organ-b\nstatus: active\ndomain: social\n")

            report = organism_stack_report(root, {})

        overlap = next(pattern for pattern in report["patterns"] if pattern["pattern"] == "overlapping_organ_domain")
        self.assertEqual(overlap["entities"], {"social": ["organ-a", "organ-b"]})

    def test_stack_detects_utility_duplicate_patterns_only_for_helpers(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _write(root / "infra/registry.json", '{"nodes": []}')
            _write(root / ".codex/config.toml", "")
            _write(root / "organs/a/MANIFEST.yaml", "id: organ-a\nstatus: active\ndomain: a\n")
            _write(root / "organs/b/MANIFEST.yaml", "id: organ-b\nstatus: active\ndomain: b\n")
            _write(root / "organs/a/src/core/a.py", "def read_pid(path):\n    return None\n\ndef domain_cycle():\n    return None\n")
            _write(root / "organs/b/src/core/b.py", "def read_pid(path):\n    return None\n\ndef domain_cycle():\n    return None\n")

            report = organism_stack_report(root, {})

        pattern = next(item for item in report["patterns"] if item["pattern"] == "utility_duplicate_pattern")
        self.assertIn("read_pid", pattern["entities"])
        self.assertNotIn("domain_cycle", pattern["entities"])

    def test_stack_ignores_utility_wrappers_delegating_to_shared_ports(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _write(root / "infra/registry.json", '{"nodes": []}')
            _write(root / ".codex/config.toml", "")
            _write(root / "organs/a/MANIFEST.yaml", "id: organ-a\nstatus: active\ndomain: a\n")
            _write(root / "organs/b/MANIFEST.yaml", "id: organ-b\nstatus: active\ndomain: b\n")
            _write(
                root / "organs/a/src/core/a.py",
                "def load_yaml(path):\n    return read_yaml_file_with_loader(path, yaml)\n",
            )
            _write(
                root / "organs/b/src/core/b.py",
                "def load_yaml(path):\n    return read_yaml_file_with_loader(path, yaml)\n",
            )

            report = organism_stack_report(root, {})

        utility_patterns = [item for item in report["patterns"] if item["pattern"] == "utility_duplicate_pattern"]
        self.assertEqual([], utility_patterns)

    def test_stack_ignores_entrypoint_wrappers_delegating_to_runtime_alias(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _write(root / "infra/registry.json", '{"nodes": []}')
            _write(root / ".codex/config.toml", "")
            _write(root / "organs/a/MANIFEST.yaml", "id: organ-a\nstatus: active\ndomain: a\n")
            _write(root / "organs/a/src/adapters/runtime.py", "def get_chrome_profile():\n    return 'profile'\n")
            _write(
                root / "organs/a/src/entrypoints/login.py",
                "def get_chrome_profile():\n    return _get_chrome_profile()\n",
            )

            report = organism_stack_report(root, {})

        utility_patterns = [item for item in report["patterns"] if item["pattern"] == "utility_duplicate_pattern"]
        self.assertEqual([], utility_patterns)


if __name__ == "__main__":
    unittest.main()
