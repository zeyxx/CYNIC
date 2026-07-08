"""Filesystem adapter for Twitter/X ingest state.

Owns cursor persistence, verdict file writes, and dataset structural health
checks. It has no kernel or browser side effects.
"""

import json
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import iter_json_objects


def load_cursor(state_path: Path) -> int:
    if state_path.exists():
        try:
            return int(state_path.read_text().strip())
        except (ValueError, OSError):
            pass
    return 0


def save_cursor(state_path: Path, offset: int) -> None:
    try:
        state_path.write_text(str(offset))
    except OSError:
        pass


def detect_auth_failure(dataset: Path, rows_to_check: int = 10) -> bool:
    """Detect extraction failure when recent rows lack tweet identity fields."""
    if not dataset.exists():
        return False
    try:
        with open(dataset, "r") as f:
            f.seek(0, 2)
            file_size = f.tell()
            if file_size == 0:
                return False
            chunk_size = min(file_size, 65536)
            f.seek(max(0, file_size - chunk_size))
            content = f.read()

        lines = [line.strip() for line in content.split("\n") if line.strip()][-rows_to_check:]
        if not lines:
            return False

        zero_identity = 0
        for row in iter_json_objects(lines):
            if not row.get("tweet_id") or not row.get("text"):
                zero_identity += 1

        return zero_identity > 0 and zero_identity >= max(1, len(lines) // 2)
    except Exception:
        return False


def write_verdict_to_organ(row: dict, verdict: dict, organ_dir: Path) -> bool:
    tweet_id = row.get("tweet_id", "")
    if not tweet_id:
        return False

    verdict_dir = organ_dir / "verdicts"
    try:
        verdict_dir.mkdir(parents=True, exist_ok=True)
    except OSError:
        return False

    verdict_file = verdict_dir / f"{tweet_id}.json"
    if verdict_file.exists():
        return True

    try:
        import datetime

        payload: dict[str, Any] = {
            "tweet_id": tweet_id,
            "signal_score": row.get("signal_score", 0),
            "captured_at": row.get("capture_ts", ""),
            "verdict": verdict,
            "written_at": datetime.datetime.now(datetime.UTC).isoformat().replace("+00:00", "Z"),
        }
        verdict_file.write_text(json.dumps(payload, indent=2))
        return True
    except OSError:
        return False
