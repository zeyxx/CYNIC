#!/usr/bin/env python3
import tempfile
import unittest
from pathlib import Path

from organs.browser.src.adapters.direct_cell_runtime import pid_alive, read_pid


class DirectCellRuntimeTests(unittest.TestCase):
    def test_read_pid_returns_integer_or_none(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "cell.pid"
            self.assertIsNone(read_pid(path))
            path.write_text("1234\n", encoding="utf-8")
            self.assertEqual(1234, read_pid(path))
            path.write_text("not-a-pid\n", encoding="utf-8")
            self.assertIsNone(read_pid(path))

    def test_pid_alive_handles_empty_pid(self):
        self.assertFalse(pid_alive(None))
        self.assertFalse(pid_alive(0))


if __name__ == "__main__":
    unittest.main()
