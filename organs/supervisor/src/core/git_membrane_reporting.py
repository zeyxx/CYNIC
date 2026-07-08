"""Git membrane closure reporting and commands."""

from __future__ import annotations

import shlex
import subprocess
from typing import Any

from organs.supervisor.src.core.git_membrane import GitMembraneClosureService


def _git_command_result(service: GitMembraneClosureService, args: list[str]) -> dict[str, Any]:
    command = ["git", *args]
    try:
        completed = subprocess.run(
            command,
            cwd=service.workspace_root,
            check=True,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError as exc:
        return {
            "ok": False,
            "command": " ".join(command),
            "returncode": None,
            "paths": [],
            "error": str(exc),
        }
    except subprocess.CalledProcessError as exc:
        stderr = (exc.stderr or "").strip()
        stdout = (exc.stdout or "").strip()
        return {
            "ok": False,
            "command": " ".join(command),
            "returncode": exc.returncode,
            "paths": [],
            "error": stderr or stdout or str(exc),
        }

    return {
        "ok": True,
        "command": " ".join(command),
        "returncode": completed.returncode,
        "paths": _paths_from_git_output(args, completed.stdout),
        "error": "",
    }


def _paths_from_git_output(args: list[str], output: str) -> list[str]:
    paths: list[str] = []
    is_status = args[:2] == ["status", "--short"]
    for raw_line in output.splitlines():
        line = raw_line.strip("\n")
        if not line:
            continue
        path = line[3:] if is_status and len(line) > 3 else line
        if " -> " in path:
            path = path.split(" -> ", 1)[1]
        paths.append(path.strip())
    return sorted(set(paths))


def _path_family(path: str) -> str:
    parts = path.split("/")
    if not parts:
        return "repo"
    if parts[0] == "crates" and len(parts) > 1:
        return f"crates/{parts[1]}"
    if parts[0] == "organs" and len(parts) > 1:
        return f"organs/{parts[1]}"
    if parts[0] in {"axioms", "docs", "infra", "scripts", "data", "crates", "organs"}:
        return parts[0]
    if parts[0].startswith("."):
        return parts[0]
    return "repo"


def _path_class(path: str) -> str:
    if path.startswith("crates/cynic-kernel/"):
        return "kernel_source"
    if path.startswith("crates/cynic-mcp/"):
        return "mcp_source"
    if path.startswith("organs/"):
        return "organ_body"
    if path.startswith("axioms/"):
        return "contract"
    if path.startswith("infra/"):
        return "infra_projection"
    if path.startswith("docs/"):
        return "document"
    if path in {"Makefile", "AGENTS.md", "CODEX.md", "CLAUDE.md", "ANTIGRAVITY.md"}:
        return "repo_control"
    return "repo_surface"


def _generated_cache_dirs(service: GitMembraneClosureService) -> list[str]:
    roots = [service.workspace_root / "organs", service.workspace_root / "crates"]
    cache_dirs: list[str] = []
    for root in roots:
        if not root.exists():
            continue
        for path in root.rglob("__pycache__"):
            if path.is_dir():
                cache_dirs.append(service.relative(path))
    return sorted(cache_dirs)


def _group_visible_paths(paths: set[str], untracked: set[str], modified: set[str], staged: set[str]) -> list[dict[str, Any]]:
    grouped: dict[tuple[str, str, str], list[str]] = {}
    for path in sorted(paths):
        if path in untracked:
            status = "untracked"
        elif path in staged:
            status = "staged"
        elif path in modified:
            status = "modified"
        else:
            status = "visible"
        key = (_path_family(path), _path_class(path), status)
        grouped.setdefault(key, []).append(path)

    return [
        {
            "family": family,
            "topological_class": topological_class,
            "status": status,
            "count": len(items),
            "sample": items[:12],
        }
        for (family, topological_class, status), items in sorted(grouped.items())
    ]


def _inference_batches(groups: list[dict[str, Any]]) -> list[dict[str, Any]]:
    batches: list[dict[str, Any]] = []
    for index, group in enumerate(groups, start=1):
        batches.append(
            {
                "batch_id": f"git-matter-{index:03d}",
                "family": group["family"],
                "topological_class": group["topological_class"],
                "status": group["status"],
                "path_count": group["count"],
                "sample_paths": group["sample"],
                "task": "classify mutation intent, ownership, risk, and closure wave; do not require file contents unless this batch is selected for deeper inspection",
                "output_schema": {
                    "decision": "track|quarantine|split|inspect|discard",
                    "risk": "low|medium|high",
                    "owner": "organ|crate|contract|infra|repo",
                    "rationale": "string",
                    "next_probe": "string",
                },
            }
        )
    return batches


def matter_report(service: GitMembraneClosureService) -> dict[str, Any]:
    git_commands = {
        "status_short": _git_command_result(service, ["status", "--short"]),
        "worktree_diff": _git_command_result(service, ["diff", "--name-only"]),
        "staged_diff": _git_command_result(service, ["diff", "--cached", "--name-only"]),
        "untracked": _git_command_result(service, ["ls-files", "--others", "--exclude-standard"]),
    }

    untracked = set(git_commands["untracked"].get("paths", []))
    modified = set(git_commands["worktree_diff"].get("paths", []))
    staged = set(git_commands["staged_diff"].get("paths", []))
    visible_paths: set[str] = set(untracked | modified | staged)
    if not visible_paths:
        visible_paths.update(git_commands["status_short"].get("paths", []))

    groups = _group_visible_paths(visible_paths, untracked, modified, staged)
    cache_dirs = _generated_cache_dirs(service)
    blockers: list[str] = []
    for name, result in git_commands.items():
        if not result.get("ok"):
            blockers.append(f"{name} failed: {result.get('error')}")
    if not service.git_metadata_writable():
        blockers.append(".git is not writable in this environment")
    if cache_dirs:
        blockers.append("generated cache directories exist inside source membranes")

    return {
        "schema": "cynic.git_matter_report.v1",
        "purpose": "read-only bottom-up Git matter perception for sovereign inference and Forgejo closure planning",
        "ok": not blockers,
        "git_writable": service.git_metadata_writable(),
        "visible_path_total": len(visible_paths),
        "untracked_total": len(untracked),
        "modified_total": len(modified),
        "staged_total": len(staged),
        "generated_cache_dirs": cache_dirs,
        "git_commands": git_commands,
        "groups": groups,
        "inference_batches": _inference_batches(groups),
        "blockers": blockers,
    }


def summary(service: GitMembraneClosureService, selected_wave: int | None = None) -> dict[str, Any]:
    plan = service.load_json(service.plan_path)
    try:
        untracked = service.git_ls_untracked()
    except (subprocess.CalledProcessError, FileNotFoundError):
        untracked = []

    waves: list[dict[str, Any]] = []
    covered: set[str] = set()
    for wave in plan.get("waves", []):
        targets: list[dict[str, Any]] = []
        wave_untracked_count = 0
        for target in wave.get("targets", []):
            path = str(target.get("path", ""))
            if not path:
                continue
            if path.endswith("/"):
                matching = [item for item in untracked if item.startswith(path)]
            else:
                matching = [item for item in untracked if item == path]
            covered.update(matching)
            wave_untracked_count += len(matching)
            targets.append(
                {
                    "path": path,
                    "disposition": wave.get("disposition"),
                    "untracked_count": len(matching),
                    "reason": target.get("reason", ""),
                    "sample": matching[:5],
                }
            )
        waves.append(
            {
                "wave": wave.get("wave"),
                "theme": wave.get("theme"),
                "disposition": wave.get("disposition"),
                "untracked_count": wave_untracked_count,
                "targets": targets,
            }
        )

    next_open_wave = next(
        (
            {"wave": wave.get("wave"), "theme": wave.get("theme"), "untracked_count": wave.get("untracked_count")}
            for wave in waves
            if wave.get("untracked_count", 0) > 0
        ),
        None,
    )

    return {
        "schema": "cynic.git_membrane_closure_summary.v1",
        "git_writable": service.git_metadata_writable(),
        "untracked_total": len(untracked),
        "untracked_planned": len(covered),
        "untracked_unplanned": sorted(set(untracked) - covered),
        "next_open_wave": next_open_wave,
        "selected_wave": selected_wave,
        "waves": [
            wave for wave in waves if selected_wave is None or wave.get("wave") == selected_wave
        ],
    }


def commands(service: GitMembraneClosureService, selected_wave: int | None = None) -> dict[str, Any]:
    summary_data = summary(service, selected_wave=selected_wave)
    waves: list[dict[str, Any]] = []
    for wave in summary_data.get("waves", []):
        pathspecs = [
            target["path"]
            for target in wave.get("targets", [])
            if target.get("untracked_count", 0) > 0
        ]
        waves.append(
            {
                "wave": wave.get("wave"),
                "theme": wave.get("theme"),
                "disposition": wave.get("disposition"),
                "untracked_count": wave.get("untracked_count"),
                "pathspecs": pathspecs,
                "git_add_command": (
                    "git add -- " + " ".join(shlex.quote(path) for path in pathspecs)
                    if pathspecs
                    else ""
                ),
                "post_add_untracked_check": (
                    "git ls-files --others --exclude-standard -- "
                    + " ".join(shlex.quote(path) for path in pathspecs)
                    if pathspecs
                    else ""
                ),
                "post_add_validation": (
                    "make lint-repo-topology-contract"
                    if pathspecs
                    else ""
                ),
            }
        )

    return {
        "schema": "cynic.git_membrane_closure_commands.v1",
        "git_writable": summary_data.get("git_writable", False),
        "untracked_total": summary_data.get("untracked_total", 0),
        "untracked_planned": summary_data.get("untracked_planned", 0),
        "untracked_unplanned": summary_data.get("untracked_unplanned", []),
        "next_open_wave": summary_data.get("next_open_wave"),
        "selected_wave": selected_wave,
        "waves": waves,
        "note": "Commands are non-mutating output only; review each wave before running git add.",
    }


def preflight(service: GitMembraneClosureService) -> dict[str, Any]:
    summary_data = summary(service)
    blockers: list[str] = []
    if not summary_data.get("git_writable"):
        blockers.append(".git is not writable in this environment")
    if summary_data.get("untracked_unplanned"):
        blockers.append("visible untracked files are not covered by the closure plan")
    if summary_data.get("untracked_total", 0) > 0 and not summary_data.get("next_open_wave"):
        blockers.append("untracked files exist but no next open wave is available")

    return {
        "schema": "cynic.git_membrane_closure_preflight.v1",
        "ok": not blockers,
        "git_writable": summary_data.get("git_writable", False),
        "untracked_total": summary_data.get("untracked_total", 0),
        "untracked_planned": summary_data.get("untracked_planned", 0),
        "untracked_unplanned_count": len(summary_data.get("untracked_unplanned", [])),
        "next_open_wave": summary_data.get("next_open_wave"),
        "blockers": blockers,
    }


def check(service: GitMembraneClosureService, selected_wave: int | None = None) -> dict[str, Any]:
    summary_data = summary(service, selected_wave=selected_wave)
    waves = summary_data.get("waves", [])
    open_waves = [
        {
            "wave": wave.get("wave"),
            "theme": wave.get("theme"),
            "untracked_count": wave.get("untracked_count", 0),
        }
        for wave in waves
        if wave.get("untracked_count", 0) > 0
    ]
    issues: list[str] = []
    if selected_wave is not None and not waves:
        issues.append(f"closure wave {selected_wave} is not declared")
    if summary_data.get("untracked_unplanned"):
        issues.append("closure plan does not cover every visible untracked file")
    if open_waves:
        issues.append("selected closure scope still has visible untracked files")

    return {
        "schema": "cynic.git_membrane_closure_check.v1",
        "ok": not issues,
        "selected_wave": selected_wave,
        "git_writable": summary_data.get("git_writable", False),
        "untracked_total": summary_data.get("untracked_total", 0),
        "untracked_planned": summary_data.get("untracked_planned", 0),
        "untracked_unplanned": summary_data.get("untracked_unplanned", []),
        "next_open_wave": summary_data.get("next_open_wave"),
        "checked_waves": [
            {
                "wave": wave.get("wave"),
                "theme": wave.get("theme"),
                "untracked_count": wave.get("untracked_count", 0),
            }
            for wave in waves
        ],
        "open_waves": open_waves,
        "issues": issues,
    }
