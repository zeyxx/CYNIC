#!/usr/bin/env python3
import os
import unittest
from unittest import mock

from pathlib import Path
import tempfile

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "surface_contracts.py"


class SurfaceContractsTests(unittest.TestCase):
    def test_command_surface_spec_manifest(self):
        module = load_module_from_path("contracts_surface_contracts", MODULE_PATH)
        spec = module.CommandSurfaceSpec(
            name="status",
            handler=lambda _args: 0,
            mutates=False,
            json_output=True,
            usage="status [--json]",
        )

        manifest = spec.as_manifest()
        self.assertEqual("status", manifest["name"])
        self.assertFalse(manifest["mutates"])
        self.assertTrue(manifest["json_output"])
        self.assertEqual("status [--json]", manifest["usage"])
        self.assertTrue(isinstance(spec, module.CommandSurface))

    def test_report_surface_spec_manifest(self):
        module = load_module_from_path("contracts_surface_contracts", MODULE_PATH)
        spec = module.ReportSurfaceSpec(
            schema="cynic.example.report.v1",
            command="cynic example",
            mutates=True,
            mutates_mempool=True,
        )

        manifest = module.report_surface_manifest(spec)
        self.assertEqual("cynic.example.report.v1", manifest["schema"])
        self.assertEqual("cynic example", manifest["command"])
        self.assertTrue(manifest["mutates"])
        self.assertTrue(manifest["mutates_mempool"])
        self.assertEqual("cynic example --json", manifest["json_command"])
        self.assertEqual("cynic example", manifest["usage"])
        self.assertTrue(isinstance(spec, module.ReportSurface))

    def test_surface_registry_manifest(self):
        module = load_module_from_path("contracts_surface_contracts", MODULE_PATH)
        command = module.CommandSurfaceSpec("status", lambda _args: 0, mutates=False, json_output=True)
        report = module.ReportSurfaceSpec("cynic.example.report.v1", "cynic example")
        registry = module.SurfaceRegistry(commands=(command,), reports=(report,))

        manifest = registry.manifest()
        self.assertEqual("cynic.surface_registry.v1", manifest["schema"])
        self.assertEqual(1, manifest["command_count"])
        self.assertEqual(1, manifest["report_count"])
        self.assertEqual(["status"], registry.command_names())
        self.assertEqual(["cynic example"], registry.report_names())
        self.assertEqual("status", manifest["commands"][0]["name"])
        self.assertEqual("cynic example", manifest["reports"][0]["command"])

    def test_attach_surface_contract(self):
        module = load_module_from_path("contracts_surface_contracts", MODULE_PATH)
        report = module.ReportSurfaceSpec("cynic.example.report.v1", "cynic example")
        payload = module.attach_surface_contract({"status": "ok"}, report)

        self.assertEqual("cynic.example.report.v1", payload["surface_contract"]["schema"])
        self.assertEqual("ok", payload["status"])

    def test_json_stdout_or_envelope(self):
        module = load_module_from_path("contracts_surface_contracts", MODULE_PATH)
        parsed = module.json_stdout_or_envelope(
            {"status": "ok", "stdout": '{"answer": 42}', "stderr": ""},
            schema="cynic.example.v1",
        )
        fallback = module.json_stdout_or_envelope(
            {"status": "blocked", "stdout": "no-json", "stderr": "err"},
            schema="cynic.example.v1",
            stdout_transform=lambda text: text[:3],
            extra={"exit_code": 1},
        )

        self.assertEqual({"answer": 42}, parsed)
        self.assertEqual("cynic.example.v1", fallback["schema"])
        self.assertEqual("blocked", fallback["status"])
        self.assertEqual("no-", fallback["stdout"])
        self.assertEqual("err", fallback["stderr"])
        self.assertEqual(1, fallback["exit_code"])

    def test_json_loads_or_default(self):
        module = load_module_from_path("contracts_surface_contracts", MODULE_PATH)

        self.assertEqual({"answer": 42}, module.json_loads_or_default('{"answer": 42}'))
        self.assertEqual({"fallback": True}, module.json_loads_or_default("no-json", {"fallback": True}))
        self.assertEqual({"answer": 42}, module.json_object_or_default('{"answer": 42}'))
        self.assertEqual({}, module.json_object_or_default(["not", "an", "object"]))
        self.assertEqual({"answer": 42}, module.json_loads_or_raise('{"answer": 42}', ValueError("bad")))
        self.assertEqual({"answer": 42}, module.json_object_or_raise('{"answer": 42}', ValueError("bad")))
        with self.assertRaises(ValueError):
            module.json_loads_or_raise("no-json", ValueError("bad"))
        with self.assertRaises(ValueError):
            module.json_object_or_raise("[]", ValueError("bad"))
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "sample.json"
            path.write_text('{"answer": 42}', encoding="utf-8")
            self.assertEqual({"answer": 42}, module.read_json_file_or_default(path))
            self.assertEqual({"answer": 42}, module.read_json_file_or_raise(path, ValueError("bad")))
            lines = ['{"answer": 42}\n', 'not-json\n', '\n', '{"answer": 7}\n']
            self.assertEqual([{"answer": 42}, {"answer": 7}], list(module.iter_json_objects(lines)))

    def test_write_json_file_helpers(self):
        module = load_module_from_path("contracts_surface_contracts", MODULE_PATH)

        with tempfile.TemporaryDirectory() as tmpdir:
            root = Path(tmpdir)
            payload = {"answer": 42, "nested": {"b": 2, "a": 1}}
            target = root / "sample.json"
            atomic_target = root / "atomic.json"

            module.write_json_file(target, payload)
            module.write_json_file_atomic(atomic_target, payload)
            jsonl_target = root / "sample.jsonl"
            jsonl_atomic_target = root / "sample-atomic.jsonl"
            rows = [{"b": 2, "a": 1}, {"c": 3}]
            module.write_jsonl_file(jsonl_target, rows)
            module.write_jsonl_file_atomic(jsonl_atomic_target, rows)

            self.assertEqual(payload, module.read_json_file_or_default(target))
            self.assertEqual(payload, module.read_json_file_or_default(atomic_target))
            self.assertEqual(rows, list(module.iter_json_objects(jsonl_target.read_text().splitlines())))
            self.assertEqual(rows, list(module.iter_json_objects(jsonl_atomic_target.read_text().splitlines())))
            self.assertFalse((root / "atomic.json.tmp").exists())

    def test_read_toml_file_with_loader(self):
        module = load_module_from_path("contracts_surface_contracts_toml", MODULE_PATH)

        class Loader:
            @staticmethod
            def loads(text):
                self.assertIn("answer", text)
                return {"answer": 42}

        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "sample.toml"
            path.write_text("answer = 42\n", encoding="utf-8")
            self.assertEqual({"answer": 42}, module.read_toml_file_with_loader(path, Loader))
        with self.assertRaises(RuntimeError):
            module.read_toml_file_with_loader("missing.toml", None)

    def test_read_yaml_file_with_loader(self):
        module = load_module_from_path("contracts_surface_contracts_yaml", MODULE_PATH)

        class Loader:
            @staticmethod
            def safe_load(text):
                self.assertIn("answer", text)
                return {"answer": 42}

        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "sample.yaml"
            path.write_text("answer: 42\n", encoding="utf-8")
            self.assertEqual({"answer": 42}, module.read_yaml_file_with_loader(path, Loader))
        with self.assertRaises(RuntimeError):
            module.read_yaml_file_with_loader("missing.yaml", None)

    def test_resolve_repo_root(self):
        module = load_module_from_path("contracts_project_root", Path(__file__).resolve().parent.parent / "project_root.py")
        root = module.resolve_repo_root(Path(__file__).resolve())

        self.assertTrue((root / "organs").exists())
        self.assertTrue((root / "infra").exists())

    def test_resolve_repo_root_can_prefer_runtime_mirror(self):
        module = load_module_from_path("contracts_project_root_runtime", Path(__file__).resolve().parent.parent / "project_root.py")
        with tempfile.TemporaryDirectory() as home:
            runtime_root = Path(home) / ".cynic" / "repos" / "CYNIC"
            (runtime_root / "organs").mkdir(parents=True)
            (runtime_root / "infra").mkdir()

            with mock.patch.dict(os.environ, {"HOME": home}, clear=False):
                root = module.resolve_repo_root(
                    Path(__file__).resolve(),
                    prefer_runtime_mirror=True,
                )

        self.assertEqual(runtime_root.resolve(), root)

    def test_resolve_repo_root_prefers_explicit_nonlocal_root_over_runtime_mirror(self):
        module = load_module_from_path("contracts_project_root_explicit", Path(__file__).resolve().parent.parent / "project_root.py")
        with tempfile.TemporaryDirectory() as home, tempfile.TemporaryDirectory() as explicit_root_dir:
            runtime_root = Path(home) / ".cynic" / "repos" / "CYNIC"
            (runtime_root / "organs").mkdir(parents=True)
            (runtime_root / "infra").mkdir()
            explicit_root = Path(explicit_root_dir)
            (explicit_root / "organs").mkdir()
            (explicit_root / "infra").mkdir()

            with mock.patch.dict(
                os.environ,
                {"HOME": home, "CYNIC_PROJECT_ROOT": str(explicit_root)},
                clear=False,
            ):
                root = module.resolve_repo_root(
                    Path(__file__).resolve(),
                    prefer_runtime_mirror=True,
                )

        self.assertEqual(explicit_root.resolve(), root)

    def test_resolve_repo_root_prefers_runtime_mirror_script_root_over_poisoned_explicit_root(self):
        module = load_module_from_path("contracts_project_root_runtime_script", Path(__file__).resolve().parent.parent / "project_root.py")
        with tempfile.TemporaryDirectory() as home, tempfile.TemporaryDirectory() as explicit_root_dir:
            runtime_root = Path(home) / ".cynic" / "repos" / "CYNIC"
            (runtime_root / "organs" / "forge" / "src").mkdir(parents=True)
            (runtime_root / "infra").mkdir()
            explicit_root = Path(explicit_root_dir)
            (explicit_root / "organs").mkdir()
            (explicit_root / "infra").mkdir()
            runtime_script = runtime_root / "organs" / "forge" / "src" / "cli.py"
            runtime_script.write_text("# runtime script\n", encoding="utf-8")

            with mock.patch.dict(
                os.environ,
                {"HOME": home, "CYNIC_PROJECT_ROOT": str(explicit_root)},
                clear=False,
            ):
                root = module.resolve_repo_root(runtime_script)

        self.assertEqual(runtime_root.resolve(), root)

    def test_bootstrap_project_root(self):
        module = load_module_from_path("contracts_project_root", Path(__file__).resolve().parent.parent / "project_root.py")
        root = module.bootstrap_project_root(__file__)

        self.assertTrue((root / "organs").exists())
        self.assertTrue((root / "infra").exists())


if __name__ == "__main__":
    unittest.main()
