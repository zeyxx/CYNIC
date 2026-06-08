#!/usr/bin/env python3
"""Dispatch one organ-anvil repo lifecycle task to the Hermes Agent executor."""

from __future__ import annotations

import json
import os
import sys
from urllib import request, error


def env(name: str, default: str = "") -> str:
    return os.environ.get(name, default)


def kernel_addr() -> str:
    raw = env("CYNIC_REST_ADDR", "127.0.0.1:3030")
    if raw.startswith(("http://", "https://")):
        return raw.rstrip("/")
    return f"http://{raw.rstrip('/')}"


def kernel_key() -> str:
    return env("CYNIC_" + "API" + "_KEY")


def main() -> int:
    key = kernel_key()
    if not key:
        print("organ-anvil dispatch skipped: kernel API key missing", file=sys.stderr)
        return 1

    content = {
        "objective": "Organ Anvil repo lifecycle perception and triage",
        "domain": "organ-anvil",
        "targets": [
            "infra/organ-anvil/state.json",
            "infra/organ-anvil/poh.json",
            "infra/organ-anvil/audit.jsonl",
            ".handoff.md",
        ],
        "actions": [
            "Run bash scripts/organ-anvil.sh state to refresh repo perception.",
            "Run bash scripts/organ-anvil.sh triage and summarize actionable scopes.",
            "Post a concise repo-health observation to /observe.",
            "Do not commit, stash, restore, push, or edit files outside declared targets unless a later task declares them.",
        ],
    }
    payload = {
        "kind": "hermes",
        "domain": "organ-anvil",
        "agent_id": "organ-anvil-cron",
        "content": json.dumps(content, separators=(",", ":")),
    }
    body = json.dumps(payload).encode("utf-8")
    req = request.Request(
        f"{kernel_addr()}/agent-tasks",
        data=body,
        method="POST",
        headers={
            "Authorization": f"Bearer {key}",
            "Content-Type": "application/json",
        },
    )
    try:
        with request.urlopen(req, timeout=10) as resp:
            text = resp.read().decode("utf-8", errors="replace")
            if resp.status not in (200, 201):
                print(f"organ-anvil dispatch failed: HTTP {resp.status} {text}", file=sys.stderr)
                return 1
            print(text)
            return 0
    except error.HTTPError as exc:
        print(f"organ-anvil dispatch failed: HTTP {exc.code} {exc.read()[:200]!r}", file=sys.stderr)
        return 1
    except OSError as exc:
        print(f"organ-anvil dispatch failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
