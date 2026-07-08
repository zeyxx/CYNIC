#!/usr/bin/env python3
"""Compatibility facade for runtime principal report helper imports."""

from organs.supervisor.src.core import runtime_principals_runtime_reports as _reports

for _name in dir(_reports):
    if _name.startswith("__"):
        continue
    globals()[_name] = getattr(_reports, _name)
