#!/usr/bin/env python3
"""Thin CLI membrane for the Hermes data organism."""

from organs.reflection.src.core import data_organism as _data_organism

for _name in dir(_data_organism):
    if _name.startswith("__"):
        continue
    globals()[_name] = getattr(_data_organism, _name)


if __name__ == "__main__":
    raise SystemExit(_data_organism.main())
