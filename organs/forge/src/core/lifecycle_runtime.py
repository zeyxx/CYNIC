"""Lifecycle runtime path helpers for organ-forge."""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path


def _default_registry_path() -> Path:
    return Path(__file__).resolve().parents[4] / "infra" / "registry.json"


def _load_registry(path: str | Path | None = None) -> dict:
    target_path = Path(path) if path else _default_registry_path()
    return json.loads(target_path.read_text())


def _expand_systemd_user_path(value: str, home: str) -> str:
    if value.startswith("%h/"):
        return os.path.join(home, value[3:])
    if value == "%h":
        return home
    return value


def build_lifecycle_runtime_paths(registry: dict | None = None) -> dict:
    from .lifecycle_workflow import build_lifecycle_activation_plan

    plan = build_lifecycle_activation_plan(registry=registry, inspect_fleet=False)
    paths = []
    for node in plan["nodes"]:
        for path_name, path_value in node.get("required_filesystem", {}).items():
            paths.append(
                {
                    "node": node["name"],
                    "role": node["role"],
                    "path": path_name,
                    "value": path_value,
                    "proof": node.get("filesystem_proofs", {}).get(path_name, {}),
                    "required_status": node["required_status"],
                    "current_status": node["current_status"],
                    "materialized_by": "lifecycle_activation",
                    "source": "infra/registry.json:lifecycle.kernel_release.runtime_paths",
                }
            )
    return {
        "status": plan["status"],
        "lifecycle": plan["lifecycle"],
        "paths": paths,
        "blocked": plan["status"] != "ok",
        "blockers": plan["blockers"],
    }


def verify_lifecycle_runtime_paths(
    node_name: str,
    registry: dict | None = None,
    home: str | None = None,
) -> dict:
    if not node_name:
        raise RuntimeError("node_name is required")
    registry = registry or _load_registry()
    paths_payload = build_lifecycle_runtime_paths(registry)
    home_path = home or str(Path.home())
    checks = []
    for path in paths_payload["paths"]:
        if path["node"] != node_name:
            continue
        expanded = _expand_systemd_user_path(path["value"], home_path)
        proof = path.get("proof", {})
        marker = proof.get("marker")
        sentinels = proof.get("sentinels", [])
        marker_path = os.path.join(expanded, marker) if marker else ""
        marker_ok = bool(marker_path) and os.path.exists(marker_path)
        sentinel_checks = [
            {
                "path": sentinel,
                "expanded": os.path.join(expanded, sentinel),
                "exists": os.path.exists(os.path.join(expanded, sentinel)),
            }
            for sentinel in sentinels
        ]
        checks.append(
            {
                "node": node_name,
                "role": path["role"],
                "path": path["path"],
                "value": path["value"],
                "expanded": expanded,
                "proof_kind": proof.get("kind"),
                "marker": marker,
                "marker_exists": marker_ok,
                "sentinels": sentinel_checks,
                "ok": (
                    proof.get("kind") == "git_worktree"
                    and marker_ok
                    and bool(sentinel_checks)
                    and all(item["exists"] for item in sentinel_checks)
                ),
            }
        )
    if not checks:
        raise RuntimeError(f"node {node_name} has no lifecycle runtime paths")
    ok = all(check["ok"] for check in checks)
    verified_at = (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )
    runtime_path_evidence = {
        check["path"]: {
            "verified": check["ok"],
            "verified_at": verified_at,
            "value": check["value"],
            "expanded": check["expanded"],
            "proof_kind": check["proof_kind"],
            "marker_exists": check["marker_exists"],
            "sentinels_verified": all(item["exists"] for item in check["sentinels"]),
        }
        for check in checks
    }
    return {
        "status": "ok" if ok else "blocked",
        "lifecycle": paths_payload["lifecycle"],
        "node": node_name,
        "checks": checks,
        "activation_evidence_patch": {
            "node": node_name,
            "activation_evidence": {
                "runtime_paths": runtime_path_evidence,
            },
        },
    }

