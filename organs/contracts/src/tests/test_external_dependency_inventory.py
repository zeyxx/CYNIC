#!/usr/bin/env python3
import unittest

from pathlib import Path
import tempfile

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


CORE_PATH = Path(__file__).resolve().parent.parent / "core" / "external_dependency_inventory.py"
ADAPTER_PATH = Path(__file__).resolve().parent.parent / "adapters" / "dependency_inventory.py"
CLI_PATH = Path(__file__).resolve().parent.parent / "entrypoints" / "cli.py"


class ExternalDependencyInventoryTests(unittest.TestCase):
    def test_inventory_round_trip(self):
        module = load_module_from_path("external_dependency_inventory_core", CORE_PATH)
        dependency = module.ExternalDependency(
            ecosystem="python",
            manifest="pyproject.toml",
            group="project.dependencies",
            name="openai",
            specifier="==2.24.0",
            scope="runtime",
        )
        inventory = module.ExternalDependencyInventory(
            observed_at="2026-07-01T00:00:00Z",
            scan_roots=(".",),
            dependencies=(dependency,),
            manifests=({"path": "pyproject.toml", "kind": "python_pyproject"},),
        )

        payload = inventory.to_dict()
        parsed = module.ExternalDependencyInventory.from_dict(payload)

        self.assertEqual("cynic.external_dependency_inventory.v1", payload["schema"])
        self.assertEqual(1, payload["dependency_count"])
        self.assertEqual("openai", parsed.dependencies[0].name)

    def test_scanner_parses_supported_manifests(self):
        adapter = load_module_from_path("dependency_inventory_adapter", ADAPTER_PATH)
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            (root / "pyproject.toml").write_text(
                """
[project]
dependencies = ["openai==2.24.0"]
[project.optional-dependencies]
voice = ["faster-whisper==1.2.1"]
""",
                encoding="utf-8",
            )
            (root / "package.json").write_text(
                '{"dependencies":{"agent-browser":"^0.26.0"},"devDependencies":{"vite":"^7.0.0"}}',
                encoding="utf-8",
            )
            (root / "Cargo.toml").write_text(
                '[dependencies]\nserde = "1"\n[dev-dependencies]\npretty_assertions = "1"\n',
                encoding="utf-8",
            )
            payload = adapter.DependencyInventoryScanner((root,)).scan("2026-07-01T00:00:00Z").to_dict()

        names = {item["name"] for item in payload["dependencies"]}
        self.assertEqual(3, payload["manifest_count"])
        self.assertTrue({"openai", "faster-whisper", "agent-browser", "vite", "serde", "pretty_assertions"} <= names)

    def test_cli_manifest_exposes_dependency_report(self):
        module = load_module_from_path("contracts_cli_dependency_inventory", CLI_PATH)
        manifest = module.surface_manifest()
        schemas = {item["schema"] for item in manifest["registry"]["reports"]}

        self.assertIn("cynic.external_dependency_inventory.v1", schemas)


if __name__ == "__main__":
    unittest.main()
