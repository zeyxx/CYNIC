import unittest
from unittest.mock import patch
from pathlib import Path


try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


class TopologyEntrypointTests(unittest.TestCase):
    def test_topology_entrypoint_delegates_to_controller_main(self):
        topology_module = load_module_from_path(
            "topology_entrypoint_under_test",
            Path(__file__).resolve().parent.parent
            / "src"
            / "entrypoints"
            / "topology.py",
        )

        with patch.object(topology_module, "_controller_main", return_value=7) as mocked:
            self.assertEqual(topology_module.main(), 7)
            mocked.assert_called_once()


if __name__ == "__main__":
    unittest.main()
