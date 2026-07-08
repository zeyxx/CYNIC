"""Static source inventory scanner for a Hermes Agent checkout."""

from __future__ import annotations

import ast
from pathlib import Path
import re
from typing import Any

from core.source_inventory import build_source_inventory_payload


SKIP_DIRS = frozenset({".git", ".venv", "node_modules", "__pycache__", ".pytest_cache"})


class HermesSourceInventory:
    def scan(self, source_root: Path) -> dict[str, Any]:
        if not source_root.exists():
            return build_source_inventory_payload(
                framework="hermes-agent",
                source_root=str(source_root),
                observed={"coverage_gaps": [{"kind": "missing_source_root", "path": str(source_root)}]},
            )

        observed = {
            "top_level_directories": self._top_level_directories(source_root),
            "python_modules": self._python_modules(source_root),
            "cli_commands": self._cli_commands(source_root),
            "toolsets": self._toolsets(source_root),
            "tools": self._registered_tools(source_root),
            "plugins": self._named_children(source_root / "plugins"),
            "skills": self._skill_dirs(source_root),
            "optional_mcps": self._named_children(source_root / "optional-mcps"),
            "providers": self._providers(source_root),
            "primitives": self._primitives(source_root),
            "layers": self._layers(source_root),
            "coverage_gaps": [],
        }
        return build_source_inventory_payload(
            framework="hermes-agent",
            source_root=str(source_root),
            observed=observed,
        )

    @staticmethod
    def _top_level_directories(root: Path) -> list[str]:
        return sorted(
            item.name
            for item in root.iterdir()
            if item.is_dir() and not item.name.startswith(".") and item.name not in SKIP_DIRS
        )

    @staticmethod
    def _python_modules(root: Path) -> list[str]:
        modules: list[str] = []
        for path in sorted(root.rglob("*.py")):
            if any(part in SKIP_DIRS for part in path.parts):
                continue
            try:
                rel = path.relative_to(root)
            except ValueError:
                rel = path
            modules.append(str(rel))
        return modules

    def _cli_commands(self, root: Path) -> list[dict[str, Any]]:
        commands: dict[str, dict[str, Any]] = {}
        main = root / "hermes_cli" / "main.py"
        if main.exists():
            text = main.read_text(encoding="utf-8", errors="replace")
            for line in text.splitlines():
                match = re.match(r"\s*hermes\s+([a-zA-Z0-9_-]+)(?:\s+([a-zA-Z0-9_-]+))?", line)
                if not match:
                    continue
                command = match.group(1)
                subcommand = match.group(2)
                commands.setdefault(command, {"name": command, "source": "main_usage", "subcommands": set()})
                if subcommand and not subcommand.startswith("--"):
                    commands[command]["subcommands"].add(subcommand)

        subcommands_dir = root / "hermes_cli" / "subcommands"
        if subcommands_dir.exists():
            for path in sorted(subcommands_dir.glob("*.py")):
                if path.name.startswith("_") or path.name == "__init__.py":
                    continue
                command = path.stem.replace("_cmd", "").replace("_", "-")
                commands.setdefault(command, {"name": command, "source": "subcommand_module", "subcommands": set()})
                for parser_name in self._argparse_add_parser_names(path):
                    commands[command]["subcommands"].add(parser_name)

        return [
            {
                "name": value["name"],
                "source": value["source"],
                "subcommands": sorted(value["subcommands"]),
            }
            for value in sorted(commands.values(), key=lambda item: item["name"])
        ]

    def _toolsets(self, root: Path) -> list[dict[str, Any]]:
        path = root / "toolsets.py"
        if not path.exists():
            return []
        try:
            tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        except SyntaxError:
            return []

        constants: dict[str, list[str]] = {}
        toolsets: list[dict[str, Any]] = []
        for node in tree.body:
            if isinstance(node, ast.Assign):
                for target in node.targets:
                    if isinstance(target, ast.Name) and target.id.startswith("_"):
                        values = self._string_list(node.value, constants)
                        if values:
                            constants[target.id] = values
                    if isinstance(target, ast.Name) and target.id == "TOOLSETS" and isinstance(node.value, ast.Dict):
                        for key, value in zip(node.value.keys, node.value.values, strict=False):
                            if not isinstance(key, ast.Constant) or not isinstance(key.value, str):
                                continue
                            item = self._dict_node(value)
                            tools = self._string_list(item.get("tools"), constants)
                            includes = self._string_list(item.get("includes"), constants)
                            description = self._string_value(item.get("description"))
                            toolsets.append(
                                {
                                    "name": key.value,
                                    "description": description,
                                    "tools": tools,
                                    "tool_count": len(tools),
                                    "includes": includes,
                                }
                            )
        return sorted(toolsets, key=lambda item: item["name"])

    def _registered_tools(self, root: Path) -> list[dict[str, Any]]:
        tools: list[dict[str, Any]] = []
        for path in sorted((root / "tools").glob("*.py")) if (root / "tools").exists() else []:
            try:
                tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
            except SyntaxError:
                continue
            for node in ast.walk(tree):
                if not isinstance(node, ast.Call) or not self._is_registry_register(node):
                    continue
                kwargs = {kw.arg: kw.value for kw in node.keywords if kw.arg}
                name = self._string_value(kwargs.get("name"))
                toolset = self._string_value(kwargs.get("toolset"))
                if name:
                    tools.append({"name": name, "toolset": toolset, "source": str(path.relative_to(root))})
        return sorted(tools, key=lambda item: (item["toolset"], item["name"]))

    @staticmethod
    def _named_children(root: Path) -> list[str]:
        if not root.exists():
            return []
        return sorted(
            item.name
            for item in root.iterdir()
            if item.is_dir() and not item.name.startswith(".") and item.name not in SKIP_DIRS
        )

    @classmethod
    def _providers(cls, root: Path) -> list[str]:
        names = set(cls._named_children(root / "providers"))
        names.update(cls._named_children(root / "plugins" / "model-providers"))
        names.update(f"image:{name}" for name in cls._named_children(root / "plugins" / "image_gen"))
        names.update(f"video:{name}" for name in cls._named_children(root / "plugins" / "video_gen"))
        return sorted(names)

    @staticmethod
    def _skill_dirs(root: Path) -> list[str]:
        skills: set[str] = set()
        for base_name in ("skills", "optional-skills"):
            base = root / base_name
            if not base.exists():
                continue
            for skill in sorted(base.rglob("SKILL.md")):
                try:
                    skills.add(str(skill.parent.relative_to(base)))
                except ValueError:
                    continue
        return sorted(skills)

    @staticmethod
    def _primitives(root: Path) -> list[str]:
        candidates = {
            "agent_loop": root / "agent",
            "tool_registry": root / "tools" / "registry.py",
            "toolsets": root / "toolsets.py",
            "mcp_bridge": root / "tools" / "mcp_tool.py",
            "gateway": root / "gateway",
            "cron_scheduler": root / "cron",
            "plugin_runtime": root / "plugins",
            "skill_runtime": root / "skills",
            "optional_skill_catalog": root / "optional-skills",
            "provider_runtime": root / "providers",
            "tui_gateway": root / "tui_gateway",
            "web_dashboard": root / "web",
            "desktop_app": root / "apps" / "desktop",
            "acp_adapter": root / "acp_adapter",
            "terminal_environments": root / "tools" / "environments",
            "source_tests": root / "tests",
        }
        return sorted(name for name, path in candidates.items() if path.exists())

    @staticmethod
    def _layers(root: Path) -> list[dict[str, Any]]:
        layer_specs = (
            ("interface", ("hermes_cli", "ui-tui", "tui_gateway", "web", "apps")),
            ("agent_runtime", ("agent", "run_agent.py", "model_tools.py")),
            ("tool_runtime", ("tools", "toolsets.py", "optional-mcps")),
            ("extension_runtime", ("plugins", "skills", "optional-skills")),
            ("transport_gateway", ("gateway",)),
            ("automation", ("cron", "batch_runner.py")),
            ("provider_runtime", ("providers",)),
            ("packaging_deployment", ("docker", "nix", "packaging", "scripts")),
            ("evaluation_research", ("tests", "datagen-config-examples", "trajectory_compressor.py")),
        )
        layers: list[dict[str, Any]] = []
        for name, paths in layer_specs:
            evidence = [path for path in paths if (root / path).exists()]
            if evidence:
                layers.append({"id": name, "evidence": evidence})
        return layers

    @staticmethod
    def _argparse_add_parser_names(path: Path) -> list[str]:
        try:
            tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        except SyntaxError:
            return []
        names: set[str] = set()
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue
            func = node.func
            if isinstance(func, ast.Attribute) and func.attr == "add_parser" and node.args:
                value = node.args[0]
                if isinstance(value, ast.Constant) and isinstance(value.value, str):
                    names.add(value.value)
        return sorted(names)

    @staticmethod
    def _is_registry_register(node: ast.Call) -> bool:
        func = node.func
        return (
            isinstance(func, ast.Attribute)
            and func.attr == "register"
            and isinstance(func.value, ast.Name)
            and func.value.id == "registry"
        )

    @staticmethod
    def _dict_node(node: ast.AST | None) -> dict[str, ast.AST]:
        if not isinstance(node, ast.Dict):
            return {}
        result: dict[str, ast.AST] = {}
        for key, value in zip(node.keys, node.values, strict=False):
            if isinstance(key, ast.Constant) and isinstance(key.value, str):
                result[key.value] = value
        return result

    @classmethod
    def _string_list(cls, node: ast.AST | None, constants: dict[str, list[str]]) -> list[str]:
        if isinstance(node, ast.Name):
            return list(constants.get(node.id, []))
        if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Add):
            return cls._string_list(node.left, constants) + cls._string_list(node.right, constants)
        if isinstance(node, (ast.List, ast.Tuple)):
            values: list[str] = []
            for item in node.elts:
                if isinstance(item, ast.Constant) and isinstance(item.value, str):
                    values.append(item.value)
                elif isinstance(item, ast.Name):
                    values.extend(constants.get(item.id, []))
            return values
        return []

    @staticmethod
    def _string_value(node: ast.AST | None) -> str:
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            return node.value
        if isinstance(node, ast.JoinedStr):
            return ""
        return ""
