#!/usr/bin/env python3
"""Switch active Hermes X account without shell wrappers."""

from __future__ import annotations

import argparse
import os
import subprocess
import time
from pathlib import Path


SERVICES = (
    "hermes-browser",
    "hermes-proxy",
    "hermes-browser-hub",
    "hermes-ingest",
    "hermes-recovery",
)


def _systemctl(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["systemctl", "--user", *args],
        text=True,
        capture_output=True,
        check=False,
    )


def _is_active(unit: str) -> bool:
    return _systemctl("is-active", unit).returncode == 0


def _active_account() -> str | None:
    for service in SERVICES:
        for account in ("cynic", "personal"):
            if _is_active(f"{service}@{account}.service"):
                return account
    return None


def _update_symlinks(state_dir: Path, account: str) -> None:
    dataset_dir = state_dir / "datasets" / account
    dataset_dir.mkdir(parents=True, exist_ok=True)
    links = {
        "dataset.jsonl": f"datasets/{account}/dataset.jsonl",
        "ingest_cursor.txt": f"datasets/{account}/ingest_cursor.txt",
        "verdicts": f"datasets/{account}/verdicts",
        "observations": f"datasets/{account}/observations",
    }
    state_dir.mkdir(parents=True, exist_ok=True)
    for link_name, target in links.items():
        link = state_dir / link_name
        if link.exists() or link.is_symlink():
            link.unlink()
        link.symlink_to(target)


def main() -> int:
    parser = argparse.ArgumentParser(description="Switch active Hermes X account.")
    parser.add_argument("account", choices=("cynic", "personal"))
    args = parser.parse_args()

    state_root = Path(os.environ.get("CYNIC_ORGANS_DIR", str(Path.home() / ".cynic/organs")))
    state_dir = state_root / "hermes" / "x"
    current = _active_account()

    if current:
        for service in SERVICES:
            unit = f"{service}@{current}.service"
            if _is_active(unit):
                _systemctl("stop", unit)
        time.sleep(2)

    _update_symlinks(state_dir, args.account)

    for service in SERVICES:
        unit = f"{service}@{args.account}.service"
        result = _systemctl("start", unit)
        if result.returncode != 0:
            print(result.stderr.strip() or f"failed to start {unit}")
            return 1

    time.sleep(3)
    failed = [f"{service}@{args.account}.service" for service in SERVICES if not _is_active(f"{service}@{args.account}.service")]
    if failed:
        print("inactive units: " + ", ".join(failed))
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
