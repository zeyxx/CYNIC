#!/usr/bin/env python3
import os
import subprocess
import sys
import unittest
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[4]
CLI = PROJECT_ROOT / "organs/contracts/src/entrypoints/cli.py"


class ContractsCliBootstrapTests(unittest.TestCase):
    def test_cli_imports_contracts_when_services_package_precedes_repo_root(self):
        env = dict(os.environ)
        env["PYTHONPATH"] = (
            f"{PROJECT_ROOT / 'services/cynic-python'}"
            f"{os.pathsep}{PROJECT_ROOT}"
        )

        result = subprocess.run(
            [sys.executable, str(CLI), "surface-manifest"],
            cwd=PROJECT_ROOT,
            env=env,
            text=True,
            capture_output=True,
            timeout=10,
            check=False,
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("cynic.contracts.surface_manifest.v1", result.stdout)


if __name__ == "__main__":
    unittest.main()
