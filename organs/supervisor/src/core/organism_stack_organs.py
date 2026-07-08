"""Organ-shape metabolism for the L0-L7 organism stack."""

from __future__ import annotations

import re
from pathlib import Path

from organs.supervisor.src.core.repo_topology_host_validation import iter_immediate_organ_dirs
from organs.supervisor.src.core.organism_stack_utils import (
    read_text,
    rel,
    yaml_multiline_value,
    yaml_scalar,
)


def organ_manifests(root: Path) -> list[dict[str, object]]:
    organs_root = root / "organs"
    manifests: list[dict[str, object]] = []
    if not organs_root.is_dir():
        return manifests
    for organ_dir in iter_immediate_organ_dirs(organs_root):
        manifest = organ_dir / "MANIFEST.yaml"
        text = read_text(manifest)
        manifests.append(
            {
                "name": yaml_scalar(text, "id") or yaml_scalar(text, "name") or f"organ-{organ_dir.name}",
                "path": rel(root, organ_dir),
                "manifest": rel(root, manifest) if manifest.is_file() else None,
                "has_manifest": manifest.is_file(),
                "status": (yaml_scalar(text, "status") or "missing_manifest").lower(),
                "domain": yaml_scalar(text, "domain"),
                "domain_role": (yaml_scalar(text, "domain_role") or "primary").lower(),
                "description": yaml_multiline_value(text, "description"),
                "entrypoint_count": len(re.findall(r"^\s*-\s+name:\s+", text, flags=re.MULTILINE)),
                "has_core": (organ_dir / "src" / "core").is_dir(),
                "has_adapters": (organ_dir / "src" / "adapters").is_dir(),
                "has_entrypoints": (organ_dir / "src" / "entrypoints").is_dir(),
                "has_systemd": (organ_dir / "systemd").is_dir(),
                "has_state": (organ_dir / "state").is_dir(),
            }
        )
    return manifests


def organ_entities(manifests: list[dict[str, object]]) -> list[dict[str, object]]:
    entities: list[dict[str, object]] = []
    for manifest in manifests:
        missing = organ_shape_missing_parts(manifest)
        status = str(manifest.get("status") or "").lower()
        bounded_exception = status in {"transitional", "quarantined"}
        health = "ok"
        if missing:
            if bounded_exception and manifest.get("has_manifest"):
                health = "unknown"
            else:
                health = "degraded" if manifest.get("has_manifest") else "blocked"
        entities.append(
            {
                "layer": "L4",
                "entity": manifest["name"],
                "kind": "organ",
                "desired": "manifest-backed bounded subsystem",
                "observed": f"status={manifest.get('status')}, missing={','.join(missing) if missing else 'none'}",
                "health": health,
                "owner": manifest.get("manifest") or manifest.get("path"),
                "next_action": "make lint-repo-topology-contract"
                if missing and not bounded_exception
                else "resolve transition or quarantine before activation"
                if missing and bounded_exception
                else "observe through organ health",
            }
        )
    return entities


def organ_shape_fragmentation_patterns(manifests: list[dict[str, object]]) -> list[dict[str, object]]:
    patterns: list[dict[str, object]] = []
    active_missing_shape = [
        str(item.get("name"))
        for item in manifests
        if organ_shape_missing_parts(item)
        and str(item.get("status") or "").lower() not in {"transitional", "quarantined"}
    ]
    if active_missing_shape:
        patterns.append(
            {
                "pattern": "organ_shape_fragmentation",
                "severity": "warning",
                "entities": active_missing_shape[:24],
                "count": len(active_missing_shape),
                "resolution": "complete canonical organ shape or declare bounded exception",
            }
        )
    bounded_shape_exceptions = [
        str(item.get("name"))
        for item in manifests
        if organ_shape_missing_parts(item)
        and str(item.get("status") or "").lower() in {"transitional", "quarantined"}
    ]
    if bounded_shape_exceptions:
        patterns.append(
            {
                "pattern": "bounded_noncanonical_surface",
                "severity": "info",
                "entities": bounded_shape_exceptions[:24],
                "count": len(bounded_shape_exceptions),
                "resolution": "keep quarantined/transitional surfaces non-active until reforged or deleted",
            }
        )
    return patterns


def organ_shape_missing_parts(manifest: dict[str, object]) -> list[str]:
    return [
        key.removeprefix("has_")
        for key in ("has_manifest", "has_core", "has_adapters", "has_entrypoints")
        if not manifest.get(key)
    ]
