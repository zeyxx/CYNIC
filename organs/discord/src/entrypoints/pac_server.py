#!/usr/bin/env python3
"""Serve organ-discord PAC state over localhost."""

from __future__ import annotations

from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import os


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    state = root / "state"
    os.chdir(state)
    server = ThreadingHTTPServer(("127.0.0.1", 8892), SimpleHTTPRequestHandler)
    server.serve_forever()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
