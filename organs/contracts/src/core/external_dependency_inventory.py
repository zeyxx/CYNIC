"""Framework-agnostic external dependency inventory contracts."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from organs.contracts.src.core.agent_framework_capabilities import utc_now_iso


EXTERNAL_DEPENDENCY_INVENTORY_SCHEMA = "cynic.external_dependency_inventory.v1"


@dataclass(frozen=True)
class ExternalDependency:
    ecosystem: str
    manifest: str
    group: str
    name: str
    specifier: str
    scope: str

    def __post_init__(self) -> None:
        missing = [
            field
            for field, value in (
                ("ecosystem", self.ecosystem),
                ("manifest", self.manifest),
                ("group", self.group),
                ("name", self.name),
                ("scope", self.scope),
            )
            if not value.strip()
        ]
        if missing:
            raise RuntimeError(f"external dependency missing fields: {','.join(missing)}")

    def to_dict(self) -> dict[str, str]:
        return {
            "ecosystem": self.ecosystem,
            "manifest": self.manifest,
            "group": self.group,
            "name": self.name,
            "specifier": self.specifier,
            "scope": self.scope,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "ExternalDependency":
        return cls(
            ecosystem=str(data.get("ecosystem", "")).strip(),
            manifest=str(data.get("manifest", "")).strip(),
            group=str(data.get("group", "")).strip(),
            name=str(data.get("name", "")).strip(),
            specifier=str(data.get("specifier", "")).strip(),
            scope=str(data.get("scope", "")).strip(),
        )


@dataclass(frozen=True)
class ExternalDependencyInventory:
    observed_at: str
    scan_roots: tuple[str, ...]
    dependencies: tuple[ExternalDependency, ...]
    manifests: tuple[dict[str, Any], ...] = ()
    coverage_gaps: tuple[dict[str, Any], ...] = ()

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema": EXTERNAL_DEPENDENCY_INVENTORY_SCHEMA,
            "observed_at": self.observed_at,
            "scan_roots": list(self.scan_roots),
            "manifest_count": len(self.manifests),
            "dependency_count": len(self.dependencies),
            "unsupported_manifest_count": len(
                [item for item in self.coverage_gaps if item.get("kind") == "unsupported_manifest"]
            ),
            "dependencies": [dependency.to_dict() for dependency in self.dependencies],
            "manifests": list(self.manifests),
            "coverage_gaps": list(self.coverage_gaps),
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "ExternalDependencyInventory":
        if data.get("schema") != EXTERNAL_DEPENDENCY_INVENTORY_SCHEMA:
            raise RuntimeError("expected cynic.external_dependency_inventory.v1")
        dependencies = data.get("dependencies", ())
        manifests = data.get("manifests", ())
        coverage_gaps = data.get("coverage_gaps", ())
        if not isinstance(dependencies, list):
            raise RuntimeError("external dependency inventory dependencies must be a list")
        return cls(
            observed_at=str(data.get("observed_at", "") or utc_now_iso()),
            scan_roots=tuple(str(item) for item in data.get("scan_roots", ())),
            dependencies=tuple(
                ExternalDependency.from_dict(item)
                for item in dependencies
                if isinstance(item, dict)
            ),
            manifests=tuple(item for item in manifests if isinstance(item, dict)),
            coverage_gaps=tuple(item for item in coverage_gaps if isinstance(item, dict)),
        )


def sorted_dependencies(dependencies: list[ExternalDependency]) -> tuple[ExternalDependency, ...]:
    return tuple(sorted(dependencies, key=lambda item: (item.ecosystem, item.manifest, item.group, item.name)))
