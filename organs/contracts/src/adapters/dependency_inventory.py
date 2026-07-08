"""Manifest-backed dependency inventory scanner."""

from __future__ import annotations

import json
from pathlib import Path
import re
import tomllib
from typing import Any

from organs.contracts.src.core.agent_framework_capabilities import utc_now_iso
from organs.contracts.src.core.external_dependency_inventory import (
    ExternalDependency,
    ExternalDependencyInventory,
    sorted_dependencies,
)


MANIFEST_NAMES = frozenset({"pyproject.toml", "requirements.txt", "package.json", "Cargo.toml"})
SKIP_DIRS = frozenset({".git", ".venv", "node_modules", "target", "__pycache__"})


class DependencyInventoryScanner:
    def __init__(self, roots: tuple[Path, ...]) -> None:
        self.roots = roots

    def scan(self, observed_at: str | None = None) -> ExternalDependencyInventory:
        dependencies: list[ExternalDependency] = []
        manifests: list[dict[str, Any]] = []
        gaps: list[dict[str, Any]] = []
        for root in self.roots:
            if not root.exists():
                gaps.append({"kind": "missing_root", "path": str(root)})
                continue
            for manifest in self._manifest_paths(root):
                try:
                    parsed_dependencies = self._parse_manifest(root, manifest)
                except (OSError, ValueError, tomllib.TOMLDecodeError, json.JSONDecodeError) as exc:
                    gaps.append({"kind": "parse_error", "path": str(manifest), "error": str(exc)})
                    continue
                manifests.append(
                    {
                        "path": self._relative(root, manifest),
                        "kind": self._manifest_kind(manifest),
                        "dependency_count": len(parsed_dependencies),
                    }
                )
                dependencies.extend(parsed_dependencies)

        return ExternalDependencyInventory(
            observed_at=observed_at or utc_now_iso(),
            scan_roots=tuple(str(root) for root in self.roots),
            dependencies=sorted_dependencies(dependencies),
            manifests=tuple(sorted(manifests, key=lambda item: str(item.get("path", "")))),
            coverage_gaps=tuple(gaps),
        )

    def _manifest_paths(self, root: Path) -> tuple[Path, ...]:
        if root.is_file():
            return (root,) if root.name in MANIFEST_NAMES else ()
        found: list[Path] = []
        for path in root.rglob("*"):
            if any(part in SKIP_DIRS for part in path.parts):
                continue
            if path.is_file() and path.name in MANIFEST_NAMES:
                found.append(path)
        return tuple(sorted(found))

    def _parse_manifest(self, root: Path, manifest: Path) -> list[ExternalDependency]:
        if manifest.name == "pyproject.toml":
            return self._parse_pyproject(root, manifest)
        if manifest.name == "package.json":
            return self._parse_package_json(root, manifest)
        if manifest.name == "Cargo.toml":
            return self._parse_cargo_toml(root, manifest)
        if manifest.name == "requirements.txt":
            return self._parse_requirements(root, manifest)
        return []

    def _parse_pyproject(self, root: Path, manifest: Path) -> list[ExternalDependency]:
        payload = tomllib.loads(manifest.read_text(encoding="utf-8"))
        project = payload.get("project", {})
        dependencies: list[ExternalDependency] = []
        for raw in project.get("dependencies", []) if isinstance(project, dict) else []:
            dependencies.append(self._python_dependency(root, manifest, "project.dependencies", str(raw), "runtime"))
        optional = project.get("optional-dependencies", {}) if isinstance(project, dict) else {}
        if isinstance(optional, dict):
            for extra, items in optional.items():
                if not isinstance(items, list):
                    continue
                for raw in items:
                    dependencies.append(
                        self._python_dependency(root, manifest, f"project.optional-dependencies.{extra}", str(raw), "optional")
                    )
        return dependencies

    def _parse_package_json(self, root: Path, manifest: Path) -> list[ExternalDependency]:
        payload = json.loads(manifest.read_text(encoding="utf-8"))
        dependencies: list[ExternalDependency] = []
        groups = (
            ("dependencies", "runtime"),
            ("devDependencies", "dev"),
            ("peerDependencies", "peer"),
            ("optionalDependencies", "optional"),
            ("overrides", "override"),
        )
        for group, scope in groups:
            values = payload.get(group, {})
            if not isinstance(values, dict):
                continue
            for name, specifier in values.items():
                if isinstance(specifier, dict):
                    specifier = json.dumps(specifier, sort_keys=True)
                dependencies.append(
                    ExternalDependency(
                        ecosystem="node",
                        manifest=self._relative(root, manifest),
                        group=group,
                        name=str(name),
                        specifier=str(specifier),
                        scope=scope,
                    )
                )
        return dependencies

    def _parse_cargo_toml(self, root: Path, manifest: Path) -> list[ExternalDependency]:
        payload = tomllib.loads(manifest.read_text(encoding="utf-8"))
        dependencies: list[ExternalDependency] = []
        for group, scope in (
            ("dependencies", "runtime"),
            ("dev-dependencies", "dev"),
            ("build-dependencies", "build"),
        ):
            values = payload.get(group, {})
            if not isinstance(values, dict):
                continue
            for name, specifier in values.items():
                dependencies.append(
                    ExternalDependency(
                        ecosystem="rust",
                        manifest=self._relative(root, manifest),
                        group=group,
                        name=str(name),
                        specifier=self._cargo_specifier(specifier),
                        scope=scope,
                    )
                )
        return dependencies

    def _parse_requirements(self, root: Path, manifest: Path) -> list[ExternalDependency]:
        dependencies: list[ExternalDependency] = []
        for line in manifest.read_text(encoding="utf-8").splitlines():
            raw = line.strip()
            if not raw or raw.startswith("#") or raw.startswith("-"):
                continue
            dependencies.append(self._python_dependency(root, manifest, "requirements", raw, "runtime"))
        return dependencies

    def _python_dependency(self, root: Path, manifest: Path, group: str, raw: str, scope: str) -> ExternalDependency:
        name, specifier = split_python_requirement(raw)
        return ExternalDependency(
            ecosystem="python",
            manifest=self._relative(root, manifest),
            group=group,
            name=name,
            specifier=specifier,
            scope=scope,
        )

    @staticmethod
    def _cargo_specifier(value: Any) -> str:
        if isinstance(value, str):
            return value
        if isinstance(value, dict):
            return json.dumps(value, sort_keys=True)
        return str(value)

    @staticmethod
    def _manifest_kind(path: Path) -> str:
        return {
            "pyproject.toml": "python_pyproject",
            "requirements.txt": "python_requirements",
            "package.json": "node_package_json",
            "Cargo.toml": "rust_cargo_toml",
        }.get(path.name, "unknown")

    @staticmethod
    def _relative(root: Path, path: Path) -> str:
        try:
            return str(path.relative_to(root))
        except ValueError:
            return str(path)


def split_python_requirement(raw: str) -> tuple[str, str]:
    clean = raw.split(";", 1)[0].strip()
    clean = clean.split("[", 1)[0].strip() if "[" in clean else clean
    match = re.match(r"^([A-Za-z0-9_.-]+)\s*(.*)$", clean)
    if not match:
        return raw, ""
    return match.group(1), match.group(2).strip()
