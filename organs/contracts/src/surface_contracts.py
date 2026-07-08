#!/usr/bin/env python3
"""Shared surface contracts for command and report surfaces."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
import json
from pathlib import Path
from typing import Any, Protocol, runtime_checkable


@runtime_checkable
class CommandSurface(Protocol):
    name: str
    mutates: bool
    json_output: bool
    usage: str | None
    mutates_host: bool
    mutates_mempool: bool

    def as_manifest(self) -> dict[str, Any]: ...


@runtime_checkable
class ReportSurface(Protocol):
    schema: str
    command: str
    mutates: bool
    mutates_mempool: bool

    def as_manifest(self) -> dict[str, Any]: ...


@dataclass(frozen=True)
class CommandSurfaceSpec:
    """Declarative contract for a CLI command surface."""

    name: str
    handler: Callable[[list[str]], int]
    mutates: bool = False
    json_output: bool = False
    usage: str | None = None
    mutates_host: bool = False
    mutates_mempool: bool = False

    def as_manifest(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "mutates": self.mutates,
            "json_output": self.json_output,
            "usage": self.usage or f"{self.name} [args]",
        }


@dataclass(frozen=True)
class ReportSurfaceSpec:
    """Declarative contract for a machine-readable report surface."""

    schema: str
    command: str
    mutates: bool = False
    mutates_mempool: bool = False
    json_command: str | None = None
    usage: str | None = None

    def as_manifest(self) -> dict[str, Any]:
        return {
            "schema": self.schema,
            "command": self.command,
            "mutates": self.mutates,
            "mutates_mempool": self.mutates_mempool,
            "json_command": self.json_command or f"{self.command} --json",
            "usage": self.usage or self.command,
        }


def report_surface_manifest(report: ReportSurfaceSpec) -> dict[str, Any]:
    return report.as_manifest()


def attach_surface_contract(payload: dict[str, Any], report: ReportSurfaceSpec) -> dict[str, Any]:
    payload["surface_contract"] = report.as_manifest()
    return payload


def json_loads_or_default(value: Any, default: Any = None) -> Any:
    if isinstance(value, bytes):
        text = value.decode("utf-8")
    elif isinstance(value, str):
        text = value
    elif isinstance(value, (dict, list, tuple, int, float, bool)) or value is None:
        return value
    else:
        text = str(value)
    try:
        return json.loads(text)
    except (TypeError, json.JSONDecodeError):
        return default


def json_object_or_default(value: Any, default: dict[str, Any] | None = None) -> dict[str, Any]:
    fallback = {} if default is None else default
    parsed = json_loads_or_default(value, fallback)
    return parsed if isinstance(parsed, dict) else fallback


def json_loads_or_raise(value: Any, error: Exception) -> Any:
    parsed = json_loads_or_default(value, None)
    if parsed is None and value not in (None, "null", b"null"):
        raise error
    return parsed


def json_object_or_raise(value: Any, error: Exception) -> dict[str, Any]:
    parsed = json_loads_or_raise(value, error)
    if not isinstance(parsed, dict):
        raise error
    return parsed


def read_json_file_or_default(path: str | Path, default: Any = None) -> Any:
    try:
        return json_loads_or_default(Path(path).read_text(encoding="utf-8"), default)
    except OSError:
        return default


def read_json_file_or_raise(path: str | Path, error: Exception) -> Any:
    try:
        text = Path(path).read_text(encoding="utf-8")
    except OSError as exc:
        raise error from exc
    return json_loads_or_raise(text, error)


def read_toml_file_with_loader(path: str | Path, toml_loader: Any) -> dict[str, Any]:
    if toml_loader is None:
        raise RuntimeError("tomllib is required to validate TOML projections")
    payload = toml_loader.loads(Path(path).read_text(encoding="utf-8"))
    return payload if isinstance(payload, dict) else {}


def read_yaml_file_with_loader(path: str | Path, yaml_loader: Any) -> Any:
    if yaml_loader is None:
        raise RuntimeError("yaml loader is required to validate YAML projections")
    return yaml_loader.safe_load(Path(path).read_text(encoding="utf-8"))


def write_json_file(
    path: str | Path,
    payload: Any,
    *,
    indent: int | None = 2,
    sort_keys: bool = True,
    ensure_ascii: bool = False,
    newline: bool = True,
) -> None:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(
        payload,
        indent=indent,
        sort_keys=sort_keys,
        ensure_ascii=ensure_ascii,
    )
    if newline and not text.endswith("\n"):
        text += "\n"
    target.write_text(text, encoding="utf-8")


def write_json_file_atomic(
    path: str | Path,
    payload: Any,
    *,
    indent: int | None = 2,
    sort_keys: bool = True,
    ensure_ascii: bool = False,
    newline: bool = True,
) -> None:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(
        payload,
        indent=indent,
        sort_keys=sort_keys,
        ensure_ascii=ensure_ascii,
    )
    if newline and not text.endswith("\n"):
        text += "\n"
    tmp_path = target.with_name(f"{target.name}.tmp")
    tmp_path.write_text(text, encoding="utf-8")
    tmp_path.replace(target)


def write_jsonl_file(
    path: str | Path,
    rows: Any,
    *,
    sort_keys: bool = True,
    ensure_ascii: bool = False,
    newline: bool = True,
) -> None:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        json.dumps(row, sort_keys=sort_keys, ensure_ascii=ensure_ascii)
        for row in rows
    ]
    text = "\n".join(lines)
    if newline and text and not text.endswith("\n"):
        text += "\n"
    target.write_text(text, encoding="utf-8")


def write_jsonl_file_atomic(
    path: str | Path,
    rows: Any,
    *,
    sort_keys: bool = True,
    ensure_ascii: bool = False,
    newline: bool = True,
) -> None:
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        json.dumps(row, sort_keys=sort_keys, ensure_ascii=ensure_ascii)
        for row in rows
    ]
    text = "\n".join(lines)
    if newline and text and not text.endswith("\n"):
        text += "\n"
    tmp_path = target.with_name(f"{target.name}.tmp")
    tmp_path.write_text(text, encoding="utf-8")
    tmp_path.replace(target)


def iter_json_objects(lines: Any):
    for line in lines:
        parsed = json_loads_or_default(line, None)
        if parsed is not None:
            yield parsed


def json_stdout_or_envelope(
    result: dict[str, Any],
    *,
    schema: str,
    stdout_transform: Callable[[str], str] | None = None,
    extra: dict[str, Any] | None = None,
) -> dict[str, Any]:
    payload = json_loads_or_default(result["stdout"], None)
    if not isinstance(payload, dict):
        payload = {
            "schema": schema,
            "status": result["status"],
            "stdout": stdout_transform(str(result["stdout"])) if stdout_transform else str(result["stdout"]),
            "stderr": result["stderr"],
        }
        if extra:
            payload.update(extra)
        return payload
    if extra:
        payload.update(extra)
    return payload


@dataclass(frozen=True)
class SurfaceRegistry:
    """Universal surface registry for command and report surfaces."""

    commands: tuple[CommandSurfaceSpec, ...] = ()
    reports: tuple[ReportSurfaceSpec, ...] = ()

    def command_names(self) -> list[str]:
        return sorted(spec.name for spec in self.commands)

    def report_names(self) -> list[str]:
        return sorted(spec.command for spec in self.reports)

    def manifest(self) -> dict[str, Any]:
        commands = sorted(self.commands, key=lambda item: item.name)
        reports = sorted(self.reports, key=lambda item: item.command)
        return {
            "schema": "cynic.surface_registry.v1",
            "command_count": len(commands),
            "report_count": len(reports),
            "commands": [spec.as_manifest() for spec in commands],
            "reports": [spec.as_manifest() for spec in reports],
        }
