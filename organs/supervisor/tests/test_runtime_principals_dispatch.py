#!/usr/bin/env python3
import unittest

from pathlib import Path

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "src" / "core" / "runtime_principals_dispatch.py"


class RuntimePrincipalsDispatchTests(unittest.TestCase):
    def test_dispatcher_executes_registered_handlers(self):
        dispatch_module = load_module_from_path(
            "runtime_principals_dispatch",
            MODULE_PATH,
        )
        calls: list[list[str]] = []

        def handler(args: list[str]) -> int:
            calls.append(args)
            return 7

        dispatcher = dispatch_module.RuntimePrincipalsCommandDispatcher(
            [
                dispatch_module.RuntimePrincipalsCommandSpec(
                    name="ping",
                    handler=handler,
                    mutates=True,
                    json_output=False,
                    usage="ping [args]",
                )
            ]
        )

        exit_code = dispatcher.execute("ping", ["--check", "repo"])
        self.assertEqual(7, exit_code)
        self.assertEqual([["--check", "repo"]], calls)

    def test_dispatcher_manifest_and_sorted_names(self):
        dispatch_module = load_module_from_path(
            "runtime_principals_dispatch",
            MODULE_PATH,
        )
        dispatcher = dispatch_module.RuntimePrincipalsCommandDispatcher(
            [
                dispatch_module.RuntimePrincipalsCommandSpec(
                    "zeta",
                    lambda _args: 0,
                    mutates=False,
                    json_output=True,
                    usage="zeta [--json]",
                ),
                dispatch_module.RuntimePrincipalsCommandSpec(
                    "alpha",
                    lambda _args: 0,
                    mutates=True,
                    json_output=False,
                    usage="alpha",
                ),
            ]
        )

        self.assertEqual(["alpha", "zeta"], dispatcher.command_names)

        manifest = dispatcher.manifest()
        self.assertEqual("cynic.runtime_principals.command_manifest.v1", manifest["schema"])
        self.assertEqual(2, manifest["count"])
        commands = manifest["commands"]
        self.assertEqual("alpha", commands[0]["name"])
        self.assertEqual("zeta", commands[1]["name"])
        self.assertTrue(commands[0]["mutates"])
        self.assertFalse(commands[0]["json_output"])
        self.assertEqual("alpha", commands[0]["usage"])

        registry = dispatcher.registry_manifest()
        self.assertEqual("cynic.surface_registry.v1", registry["schema"])
        self.assertEqual(2, registry["command_count"])
        self.assertEqual([], registry["reports"])


if __name__ == "__main__":
    unittest.main()
