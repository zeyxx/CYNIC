"""Body supervision inventory and report generation."""

from __future__ import annotations

import re
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from organs.contracts.src.surface_contracts import read_json_file_or_default, write_json_file, write_jsonl_file_atomic


FRESH_SECONDS = 15 * 60


@dataclass
class OrganRecord:
    name: str
    directory: str | None = None
    registry_present: bool = False
    registry_status: str | None = None
    registry_instance: str | None = None
    registry_role: str | None = None
    filesystem_present: bool = False
    manifest_present: bool = False
    manifest_status: str | None = None
    manifest_role: str | None = None
    local_systemd_units: list[str] = field(default_factory=list)
    infra_systemd_units: list[str] = field(default_factory=list)
    runtime_proven: bool = False
    ssot_path: str | None = None
    ssot_age_seconds: int | None = None
    ssot_status: str | None = None
    ssot_fresh: bool = False
    classification: str = "unknown"
    incoherences: list[str] = field(default_factory=list)


@dataclass
class Incoherence:
    severity: str
    kind: str
    organ: str
    detail: str


@dataclass
class RepairSuggestion:
    severity: str
    action: str
    organ: str
    reason: str
    manual_gate: str = "human_approval_before_repo_mutation"


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat()


def load_json(path: Path) -> dict[str, Any]:
    payload = read_json_file_or_default(path, {})
    return payload if isinstance(payload, dict) else {}


def read_manifest(path: Path) -> dict[str, str]:
    if not path.exists():
        return {}
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        match = re.match(r"^([A-Za-z_][A-Za-z0-9_-]*)\s*:\s*(.+?)\s*$", line)
        if not match:
            continue
        key, value = match.groups()
        result[key] = value.strip().strip('"').strip("'")
    return result


def canonical_name(directory_name: str) -> str:
    return directory_name if directory_name.startswith("organ-") else f"organ-{directory_name}"


def directory_name_for(organ_name: str) -> str:
    return organ_name.removeprefix("organ-")


def list_systemd_units(directory: Path, organ_name: str) -> list[str]:
    if not directory.exists():
        return []
    patterns = [
        f"*{organ_name}*.service",
        f"*{organ_name}*.timer",
        f"*{directory_name_for(organ_name)}*.service",
        f"*{directory_name_for(organ_name)}*.timer",
    ]
    units: set[str] = set()
    for pattern in patterns:
        for path in directory.glob(pattern):
            units.add(path.name)
    return sorted(units)


def unit_matches_instance(instance: str | None, units: list[str]) -> bool:
    if not instance:
        return False
    candidates = {instance, f"{instance}.service"}
    return any(unit in candidates or Path(unit).stem == instance for unit in units)


def ssot_freshness(path: Path) -> tuple[int | None, str | None, bool]:
    if not path.exists():
        return None, None, False
    payload = read_json_file_or_default(path, None)
    if not isinstance(payload, dict):
        return None, "invalid_json", False
    age = max(0, int(datetime.now(timezone.utc).timestamp() - path.stat().st_mtime))
    status = payload.get("status")
    fresh = age <= FRESH_SECONDS and status == "active"
    return age, status, fresh


def has_cli_runtime_evidence(directory: Path | None) -> bool:
    if directory is None:
        return False
    candidates = [
        directory / "src/entrypoints/cli.py",
        directory / "controllers/supervise_body.py",
    ]
    return any(path.exists() for path in candidates)


def is_legacy_blocked(record: OrganRecord) -> bool:
    return record.manifest_status == "legacy_blocked"


def classify(record: OrganRecord) -> str:
    if record.filesystem_present and record.manifest_present and not record.registry_present and is_legacy_blocked(record):
        return "legacy_compatibility_implementation"
    if record.registry_present and record.filesystem_present and record.manifest_present:
        if record.registry_status == "active" and record.runtime_proven and record.ssot_fresh:
            return "declared_and_runtime_wired_fresh"
        if record.registry_status == "active" and record.runtime_proven:
            return "declared_and_runtime_wired"
        return "declared_implemented"
    if record.registry_present and not record.filesystem_present:
        return "declared_without_filesystem"
    if record.filesystem_present and record.manifest_present and not record.registry_present:
        return "emergent_unregistered"
    if record.filesystem_present and not record.manifest_present and not record.registry_present:
        return "unclassified_directory"
    if record.filesystem_present and not record.manifest_present:
        return "filesystem_without_manifest"
    return "unknown"


def build_inventory(
    *,
    workspace_root: Path,
    organs_root: Path,
    registry_path: Path,
    infra_systemd: Path,
) -> tuple[list[OrganRecord], list[Incoherence], list[RepairSuggestion]]:
    registry = load_json(registry_path)
    registered = {
        item["name"]: item
        for item in registry.get("organs", [])
        if isinstance(item, dict) and item.get("name")
    }
    directories = {
        canonical_name(path.name): path
        for path in organs_root.iterdir()
        if path.is_dir() and not path.name.startswith("__")
    }
    names = sorted(set(registered) | set(directories))

    records: list[OrganRecord] = []
    incoherences: list[Incoherence] = []
    repairs: list[RepairSuggestion] = []

    for name in names:
        reg = registered.get(name, {})
        directory = directories.get(name)
        manifest = read_manifest(directory / "MANIFEST.yaml") if directory else {}
        local_units = list_systemd_units(directory / "systemd", name) if directory else []
        infra_units = list_systemd_units(infra_systemd, name)
        instance = reg.get("instance")
        runtime_proven = unit_matches_instance(instance, local_units + infra_units)
        if not runtime_proven and reg.get("cron") == "on demand":
            runtime_proven = has_cli_runtime_evidence(directory)
        ssot_path = directory / "state" / "SSOT.json" if directory else None
        ssot_age_seconds = None
        ssot_status = None
        ssot_fresh = False
        if ssot_path is not None:
            ssot_age_seconds, ssot_status, ssot_fresh = ssot_freshness(ssot_path)

        record = OrganRecord(
            name=name,
            directory=str(directory.relative_to(workspace_root)) if directory else None,
            registry_present=bool(reg),
            registry_status=reg.get("status"),
            registry_instance=instance,
            registry_role=reg.get("role"),
            filesystem_present=directory is not None,
            manifest_present=bool(manifest),
            manifest_status=(manifest.get("status") or "").lower() or None,
            manifest_role=manifest.get("role") or manifest.get("domain"),
            local_systemd_units=local_units,
            infra_systemd_units=infra_units,
            runtime_proven=runtime_proven,
            ssot_path=str(ssot_path.relative_to(workspace_root)) if ssot_path else None,
            ssot_age_seconds=ssot_age_seconds,
            ssot_status=ssot_status,
            ssot_fresh=ssot_fresh,
        )

        if record.registry_status == "active" and not record.filesystem_present:
            record.incoherences.append("registry_active_without_filesystem")
            incoherences.append(
                Incoherence(
                    "high",
                    "registry_active_without_filesystem",
                    name,
                    "Registry declares active organ, but no matching organs/<name> directory exists.",
                )
            )
            repairs.append(
                RepairSuggestion(
                    "high",
                    "create_or_demote_registry_entry",
                    name,
                    "Active registry entry has no filesystem implementation.",
                )
            )

        if record.registry_present and record.manifest_status and record.registry_status:
            if record.manifest_status != str(record.registry_status).lower():
                record.incoherences.append("registry_manifest_status_mismatch")
                incoherences.append(
                    Incoherence(
                        "medium",
                        "registry_manifest_status_mismatch",
                        name,
                        f"registry={record.registry_status} manifest={record.manifest_status}",
                    )
                )
                repairs.append(
                    RepairSuggestion(
                        "medium",
                        "align_manifest_and_registry_status",
                        name,
                        "Manifest and registry disagree on organ status.",
                    )
                )

        if record.filesystem_present and record.manifest_present and not record.registry_present and not is_legacy_blocked(record):
            record.incoherences.append("manifested_organ_missing_registry")
            incoherences.append(
                Incoherence(
                    "medium",
                    "manifested_organ_missing_registry",
                    name,
                    "Organ has a manifest but no registry declaration.",
                )
            )
            repairs.append(
                RepairSuggestion(
                    "medium",
                    "classify_emergence_register_absorb_or_archive",
                    name,
                    "Manifested filesystem organ is outside registry governance.",
                )
            )

        if record.filesystem_present and not record.manifest_present and not record.registry_present:
            record.incoherences.append("unclassified_organ_directory")
            incoherences.append(
                Incoherence(
                    "low",
                    "unclassified_organ_directory",
                    name,
                    "Directory under organs/ is neither registered nor manifested.",
                )
            )

        if record.registry_status == "active" and record.registry_instance and not record.runtime_proven:
            record.incoherences.append("active_instance_without_systemd_unit_evidence")
            incoherences.append(
                Incoherence(
                    "medium",
                    "active_instance_without_systemd_unit_evidence",
                    name,
                    f"Instance {record.registry_instance} is not found in organ-local or infra systemd units.",
                )
            )
        if record.registry_status == "active" and record.runtime_proven and record.ssot_age_seconds is not None and not record.ssot_fresh:
            record.incoherences.append("active_instance_without_fresh_ssot")
            incoherences.append(
                Incoherence(
                    "low",
                    "active_instance_without_fresh_ssot",
                    name,
                    f"Instance {record.registry_instance} has stale or inactive SSOT evidence at {record.ssot_path}.",
                )
            )

        record.classification = classify(record)
        records.append(record)

    repairs.sort(key=lambda item: (item.severity != "high", item.organ, item.action))
    incoherences.sort(key=lambda item: (item.severity != "high", item.organ, item.kind))
    return records, incoherences, repairs


def build_report(
    *,
    workspace_root: Path,
    desired_path: Path,
    registry_path: Path,
    organs_root: Path,
    infra_systemd: Path,
) -> dict[str, Any]:
    desired = load_json(desired_path)
    records, incoherences, repairs = build_inventory(
        workspace_root=workspace_root,
        organs_root=organs_root,
        registry_path=registry_path,
        infra_systemd=infra_systemd,
    )
    classifications: dict[str, int] = {}
    for record in records:
        classifications[record.classification] = classifications.get(record.classification, 0) + 1

    return {
        "schema_version": "2.0.0",
        "generated_by": "organs/supervisor/controllers/supervise_body.py",
        "generated_at": now_utc(),
        "policy": {
            "source": "docs/foundation/CYNIC_ORGANISM_POLICY_MATRIX.md",
            "scale": "organ",
            "dimension": "observability",
            "authority": "infra/registry.json + organs/*/MANIFEST.yaml + systemd evidence",
        },
        "desired_state": desired.get("source_of_truth", str(desired_path)),
        "summary": {
            "organ_count": len(records),
            "incoherence_count": len(incoherences),
            "repair_suggestion_count": len(repairs),
            "classifications": classifications,
            "hard_blocks": [
                "new_organ_creation_requires_supervisor_classification"
            ] if incoherences else [],
        },
        "records": [asdict(record) for record in records],
        "incoherences": [asdict(item) for item in incoherences],
        "repair_suggestions": [asdict(item) for item in repairs],
    }


def write_report(
    *,
    report: dict[str, Any],
    workspace_root: Path,
    supervisor_root: Path,
    desired_path: Path,
    supervision_state: Path,
    ssot_state: Path,
    action_queue: Path,
) -> None:
    supervision_state.parent.mkdir(parents=True, exist_ok=True)
    write_json_file(supervision_state, report, sort_keys=False)
    write_jsonl_file_atomic(action_queue, report["repair_suggestions"], sort_keys=False)
    ssot = {
        "schema_version": "2.0.0",
        "organ": "organ-supervisor",
        "source_of_truth": str(ssot_state.relative_to(workspace_root)),
        "desired_state": str(desired_path.relative_to(workspace_root)),
        "controller": str((supervisor_root / "controllers/supervise_body.py").relative_to(workspace_root)),
        "outputs": [
            str(supervision_state.relative_to(workspace_root)),
            str(action_queue.relative_to(workspace_root)),
        ],
        "last_generated_at": report["generated_at"],
    }
    write_json_file(ssot_state, ssot, sort_keys=False)
