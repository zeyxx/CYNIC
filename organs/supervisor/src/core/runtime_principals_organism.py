"""Pure builders for organism activation reports."""

from __future__ import annotations


def normalize_activation_action(request: dict[str, object]) -> dict[str, object]:
    coord_targets = request.get("coord_targets")
    if not isinstance(coord_targets, list):
        coord_targets = []
    target = request.get("node") or request.get("code")
    if not target and coord_targets:
        target = ",".join(str(item) for item in coord_targets)
    return {
        "action": request.get("action"),
        "target": target,
        "node": request.get("node"),
        "role": request.get("role"),
        "code": request.get("code"),
        "priority": request.get("priority"),
        "consumer": request.get("consumer"),
        "coord_targets": coord_targets,
    }


def unique_strings(values: list[object]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        if value is None:
            continue
        text = str(value)
        if text in seen:
            continue
        seen.add(text)
        result.append(text)
    return result


def activation_dispatch_plan(actions: list[dict[str, object]], dispatch_command: object) -> dict[str, object]:
    command = str(dispatch_command or "cynic activate")
    coord_targets = unique_strings(
        [
            target
            for action in actions
            for target in action.get("coord_targets", [])
        ]
    )
    consumers = unique_strings([action.get("consumer") for action in actions])
    return {
        "command": command,
        "mutates_mempool": True,
        "request_count": len(actions),
        "consumers": consumers,
        "coord_targets": coord_targets,
        "operator_sequence": [
            {"step": "inspect", "command": "cynic activate --dry-run", "mutates_mempool": False},
            {"step": "dispatch", "command": command, "mutates_mempool": True},
            {"step": "dispatch_json", "command": "cynic activate --json", "mutates_mempool": True},
            {"step": "observe", "command": "cynic actions", "mutates_mempool": False},
            {"step": "observe", "command": "cynic organism", "mutates_mempool": False},
        ],
        "proof_commands": ["cynic actions", "cynic organism"],
        "read_back": "cynic activate or cynic activate --json must report dispatched task ids or duplicate pending tasks visible from the Kernel mempool",
        "done_when": [
            "all activation requests have a consumer, priority and coord_targets",
            "dispatch returns read-back evidence from organ-forge/kernel",
            "cynic actions and cynic organism show reduced activation pressure after consumers complete the tasks",
        ],
    }


def activation_proof_requirements(
    actions: list[dict[str, object]],
    activation: dict[str, object],
    dispatch_command: object,
) -> list[dict[str, object]]:
    has_actions = bool(actions)
    action_contract_ok = all(
        action.get("consumer")
        and action.get("priority") is not None
        and isinstance(action.get("coord_targets"), list)
        and action.get("coord_targets")
        for action in actions
    )
    dispatch_status = str(activation.get("status", "unknown"))
    return [
        {
            "check": "activation_request_inventory",
            "status": "satisfied" if has_actions else "missing",
            "command": "cynic actions --json",
            "expected": "activation_request_summary is visible and contains pending organ actions",
            "observed": len(actions),
        },
        {
            "check": "activation_request_contract",
            "status": "satisfied" if action_contract_ok else "missing",
            "command": "cynic activate --dry-run",
            "expected": "each activation request declares consumer, priority and coord_targets",
            "observed": action_contract_ok,
        },
        {
            "check": "activation_dispatch_read_back",
            "status": "satisfied" if dispatch_status in {"ok", "ready"} else "missing",
            "command": f"{dispatch_command or 'cynic activate'} --json",
            "mutates_mempool": True,
            "expected": "dispatch reports task ids or duplicate pending tasks with Kernel read-back",
            "observed": dispatch_status,
        },
    ]
