"""Pattern metabolism for the L0-L7 organism stack."""

from __future__ import annotations

import hashlib
import json


def annotate_patterns(patterns: list[dict[str, object]]) -> list[dict[str, object]]:
    annotated: list[dict[str, object]] = []
    for pattern in patterns:
        item = dict(pattern)
        fingerprint_source = {
            "pattern": item.get("pattern"),
            "severity": item.get("severity"),
            "entities": item.get("entities"),
            "observed": item.get("observed"),
        }
        fingerprint = hashlib.sha256(
            json.dumps(fingerprint_source, sort_keys=True, default=str).encode("utf-8")
        ).hexdigest()[:16]
        item["pattern_id"] = f"{item.get('pattern', 'unknown')}:{fingerprint}"
        item["fingerprint"] = fingerprint
        annotated.append(item)
    return annotated


def attach_pattern_causes(actions: list[dict[str, object]], patterns: list[dict[str, object]]) -> None:
    for action in actions:
        caused_by: set[str] = set()
        action_text = str(action.get("action") or "")
        entity = str(action.get("entity") or "")
        for pattern in patterns:
            pattern_id = pattern.get("pattern_id")
            if not pattern_id or pattern.get("severity") == "info":
                continue
            entities = pattern.get("entities")
            if pattern.get("resolution") == action_text or _pattern_mentions_entity(entities, entity):
                caused_by.add(str(pattern_id))
        if caused_by:
            action["caused_by_pattern_ids"] = sorted(caused_by)


def pattern_evolution(
    patterns: list[dict[str, object]],
    actions: list[dict[str, object]],
    requests: list[dict[str, object]],
) -> dict[str, object]:
    by_severity: dict[str, int] = {}
    active_ids: list[str] = []
    blocking_ids: list[str] = []
    informational_ids: list[str] = []
    family_index: dict[str, list[str]] = {}
    for pattern in patterns:
        family = str(pattern.get("pattern") or "unknown")
        pattern_id = str(pattern.get("pattern_id") or family)
        severity = str(pattern.get("severity") or "unknown")
        active_ids.append(pattern_id)
        family_index.setdefault(family, []).append(pattern_id)
        by_severity[severity] = by_severity.get(severity, 0) + 1
        if severity == "info":
            informational_ids.append(pattern_id)
        else:
            blocking_ids.append(pattern_id)
    return {
        "schema": "cynic.organism.pattern_evolution.v1",
        "current_active_ids": sorted(active_ids),
        "family_index": {key: sorted(value) for key, value in sorted(family_index.items())},
        "blocking_ids": sorted(blocking_ids),
        "informational_ids": sorted(informational_ids),
        "action_pattern_ids": sorted(_action_pattern_ids(actions)),
        "dispatchable_pattern_ids": sorted(_request_pattern_ids(requests)),
        "by_severity": dict(sorted(by_severity.items())),
        "comparison_contract": {
            "stable_key": "pattern_id",
            "family_key": "pattern",
            "disappeared": "pattern_id present in previous snapshot and absent from current_active_ids",
            "persisted": "pattern_id present in both snapshots",
            "mutated": "same pattern family with different fingerprint",
            "new": "pattern_id absent from previous snapshot and present in current_active_ids",
        },
    }


def _action_pattern_ids(actions: list[dict[str, object]]) -> set[str]:
    ids: set[str] = set()
    for action in actions:
        if action.get("pattern_id"):
            ids.add(str(action["pattern_id"]))
        caused_by = action.get("caused_by_pattern_ids")
        if isinstance(caused_by, list):
            ids.update(str(item) for item in caused_by if item)
    return ids


def _request_pattern_ids(requests: list[dict[str, object]]) -> set[str]:
    ids: set[str] = set()
    for request in requests:
        if request.get("pattern_id"):
            ids.add(str(request["pattern_id"]))
        caused_by = request.get("caused_by_pattern_ids")
        if isinstance(caused_by, list):
            ids.update(str(item) for item in caused_by if item)
    return ids


def _pattern_mentions_entity(entities: object, entity: str) -> bool:
    if not entity:
        return False
    if isinstance(entities, list):
        return entity in {str(item) for item in entities}
    if isinstance(entities, dict):
        for value in entities.values():
            if isinstance(value, list) and entity in {str(item) for item in value}:
                return True
    return False
