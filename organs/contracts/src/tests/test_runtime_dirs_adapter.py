#!/usr/bin/env python3
import tempfile
import unittest
from pathlib import Path

from organs.contracts.src.adapters.runtime_dirs import ensure_dirs, ensure_parent_dirs


class RuntimeDirsAdapterTests(unittest.TestCase):
    def test_ensure_parent_dirs_and_dirs(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            file_target = root / "a" / "b" / "messages.db"
            dir_target = root / "media"

            ensure_parent_dirs((file_target,))
            ensure_dirs((dir_target,))

            self.assertTrue(file_target.parent.is_dir())
            self.assertTrue(dir_target.is_dir())
            self.assertFalse(file_target.exists())


if __name__ == "__main__":
    unittest.main()
