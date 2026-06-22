#!/usr/bin/env python3
"""Audit CYNIC organ runtime health.

This controller complements audit_organs.py.

audit_organs.py answers: does this directory expose an organ contract?
audit_runtime.py answers: is the contracted organ producing observable state now?

It is intentionally local, deterministic, and kernel-independent. It does not
start services, call network APIs, or mutate organ runtime databases. It only
reads declared organ state and writes the generated body health view.
"""

from __future__ import annotations

import json
import sqlite3
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ORGANS_ROOT = Path(__file__).resolve().parents[1]
CYNIC_ROOT = ORGANS_ROOT.parent
WORKSPACE_ROOT = CYNIC_ROOT.parent
STATIC_AUDIT = ORGANS_ROOT / "organ-contract-audit.json"
OUTPUT = ORGANS_ROOT / "state/body-health.json"

FRESH_SECONDS = 15 * 60
STALE_SECONDS = 6 * 60 * 60


def now_utc() -> datetime:
    return datetime.now(timezone.utc)


def load_json(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {"_error": "invalid_json", "_path": str(path)}


def resolve_path(value: str) -> Path | None:
    if not value:
        return None
    if value.startswith(("kernel ", "http://", "https://")):
        return None
    path = Path(value).expanduser()
    if path.is_absolute():
        return path
    workspace_path = WORKSPACE_ROOT / path
    if workspace_path.exists() or value.startswith(("percolator/", "CYNIC/")):
        return workspace_path
    return ORGANS_ROOT / path


def age_seconds(path: Path, reference: datetime) -> float | None:
    if not path.exists():
        return None
    modified = datetime.fromtimestamp(path.stat().st_mtime, tz=timezone.utc)
    return max(0.0, (reference - modified).total_seconds())


def classify_file(path: Path, reference: datetime) -> dict[str, Any]:
    exists = path.exists()
    age = age_seconds(path, reference)
    status = "missing"
    if exists and age is not None:
        if age <= FRESH_SECONDS:
            status = "fresh"
        elif age <= STALE_SECONDS:
            status = "present"
        else:
            status = "stale"
    return {
        "path": str(path),
        "exists": exists,
        "age_seconds": int(age) if age is not None else None,
        "status": status,
    }


def sqlite_probe(path: Path) -> dict[str, Any] | None:
    if not path.exists() or path.suffix not in {".db", ".sqlite", ".sqlite3"}:
        return None
    probe: dict[str, Any] = {"path": str(path), "openable": False}
    try:
        conn = sqlite3.connect(f"file:{path}?mode=ro", uri=True, timeout=2)
        probe["openable"] = True
        tables = {
            row[0]
            for row in conn.execute(
                "SELECT name FROM sqlite_master WHERE type='table'"
            ).fetchall()
        }
        probe["tables"] = sorted(tables)
        if "messages" in tables:
            probe["messages_total"] = conn.execute("SELECT count(*) FROM messages").fetchone()[0]
            probe["latest_message_date"] = conn.execute(
                "SELECT max(date) FROM messages"
            ).fetchone()[0]
        if "raw_events" in tables:
            probe["raw_events_total"] = conn.execute("SELECT count(*) FROM raw_events").fetchone()[0]
            probe["latest_raw_event"] = conn.execute(
                "SELECT max(captured_at) FROM raw_events"
            ).fetchone()[0]
        if "pipeline_processed" in tables:
            probe["pipeline_processed_total"] = conn.execute(
                "SELECT count(*) FROM pipeline_processed"
            ).fetchone()[0]
        conn.close()
    except Exception as exc:  # noqa: BLE001 - health should record, not crash.
        probe["error"] = exc.__class__.__name__
        probe["detail"] = str(exc)[:240]
    return probe


def declared_paths(ssot: dict[str, Any]) -> list[str]:
    paths: list[str] = []
    for key in ("runtime_outputs", "runtime_inputs", "generated_views"):
        value = ssot.get(key)
        if isinstance(value, list):
            paths.extend(str(item) for item in value if isinstance(item, str))
    storage = ssot.get("storage_contract")
    if isinstance(storage, dict) and storage.get("operational_stream"):
        paths.append(str(storage["operational_stream"]))
    return sorted(dict.fromkeys(paths))


def score_contract(static: dict[str, Any]) -> int:
    score = 0
    for key in (
        "has_manifest",
        "has_desired_state",
        "has_runbook",
        "has_controller",
        "has_ssot_state",
        "runtime_stream_declared",
        "heartbeat_or_queue_declared",
        "runtime_evidence_detected",
    ):
        if static.get(key):
            score += 1
    return score


def classify_organ(static: dict[str, Any], files: list[dict[str, Any]], probes: list[dict[str, Any]]) -> str:
    if static.get("status") not in {"production_organ", "contracted_organ_runtime_unverified", "contracted_organ"}:
        return "not_runtime_trusted"
    if not files:
        return "no_runtime_paths"
    if any(item["status"] == "fresh" for item in files):
        return "live"
    if any(item["status"] == "present" for item in files):
        if any(probe.get("openable") for probe in probes):
            return "warm"
        return "present_unproven"
    if any(item["status"] == "stale" for item in files):
        return "stale"
    return "missing_runtime"


def audit_one(static: dict[str, Any], reference: datetime) -> dict[str, Any]:
    organ = str(static.get("organ"))
    ssot_path = ORGANS_ROOT / organ / "state/SSOT.json"
    ssot = load_json(ssot_path)
    files: list[dict[str, Any]] = []
    probes: list[dict[str, Any]] = []
    for declared in declared_paths(ssot):
        resolved = resolve_path(declared)
        if resolved is None:
            files.append({
                "declared": declared,
                "path": None,
                "exists": None,
                "age_seconds": None,
                "status": "external_or_virtual",
            })
            continue
        status = classify_file(resolved, reference)
        status["declared"] = declared
        files.append(status)
        probe = sqlite_probe(resolved)
        if probe:
            probes.append(probe)

    runtime_status = classify_organ(static, files, probes)
    gaps = list(static.get("gaps") or [])
    if runtime_status in {"missing_runtime", "stale", "no_runtime_paths", "present_unproven"}:
        gaps.append(f"runtime_status={runtime_status}")

    return {
        "organ": organ,
        "contract_status": static.get("status"),
        "runtime_status": runtime_status,
        "contract_score": score_contract(static),
        "manifest_status": static.get("manifest_status"),
        "declared_runtime": files,
        "sqlite_probes": probes,
        "gaps": gaps,
        "next_action": next_action(static, runtime_status),
    }


def next_action(static: dict[str, Any], runtime_status: str) -> str:
    contract = static.get("status")
    if contract in {"uncontracted_directory", "manifest_only", "partial_workflow_organ"}:
        return "Repair or demote the organ contract before relying on runtime behavior."
    if runtime_status == "live":
        return "Keep observing freshness and downstream outputs."
    if runtime_status == "warm":
        return "Runtime exists; add heartbeat freshness or stronger output proof."
    if runtime_status == "stale":
        return "Restart or reconcile the organ, then regenerate downstream views."
    if runtime_status == "missing_runtime":
        return "Run the organ controller or capture path that creates declared runtime outputs."
    return "Add explicit runtime outputs, heartbeat, or degraded queue evidence."


def body_status(organs: list[dict[str, Any]]) -> str:
    production = [organ for organ in organs if organ["contract_status"] == "production_organ"]
    live = [organ for organ in production if organ["runtime_status"] == "live"]
    if not production:
        return "no_production_organs"
    if len(live) == len(production):
        return "live"
    if live:
        return "degraded"
    return "unstable"


def main() -> int:
    reference = now_utc()
    static = load_json(STATIC_AUDIT)
    static_organs = static.get("organs", [])
    organs = [
        audit_one(item, reference)
        for item in static_organs
        if isinstance(item, dict) and item.get("organ")
    ]
    summary: dict[str, Any] = {
        "total_organs": len(organs),
        "body_status": body_status(organs),
        "by_runtime_status": {},
        "by_contract_status": {},
    }
    for organ in organs:
        summary["by_runtime_status"][organ["runtime_status"]] = summary["by_runtime_status"].get(organ["runtime_status"], 0) + 1
        summary["by_contract_status"][organ["contract_status"]] = summary["by_contract_status"].get(organ["contract_status"], 0) + 1

    report = {
        "schema_version": "1.0.0",
        "generated_by": "CYNIC/organs/controllers/audit_runtime.py",
        "generated_at": reference.isoformat(),
        "purpose": "Expose whether CYNIC organs are alive, stale, or merely declared.",
        "freshness_policy": {
            "fresh_seconds": FRESH_SECONDS,
            "stale_seconds": STALE_SECONDS,
            "rule": "A production organ is not trusted unless declared runtime outputs are present and fresh or otherwise probed.",
        },
        "summary": summary,
        "organs": organs,
        "hard_rule": "Do not add a new organ to solve instability; first repair the nearest organ whose runtime_status is not live or warm.",
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    print(json.dumps({"output": str(OUTPUT), "summary": summary}, indent=2, ensure_ascii=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
