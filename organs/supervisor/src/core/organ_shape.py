"""Organ shape and manifest policy validation."""

from __future__ import annotations

import subprocess
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml


FindingFactory = Callable[[str, str, str, str], Any]


@dataclass(frozen=True)
class OrganShapeService:
    workspace_root: Path
    organs_root: Path
    contract_path: Path
    manifest_script_reference_suffixes: set[str]
    manifest_untracked_script_allowed_statuses: set[str]
    manifest_untracked_script_required_fields: set[str]
    load_yaml: Callable[[Path], Any]
    relative: Callable[[Path], str]
    git_ls_files: Callable[..., list[str]]
    git_ls_untracked: Callable[..., list[str]]
    declared_paths: Callable[[dict[str, Any], str], set[str]]
    validate_exception_metadata: Callable[..., list[Any]]
    finding: FindingFactory

    def validate_organ_boundaries(self, contract: dict[str, Any]) -> list[Any]:
        findings: list[Any] = []
        if not self.organs_root.exists():
            return findings

        exception_paths = self.declared_paths(contract, "organ_boundary_exceptions")
        findings.extend(
            self.validate_exception_metadata(
                contract,
                "organ_boundary_exceptions",
                {"path", "status", "owner", "replacement", "expiry_condition"},
                directory_paths=True,
            )
        )

        for organ_dir in sorted(path for path in self.organs_root.iterdir() if path.is_dir()):
            rel = self.relative(organ_dir).rstrip("/") + "/"
            if (organ_dir / "MANIFEST.yaml").exists():
                continue
            if rel in exception_paths:
                continue
            findings.append(
                self.finding(
                    "critical",
                    "organ_without_manifest",
                    rel,
                    "Every persistent organ directory needs MANIFEST.yaml or an explicit organ_boundary_exceptions entry.",
                )
            )

        for rel in sorted(exception_paths):
            if not (self.workspace_root / rel).exists():
                findings.append(
                    self.finding(
                        "medium",
                        "declared_organ_boundary_exception_missing",
                        rel,
                        "Declared organ boundary exception does not exist in the current tree.",
                    )
                )
        return findings

    def validate_organ_manifest_policy(self, contract: dict[str, Any]) -> list[Any]:
        findings: list[Any] = []
        if not self.organs_root.exists():
            return findings

        policy = contract.get("organ_shape", {}).get("manifest_policy", {})
        identity_fields = policy.get("identity_fields_any", ["id", "name"])
        required_fields = policy.get("required_fields", ["status", "description"])
        allowed_statuses = {str(item).lower() for item in policy.get("allowed_statuses", [])}
        case_insensitive = policy.get("status_case") == "case_insensitive"

        for manifest in sorted(self.organs_root.glob("*/MANIFEST.yaml")):
            rel = self.relative(manifest)
            try:
                payload = self.load_yaml(manifest) or {}
            except yaml.YAMLError as exc:
                findings.append(self.finding("critical", "organ_manifest_yaml_invalid", rel, str(exc)))
                continue
            if not isinstance(payload, dict):
                findings.append(
                    self.finding(
                        "critical",
                        "organ_manifest_not_mapping",
                        rel,
                        "MANIFEST.yaml must be a YAML mapping.",
                    )
                )
                continue
            if not any(payload.get(field) for field in identity_fields):
                findings.append(
                    self.finding(
                        "critical",
                        "organ_manifest_identity_missing",
                        rel,
                        f"MANIFEST.yaml must declare at least one identity field from {identity_fields}.",
                    )
                )
            for field in required_fields:
                if not payload.get(field):
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_manifest_required_field_missing",
                            rel,
                            f"MANIFEST.yaml must declare {field}.",
                        )
                    )
            status = payload.get("status")
            status_value = str(status).lower() if case_insensitive else str(status)
            if status and allowed_statuses and status_value not in allowed_statuses:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_manifest_status_unknown",
                        rel,
                        f"status {status!r} is not declared in organ_shape.manifest_policy.allowed_statuses.",
                    )
                )
        return findings

    def validate_organ_manifest_state_paths(self, contract: dict[str, Any]) -> list[Any]:
        findings: list[Any] = []
        if not self.organs_root.exists():
            return findings

        state_policy = contract.get("organ_shape", {}).get("state_policy", {})
        manifest_state_policy = state_policy.get("manifest_state_paths", {})
        if not isinstance(manifest_state_policy, dict):
            findings.append(
                self.finding(
                    "critical",
                    "organ_manifest_state_policy_missing",
                    self.relative(self.contract_path),
                    "organ_shape.state_policy must declare manifest_state_paths to keep organ-local state membranes explicit.",
                )
            )
            manifest_state_policy = {}

        state_paths_field = str(manifest_state_policy.get("field") or "state_paths")
        allowed_prefixes = [str(item) for item in manifest_state_policy.get("allowed_prefixes", [])]
        require_relative_paths = manifest_state_policy.get("require_relative_paths") is True
        if not allowed_prefixes:
            findings.append(
                self.finding(
                    "critical",
                    "organ_manifest_state_paths_prefixes_missing",
                    self.relative(self.contract_path),
                    "manifest_state_paths must declare allowed_prefixes.",
                )
            )

        for manifest in sorted(self.organs_root.glob("*/MANIFEST.yaml")):
            rel = self.relative(manifest)
            try:
                payload = self.load_yaml(manifest) or {}
            except yaml.YAMLError as exc:
                findings.append(self.finding("critical", "organ_manifest_yaml_invalid", rel, str(exc)))
                continue
            if not isinstance(payload, dict) or state_paths_field not in payload:
                continue

            manifest_status = str(payload.get("status") or "").lower()
            require_existing_paths = manifest_status in {"active", "canonical"}
            state_paths = payload.get(state_paths_field)
            if not isinstance(state_paths, list) or not state_paths:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_manifest_state_paths_invalid",
                        rel,
                        f"{state_paths_field} must be a non-empty list when declared.",
                    )
                )
                continue

            for index, state_path in enumerate(state_paths):
                item_path = f"{rel}:{state_paths_field}[{index}]"
                if not isinstance(state_path, str) or not state_path.strip():
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_manifest_state_path_invalid",
                            item_path,
                            f"{state_paths_field} entries must be non-empty relative paths.",
                        )
                    )
                    continue

                path_value = state_path.strip()
                path = Path(path_value)
                if path.is_absolute() or require_relative_paths and path_value.startswith(".."):
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_manifest_state_path_not_relative",
                            item_path,
                            "state_paths entries must be relative to the organ directory.",
                        )
                    )
                    continue

                if allowed_prefixes and not any(
                    path_value == prefix or path_value.startswith(prefix)
                    for prefix in allowed_prefixes
                ):
                    findings.append(
                        self.finding(
                            "critical",
                            "organ_manifest_state_path_outside_allowed_prefix",
                            item_path,
                            f"state_paths entries must stay under one of {allowed_prefixes!r}.",
                        )
                    )
                    continue

                resolved = manifest.parent / path
                if not resolved.exists():
                    findings.append(
                        self.finding(
                            "critical" if require_existing_paths else "medium",
                            "organ_manifest_state_path_missing"
                            if require_existing_paths
                            else "organ_manifest_state_path_pending",
                            item_path,
                            "Declared state_paths entry does not exist relative to the organ directory.",
                        )
                    )
        return findings

    def manifest_script_reference_path(self, organ_dir: Path, name: str) -> str | None:
        if not name or name.startswith(("/", "@", "$")):
            return None
        if " " in name:
            return None
        path = Path(name)
        if not path.suffix and not name.endswith("/"):
            return None
        if path.suffix and path.suffix not in self.manifest_script_reference_suffixes:
            return None
        return self.relative(organ_dir / path).rstrip("/") + ("/" if name.endswith("/") else "")

    @staticmethod
    def is_git_tracked_path(path: str, tracked: set[str]) -> bool:
        normalized = path.rstrip("/")
        if normalized in tracked:
            return True
        prefix = normalized + "/"
        return any(item.startswith(prefix) for item in tracked)

    def manifest_untracked_script_status_finding(
        self,
        item: dict[str, Any],
        manifest: Path,
        index: int,
        rel: str,
    ) -> Any | None:
        missing = sorted(
            field for field in self.manifest_untracked_script_required_fields if not item.get(field)
        )
        path = f"{self.relative(manifest)}:scripts[{index}]"
        if missing:
            return self.finding(
                "critical",
                "manifest_untracked_script_status_missing",
                path,
                f"Untracked manifest script reference resolves to {rel!r} and must declare {', '.join(missing)}.",
            )
        status = str(item.get("integration_status"))
        if status not in self.manifest_untracked_script_allowed_statuses:
            return self.finding(
                "critical",
                "manifest_untracked_script_status_unknown",
                path,
                f"Untracked manifest script reference resolves to {rel!r} with unsupported integration_status {status!r}.",
            )
        return None

    def validate_manifest_script_references(self) -> list[Any]:
        findings: list[Any] = []
        if not self.organs_root.exists():
            return findings

        try:
            tracked = set(self.git_ls_files("organs"))
            untracked = set(self.git_ls_untracked("organs"))
        except (subprocess.CalledProcessError, FileNotFoundError) as exc:
            return [
                self.finding(
                    "critical",
                    "manifest_script_reference_scan_failed",
                    "organs/",
                    f"unable to query git organ files: {exc}",
                )
            ]

        for manifest in sorted(self.organs_root.glob("*/MANIFEST.yaml")):
            try:
                payload = self.load_yaml(manifest) or {}
            except yaml.YAMLError:
                continue
            scripts = payload.get("scripts", [])
            if not isinstance(scripts, list):
                continue
            for index, item in enumerate(scripts):
                if not isinstance(item, dict):
                    continue
                name = str(item.get("name") or "")
                rel = self.manifest_script_reference_path(manifest.parent, name)
                if rel is None:
                    continue
                if self.is_git_tracked_path(rel, tracked):
                    continue
                if (self.workspace_root / rel).exists() or self.is_git_tracked_path(rel, untracked):
                    status_finding = self.manifest_untracked_script_status_finding(item, manifest, index, rel)
                    if status_finding is None:
                        continue
                    findings.append(status_finding)
                    continue
                findings.append(
                    self.finding(
                        "medium",
                        "manifest_script_reference_missing",
                        f"{self.relative(manifest)}:scripts[{index}]",
                        f"Manifest script reference {name!r} resolves to missing path {rel!r}.",
                    )
                )
        return findings

    @staticmethod
    def has_python_sources(path: Path) -> bool:
        src = path / "src"
        return src.exists() and any(src.rglob("*.py"))

    def validate_python_organ_shape(self, contract: dict[str, Any]) -> list[Any]:
        findings: list[Any] = []
        if not self.organs_root.exists():
            return findings

        required_paths = contract.get("organ_shape", {}).get("python_executable_required_paths", [])
        if not required_paths:
            return findings

        exception_paths = self.declared_paths(contract, "organ_shape_exceptions")
        findings.extend(
            self.validate_exception_metadata(
                contract,
                "organ_shape_exceptions",
                {"path", "status", "owner", "missing_paths", "replacement", "expiry_condition"},
                directory_paths=True,
            )
        )

        for organ_dir in sorted(path for path in self.organs_root.iterdir() if path.is_dir()):
            if not (organ_dir / "MANIFEST.yaml").exists() or not self.has_python_sources(organ_dir):
                continue
            missing = [item for item in required_paths if not (organ_dir / item).exists()]
            if not missing:
                continue
            rel = self.relative(organ_dir).rstrip("/") + "/"
            if rel in exception_paths:
                continue
            findings.append(
                self.finding(
                    "critical",
                    "python_organ_shape_incomplete",
                    rel,
                    f"Python organs need {required_paths}; missing {missing}. Add the boundary or declare an organ_shape_exceptions entry.",
                )
            )

        for item in contract.get("organ_shape_exceptions", []):
            path_value = item.get("path")
            if not path_value:
                continue
            organ_dir = self.workspace_root / path_value
            declared_missing = set(item.get("missing_paths", []))
            actual_missing = {path for path in required_paths if not (organ_dir / path).exists()}
            if organ_dir.exists() and declared_missing != actual_missing:
                findings.append(
                    self.finding(
                        "critical",
                        "organ_shape_exception_stale",
                        str(path_value),
                        f"declares missing_paths={sorted(declared_missing)} but actual missing paths are {sorted(actual_missing)}.",
                    )
                )
        return findings
