"""Core lifecycle controller for organ-lifecycle."""

from __future__ import annotations

import json
import os
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict

from organs.contracts.src.surface_contracts import read_json_file_or_default
from organs.lifecycle.src.entrypoints._bootstrap import bootstrap_repo_path


FRESHNESS_STALE_SECONDS = 15 * 60


def repo_root() -> Path:
    project_root = os.environ.get("CYNIC_PROJECT_ROOT")
    if project_root:
        return Path(project_root).expanduser().resolve()
    return bootstrap_repo_path()


@dataclass(frozen=True)
class LifecycleControllerPaths:
    desired_path: Path
    ssot_path: Path

    @classmethod
    def from_repo_root(cls, root: Path) -> "LifecycleControllerPaths":
        lifecycle_root = root / "organs" / "lifecycle"
        return cls(
            desired_path=lifecycle_root / "desired" / "state.json",
            ssot_path=lifecycle_root / "state" / "SSOT.json",
        )


class LifecycleController:
    """Owns the local lifecycle convergence loop for organ-lifecycle."""

    def __init__(self, runtime=None, paths: LifecycleControllerPaths | None = None):
        self.runtime = runtime
        self.paths = paths or LifecycleControllerPaths.from_repo_root(repo_root())

    def _runtime(self):
        if self.runtime is None:
            from organs.lifecycle.src.entrypoints.runtime import build_lifecycle_runtime

            self.runtime = build_lifecycle_runtime()
        return self.runtime

    def _read_json(self, path: Path, default: Dict[str, Any]) -> Dict[str, Any]:
        data = read_json_file_or_default(path, dict(default))
        return data if isinstance(data, dict) else dict(default)

    def _write_json(self, path: Path, payload: Dict[str, Any]) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")

    def _freshness_age_seconds(self) -> int | None:
        if not self.paths.ssot_path.exists():
            return None
        age = datetime.now(timezone.utc).timestamp() - self.paths.ssot_path.stat().st_mtime
        return max(0, int(age))

    def _paths_payload(self) -> dict[str, str]:
        return {
            "desired": str(self.paths.desired_path),
            "ssot": str(self.paths.ssot_path),
        }

    def health(self) -> Dict[str, Any]:
        desired = self._read_json(self.paths.desired_path, {})
        ssot = self._read_json(self.paths.ssot_path, {})
        age = self._freshness_age_seconds()
        if not desired.get("desired", {}).get("auto_remediation_enabled", False):
            return {
                "status": "blocked",
                "reason": "auto_remediation_disabled",
                "fresh": False,
                "ssot_age_seconds": age,
                "paths": self._paths_payload(),
            }
        if age is None:
            return {
                "status": "blocked",
                "reason": "ssot_missing",
                "fresh": False,
                "ssot_age_seconds": None,
                "paths": self._paths_payload(),
            }
        ssot_status = ssot.get("status", "unknown")
        if age > FRESHNESS_STALE_SECONDS:
            return {
                "status": "degraded",
                "reason": "ssot_stale",
                "fresh": False,
                "ssot_status": ssot_status,
                "ssot_age_seconds": age,
                "paths": self._paths_payload(),
            }
        if ssot_status != "active":
            return {
                "status": "degraded",
                "reason": "ssot_not_active",
                "fresh": True,
                "ssot_status": ssot_status,
                "ssot_age_seconds": age,
                "paths": self._paths_payload(),
            }
        return {
            "status": "ok",
            "reason": "fresh_active_ssot",
            "fresh": True,
            "ssot_status": ssot_status,
            "ssot_age_seconds": age,
            "paths": self._paths_payload(),
        }

    async def run_once(self) -> Dict[str, Any]:
        desired = self._read_json(self.paths.desired_path, {})
        if not desired.get("desired", {}).get("auto_remediation_enabled", False):
            return {
                "status": "blocked",
                "reason": "auto_remediation_disabled",
                "paths": self._paths_payload(),
            }

        report = await self._runtime().run_once()
        ssot = self._read_json(self.paths.ssot_path, {})
        analysis = report.get("report", {})
        observed_at = report.get("signal", {}).get("timestamp") or (
            datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
        )
        ssot.update(
            {
                "last_synced": observed_at,
                "active_provisioning_tasks": len(analysis.get("unresolved_dependencies", [])),
                "status": "active" if report.get("status") == "ok" else "degraded",
                "dependency_cycles": analysis.get("cycles", []),
                "unresolved_dependencies": analysis.get("unresolved_dependencies", []),
                "supply_chain_summary": analysis.get("summary", {}),
                "execution_layers": analysis.get("execution", {}).get("state_layers", {}),
            }
        )
        self._write_json(self.paths.ssot_path, ssot)
        return {
            "status": report.get("status", "unknown"),
            "report": report,
            "ssot": ssot,
            "paths": self._paths_payload(),
        }

    async def reconcile(self) -> Dict[str, Any]:
        health_before = self.health()
        if health_before.get("status") == "ok":
            return {
                "action": "noop",
                "health_before": health_before,
                "health_after": health_before,
            }
        run_result = await self.run_once()
        health_after = self.health()
        return {
            "action": "reconciled",
            "health_before": health_before,
            "run_result": run_result,
            "health_after": health_after,
        }


async def run_controller_once() -> Dict[str, Any]:
    controller = LifecycleController()
    return await controller.run_once()
