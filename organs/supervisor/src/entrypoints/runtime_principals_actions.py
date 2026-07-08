#!/usr/bin/env python3
"""Thin command membrane for runtime principals."""

from organs.supervisor.src.core import runtime_principals_actions as _actions

for _name in dir(_actions):
    if _name.startswith("__"):
        continue
    globals()[_name] = getattr(_actions, _name)


if __name__ == "__main__":
    raise SystemExit(_actions.main())
