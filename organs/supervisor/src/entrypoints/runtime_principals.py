#!/usr/bin/env python3
"""Runtime principal activation facade.

Executable behavior lives in ``runtime_principals_views`` and
``runtime_principals_actions``; this module re-exports a stable entrypoint
surface while keeping the implementation split for topology readability.
"""

from __future__ import annotations

from organs.supervisor.src.entrypoints.runtime_principals_actions import *  # noqa: F401,F403
from organs.supervisor.src.entrypoints.runtime_principals_views import *  # noqa: F401,F403
from organs.supervisor.src.entrypoints.runtime_principals_views import _run_capture  # noqa: F401


if __name__ == "__main__":
    raise SystemExit(main())
