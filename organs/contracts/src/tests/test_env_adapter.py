#!/usr/bin/env python3
import tempfile
import unittest
import os
from pathlib import Path
from unittest import mock

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "adapters" / "env.py"


class EnvAdapterTests(unittest.TestCase):
    def test_load_env_file_respects_allowlist_and_existing_values(self):
        module = load_module_from_path("contracts_env_adapter", MODULE_PATH)

        with tempfile.TemporaryDirectory() as tmpdir:
            env_file = Path(tmpdir) / ".cynic-env"
            env_file.write_text(
                "\n".join(
                    [
                        "export CYNIC_REST_ADDR=http://kernel:3030",
                        "CYNIC_API_KEY='redacted-from-file'",
                        "UNRELATED=value",
                    ]
                )
                + "\n",
                encoding="utf-8",
            )
            with mock.patch.dict("os.environ", {"CYNIC_API_KEY": "already-present"}, clear=True):
                loaded = module.load_env_file(env_file, allowed_keys=("CYNIC_REST_ADDR", "CYNIC_API_KEY"))

                self.assertEqual(["CYNIC_REST_ADDR"], loaded)
                self.assertEqual("http://kernel:3030", os.environ["CYNIC_REST_ADDR"])
                self.assertEqual("already-present", os.environ["CYNIC_API_KEY"])
                self.assertNotIn("UNRELATED", os.environ)

    def test_load_required_env_report_is_redacted(self):
        module = load_module_from_path("contracts_env_adapter_redacted", MODULE_PATH)

        with tempfile.TemporaryDirectory() as tmpdir:
            env_file = Path(tmpdir) / "env"
            env_file.write_text(
                "CYNIC_REST_ADDR=http://kernel:3030\nCYNIC_API_KEY=redacted-from-file\n",
                encoding="utf-8",
            )
            with mock.patch.dict("os.environ", {"CYNIC_ENV_FILE": str(env_file)}, clear=True):
                report = module.load_required_env_from_default_file(("CYNIC_REST_ADDR", "CYNIC_API_KEY"))

        self.assertTrue(report["loaded"])
        self.assertEqual(["CYNIC_REST_ADDR", "CYNIC_API_KEY"], report["loaded_keys"])
        self.assertTrue(report["present"]["CYNIC_REST_ADDR"])
        self.assertTrue(report["present"]["CYNIC_API_KEY"])
        self.assertNotIn("redacted-from-file", str(report))

    def test_load_required_env_can_be_disabled(self):
        module = load_module_from_path("contracts_env_adapter_disabled", MODULE_PATH)

        with tempfile.TemporaryDirectory() as tmpdir:
            env_file = Path(tmpdir) / "env"
            env_file.write_text("CYNIC_REST_ADDR=http://kernel:3030\n", encoding="utf-8")
            with mock.patch.dict(
                "os.environ",
                {"CYNIC_ENV_FILE": str(env_file), "CYNIC_LOAD_ENV_FILE": "0"},
                clear=True,
            ):
                report = module.load_required_env_from_default_file(("CYNIC_REST_ADDR",))

        self.assertEqual("disabled", report["source"])
        self.assertFalse(report["loaded"])
        self.assertFalse(report["present"]["CYNIC_REST_ADDR"])


if __name__ == "__main__":
    unittest.main()
