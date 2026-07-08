#!/usr/bin/env python3
"""Compatibility facade for runtime principal helper imports."""

from organs.supervisor.src.core import runtime_principals_runtime as _runtime

for _name in dir(_runtime):
    if _name.startswith("__"):
        continue
    globals()[_name] = getattr(_runtime, _name)
