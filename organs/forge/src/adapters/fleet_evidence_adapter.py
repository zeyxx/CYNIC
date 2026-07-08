"""Fleet evidence collection adapters for organ-forge lifecycle checks."""

from __future__ import annotations

import json
import os
import subprocess
from collections.abc import Mapping

from organs.contracts.src.surface_contracts import json_object_or_default


from ..core.lifecycle_readiness import inspect_fleet as _readiness_inspect_fleet


class FleetEvidenceAdapter:
    @staticmethod
    def extract_inventory_from_health(payload: dict) -> dict:
        environment = payload.get("environment")
        if not isinstance(environment, dict):
            return {"enabled": True, "status": "missing_environment"}

        probes = environment.get("probes")
        if not isinstance(probes, list):
            return {"enabled": True, "status": "missing_probes"}

        for probe in probes:
            if not isinstance(probe, dict):
                continue
            if probe.get("name") != "fleet_inventory":
                continue
            details = probe.get("details")
            if isinstance(details, dict) and details.get("kind") == "fleet_inventory":
                data = details.get("data", {})
            else:
                data = details if isinstance(details, dict) else {}
            if not isinstance(data, dict):
                return {
                    "enabled": True,
                    "status": str(probe.get("status", "unknown")),
                    "error": "fleet_inventory_data_not_object",
                }
            summary = data.get("summary", {})
            nodes = data.get("nodes", [])
            extracted = {
                "enabled": True,
                "status": str(probe.get("status", "unknown")),
                "name": probe.get("name", "fleet_inventory"),
                "summary": summary if isinstance(summary, dict) else {},
            }
            if nodes:
                extracted["nodes"] = nodes
            if probe.get("error"):
                extracted["error"] = str(probe.get("error"))
            if probe.get("duration_ms") is not None:
                extracted["duration_ms"] = probe["duration_ms"]
            return extracted
        return {"enabled": True, "status": "missing_probe"}

    @staticmethod
    def extract_inventory_from_fleet_report(report: dict) -> dict:
        summary = report.get("summary", {})
        if not isinstance(summary, dict):
            return {
                "enabled": True,
                "status": "error",
                "error": "fleet_report_summary_missing",
            }
        anomalies = report.get("anomalies", [])
        evidence_gaps = [
            f"{item.get('kind')}:{item.get('target')}"
            for item in anomalies
            if isinstance(item, dict) and item.get("severity") == "critical"
        ]
        return {
            "enabled": True,
            "status": "ok" if summary.get("status") == "ok" else "degraded",
            "name": "organ_fleet_report",
            "summary": {
                "declared_nodes": summary.get("declared_nodes", 0),
                "observed_nodes": summary.get("observed_resources", 0),
                "matched_nodes": max(
                    0,
                    int(summary.get("declared_nodes", 0))
                    - int(summary.get("critical_anomaly_count", 0)),
                ),
                "missing_nodes": summary.get("critical_anomaly_count", 0),
                "unexpected_nodes": 0,
                "provider_evidence_gaps": evidence_gaps,
            },
        }

    def probe_fleet_inventory_report(
        self,
        env: Mapping[str, str] | None = None,
    ) -> dict:
        env = env or os.environ
        output = subprocess.run(
            [
                "python3",
                "organs/fleet/src/entrypoints/cli.py",
                "report",
                "--json",
                "--registry",
                "infra/registry.json",
            ]
            + (
                ["--proxmox-list-text", env["CYNIC_FLEET_PROXMOX_LIST_TEXT"]]
                if env.get("CYNIC_FLEET_PROXMOX_LIST_TEXT")
                else []
            ),
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
        if output.returncode != 0:
            return {
                "enabled": True,
                "name": "organ_fleet_report",
                "status": "error",
                "error": "organ_fleet_report_failed",
            }
        report = json_object_or_default(output.stdout, {})
        if not report:
            return {
                "enabled": True,
                "name": "organ_fleet_report",
                "status": "error",
                "error": "organ_fleet_report_invalid_json",
            }
        return self.extract_inventory_from_fleet_report(report)

    def inspect_fleet(self, build_target: str, deploy_target: str) -> dict:
        return _readiness_inspect_fleet(build_target, deploy_target)
