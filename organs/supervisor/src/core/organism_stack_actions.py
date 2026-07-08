"""Convergence action derivation for the L0-L7 organism stack."""

from __future__ import annotations

import json


def convergence_actions(entities: list[dict[str, object]], patterns: list[dict[str, object]]) -> list[dict[str, object]]:
    actions: list[dict[str, object]] = []
    seen: set[str] = set()
    for entity in entities:
        health = entity.get("health")
        command = str(entity.get("next_action") or "")
        if health not in {"blocked", "degraded"} or not command or command in seen:
            continue
        seen.add(command)
        actions.append(_entity_action(entity, command))
    for pattern in patterns:
        if pattern.get("severity") == "info":
            continue
        resolution = str(pattern.get("resolution") or "")
        if not resolution or resolution in seen:
            continue
        seen.add(resolution)
        actions.append(_pattern_action(pattern, resolution))
    return actions


def dispatchable_requests(actions: list[dict[str, object]]) -> list[dict[str, object]]:
    requests: list[dict[str, object]] = []
    for action in actions:
        if not action.get("mutates_mempool"):
            continue
        idempotency_key = _action_idempotency_key(action)
        content = {
            "lifecycle": "organism_convergence",
            "layer": action.get("layer"),
            "entity": action.get("entity"),
            "kind": action.get("kind"),
            "objective": action.get("action"),
            "reason": action.get("reason"),
            "consumer": action.get("consumer"),
            "coord_targets": action.get("coord_targets", []),
            "proof_commands": action.get("proof_commands", []),
            "done_when": action.get("done_when", []),
            "causal_parent": "cynic.organism.stack.v1",
            "idempotency_key": idempotency_key,
        }
        if action.get("pattern_id"):
            content["pattern_id"] = action.get("pattern_id")
        if action.get("fingerprint"):
            content["fingerprint"] = action.get("fingerprint")
        if action.get("caused_by_pattern_ids"):
            content["caused_by_pattern_ids"] = action.get("caused_by_pattern_ids")
        request = {
            "kind": "hermes",
            "domain": "organism-convergence",
            "agent_id": "organ-supervisor",
            "consumer": action.get("consumer"),
            "priority": action.get("priority"),
            "coord_targets": action.get("coord_targets", []),
            "content": json.dumps(content, sort_keys=True),
            "mutates_mempool": True,
            "read_back": "pending or completed Kernel task with the same idempotency_key",
        }
        if action.get("pattern_id"):
            request["pattern_id"] = action.get("pattern_id")
        if action.get("fingerprint"):
            request["fingerprint"] = action.get("fingerprint")
        if action.get("caused_by_pattern_ids"):
            request["caused_by_pattern_ids"] = action.get("caused_by_pattern_ids")
        requests.append(request)
    return requests


def _action_idempotency_key(action: dict[str, object]) -> str:
    pattern_id = action.get("pattern_id")
    if pattern_id:
        return f"organism-convergence:{pattern_id}"
    return f"organism-convergence:{action.get('layer')}:{action.get('entity')}"


def _entity_action(entity: dict[str, object], command: str) -> dict[str, object]:
    owner = str(entity.get("owner") or "")
    action = {
        "schema": "cynic.organism.convergence_action.v1",
        "layer": entity.get("layer"),
        "entity": entity.get("entity"),
        "kind": "operator_check",
        "action": command,
        "reason": entity.get("observed"),
        "consumer": _consumer_for_entity(entity),
        "priority": _priority_for_health(entity.get("health")),
        "coord_targets": _coord_targets_for_owner(owner),
        "mutates_files": False,
        "mutates_mempool": False,
        "proof_commands": [command],
        "done_when": [
            f"{entity.get('entity')} health becomes ok or the blocker is replaced by a narrower blocker",
            "cynic organism --json reports updated L0-L7 stack evidence",
        ],
    }
    if command == "cynic mcp-reload":
        action.update(
            {
                "kind": "lifecycle_dispatch",
                "consumer": "organ-forge-hermes-agent",
                "coord_targets": ["crates/cynic-mcp", ".cortex/mcp/run_cynic_mcp.py", ".codex/config.toml"],
                "mutates_mempool": True,
                "proof_commands": ["cynic mcp-reload --json", "cynic organism --json"],
                "done_when": [
                    "cynic mcp-reload --json returns task ids or duplicate pending read-back",
                    "CYNIC_MCP_CHECK_ONLY=1 python3 ./.cortex/mcp/run_cynic_mcp.py reports a fresh executable",
                    "cynic organism --json reports cynic-mcp health ok",
                ],
            }
        )
    elif command == "python3 organs/browser/src/entrypoints/activate_codex_browser_mcp.py --install-deps":
        action.update(
            {
                "kind": "organ_activation",
                "consumer": "organ-browser",
                "coord_targets": ["organs/browser", ".cortex/mcp/run_browser_mcp.py"],
                "mutates_files": False,
                "proof_commands": [
                    "python3 organs/browser/src/entrypoints/activate_codex_browser_mcp.py --install-deps",
                    "python3 organs/browser/src/entrypoints/diagnose_browser_mcp.py",
                    "cynic organism --json",
                ],
                "done_when": [
                    ".cynic/organs/browser/.venv/bin/python can import organs.browser.src.adapters.mcp_server_runtime",
                    "cynic organism --json reports organ-browser-mcp health ok",
                ],
            }
        )
    elif command == "python3 organs/browser/src/entrypoints/activate_codex_browser_mcp.py --start-services":
        action.update(
            {
                "kind": "organ_activation",
                "consumer": "organ-browser",
                "coord_targets": ["organs/browser", "infra/systemd/organ-browser-hub.service"],
                "mutates_files": False,
                "proof_commands": [
                    "python3 organs/browser/src/entrypoints/activate_codex_browser_mcp.py --start-services",
                    "python3 organs/browser/src/entrypoints/diagnose_browser_mcp.py",
                    "cynic organism --json",
                ],
                "done_when": [
                    "Browser Hub /status responds on ORGAN_BROWSER_HUB_URL or http://127.0.0.1:40770",
                    "Browser Hub /status reports chrome=reachable when browser perception is required",
                    "cynic organism --json reports organ-browser-hub and organ-browser-cdp health ok",
                ],
            }
        )
    elif command == "make lint-repo-topology-contract":
        action.update(
            {
                "kind": "contract_refactor",
                "consumer": "organ-supervisor",
                "mutates_files": True,
                "proof_commands": ["make lint-repo-topology-contract", "cynic organism --json"],
                "done_when": [
                    "organ shape gaps are either completed or declared as bounded topology exceptions",
                    "cynic organism --json no longer reports organ_shape_fragmentation for the same organs",
                ],
            }
        )
    return action


def _pattern_action(pattern: dict[str, object], resolution: str) -> dict[str, object]:
    pattern_name = str(pattern.get("pattern") or "unknown_pattern")
    action = {
        "schema": "cynic.organism.convergence_action.v1",
        "layer": "cross-layer",
        "entity": pattern_name,
        "pattern_id": pattern.get("pattern_id"),
        "fingerprint": pattern.get("fingerprint"),
        "kind": "pattern_convergence",
        "action": resolution,
        "reason": pattern.get("observed") or pattern.get("severity"),
        "consumer": "organ-supervisor",
        "priority": _priority_for_severity(pattern.get("severity")),
        "coord_targets": _coord_targets_for_pattern(pattern),
        "mutates_files": pattern_name in {
            "duplicate_organ_identity",
            "overlapping_organ_domain",
            "domain_without_primary_owner",
            "lifecycle_supply_chain_topology_gap",
            "organ_shape_fragmentation",
            "legacy_script_muscle_fragmentation",
            "utility_duplicate_pattern",
            "async_muscle_without_readback",
        },
        "mutates_mempool": False,
        "proof_commands": ["cynic organism --json"],
        "done_when": [
            f"cynic organism --json no longer reports {pattern_name}",
            "remaining drift is represented by narrower entity-level blockers",
        ],
    }
    if pattern_name == "overlapping_organ_domain":
        action["proof_commands"] = ["cynic organism --json", "make lint-repo-topology-contract"]
        action["done_when"] = [
            "each overlapping domain has exactly one declared primary owner",
            "secondary organs declare adapter/consumer role instead of competing function ownership",
        ]
    elif pattern_name == "domain_without_primary_owner":
        action["kind"] = "topology_refactor"
        action["proof_commands"] = ["cynic organism --json", "make lint-repo-topology-contract"]
        action["done_when"] = [
            "each reported domain has exactly one primary owner",
            "sensor/consumer/adapter organs no longer form ownerless functional islands",
        ]
    elif pattern_name == "lifecycle_supply_chain_topology_gap":
        action["kind"] = "supply_chain_topology_refactor"
        action["consumer"] = "organ-forge-hermes-agent"
        action["coord_targets"] = ["infra/registry.json", "organs/forge", "organs/supervisor"]
        action["proof_commands"] = ["cynic lifecycle diagnose", "cynic organism --json"]
        action["done_when"] = [
            "registry resolves exactly one build runner and one deploy target",
            "deploy target role has exactly one deploy_target=true node",
            "build runner and deploy target satisfy lifecycle.required_node_status",
        ]
    elif pattern_name == "legacy_script_muscle_fragmentation":
        action["coord_targets"] = ["scripts", "axioms/repo-topology-v1.json"]
        action["proof_commands"] = ["make lint-repo-topology-contract"]
    elif pattern_name == "utility_duplicate_pattern":
        action["kind"] = "adapter_refactor"
        action["proof_commands"] = ["cynic converge --json"]
        action["done_when"] = [
            "duplicated utility functions have one shared owner adapter",
            "cynic converge --json no longer reports the same utility_duplicate_pattern entries",
        ]
    elif pattern_name == "async_muscle_without_readback":
        action["kind"] = "async_contract_refactor"
        action["consumer"] = "organ-forge-hermes-agent"
        action["proof_commands"] = ["cynic converge --json", "cynic organism --json"]
        action["done_when"] = [
            "each async dispatch path proves pending or completed read-back",
            "dispatch payloads include a stable causal_parent and idempotency_key where applicable",
            "cynic converge --json no longer reports async_muscle_without_readback for the same files",
        ]
    return action


def _consumer_for_entity(entity: dict[str, object]) -> str:
    layer = str(entity.get("layer") or "")
    owner = str(entity.get("owner") or "")
    if layer == "L3" and "cynic-mcp" in str(entity.get("entity")):
        return "organ-forge-hermes-agent"
    if "organs/browser" in owner:
        return "organ-browser"
    if layer == "L4":
        return "organ-supervisor"
    if layer == "L5":
        return "organ-forge-hermes-agent"
    return "organ-supervisor"


def _priority_for_health(health: object) -> int:
    return {"blocked": 90, "degraded": 60}.get(str(health), 30)


def _priority_for_severity(severity: object) -> int:
    return {"critical": 95, "warning": 55}.get(str(severity), 30)


def _coord_targets_for_owner(owner: str) -> list[str]:
    if not owner:
        return []
    if owner.startswith("organs/"):
        parts = owner.split("/")
        return ["/".join(parts[:2])] if len(parts) >= 2 else [owner]
    if owner.startswith(".codex/") or owner.startswith(".cortex/"):
        return [owner]
    if owner.startswith("crates/"):
        parts = owner.split("/")
        return ["/".join(parts[:2])] if len(parts) >= 2 else [owner]
    if owner.startswith("infra/"):
        return [owner.split(":", 1)[0]]
    if owner.startswith("axioms/"):
        return [owner]
    return [owner.split(":", 1)[0]]


def _coord_targets_for_pattern(pattern: dict[str, object]) -> list[str]:
    pattern_name = str(pattern.get("pattern") or "")
    if pattern_name == "overlapping_organ_domain":
        entities = pattern.get("entities")
        targets: list[str] = ["axioms/organ-layer-topology-v1.json", "axioms/organ-maturity-inventory-v1.json"]
        if isinstance(entities, dict):
            for names in entities.values():
                if isinstance(names, list):
                    targets.extend(f"organs/{str(name).removeprefix('organ-')}" for name in names)
        return _unique(targets)
    if pattern_name == "domain_without_primary_owner":
        entities = pattern.get("entities")
        targets = ["axioms/organ-layer-topology-v1.json", "axioms/organ-maturity-inventory-v1.json"]
        if isinstance(entities, dict):
            for names in entities.values():
                if isinstance(names, list):
                    targets.extend(f"organs/{str(name).removeprefix('organ-')}" for name in names)
        return _unique(targets)
    if pattern_name == "lifecycle_supply_chain_topology_gap":
        return ["infra/registry.json", "organs/forge", "organs/supervisor"]
    if pattern_name == "organ_shape_fragmentation":
        entities = pattern.get("entities")
        if isinstance(entities, list):
            return _unique(f"organs/{str(name).removeprefix('organ-')}" for name in entities)
    if pattern_name == "duplicate_organ_identity":
        return ["organs", "axioms/organ-maturity-inventory-v1.json"]
    if pattern_name == "utility_duplicate_pattern":
        entities = pattern.get("entities")
        targets: list[str] = []
        if isinstance(entities, dict):
            for paths in entities.values():
                if isinstance(paths, list):
                    targets.extend(str(path) for path in paths)
        return _unique(targets)
    if pattern_name == "async_muscle_without_readback":
        entities = pattern.get("entities")
        return _unique(entities if isinstance(entities, list) else [])
    return []


def _unique(values: object) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    if isinstance(values, (str, bytes)):
        iterable = [values]
    else:
        try:
            iterable = list(values)  # type: ignore[arg-type]
        except TypeError:
            iterable = []
    for value in iterable:
        text = str(value)
        if not text or text in seen:
            continue
        seen.add(text)
        result.append(text)
    return result
