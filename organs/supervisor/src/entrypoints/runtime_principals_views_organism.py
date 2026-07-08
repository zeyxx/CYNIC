#!/usr/bin/env python3
"""Compatibility facade for runtime principal organism report imports."""

from organs.supervisor.src.core import runtime_principals_organism_reports as _reports

for _name in dir(_reports):
    if _name.startswith("__"):
        continue
    globals()[_name] = getattr(_reports, _name)
