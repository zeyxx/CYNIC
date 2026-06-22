#!/usr/bin/env python3
"""fleet-gen compatibility shim for legacy deployment scripts.

This repository references ``scripts/fleet-gen.py`` from runtime bootstrap hooks
and operational workflows. The historical implementation is not present in this
workspace, so this shim restores the behavior required for current tooling:

* file-based placeholder substitution for `<TAILSCALE_*>` values (input path mode)
* a legacy no-op `node-kernels` mode used by systemd bootstrap

Usage:
  scripts/fleet-gen.py <path/to/config.toml>         # resolve placeholders to stdout
  scripts/fleet-gen.py --mode node-kernels            # compatibility no-op

Both modes are intentionally conservative:
* unresolved placeholders are preserved in output
* unresolved placeholders emit warnings to stderr when ``--strict`` is set
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path
from typing import Iterable

PLACEHOLDER_RE = re.compile(r"<([A-Z0-9_]+)>")


def resolve_placeholders(text: str) -> tuple[str, list[str]]:
    unresolved: list[str] = []

    def repl(match: re.Match[str]) -> str:
        token = match.group(1)
        value = os.getenv(token, "")
        if value:
            return value.strip()
        unresolved.append(match.group(0))
        return match.group(0)

    return PLACEHOLDER_RE.sub(repl, text), unresolved


def resolve_file(path: Path, strict: bool = False) -> int:
    raw = path.read_text(encoding="utf-8")
    resolved, unresolved = resolve_placeholders(raw)
    if strict and unresolved:
        print(
            f"fleet-gen: unresolved placeholders in {path}:",
            ", ".join(dict.fromkeys(unresolved)),
            file=sys.stderr,
        )
        return 2
    if unresolved and os.getenv("CYNIC_ALLOW_UNRESOLVED_PLACEHOLDERS", "0") != "1":
        print(
            f"fleet-gen: unresolved placeholders in {path} (set CYNIC_ALLOW_UNRESOLVED_PLACEHOLDERS=1 to bypass): "
            + ", ".join(dict.fromkeys(unresolved)),
            file=sys.stderr,
        )
    sys.stdout.write(resolved)
    return 0


def run_node_kernels(strict: bool = False) -> int:
    # Legacy bootstrap compatibility entry point.
    # Historically produced one or more rendered node configs.
    # Current stack boots node runtime with explicit configs in crates/..
    # Keep this as a no-op / validation path to avoid hard failure in systemd.
    repo_root = Path(__file__).resolve().parent.parent
    template_paths = sorted(repo_root.glob("crates/cynic-node/*.toml.example"))

    unresolved_any = []
    for template in template_paths:
        if not template.is_file():
            continue
        _, unresolved = resolve_placeholders(template.read_text(encoding="utf-8"))
        unresolved_any.extend(unresolved)

    unresolved_any = sorted(set(unresolved_any))
    if unresolved_any:
        print(
            "fleet-gen: unresolved placeholders while checking node templates:",
            ", ".join(unresolved_any),
            file=sys.stderr,
        )
        if strict:
            return 2
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Resolve placeholders for CYNIC deployment artifacts."
    )
    parser.add_argument("path", nargs="?", help="Input file to resolve.")
    parser.add_argument(
        "--mode",
        choices=["file", "node-kernels"],
        default="file",
        help="Operation mode (legacy).",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Fail when unresolved placeholders remain.",
    )
    args = parser.parse_args()

    if args.mode == "node-kernels":
        return run_node_kernels(strict=args.strict)

    if not args.path:
        print("fleet-gen: file mode requires a target path", file=sys.stderr)
        return 2

    input_path = Path(args.path)
    if not input_path.exists():
        print(f"fleet-gen: file not found: {input_path}", file=sys.stderr)
        return 2

    return resolve_file(input_path, strict=args.strict)


if __name__ == "__main__":
    sys.exit(main())
