from __future__ import annotations

import grp
import os
import pwd
import shutil
import subprocess
from pathlib import Path

from organs.supervisor.src.entrypoints.runtime_principals_views import (
    CYNIC_CLI,
    GROUP_NAME,
    HOME_DIR,
    PROFILE_DEFAULTS,
    REQUIRED_OPERATOR_BINARIES,
    USER_NAME,
)
from organs.supervisor.src.entrypoints.runtime_principals_views_runtime import (
    _repo_access_manager,
    cynic_cli_body,
    launcher_body,
    profile_body,
)
from organs.supervisor.src.entrypoints.runtime_principals_views_runtime_reports import (
    _exists_group,
    _exists_user,
    status,
)


def require_root(action: str) -> None:
    if os.geteuid() != 0:
        raise SystemExit(f"ERROR: {action} requires root")


def run(command: list[str]) -> None:
    subprocess.run(command, check=True)


def grant_project_access() -> None:
    result = _repo_access_manager().ensure_canonical_entry(grant=True)
    if result.status == "missing":
        raise SystemExit(f"ERROR: canonical project root is missing or not a directory: {HOME_DIR}")


def grant_path_read_execute(path: Path) -> None:
    resolved = path.resolve()
    if shutil.which("setfacl"):
        for parent in resolved.parents:
            run(["setfacl", "-m", f"u:{USER_NAME}:x", str(parent)])
        run(["setfacl", "-m", f"u:{USER_NAME}:rx", str(resolved)])
        return
    run(["chmod", "a+rx", str(resolved)])


def binary_bridge_body(binary: str, source: Path) -> str:
    return f"""#!/usr/bin/env bash
set -euo pipefail
exec "{source}" "$@"
"""


def install_binary_bridges() -> None:
    bin_dir = HOME_DIR / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    for bridge_name, binary_name in REQUIRED_OPERATOR_BINARIES.items():
        source = shutil.which(binary_name)
        if source is None:
            continue
        source_path = Path(source)
        grant_path_read_execute(source_path)
        target = bin_dir / bridge_name
        target.write_text(binary_bridge_body(binary_name, source_path), encoding="utf-8")
        target.chmod(0o755)


def install_launchers() -> None:
    bin_dir = HOME_DIR / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    launchers = {
        "codex-yolo": launcher_body("codex"),
        "antigravity-yolo": launcher_body("agy"),
        "hermes-yolo": launcher_body("hermes"),
    }
    for name, body in launchers.items():
        target = bin_dir / name
        target.write_text(body, encoding="utf-8")
        target.chmod(0o755)


def install_cynic_cli() -> None:
    CYNIC_CLI.write_text(cynic_cli_body(), encoding="utf-8")
    CYNIC_CLI.chmod(0o755)


def chown_home() -> None:
    uid = pwd.getpwnam(USER_NAME).pw_uid
    gid = grp.getgrnam(GROUP_NAME).gr_gid
    for root, dirs, files in os.walk(HOME_DIR):
        os.chown(root, uid, gid)
        for name in dirs + files:
            try:
                os.chown(Path(root) / name, uid, gid)
            except FileNotFoundError:
                # Ephemeral files (Chrome SingletonCookie, sockets, lock files)
                # may vanish between os.walk enumeration and os.chown — skip silently.
                pass


def apply() -> dict[str, object]:
    require_root("apply")
    if not _exists_group(GROUP_NAME):
        run(["groupadd", GROUP_NAME])
    if not _exists_user(USER_NAME):
        run(["useradd", "-m", "-d", str(HOME_DIR), "-s", "/bin/bash", "-g", GROUP_NAME, USER_NAME])
    grant_project_access()
    install_binary_bridges()
    install_launchers()
    install_cynic_cli()
    PROFILE_DEFAULTS.write_text(profile_body(), encoding="utf-8")
    PROFILE_DEFAULTS.chmod(0o644)
    chown_home()
    return status()


def install_defaults() -> dict[str, object]:
    require_root("install-defaults")
    install_cynic_cli()
    PROFILE_DEFAULTS.write_text(profile_body(), encoding="utf-8")
    PROFILE_DEFAULTS.chmod(0o644)
    return status()
