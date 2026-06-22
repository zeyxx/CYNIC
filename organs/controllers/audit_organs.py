#!/usr/bin/env python3
"""Audit CYNIC organ contracts.

This controller is intentionally static and local. It does not start services,
touch runtime databases, or call the kernel. Its job is to keep the body honest:
a directory is not an organ until it exposes the contract pieces the organism
needs to rely on it.
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_ROOT = ROOT.parent.parent
OUTPUT = ROOT / "organ-contract-audit.json"
IGNORED_DIRS = {
    "__pycache__",
    ".git",
    ".next",
    ".venv",
    "node_modules",
    "controllers",
    "state",
}


def has_any(path: Path, patterns: tuple[str, ...]) -> bool:
    if not path.exists() or not path.is_dir():
        return False
    for pattern in patterns:
        if any(path.glob(pattern)):
            return True
    return False


def read_manifest_status(manifest: Path) -> str | None:
    if not manifest.exists():
        return None
    for line in manifest.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("status:"):
            return stripped.split(":", 1)[1].strip().strip('"')
    return None


def declared_path_exists(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    if value.startswith("kernel ") or value.startswith("http://") or value.startswith("https://"):
        return False
    path = Path(value).expanduser()
    if path.is_absolute():
        return path.exists()
    return (WORKSPACE_ROOT / path).exists() or (ROOT / path).exists()


def classify(facts: dict[str, Any]) -> str:
    if facts["has_manifest"] and facts["has_desired_state"] and facts["has_runbook"] and facts["has_controller"] and facts["has_ssot_state"]:
        if facts["runtime_stream_declared"] and facts["heartbeat_or_queue_declared"] and facts["runtime_evidence_detected"]:
            return "production_organ"
        if facts["runtime_stream_declared"] or facts["heartbeat_or_queue_declared"]:
            return "contracted_organ_runtime_unverified"
        return "contracted_organ"
    if facts["has_desired_state"] or facts["has_runbook"] or facts["has_controller"] or facts["has_ssot_state"]:
        return "partial_workflow_organ"
    if facts["has_manifest"]:
        return "manifest_only"
    return "uncontracted_directory"


def gaps(facts: dict[str, Any]) -> list[str]:
    missing: list[str] = []
    checks = [
        ("has_manifest", "missing MANIFEST.yaml"),
        ("has_desired_state", "missing desired/*.json"),
        ("has_runbook", "missing runbooks/*.json"),
        ("has_controller", "missing controllers/*"),
        ("has_ssot_state", "missing state/SSOT.json"),
        ("runtime_stream_declared", "missing declared runtime stream"),
        ("heartbeat_or_queue_declared", "missing heartbeat or degraded queue declaration"),
    ]
    for key, message in checks:
        if not facts[key]:
            missing.append(message)
    if facts["runtime_stream_declared"] and not facts["runtime_evidence_detected"]:
        missing.append("runtime stream declared but no local runtime evidence detected")
    return missing


def audit_organ(path: Path) -> dict[str, Any]:
    manifest = path / "MANIFEST.yaml"
    desired = path / "desired"
    runbooks = path / "runbooks"
    controllers = path / "controllers"
    state = path / "state"
    ssot = state / "SSOT.json"

    has_manifest = manifest.exists()
    has_desired_state = has_any(desired, ("*.json", "*.yaml", "*.yml"))
    has_runbook = has_any(runbooks, ("*.json", "*.md", "*.yaml", "*.yml"))
    has_controller = has_any(controllers, ("*.py", "*.sh", "*.ts", "*.js", "*.rs"))
    has_ssot_state = ssot.exists()

    declared_runtime_paths: list[str] = []
    runtime_stream_declared = None
    heartbeat_or_queue_declared = False
    if has_ssot_state:
        try:
            ssot_data = json.loads(ssot.read_text(encoding="utf-8"))
            runtime_outputs = ssot_data.get("runtime_outputs") or []
            runtime_inputs = ssot_data.get("runtime_inputs") or []
            declared_runtime_paths.extend(str(item) for item in runtime_outputs)
            declared_runtime_paths.extend(str(item) for item in runtime_inputs)
            storage_contract = ssot_data.get("storage_contract") or {}
            runtime_stream_declared = storage_contract.get("operational_stream")
            if runtime_stream_declared:
                declared_runtime_paths.append(str(runtime_stream_declared))
            if not runtime_stream_declared and runtime_outputs:
                runtime_stream_declared = runtime_outputs[0]
            if not runtime_stream_declared and runtime_inputs:
                runtime_stream_declared = runtime_inputs[0]
            heartbeat_or_queue_declared = bool(
                ssot_data.get("heartbeat")
                or ssot_data.get("degraded_queue")
                or ssot_data.get("manual_gates")
                or any("queue" in str(item) for item in runtime_outputs)
            )
        except json.JSONDecodeError:
            runtime_stream_declared = "state/SSOT.json is not valid JSON"

    local_runtime_markers = [
        state / "state.json",
        state / "dataset.jsonl",
        state / "audit.jsonl",
        state / "percolator-watch.json",
        state / "proxy-lifecycle.json",
    ]
    runtime_evidence_detected = any(marker.exists() for marker in local_runtime_markers) or any(
        declared_path_exists(item) for item in declared_runtime_paths
    )

    facts: dict[str, Any] = {
        "organ": path.name,
        "manifest_status": read_manifest_status(manifest),
        "has_manifest": has_manifest,
        "has_desired_state": has_desired_state,
        "has_runbook": has_runbook,
        "has_controller": has_controller,
        "has_ssot_state": has_ssot_state,
        "runtime_stream_declared": runtime_stream_declared,
        "runtime_evidence_detected": runtime_evidence_detected,
        "heartbeat_or_queue_declared": heartbeat_or_queue_declared,
    }
    facts["status"] = classify(facts)
    facts["gaps"] = gaps(facts)
    facts["next_action"] = next_action(facts)
    return facts


def next_action(facts: dict[str, Any]) -> str:
    if facts["status"] == "production_organ":
        return "Keep observing heartbeat and runtime outputs."
    if facts["status"].startswith("contracted_organ"):
        return "Verify runtime evidence and heartbeat before relying on autonomy."
    if facts["status"] == "partial_workflow_organ":
        return "Complete missing contract pieces before adding sibling organs."
    if facts["status"] == "manifest_only":
        return "Add desired state, runbook, controller, SSOT, and runtime contract or mark as experimental."
    return "Either add a full organ contract or move/demote this directory to tooling or sandbox."


def main() -> None:
    organs = []
    for child in sorted(ROOT.iterdir()):
        if not child.is_dir() or child.name in IGNORED_DIRS or child.name.startswith("."):
            continue
        organs.append(audit_organ(child))

    summary: dict[str, int] = {"total_organ_directories": len(organs)}
    for organ in organs:
        summary[organ["status"]] = summary.get(organ["status"], 0) + 1

    report = {
        "schema_version": "1.1.0",
        "generated_by": "CYNIC/organs/controllers/audit_organs.py",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "purpose": "Expose organ maturity so CYNIC stops adding organs onto an unstable body.",
        "contract": {
            "required_for_production": [
                "manifest",
                "desired_state",
                "runbook",
                "controller",
                "ssot_state",
                "runtime_stream",
                "heartbeat_or_degraded_queue",
                "runtime_evidence",
            ],
            "rule": "An organ without desired/runbook/controller/state/runtime evidence is not autonomous.",
        },
        "organs": organs,
        "summary": summary,
        "policy": {
            "before_new_organ": [
                "run this controller",
                "check whether an existing organ owns the domain",
                "if found, extend its desired state instead of creating a sibling",
                "if missing contract pieces, repair them first",
                "only then add new code",
            ],
            "before_bounty_automation": [
                "discord organ remains the observation source",
                "percolator organ owns bounty workflow",
                "no submission automation bypasses human approval",
                "duplicate state must be current before proof work is promoted",
            ],
        },
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=False) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
