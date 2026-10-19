#!/usr/bin/env python3
"""Re-freeze the producer DDL snapshot used by the conservation proof.

Reads SCHEMA_VERSION + SCHEMA_SQL from the producer's own source
(hermes-agent tree) and writes tests/parity/producer_schema.sql.
Run this when the producer bumps its schema — the proof test then
compares the snapshot against the new live source and fails loudly
until the contract is re-derived.

Usage: python3 crates/cynic-traces/tools/snapshot_producer_schema.py
"""

from __future__ import annotations

import os
import re
import sys

PRODUCER_SRC = os.environ.get(
    "CYNIC_PRODUCER_SRC",
    "/home/user/.hermes/hermes-agent/hermes_state_common.py",
)
HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(HERE, "tests", "parity", "producer_schema.sql")


def main() -> int:
    try:
        src = open(PRODUCER_SRC, encoding="utf-8").read()
    except OSError as exc:
        print(f"producer source absent: {PRODUCER_SRC}: {exc}", file=sys.stderr)
        return 1
    ver = re.search(r"^SCHEMA_VERSION\s*=\s*(\d+)", src, re.M)
    ddl = re.search(r'^SCHEMA_SQL\s*=\s*"""(.*?)"""', src, re.M | re.S)
    if not ver or not ddl:
        print("SCHEMA_VERSION or SCHEMA_SQL not found in producer source", file=sys.stderr)
        return 1
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(
            f"-- Frozen producer DDL snapshot (hermes-agent tree, SCHEMA_VERSION = {ver.group(1)}).\n"
            "-- Re-freeze with crates/cynic-traces/tools/snapshot_producer_schema.py "
            "when the producer bumps.\n" + ddl.group(1)
        )
    print(f"wrote {OUT} (SCHEMA_VERSION = {ver.group(1)})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
