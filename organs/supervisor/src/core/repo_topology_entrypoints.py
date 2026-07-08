#!/usr/bin/env python3
"""Repo topology checks for executable entrypoint membranes."""

from __future__ import annotations

import os
import re
import subprocess
from pathlib import Path
from typing import Any

import organs.supervisor.src.core.repo_topology_host as host


def validate_prohibited_script_entry_files(contract: dict[str, Any]) -> list[host.Finding]:
    policy = contract.get("prohibited_script_entry_files", {})
    if not isinstance(policy, dict):
        return [
            host.Finding(
                "critical",
                "prohibited_script_entry_policy_invalid",
                "prohibited_script_entry_files",
                "prohibited_script_entry_files must be an object declaring suffixes and replacement metadata.",
            )
        ]

    required = {"owner", "replacement", "expiry_condition"}
    findings: list[host.Finding] = []
    for field in sorted(required):
        if not policy.get(field):
            findings.append(
                host.Finding(
                    "critical",
                    "prohibited_script_entry_policy_missing_metadata",
                    "prohibited_script_entry_files",
                    f"prohibited_script_entry_files must declare {field}.",
                )
            )

    suffixes = policy.get("suffixes", [])
    if not isinstance(suffixes, list) or not all(isinstance(item, str) and item.startswith(".") for item in suffixes):
        findings.append(
            host.Finding(
                "critical",
                "prohibited_script_entry_policy_invalid_suffixes",
                "prohibited_script_entry_files",
                "prohibited_script_entry_files.suffixes must be a list of file suffix strings such as .sh.",
            )
        )
        return findings

    shebang_interpreters = policy.get("shebang_interpreters", [])
    if not isinstance(shebang_interpreters, list) or not all(isinstance(item, str) for item in shebang_interpreters):
        findings.append(
            host.Finding(
                "critical",
                "prohibited_script_entry_policy_invalid_shebangs",
                "prohibited_script_entry_files",
                "prohibited_script_entry_files.shebang_interpreters must be a list of interpreter names such as bash.",
            )
        )
        return findings
    prohibited_interpreters = {item.strip().lower() for item in shebang_interpreters if item.strip()}
    ignored_prefixes = [
        str(item).strip().rstrip("/") + "/"
        for item in policy.get("ignored_prefixes", [])
        if isinstance(item, str) and item.strip()
    ]

    ignored_dirs = {".git", ".gradle", ".gradle-user-home", "target", "node_modules", ".venv", "__pycache__"}
    for root, dirs, files in os.walk(host.WORKSPACE_ROOT):
        dirs[:] = [item for item in dirs if item not in ignored_dirs]
        root_path = Path(root)
        for filename in files:
            path = root_path / filename
            rel = host.relative(path)
            if any(rel == prefix.rstrip("/") or rel.startswith(prefix) for prefix in ignored_prefixes):
                continue
            prohibited_by_suffix = path.suffix in suffixes
            prohibited_by_shebang = False
            try:
                with path.open("rb") as handle:
                    first_line = handle.readline(200)
            except OSError:
                first_line = b""
            if first_line.startswith(b"#!"):
                shebang = first_line[2:].decode("utf-8", errors="ignore").strip()
                parts = shebang.split()
                executable = Path(parts[0]).name if parts else ""
                if executable == "env" and len(parts) > 1:
                    executable = Path(parts[1]).name
                prohibited_by_shebang = executable.lower() in prohibited_interpreters
            if not prohibited_by_suffix and not prohibited_by_shebang:
                continue
            findings.append(
                host.Finding(
                    "critical",
                    "prohibited_script_entry_file",
                    rel,
                    "Shell files are no longer valid organism entrypoints; reforge them as organ-owned Python/Rust entrypoints, Make fragments, infra contracts, or MCP operations.",
                )
            )
    return findings


def validate_tracked_executable_file_policy(contract: dict[str, Any]) -> list[host.Finding]:
    policy = contract.get("tracked_executable_file_policy", {})
    if not isinstance(policy, dict):
        return [
            host.Finding(
                "critical",
                "tracked_executable_file_policy_invalid",
                "tracked_executable_file_policy",
                "tracked_executable_file_policy must be an object declaring owner, allowed_paths, replacement and expiry_condition.",
            )
        ]

    findings: list[host.Finding] = []
    for field in ("owner", "replacement", "expiry_condition"):
        if not policy.get(field):
            findings.append(
                host.Finding(
                    "critical",
                    "tracked_executable_file_policy_missing_metadata",
                    "tracked_executable_file_policy",
                    f"tracked_executable_file_policy must declare {field}.",
                )
            )

    allowed_paths = policy.get("allowed_paths", [])
    if not isinstance(allowed_paths, list) or not all(isinstance(item, str) for item in allowed_paths):
        findings.append(
            host.Finding(
                "critical",
                "tracked_executable_file_policy_invalid_allowlist",
                "tracked_executable_file_policy",
                "tracked_executable_file_policy.allowed_paths must be a list of repo-relative paths.",
            )
        )
        return findings
    allowed = set(allowed_paths)

    try:
        staged = subprocess.run(
            ["git", "ls-files", "-s"],
            cwd=host.WORKSPACE_ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.splitlines()
        tracked_modes = {
            line.split(maxsplit=3)[3]: line.split(maxsplit=1)[0]
            for line in staged
            if len(line.split(maxsplit=3)) == 4
        }
    except (subprocess.CalledProcessError, FileNotFoundError):
        try:
            tracked_modes = {path: "100755" for path in host.git_ls_files()}
        except (subprocess.CalledProcessError, FileNotFoundError) as exc:
            return [
                host.Finding(
                    "critical",
                    "tracked_executable_file_scan_failed",
                    ".",
                    f"unable to query git tracked files: {exc}",
                )
            ]

    for rel_path, git_mode in sorted(tracked_modes.items()):
        if rel_path in allowed:
            continue
        if git_mode != "100755":
            continue
        path = host.WORKSPACE_ROOT / rel_path
        if not path.is_file():
            continue
        try:
            mode = path.stat().st_mode
        except OSError:
            continue
        if mode & 0o111:
            findings.append(
                host.Finding(
                    "critical",
                    "tracked_executable_file_not_allowlisted",
                    rel_path,
                    "Git-tracked source files must not carry an executable bit; expose behavior through organ entrypoints, make targets, systemd units, or explicit interpreter commands.",
                )
            )
    return findings


def validate_top_level_source_file_policy(contract: dict[str, Any]) -> list[host.Finding]:
    policy = contract.get("top_level_source_file_policy", {})
    if not isinstance(policy, dict):
        return [
            host.Finding(
                "critical",
                "top_level_source_file_policy_invalid",
                "top_level_source_file_policy",
                "top_level_source_file_policy must be an object declaring owner, source_suffixes, allowed_paths, replacement and expiry_condition.",
            )
        ]

    findings: list[host.Finding] = []
    for field in ("owner", "replacement", "expiry_condition"):
        if not policy.get(field):
            findings.append(
                host.Finding(
                    "critical",
                    "top_level_source_file_policy_missing_metadata",
                    "top_level_source_file_policy",
                    f"top_level_source_file_policy must declare {field}.",
                )
            )

    suffixes = policy.get("source_suffixes", [])
    if not isinstance(suffixes, list) or not all(isinstance(item, str) and item.startswith(".") for item in suffixes):
        findings.append(
            host.Finding(
                "critical",
                "top_level_source_file_policy_invalid_suffixes",
                "top_level_source_file_policy",
                "top_level_source_file_policy.source_suffixes must be a list of suffix strings such as .py.",
            )
        )
        return findings
    source_suffixes = set(suffixes)

    allowed_paths = policy.get("allowed_paths", [])
    if not isinstance(allowed_paths, list) or not all(isinstance(item, str) for item in allowed_paths):
        findings.append(
            host.Finding(
                "critical",
                "top_level_source_file_policy_invalid_allowlist",
                "top_level_source_file_policy",
                "top_level_source_file_policy.allowed_paths must be a list of repo-relative paths.",
            )
        )
        return findings
    allowed = set(allowed_paths)

    try:
        tracked = host.git_ls_files()
    except (subprocess.CalledProcessError, FileNotFoundError) as exc:
        return [
            host.Finding(
                "critical",
                "top_level_source_file_scan_failed",
                ".",
                f"unable to query git tracked files: {exc}",
            )
        ]

    for rel_path in sorted(tracked):
        if "/" in rel_path or rel_path in allowed:
            continue
        path = host.WORKSPACE_ROOT / rel_path
        if not path.is_file():
            continue
        if path.suffix in source_suffixes:
            findings.append(
                host.Finding(
                    "critical",
                    "top_level_source_file_not_allowlisted",
                    rel_path,
                    "Tracked source files must not live at the repo root; move them under the owning organ, crate, app, package, service or infra boundary.",
                )
            )
    return findings


def validate_generated_dependency_directory_policy(contract: dict[str, Any]) -> list[host.Finding]:
    policy = contract.get("generated_dependency_directory_policy", {})
    if not isinstance(policy, dict):
        return [
            host.Finding(
                "critical",
                "generated_dependency_directory_policy_invalid",
                "generated_dependency_directory_policy",
                "generated_dependency_directory_policy must be an object declaring owner, directory_names, scan_roots, allowed_paths, replacement and expiry_condition.",
            )
        ]

    findings: list[host.Finding] = []
    for field in ("owner", "replacement", "expiry_condition"):
        if not policy.get(field):
            findings.append(
                host.Finding(
                    "critical",
                    "generated_dependency_directory_policy_missing_metadata",
                    "generated_dependency_directory_policy",
                    f"generated_dependency_directory_policy must declare {field}.",
                )
            )

    directory_names = policy.get("directory_names", [])
    if not isinstance(directory_names, list) or not all(isinstance(item, str) and item for item in directory_names):
        findings.append(
            host.Finding(
                "critical",
                "generated_dependency_directory_policy_invalid_names",
                "generated_dependency_directory_policy",
                "generated_dependency_directory_policy.directory_names must be a list of directory names.",
            )
        )
        return findings
    generated_names = set(directory_names)

    scan_roots = policy.get("scan_roots", [])
    if not isinstance(scan_roots, list) or not all(isinstance(item, str) for item in scan_roots):
        findings.append(
            host.Finding(
                "critical",
                "generated_dependency_directory_policy_invalid_roots",
                "generated_dependency_directory_policy",
                "generated_dependency_directory_policy.scan_roots must be a list of repo-relative root paths.",
            )
        )
        return findings

    allowed_paths = policy.get("allowed_paths", [])
    if not isinstance(allowed_paths, list) or not all(isinstance(item, str) for item in allowed_paths):
        findings.append(
            host.Finding(
                "critical",
                "generated_dependency_directory_policy_invalid_allowlist",
                "generated_dependency_directory_policy",
                "generated_dependency_directory_policy.allowed_paths must be a list of repo-relative paths.",
            )
        )
        return findings
    allowed = {item.rstrip("/") for item in allowed_paths}

    for rel_root in scan_roots:
        root = host.WORKSPACE_ROOT / rel_root
        if not root.is_dir():
            continue
        for current_root, dirs, _files in os.walk(root):
            current_path = Path(current_root)
            blocked_here = sorted(name for name in dirs if name in generated_names)
            for name in blocked_here:
                generated_path = current_path / name
                rel_path = host.relative(generated_path).rstrip("/")
                if rel_path in allowed:
                    continue
                findings.append(
                    host.Finding(
                        "critical",
                        "generated_dependency_directory_in_source_tree",
                        rel_path + "/",
                        "Generated dependency/cache directories must not live inside source membrane roots; recreate them from manifests outside the source tree when needed.",
                    )
                )
            dirs[:] = [name for name in dirs if name not in generated_names]
    return findings


def validate_entrypoint_thin_membrane_policy(contract: dict[str, Any]) -> list[host.Finding]:
    policy = contract.get("entrypoint_thin_membrane_policy", {})
    if not isinstance(policy, dict):
        return [
            host.Finding(
                "critical",
                "entrypoint_thin_membrane_policy_invalid",
                "entrypoint_thin_membrane_policy",
                "entrypoint_thin_membrane_policy must be an object declaring scan_globs, max_lines, exceptions, replacement and expiry_condition.",
            )
        ]

    findings: list[host.Finding] = []
    for field in (
        "owner",
        "scan_globs",
        "max_lines",
        "forbid_executable_bit",
        "exceptions",
        "replacement",
        "expiry_condition",
    ):
        empty_value = policy.get(field) in (None, "")
        empty_required_list = field == "scan_globs" and policy.get(field) == []
        if field not in policy or empty_value or empty_required_list:
            findings.append(
                host.Finding(
                    "critical",
                    "entrypoint_thin_membrane_policy_missing_metadata",
                    "entrypoint_thin_membrane_policy",
                    f"entrypoint_thin_membrane_policy must declare {field}.",
                )
            )

    if policy.get("forbid_executable_bit") is not True:
        findings.append(
            host.Finding(
                "critical",
                "entrypoint_thin_membrane_policy_invalid_executable_bit_rule",
                "entrypoint_thin_membrane_policy",
                "entrypoint_thin_membrane_policy.forbid_executable_bit must be true; entrypoints are invocation membranes, not executable scripts.",
            )
        )

    scan_globs = policy.get("scan_globs", [])
    if not isinstance(scan_globs, list) or not all(isinstance(item, str) and item for item in scan_globs):
        findings.append(
            host.Finding(
                "critical",
                "entrypoint_thin_membrane_policy_invalid_globs",
                "entrypoint_thin_membrane_policy",
                "entrypoint_thin_membrane_policy.scan_globs must be a list of repo-relative glob strings.",
            )
        )
        return findings

    max_lines = policy.get("max_lines")
    if not isinstance(max_lines, int) or max_lines <= 0:
        findings.append(
            host.Finding(
                "critical",
                "entrypoint_thin_membrane_policy_invalid_limit",
                "entrypoint_thin_membrane_policy",
                "entrypoint_thin_membrane_policy.max_lines must be a positive integer.",
            )
        )
        return findings

    exceptions = policy.get("exceptions", [])
    if not isinstance(exceptions, list) or not all(isinstance(item, dict) for item in exceptions):
        findings.append(
            host.Finding(
                "critical",
                "entrypoint_thin_membrane_policy_invalid_exceptions",
                "entrypoint_thin_membrane_policy",
                "entrypoint_thin_membrane_policy.exceptions must be a list of exception objects.",
            )
        )
        return findings

    exception_fields = {"path", "owner", "replacement", "expiry_condition"}
    exception_by_path: dict[str, dict[str, Any]] = {}
    for index, item in enumerate(exceptions):
        missing = sorted(field for field in exception_fields if not item.get(field))
        path = item.get("path")
        if missing or not isinstance(path, str):
            findings.append(
                host.Finding(
                    "critical",
                    "entrypoint_thin_membrane_exception_invalid",
                    f"entrypoint_thin_membrane_policy.exceptions[{index}]",
                    f"entrypoint thin membrane exceptions must declare {sorted(exception_fields)}.",
                )
            )
            continue
        exception_by_path[path] = item

    candidates: set[Path] = set()
    for pattern in scan_globs:
        candidates.update(path for path in host.WORKSPACE_ROOT.glob(pattern) if path.is_file())

    seen_exceptions: set[str] = set()
    for path in sorted(candidates):
        rel = host.relative(path)
        if path.stat().st_mode & 0o111:
            findings.append(
                host.Finding(
                    "critical",
                    "entrypoint_executable_bit_violation",
                    rel,
                    "Entrypoint files must not carry an executable bit; invoke them through python -m, console wrappers, make targets, systemd units, or MCP tools.",
                )
            )
        try:
            line_count = len(path.read_text(encoding="utf-8").splitlines())
        except UnicodeDecodeError:
            continue
        exception = exception_by_path.get(rel)
        if line_count <= max_lines:
            if exception is not None:
                seen_exceptions.add(rel)
                findings.append(
                    host.Finding(
                        "critical",
                        "entrypoint_thin_membrane_exception_stale",
                        rel,
                        "Entrypoint thin membrane exception is stale; remove it now that the file is within the line budget.",
                    )
                )
            continue
        if exception is not None:
            seen_exceptions.add(rel)
            recorded_line_count = exception.get("line_count")
            if isinstance(recorded_line_count, int) and recorded_line_count < line_count:
                findings.append(
                    host.Finding(
                        "critical",
                        "entrypoint_thin_membrane_exception_regressed",
                        rel,
                        f"Entrypoint grew from declared {recorded_line_count} lines to {line_count}; move logic behind core/adapters before extending it.",
                    )
                )
            continue
        findings.append(
            host.Finding(
                "critical",
                "entrypoint_thin_membrane_violation",
                rel,
                f"Entrypoints are invocation membranes, not scripts carrying organ logic; {rel} has {line_count} lines, limit is {max_lines}.",
            )
        )

    for rel in sorted(set(exception_by_path) - seen_exceptions):
        findings.append(
            host.Finding(
                "critical",
                "entrypoint_thin_membrane_exception_missing_target",
                rel,
                "Entrypoint thin membrane exception points to a file that is no longer present.",
            )
        )
    return findings


def validate_active_document_script_entrypoint_policy(contract: dict[str, Any]) -> list[host.Finding]:
    policy = contract.get("active_document_script_entrypoint_policy", {})
    if not isinstance(policy, dict):
        return [
            host.Finding(
                "critical",
                "active_document_script_entrypoint_policy_invalid",
                "active_document_script_entrypoint_policy",
                "active_document_script_entrypoint_policy must be an object declaring scan_paths, forbidden_patterns, replacement and expiry_condition.",
            )
        ]

    findings: list[host.Finding] = []
    for field in ("owner", "scan_paths", "forbidden_patterns", "replacement", "expiry_condition"):
        if not policy.get(field):
            findings.append(
                host.Finding(
                    "critical",
                    "active_document_script_entrypoint_policy_missing_metadata",
                    "active_document_script_entrypoint_policy",
                    f"active_document_script_entrypoint_policy must declare {field}.",
                )
            )

    scan_paths = policy.get("scan_paths", [])
    if not isinstance(scan_paths, list) or not all(isinstance(item, str) and item for item in scan_paths):
        findings.append(
            host.Finding(
                "critical",
                "active_document_script_entrypoint_policy_invalid_scan_paths",
                "active_document_script_entrypoint_policy",
                "active_document_script_entrypoint_policy.scan_paths must be a list of repo-relative paths.",
            )
        )
        return findings

    ignored_globs = policy.get("ignored_globs", [])
    if not isinstance(ignored_globs, list) or not all(isinstance(item, str) for item in ignored_globs):
        findings.append(
            host.Finding(
                "critical",
                "active_document_script_entrypoint_policy_invalid_ignored_globs",
                "active_document_script_entrypoint_policy",
                "active_document_script_entrypoint_policy.ignored_globs must be a list of glob strings.",
            )
        )
        return findings

    pattern_values = policy.get("forbidden_patterns", [])
    if not isinstance(pattern_values, list) or not all(isinstance(item, str) and item for item in pattern_values):
        findings.append(
            host.Finding(
                "critical",
                "active_document_script_entrypoint_policy_invalid_patterns",
                "active_document_script_entrypoint_policy",
                "active_document_script_entrypoint_policy.forbidden_patterns must be a list of regex strings.",
            )
        )
        return findings
    try:
        patterns = [re.compile(item) for item in pattern_values]
    except re.error as exc:
        return [
            host.Finding(
                "critical",
                "active_document_script_entrypoint_policy_invalid_regex",
                "active_document_script_entrypoint_policy",
                f"forbidden_patterns contains an invalid regex: {exc}",
            )
        ]

    candidates: set[Path] = set()
    for item in scan_paths:
        root = host.WORKSPACE_ROOT / item
        if root.is_file():
            candidates.add(root)
        elif root.is_dir():
            candidates.update(path for path in root.rglob("*") if path.is_file())

    for path in sorted(candidates):
        rel = host.relative(path)
        if host.path_matches_any(rel, ignored_globs):
            continue
        if path.suffix not in {".md", ".json", ".toml"}:
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except (FileNotFoundError, UnicodeDecodeError):
            continue
        for line_no, line in enumerate(lines, start=1):
            if any(pattern.search(line) for pattern in patterns):
                findings.append(
                    host.Finding(
                        "critical",
                        "active_document_script_entrypoint_reference",
                        f"{rel}:{line_no}",
                        "Active agent/operator documents must not instruct humans or agents to use script entrypoints; document organ entrypoints, make targets, systemd units, registry lifecycle profiles, or MCP tools instead.",
                    )
                )
    return findings
