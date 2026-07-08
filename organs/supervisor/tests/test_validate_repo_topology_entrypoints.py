import tempfile
import unittest
from pathlib import Path

try:
    from ._test_utils import load_module_from_path
except ImportError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from _test_utils import load_module_from_path


MODULE_PATH = Path(__file__).resolve().parent.parent / "controllers" / "validate_repo_topology.py"


def load_module():
    return load_module_from_path("validate_repo_topology", MODULE_PATH)


class EntrypointTopologyPolicyTests(unittest.TestCase):
    def test_rejects_shell_file_entrypoint_anywhere_in_source_tree(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = root / "organs" / "twitter" / "bin" / "launch.sh"
            script.parent.mkdir(parents=True)
            script.write_text("#!/bin/sh\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_prohibited_script_entry_files(
                    {
                        "prohibited_script_entry_files": {
                            "suffixes": [".sh"],
                            "shebang_interpreters": ["sh", "bash"],
                            "owner": "cortex",
                            "replacement": "organ entrypoint",
                            "expiry_condition": "no shell files remain",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual(["prohibited_script_entry_file"], [item.kind for item in findings])

    def test_ignores_shell_files_inside_build_cache_directories(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = root / "target" / "debug" / "build.sh"
            script.parent.mkdir(parents=True)
            script.write_text("#!/bin/sh\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_prohibited_script_entry_files(
                    {
                        "prohibited_script_entry_files": {
                            "suffixes": [".sh"],
                            "shebang_interpreters": ["sh", "bash"],
                            "owner": "cortex",
                            "replacement": "organ entrypoint",
                            "expiry_condition": "no shell files remain",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_shell_shebang_entrypoint_without_shell_suffix(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = root / "apps" / "kyon-mobile" / "gradlew"
            script.parent.mkdir(parents=True)
            script.write_text("#!/bin/sh\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_prohibited_script_entry_files(
                    {
                        "prohibited_script_entry_files": {
                            "suffixes": [".sh"],
                            "shebang_interpreters": ["sh", "bash"],
                            "owner": "cortex",
                            "replacement": "organ entrypoint",
                            "expiry_condition": "no shell files remain",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual(["prohibited_script_entry_file"], [item.kind for item in findings])

    def test_rejects_script_entrypoint_reference_in_active_operator_document(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            doc = root / "docs" / "ops" / "RUNBOOK.md"
            doc.parent.mkdir(parents=True)
            doc.write_text("Run python3 scripts/repair.py when storage fails.\n", encoding="utf-8")
            archive = root / "docs" / "archive" / "OLD.md"
            archive.parent.mkdir(parents=True)
            archive.write_text("Historical note: ./scripts/old.sh\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_active_document_script_entrypoint_policy(
                    {
                        "active_document_script_entrypoint_policy": {
                            "owner": "cortex",
                            "scan_paths": ["docs/ops/"],
                            "ignored_globs": ["docs/archive/**"],
                            "forbidden_patterns": [
                                r"\b[\w./~-]+\.sh\b",
                                r"(^|[\s`(])(?:python3|python|bash|sh|node|npx|tsx)?\s*(?:\./)?scripts/[\w./-]+",
                            ],
                            "replacement": "organ entrypoint",
                            "expiry_condition": "no active doc script refs",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual(["active_document_script_entrypoint_reference"], [item.kind for item in findings])

    def test_allows_active_document_without_script_entrypoint_reference(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            doc = root / "docs" / "ops" / "RUNBOOK.md"
            doc.parent.mkdir(parents=True)
            doc.write_text("Run python3 organs/keep/src/entrypoints/cli.py storage-primary status.\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_active_document_script_entrypoint_policy(
                    {
                        "active_document_script_entrypoint_policy": {
                            "owner": "cortex",
                            "scan_paths": ["docs/ops/"],
                            "ignored_globs": [],
                            "forbidden_patterns": [
                                r"\b[\w./~-]+\.sh\b",
                                r"(^|[\s`(])(?:python3|python|bash|sh|node|npx|tsx)?\s*(?:\./)?scripts/[\w./-]+",
                            ],
                            "replacement": "organ entrypoint",
                            "expiry_condition": "no active doc script refs",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_tracked_executable_source_file_without_allowlist(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "organs" / "forge" / "src" / "dispatch.py"
            source.parent.mkdir(parents=True)
            source.write_text("print('dispatch')\n", encoding="utf-8")
            source.chmod(0o755)
            old_values = (module.WORKSPACE_ROOT, module.git_ls_files)
            try:
                module.WORKSPACE_ROOT = root
                module.git_ls_files = lambda *args: ["organs/forge/src/dispatch.py"]
                findings = module.validate_tracked_executable_file_policy(
                    {
                        "tracked_executable_file_policy": {
                            "owner": "cortex",
                            "allowed_paths": [],
                            "replacement": "explicit interpreter invocation",
                            "expiry_condition": "no executable tracked source files",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.git_ls_files = old_values

        self.assertEqual(["tracked_executable_file_not_allowlisted"], [item.kind for item in findings])

    def test_allows_tracked_executable_file_when_explicitly_allowlisted(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            binary = root / "cynic-lxc-bridge" / "cynic-lxc-exec-bridge"
            binary.parent.mkdir(parents=True)
            binary.write_text("binary fixture\n", encoding="utf-8")
            binary.chmod(0o755)
            old_values = (module.WORKSPACE_ROOT, module.git_ls_files)
            try:
                module.WORKSPACE_ROOT = root
                module.git_ls_files = lambda *args: ["cynic-lxc-bridge/cynic-lxc-exec-bridge"]
                findings = module.validate_tracked_executable_file_policy(
                    {
                        "tracked_executable_file_policy": {
                            "owner": "cortex",
                            "allowed_paths": ["cynic-lxc-bridge/cynic-lxc-exec-bridge"],
                            "replacement": "explicit binary artifact",
                            "expiry_condition": "binary remains intentionally allowlisted",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.git_ls_files = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_tracked_top_level_source_file_without_allowlist(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "get_token_info.py"
            source.write_text("print('token')\n", encoding="utf-8")
            old_values = (module.WORKSPACE_ROOT, module.git_ls_files)
            try:
                module.WORKSPACE_ROOT = root
                module.git_ls_files = lambda *args: ["get_token_info.py"]
                findings = module.validate_top_level_source_file_policy(
                    {
                        "top_level_source_file_policy": {
                            "owner": "cortex",
                            "source_suffixes": [".py"],
                            "allowed_paths": [],
                            "replacement": "owned organ entrypoint",
                            "expiry_condition": "no root source files",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.git_ls_files = old_values

        self.assertEqual(["top_level_source_file_not_allowlisted"], [item.kind for item in findings])

    def test_allows_tracked_top_level_config_file(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            config = root / "Cargo.toml"
            config.write_text("[workspace]\n", encoding="utf-8")
            old_values = (module.WORKSPACE_ROOT, module.git_ls_files)
            try:
                module.WORKSPACE_ROOT = root
                module.git_ls_files = lambda *args: ["Cargo.toml"]
                findings = module.validate_top_level_source_file_policy(
                    {
                        "top_level_source_file_policy": {
                            "owner": "cortex",
                            "source_suffixes": [".py"],
                            "allowed_paths": [],
                            "replacement": "owned organ entrypoint",
                            "expiry_condition": "no root source files",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT, module.git_ls_files = old_values

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_generated_dependency_directory_inside_organ(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            generated = root / "organs" / "jupyter" / ".venv"
            generated.mkdir(parents=True)
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_generated_dependency_directory_policy(
                    {
                        "generated_dependency_directory_policy": {
                            "owner": "cortex",
                            "directory_names": [".venv", "node_modules"],
                            "scan_roots": ["organs/"],
                            "allowed_paths": [],
                            "replacement": "external dependency cache",
                            "expiry_condition": "no generated dirs inside source membranes",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual(["generated_dependency_directory_in_source_tree"], [item.kind for item in findings])

    def test_rejects_python_cache_directory_inside_entrypoints(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            generated = root / "organs" / "twitter" / "src" / "entrypoints" / "__pycache__"
            generated.mkdir(parents=True)
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_generated_dependency_directory_policy(
                    {
                        "generated_dependency_directory_policy": {
                            "owner": "cortex",
                            "directory_names": [".venv", "node_modules", "__pycache__"],
                            "scan_roots": ["organs/"],
                            "allowed_paths": [],
                            "replacement": "external dependency cache",
                            "expiry_condition": "no generated dirs inside source membranes",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual(["generated_dependency_directory_in_source_tree"], [item.kind for item in findings])

    def test_allows_generated_dependency_directory_outside_scanned_roots(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            generated = root / "node_modules"
            generated.mkdir()
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_generated_dependency_directory_policy(
                    {
                        "generated_dependency_directory_policy": {
                            "owner": "cortex",
                            "directory_names": [".venv", "node_modules"],
                            "scan_roots": ["organs/"],
                            "allowed_paths": [],
                            "replacement": "external dependency cache",
                            "expiry_condition": "no generated dirs inside source membranes",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_large_entrypoint_without_declared_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            entrypoint = root / "organs" / "twitter" / "src" / "entrypoints" / "x_poster.py"
            entrypoint.parent.mkdir(parents=True)
            entrypoint.write_text("\n".join("print('logic')" for _ in range(4)), encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_entrypoint_thin_membrane_policy(
                    {
                        "entrypoint_thin_membrane_policy": {
                            "owner": "cortex",
                            "scan_globs": ["organs/*/src/entrypoints/*.py"],
                            "max_lines": 3,
                            "forbid_executable_bit": True,
                            "exceptions": [],
                            "replacement": "thin organ entrypoint",
                            "expiry_condition": "entrypoint delegates to core",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual(["entrypoint_thin_membrane_violation"], [item.kind for item in findings])

    def test_allows_large_entrypoint_with_declared_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            entrypoint = root / "organs" / "twitter" / "src" / "entrypoints" / "x_poster.py"
            entrypoint.parent.mkdir(parents=True)
            entrypoint.write_text("\n".join("print('logic')" for _ in range(4)), encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_entrypoint_thin_membrane_policy(
                    {
                        "entrypoint_thin_membrane_policy": {
                            "owner": "cortex",
                            "scan_globs": ["organs/*/src/entrypoints/*.py"],
                            "max_lines": 3,
                            "forbid_executable_bit": True,
                            "exceptions": [
                                {
                                    "path": "organs/twitter/src/entrypoints/x_poster.py",
                                    "owner": "organ-twitter",
                                    "line_count": 4,
                                    "replacement": "move logic to core",
                                    "expiry_condition": "thin entrypoint",
                                }
                            ],
                            "replacement": "thin organ entrypoint",
                            "expiry_condition": "entrypoint delegates to core",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual([], [item for item in findings if item.severity == "critical"])

    def test_rejects_stale_large_entrypoint_exception(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            entrypoint = root / "organs" / "twitter" / "src" / "entrypoints" / "x_poster.py"
            entrypoint.parent.mkdir(parents=True)
            entrypoint.write_text("print('thin')\n", encoding="utf-8")
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_entrypoint_thin_membrane_policy(
                    {
                        "entrypoint_thin_membrane_policy": {
                            "owner": "cortex",
                            "scan_globs": ["organs/*/src/entrypoints/*.py"],
                            "max_lines": 3,
                            "forbid_executable_bit": True,
                            "exceptions": [
                                {
                                    "path": "organs/twitter/src/entrypoints/x_poster.py",
                                    "owner": "organ-twitter",
                                    "replacement": "move logic to core",
                                    "expiry_condition": "thin entrypoint",
                                }
                            ],
                            "replacement": "thin organ entrypoint",
                            "expiry_condition": "entrypoint delegates to core",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual(["entrypoint_thin_membrane_exception_stale"], [item.kind for item in findings])

    def test_rejects_executable_entrypoint_even_when_thin(self):
        module = load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            entrypoint = root / "organs" / "fleet" / "src" / "entrypoints" / "cli.py"
            entrypoint.parent.mkdir(parents=True)
            entrypoint.write_text("print('thin')\n", encoding="utf-8")
            entrypoint.chmod(0o755)
            old_root = module.WORKSPACE_ROOT
            try:
                module.WORKSPACE_ROOT = root
                findings = module.validate_entrypoint_thin_membrane_policy(
                    {
                        "entrypoint_thin_membrane_policy": {
                            "owner": "cortex",
                            "scan_globs": ["organs/*/src/entrypoints/*.py"],
                            "max_lines": 3,
                            "forbid_executable_bit": True,
                            "exceptions": [],
                            "replacement": "thin organ entrypoint",
                            "expiry_condition": "entrypoint delegates to core",
                        }
                    }
                )
            finally:
                module.WORKSPACE_ROOT = old_root

        self.assertEqual(["entrypoint_executable_bit_violation"], [item.kind for item in findings])
