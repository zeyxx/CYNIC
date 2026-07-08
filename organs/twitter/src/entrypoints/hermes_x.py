#!/usr/bin/env python3
"""Organ-twitter compatibility entrypoint for Hermes X runtime modules.

Systemd projections call this organ-owned entrypoint. Legacy scripts under
root-level legacy Hermes X paths are closed and are not injected into the
runtime import path.
"""

from __future__ import annotations

import argparse
import runpy
import sys

from organs.twitter.src.core.hermes_x_router import (
    HermesXComponent,
    component_table,
    resolve_component as _resolve_component,
)

try:
    from organs.twitter.src.entrypoints._bootstrap import bootstrap_repo_path
except ImportError:
    from _bootstrap import bootstrap_repo_path


PROJECT_ROOT = bootstrap_repo_path()
LEGACY_ROOT = PROJECT_ROOT / "scripts" / "hermes-x"
ORGAN_TWITTER_ADAPTERS = PROJECT_ROOT / "organs" / "twitter" / "src" / "adapters"
COMPONENTS = component_table(PROJECT_ROOT)


def resolve_component(name: str) -> HermesXComponent:
    return _resolve_component(COMPONENTS, name)


def prepare_legacy_imports(component: HermesXComponent) -> None:
    paths = [
        PROJECT_ROOT,
        ORGAN_TWITTER_ADAPTERS,
        *component.extra_paths,
    ]
    for path in reversed(paths):
        text = str(path)
        if text not in sys.path:
            sys.path.insert(0, text)


def run_component(component_name: str, argv: list[str]) -> int:
    component = resolve_component(component_name)
    prepare_legacy_imports(component)
    old_argv = sys.argv
    sys.argv = [str(component.script), *argv]
    try:
        runpy.run_path(str(component.script), run_name="__main__")
    except SystemExit as exc:
        code = exc.code
        if code is None:
            return 0
        if isinstance(code, int):
            return code
        print(code, file=sys.stderr)
        return 1
    finally:
        sys.argv = old_argv
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="organ-twitter Hermes X compatibility entrypoint",
    )
    parser.add_argument("component", choices=sorted(COMPONENTS))
    parser.add_argument("args", nargs=argparse.REMAINDER)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    return run_component(args.component, args.args)


if __name__ == "__main__":
    raise SystemExit(main())
