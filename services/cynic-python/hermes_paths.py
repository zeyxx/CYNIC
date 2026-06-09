"""
Tier 2 INFRASTRUCTURE: Canonical path resolution for Hermes organ (cynic-python).

Thin forwarder — loads the canonical module from scripts/hermes-x/core/hermes_paths.py
via importlib to avoid self-import. Falls back to standalone MANIFEST.json resolution.
SSOT: ~/.cynic/organs/hermes/x/config/MANIFEST.json
"""

import importlib.util
import sys
from pathlib import Path

_core_path = Path(__file__).resolve().parent.parent / "scripts" / "hermes-x" / "core" / "hermes_paths.py"

if _core_path.exists():
    _spec = importlib.util.spec_from_file_location("_hermes_paths_core", str(_core_path))
    _mod = importlib.util.module_from_spec(_spec)
    _spec.loader.exec_module(_mod)
    # Re-export all public names
    for _name in dir(_mod):
        if not _name.startswith("_"):
            globals()[_name] = getattr(_mod, _name)
else:
    # Standalone fallback — reads MANIFEST.json directly
    import json
    import os
    import logging

    log = logging.getLogger(__name__)

    HERMES_DIR = Path(os.environ.get("HERMES_DIR", Path.home() / ".cynic" / "organs" / "hermes"))
    HERMES_X_DIR = Path(os.environ.get("X_ORGAN_DIR", HERMES_DIR / "x"))

    _MANIFEST_PATH = HERMES_X_DIR / "config" / "MANIFEST.json"
    _canonical: dict[str, str] = {}
    try:
        if _MANIFEST_PATH.exists():
            with open(_MANIFEST_PATH) as f:
                _canonical = json.load(f).get("canonical_paths", {})
    except (json.JSONDecodeError, OSError) as e:
        log.warning("hermes_paths: failed to load %s: %s", _MANIFEST_PATH, e)

    def _resolve(key: str, default: Path) -> Path:
        raw = _canonical.get(key)
        if raw:
            return Path(os.path.expanduser(raw))
        return default

    DATASET = _resolve("dataset", HERMES_X_DIR / "dataset.jsonl")
    VERDICTS_DIR = _resolve("verdicts", HERMES_X_DIR / "verdicts")
    OBSERVATIONS_DIR = _resolve("observations", HERMES_X_DIR / "observations")
    AGENT_TASKS_DIR = _resolve("agent_tasks", HERMES_X_DIR / "agent-tasks")
    CAPTURES_DIR = _resolve("captures", HERMES_X_DIR / "captures")
    CURATED_DIR = _resolve("curated", HERMES_X_DIR / "curated")
    DATASETS_DIR = _resolve("datasets", HERMES_X_DIR / "datasets")
    BEHAVIOR_LOG = HERMES_DIR / "behavior" / "behavior_log.jsonl"
