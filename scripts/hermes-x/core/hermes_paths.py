"""
Tier 2 INFRASTRUCTURE: Canonical path resolution for Hermes organ.

SSOT: ~/.cynic/organs/hermes/x/config/MANIFEST.json (canonical_paths section)
K15 Consumer: All hermes scripts import paths from here instead of hardcoding.
Systemd: used by all hermes-*.service units
Stability: new module (2026-05-14)

Failure mode: if MANIFEST.json missing, falls back to hardcoded defaults matching
the manifest's documented canonical_paths.
"""

__version__ = "0.1.0"

import json
import logging
import os
from pathlib import Path

log = logging.getLogger(__name__)

# Base directories — env-var overridable for systemd
HERMES_DIR = Path(os.environ.get(
    "HERMES_DIR",
    Path.home() / ".cynic" / "organs" / "hermes",
))
HERMES_X_DIR = Path(os.environ.get(
    "X_ORGAN_DIR",
    HERMES_DIR / "x",
))

# Load canonical_paths from MANIFEST.json (SSOT)
_MANIFEST_PATH = HERMES_X_DIR / "config" / "MANIFEST.json"
_canonical: dict[str, str] = {}
try:
    if _MANIFEST_PATH.exists():
        with open(_MANIFEST_PATH) as f:
            _canonical = json.load(f).get("canonical_paths", {})
except (json.JSONDecodeError, OSError) as e:
    log.warning("hermes_paths: failed to load %s: %s", _MANIFEST_PATH, e)


def _resolve(key: str, default: Path) -> Path:
    """Resolve path: MANIFEST.json > env var > hardcoded default."""
    raw = _canonical.get(key)
    if raw:
        return Path(os.path.expanduser(raw))
    return default


# --- Account-aware paths (L0: multi-account support) ---
ACCOUNT_ID = os.environ.get("HERMES_ACCOUNT", "cynic")

# DATASET: Per-account via X_DATASET_PATH env var (set by systemd) or symlink
_dataset_path = os.environ.get("X_DATASET_PATH")
if _dataset_path:
    DATASET = Path(_dataset_path)
else:
    # Fallback: use symlink (backward compat)
    DATASET = _resolve("dataset", HERMES_X_DIR / "dataset.jsonl")

# CHROME_PROFILE: Account-specific profile directory
_chrome_profile = os.environ.get("HERMES_CHROME_PROFILE")
if _chrome_profile:
    CHROME_PROFILE = Path(os.path.expanduser(_chrome_profile))
else:
    # Fallback: use account-aware default
    CHROME_PROFILE = HERMES_X_DIR / "chrome-profiles" / ACCOUNT_ID

# --- Canonical paths (match MANIFEST.json canonical_paths keys) ---
VERDICTS_DIR = _resolve("verdicts", HERMES_X_DIR / "verdicts")
OBSERVATIONS_DIR = _resolve("observations", HERMES_X_DIR / "observations")
AGENT_TASKS_DIR = _resolve("agent_tasks", HERMES_X_DIR / "agent-tasks")
SKILL_MD = _resolve("skill_md", HERMES_X_DIR / "SKILL.md")
CAPTURES_DIR = _resolve("captures", HERMES_X_DIR / "captures")
CURATED_DIR = _resolve("curated", HERMES_X_DIR / "curated")
DATASETS_DIR = _resolve("datasets", HERMES_X_DIR / "datasets")

# --- Additional paths used by scripts (not yet in MANIFEST.json) ---
FARMING_LOG = HERMES_X_DIR / "farming_log.jsonl"
SEARCH_TASKS = HERMES_X_DIR / "search_tasks.jsonl"
SEARCH_EXECUTION_LOG = HERMES_X_DIR / "search_execution_log.jsonl"
SEARCH_RESULTS_LOG = HERMES_X_DIR / "search_results.jsonl"
NAVIGATION_LOG = HERMES_X_DIR / "navigation_log.jsonl"
TWEET_ID_INDEX = HERMES_X_DIR / "tweet_id_index.db"
RECOVERY_LOG = HERMES_X_DIR / "recovery.log"
STATE_DIR = HERMES_X_DIR / ".state"
CONFIG_DIR = HERMES_X_DIR / "config"
BEHAVIOR_LOG = HERMES_DIR / "behavior" / "behavior_log.jsonl"
CRON_REPORTS_DIR = HERMES_X_DIR / "cron_reports"
