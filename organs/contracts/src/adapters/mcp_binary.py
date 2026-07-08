"""Shared MCP binary selection and freshness adapter."""

from __future__ import annotations

import os
import shutil
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class McpBinarySelection:
    binary: Path
    exists: bool
    executable: bool
    stale_source: Path | None = None

    @property
    def usable(self) -> bool:
        return self.exists and self.executable and self.stale_source is None


from pathlib import Path

def get_fallback_mcp_binary_path() -> Path:
    """Returns the default fallback path for the MCP binary."""
    # Use the home directory to construct the path, making it portable.
    return Path.home() / "bin" / "cynic-kernel-mcp"

LEGACY_CORTEX_BINARIES = (
    get_fallback_mcp_binary_path(),
)


def select_mcp_binary(project_root: Path) -> McpBinarySelection:
    """Select the cynic-mcp binary using the organism-wide precedence order."""

    binary = selected_mcp_binary_path(project_root)
    exists = binary.exists()
    executable = exists and os.access(binary, os.X_OK)
    stale = newest_stale_source(project_root / "crates" / "cynic-mcp", binary) if executable else None
    return McpBinarySelection(binary=binary, exists=exists, executable=executable, stale_source=stale)


def selected_mcp_binary_path(project_root: Path) -> Path:
    env_bin = os.environ.get("CYNIC_MCP_BINARY", "").strip()
    if env_bin:
        return Path(env_bin).expanduser()

    repo_bin = project_root / "target" / "release" / "cynic-mcp"
    if repo_bin.exists() and os.access(repo_bin, os.X_OK):
        return repo_bin

    path_bin = shutil.which("cynic-mcp")
    if path_bin:
        return Path(path_bin)

    legacy_path_bin = shutil.which("cynic-kernel-mcp")
    if legacy_path_bin:
        return Path(legacy_path_bin)

    for legacy_binary in LEGACY_CORTEX_BINARIES:
        if legacy_binary.exists() and os.access(legacy_binary, os.X_OK):
            return legacy_binary

    fallback = os.environ.get("CYNIC_MCP_FALLBACK_BIN", str(Path.home() / "bin" / "cynic-kernel-mcp"))
    return Path(fallback).expanduser()


def newest_stale_source(src_dir: Path, binary: Path) -> Path | None:
    try:
        binary_mtime = binary.stat().st_mtime
    except FileNotFoundError:
        return None

    roots = (src_dir / "src", src_dir / "Cargo.toml", src_dir / "build.rs")
    for root in roots:
        if root.is_file():
            if root.stat().st_mtime > binary_mtime:
                return root
            continue
        if root.is_dir():
            for path in root.rglob("*"):
                if path.is_file() and path.stat().st_mtime > binary_mtime:
                    return path
    return None
