#!/usr/bin/env python3
import os
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "adapters" / "mcp_binary.py"


class McpBinaryAdapterTests(unittest.TestCase):
    def test_select_mcp_binary_prefers_explicit_env_binary(self):
        module = load_module_from_path("contracts_mcp_binary_adapter", MODULE_PATH)

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            explicit = root / "custom-cynic-mcp"
            explicit.write_text("#!/bin/sh\n", encoding="utf-8")
            explicit.chmod(0o755)

            with mock.patch.dict("os.environ", {"CYNIC_MCP_BINARY": str(explicit)}, clear=True):
                selection = module.select_mcp_binary(root)

        self.assertEqual(explicit, selection.binary)
        self.assertTrue(selection.usable)

    def test_select_mcp_binary_reports_stale_source(self):
        module = load_module_from_path("contracts_mcp_binary_stale", MODULE_PATH)

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            binary = root / "target" / "release" / "cynic-mcp"
            source = root / "crates" / "cynic-mcp" / "src" / "main.rs"
            binary.parent.mkdir(parents=True)
            source.parent.mkdir(parents=True)
            binary.write_text("#!/bin/sh\n", encoding="utf-8")
            binary.chmod(0o755)
            source.write_text("fn main() {}\n", encoding="utf-8")
            old = time.time() - 60
            new = time.time()
            os.utime(binary, (old, old))
            os.utime(source, (new, new))

            with mock.patch.dict("os.environ", {}, clear=True):
                selection = module.select_mcp_binary(root)

        self.assertEqual(binary, selection.binary)
        self.assertEqual(source, selection.stale_source)
        self.assertFalse(selection.usable)

    def test_select_mcp_binary_accepts_legacy_path_binary_name(self):
        module = load_module_from_path("contracts_mcp_binary_legacy_path", MODULE_PATH)

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            legacy = root / "cynic-kernel-mcp"
            legacy.write_text("#!/bin/sh\n", encoding="utf-8")
            legacy.chmod(0o755)

            with mock.patch.dict("os.environ", {}, clear=True):
                with mock.patch.object(module.shutil, "which", side_effect=[None, str(legacy)]):
                    selection = module.select_mcp_binary(root)

        self.assertEqual(legacy, selection.binary)
        self.assertTrue(selection.usable)

    def test_select_mcp_binary_uses_legacy_cortex_binary_location_when_present(self):
        module = load_module_from_path("contracts_mcp_binary_legacy_location", MODULE_PATH)

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            legacy = root / "cynic-kernel-mcp"
            legacy.write_text("#!/bin/sh\n", encoding="utf-8")
            legacy.chmod(0o755)

            with mock.patch.dict("os.environ", {}, clear=True):
                with mock.patch.object(module.shutil, "which", side_effect=[None, None]):
                    with mock.patch.object(module, "LEGACY_CORTEX_BINARIES", (legacy,)):
                        selection = module.select_mcp_binary(root)

        self.assertEqual(legacy, selection.binary)
        self.assertTrue(selection.usable)


if __name__ == "__main__":
    unittest.main()
