#!/usr/bin/env python3
"""Thin CLI membrane for organ-forge."""

import json
import os
import sys
from pathlib import Path


def _bootstrap_repo_path() -> Path:
    start = Path(__file__).resolve()
    for candidate in (start, *start.parents):
        if (candidate / "organs").is_dir() and (candidate / "infra").is_dir():
            return candidate
    return start.parent


PROJECT_ROOT = _bootstrap_repo_path()
os.environ["CYNIC_PROJECT_ROOT"] = str(PROJECT_ROOT)
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from organs.forge.src.core import cli_runtime as _cli_runtime

for _name in dir(_cli_runtime):
    if _name.startswith("__"):
        continue
    globals()[_name] = getattr(_cli_runtime, _name)


async def run_delegate_profile(repo_path, profile, json_mode: bool = False):
    try:
        intents = lifecycle_profile_intents(profile)
    except Exception as e:
        print(json.dumps({"status": "error", "error": str(e)}, separators=(",", ":")), file=sys.stderr)
        return 1

    results = []
    strict_lifecycle = not _cli_runtime._profile_uses_bootstrap_lane(profile)
    for intent in intents:
        try:
            result = await delegate_intent(
                repo_path,
                intent,
                emit_logs=not json_mode,
                strict_lifecycle=strict_lifecycle,
            )
            results.append({"intent": intent, "status": "ok", "result": result})
        except Exception as e:
            results.append({"intent": intent, "status": "error", "error": str(e)})

    payload = {"status": "ok", "profile": profile, "results": results}
    if any(item.get("status") != "ok" for item in results):
        payload["status"] = "error"
    print(json.dumps(payload, separators=(",", ":")))
    return 0 if payload["status"] == "ok" else 1


if __name__ == "__main__":
    raise SystemExit(_cli_runtime.main())
