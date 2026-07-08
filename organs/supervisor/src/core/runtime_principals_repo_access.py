"""Repo access profile and ACL/grant helpers for runtime principals."""

from __future__ import annotations

import json
import os
import pwd
import shutil
import subprocess
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import read_json_file_or_default, write_json_file_atomic

REPO_ACCESS_PROFILE_SCHEMA = "cynic.runtime_principals.repo_access.v1"


@dataclass(frozen=True)
class RepoAccessEntry:
    path: str
    owner: str | None
    added_by: str | None
    discovered: bool
    added_at: str
    source: str

    def to_payload(self) -> dict[str, Any]:
        return {
            "path": self.path,
            "owner": self.owner,
            "added_by": self.added_by,
            "discovered": self.discovered,
            "added_at": self.added_at,
            "source": self.source,
        }


@dataclass
class RepoAccessCommandResult:
    path: str
    status: str
    granted: bool
    owner: str | None = None
    source: str | None = None

    def to_payload(self) -> dict[str, Any]:
        return {
            "path": self.path,
            "status": self.status,
            "granted": self.granted,
            "owner": self.owner,
            "source": self.source,
        }


class RepoAccessProfile:
    def __init__(self, path: Path):
        self.path = path

    def load(self) -> list[RepoAccessEntry]:
        if not self.path.is_file():
            return []
        data = read_json_file_or_default(self.path, {})
        items: list[RepoAccessEntry] = []
        raw_items = data.get("repositories", []) if isinstance(data, dict) else []
        for raw in raw_items:
            if not isinstance(raw, dict):
                continue
            path = str(raw.get("path", "")).strip()
            if not path:
                continue
            items.append(
                RepoAccessEntry(
                    path=path,
                    owner=_none_or_str(raw.get("owner")),
                    added_by=_none_or_str(raw.get("added_by")),
                    discovered=bool(raw.get("discovered")),
                    added_at=str(raw.get("added_at") or datetime.now(timezone.utc).isoformat()),
                    source=str(raw.get("source") or "sync"),
                )
            )
        deduped: list[RepoAccessEntry] = []
        seen: set[str] = set()
        for item in items:
            resolved = str(Path(item.path).resolve())
            if resolved in seen:
                continue
            seen.add(resolved)
            deduped.append(item)
        return deduped

    def save(self, entries: list[RepoAccessEntry]) -> None:
        payload = {
            "schema": REPO_ACCESS_PROFILE_SCHEMA,
            "version": 1,
            "updated_at": datetime.now(timezone.utc).isoformat(),
            "repositories": [entry.to_payload() for entry in entries],
        }
        write_json_file_atomic(self.path, payload)


class RepoAccessManager:
    def __init__(
        self,
        dedicated_user: str,
        dedicated_group: str,
        profile: Path,
        operator_user: str | None,
        canonical_project_root: Path | None = None,
    ):
        self.dedicated_user = dedicated_user
        self.dedicated_group = dedicated_group
        self.profile_path = profile
        self.operator_user = operator_user
        self.canonical_project_root = canonical_project_root
        self.profile = RepoAccessProfile(self.profile_path)

    def canonical_repo_targets(self) -> list[Path]:
        if self.canonical_project_root is None:
            return []
        targets: list[Path] = [self.canonical_project_root]
        for entry in self.profile.load():
            path = Path(entry.path).resolve()
            if path in targets:
                continue
            if path.exists():
                targets.append(path)
            else:
                targets.append(path)
        return targets

    def ensure_canonical_entry(self, *, grant: bool = True) -> RepoAccessCommandResult:
        if self.canonical_project_root is None:
            return RepoAccessCommandResult(
                path="",
                status="missing",
                granted=False,
                owner=None,
                source="apply",
            )
        return self.sync_repo(
            path=self.canonical_project_root,
            added_by="runtime-principals-apply",
            source="apply",
            discovered=False,
            grant=grant,
        )

    def ensure_recorded_repos(self, *, added_by: str, source: str, grant: bool = True) -> dict[str, object]:
        return self.ensure_repos(
            list(self.canonical_repo_targets()),
            added_by=added_by,
            source=source,
            discovered=True,
            grant=grant,
        )

    def sync_repo(
        self,
        *,
        path: Path,
        added_by: str,
        source: str,
        discovered: bool,
        grant: bool = True,
    ) -> RepoAccessCommandResult:
        entries = self.profile.load()
        normalized = path.resolve()
        owner = _resolve_owner(normalized)
        now = datetime.now(timezone.utc).isoformat()
        existing_index = _entry_index(entries, normalized)
        status = "ok"
        granted = False
        if existing_index is None:
            entries.append(
                RepoAccessEntry(
                    path=str(normalized),
                    owner=owner,
                    added_by=added_by,
                    discovered=discovered,
                    added_at=now,
                    source=source,
                )
            )
            self.profile.save(entries)
            status = "added"
        else:
            updated = list(entries)
            current = updated[existing_index]
            changed = False
            if current.added_by != added_by:
                current = RepoAccessEntry(
                    path=current.path,
                    owner=current.owner or owner,
                    added_by=added_by,
                    discovered=bool(current.discovered or discovered),
                    added_at=current.added_at,
                    source=current.source,
                )
                updated[existing_index] = current
                changed = True
            if changed:
                self.profile.save(updated)
                entries = updated
                status = "updated"
            else:
                status = "present"
        if grant:
            granted = _grant_repo_access(self.dedicated_user, self.dedicated_group, self.operator_user, normalized)
        return RepoAccessCommandResult(
            path=str(normalized),
            status=status,
            granted=granted,
            owner=owner,
            source=source,
        )

    def ensure_repos(
        self,
        paths: list[Path],
        *,
        added_by: str,
        source: str,
        discovered: bool = False,
        grant: bool = True,
    ) -> dict[str, object]:
        results: list[RepoAccessCommandResult] = []
        for path in paths:
            if path.exists() and path.is_dir():
                results.append(
                    self.sync_repo(
                        path=path,
                        added_by=added_by,
                        source=source,
                        discovered=discovered,
                        grant=grant,
                    )
                )
            else:
                results.append(
                    RepoAccessCommandResult(
                        path=str(path),
                        status="missing",
                        granted=False,
                        owner=None,
                        source=source,
                    )
                )
        payload = {
            "schema": "cynic.runtime_principals.repo_access.ensure.v1",
            "mutates": True,
            "status": "ok" if all(item.status in {"added", "present", "updated", "missing"} for item in results) else "partial",
            "entries": [item.to_payload() for item in results],
            "count": len(results),
        }
        if any(item.granted for item in results):
            payload["command"] = f"grant {self.dedicated_user}"
        return payload

    def report(self) -> dict[str, object]:
        entries = self.profile.load()
        canonical = str(self.canonical_project_root.resolve()) if self.canonical_project_root else None
        known_paths: list[str] = []
        if canonical:
            known_paths.append(canonical)
        for path in _paths_from_profile(entries):
            resolved = path.resolve()
            if not resolved.is_dir():
                continue
            resolved_text = str(resolved)
            if canonical and resolved_text == canonical:
                continue
            known_paths.append(resolved_text)
        access_readiness = []
        for item in entries:
            repo_path = Path(item.path).resolve()
            access_readiness.append(
                {
                    "path": str(repo_path),
                    "owner": item.owner,
                    "source": item.source,
                    "discovered": item.discovered,
                    "added_by": item.added_by,
                    "added_at": item.added_at,
                    "exists": repo_path.exists(),
                }
            )
        return {
            "schema": "cynic.runtime_principals.repo_access.status.v1",
            "mutates": False,
            "profile_path": str(self.profile_path),
            "profile_exists": self.profile_path.is_file(),
            "canonical_project_root": canonical,
            "known_repos": sorted(set(known_paths)),
            "entries": access_readiness,
        }

    def remove_path(self, path: Path) -> dict[str, object]:
        normalized = path.resolve()
        entries = self.profile.load()
        updated = [item for item in entries if Path(item.path).resolve() != normalized]
        removed = len(entries) - len(updated)
        if removed:
            self.profile.save(updated)
        return {
            "schema": "cynic.runtime_principals.repo_access.remove.v1",
            "status": "removed" if removed else "missing",
            "path": str(normalized),
            "mutates": removed > 0,
        }


def _entry_index(entries: list[RepoAccessEntry], target: Path) -> int | None:
    normalized = str(target.resolve())
    for index, item in enumerate(entries):
        if str(Path(item.path).resolve()) == normalized:
            return index
    return None


def _paths_from_profile(entries: list[RepoAccessEntry]) -> list[Path]:
    return [Path(item.path).resolve() for item in entries if item.path]


def _none_or_str(value: object) -> str | None:
    if value is None:
        return None
    text = str(value).strip()
    return text or None


def _resolve_owner(path: Path) -> str | None:
    try:
        return pwd.getpwuid(path.stat().st_uid).pw_name
    except (FileNotFoundError, PermissionError, OSError):
        return None


def _run(command: list[str]) -> None:
    if shutil.which("sudo") and os.geteuid() != 0:
        subprocess.run(command, check=False)
    else:
        subprocess.run(command, check=False)


def _grant_repo_access(user: str, group: str, operator_user: str | None, repo_root: Path) -> bool:
    if not repo_root.is_dir():
        return False
    try:
        if shutil.which("setfacl"):
            for parent in repo_root.parents:
                _run(["setfacl", "-m", f"u:{user}:x", str(parent)])
            acl_users = [user]
            if operator_user and operator_user not in {"", "root", user}:
                acl_users.append(operator_user)
            acl_spec = ",".join(f"u:{name}:rwX" for name in acl_users)
            default_acl_spec = ",".join(f"d:u:{name}:rwX" for name in acl_users)
            _run(["setfacl", "-R", "-m", acl_spec, str(repo_root)])
            _run(["find", str(repo_root), "-type", "d", "-exec", "setfacl", "-m", default_acl_spec, "{}", "+"])
            return True
        _run(["chgrp", "-R", group, str(repo_root)])
        _run(["chmod", "-R", "g+rwX", str(repo_root)])
        return os.geteuid() == 0
    except OSError:
        return False
