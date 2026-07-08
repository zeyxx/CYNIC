from __future__ import annotations
import os
from pathlib import Path
from organs.supervisor.src.entrypoints.runtime_principals_views import USER_NAME
from organs.supervisor.src.entrypoints.runtime_principals_views_runtime import _repo_access_manager

def _repo_paths_from_args(paths: list[str]) -> list[Path]:
    return [Path(path).expanduser().resolve() for path in paths]

def repos_add(args: list[str]) -> dict[str, object]:
    if not args:
        raise SystemExit("ERROR: cynic repos add requires one or more paths")
    paths = _repo_paths_from_args(args)
    return _repo_access_manager().ensure_repos(paths, added_by=USER_NAME, source="cli")

def repos_remove(args: list[str]) -> dict[str, object]:
    if not args:
        raise SystemExit("ERROR: cynic repos remove requires one or more paths")
    paths = _repo_paths_from_args(args)
    manager = _repo_access_manager()
    for path in paths:
        manager.remove_path(path)
    return manager.report()

def repos_status(args: list[str]) -> dict[str, object]:
    del args
    return _repo_access_manager().report()

def repos_sync() -> dict[str, object]:
    manager = _repo_access_manager()
    entries = manager.profile.load()
    for entry in entries:
        if entry.path:
            manager.sync_repo(path=Path(entry.path), added_by=USER_NAME, source="cli")
    return manager.report()
