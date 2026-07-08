"""Runtime principals operator sequencing helpers."""

from __future__ import annotations


def organism_blocker_check(surface: object) -> str | None:
    checks = {
        "cortex_runtime": "cynic status",
        "mcp_freshness": "cynic mcp-reload --dry-run",
        "storage_primary": "cynic storage status",
        "kernel_health": "cynic organism --json",
        "lifecycle_activation": "cynic actions",
        "lifecycle.mcp_reload": "cynic mcp-reload --dry-run",
        "lifecycle.gate_2": "cynic gate-2 --dry-run",
        "lifecycle.deploy": "cynic deploy --dry-run",
    }
    return checks.get(str(surface))


def dedupe_commands(commands: list[object]) -> list[str]:
    deduped: list[str] = []
    for item in commands:
        command = str(item).strip()
        if command and command not in deduped:
            deduped.append(command)
    return deduped


def organism_next_checks(commands: list[object]) -> list[str]:
    return [
        command
        for command in dedupe_commands(commands)
        if command not in {"cynic organism", "cynic organism --json"}
    ]


def command_equivalent(left: str, right: str) -> bool:
    if left == right:
        return True
    left_no_json = left.removesuffix(" --json")
    right_no_json = right.removesuffix(" --json")
    if left_no_json == right_no_json:
        return True
    if left.startswith(right + " "):
        return True
    if right.startswith(left + " "):
        return True
    return False


def order_next_checks_by_operator_sequence(next_checks: list[str], operator_sequence: list[dict[str, object]]) -> list[str]:
    ordered: list[str] = []
    remaining = list(next_checks)
    for item in operator_sequence:
        command = item.get("command") if isinstance(item, dict) else None
        if not isinstance(command, str):
            continue
        for candidate in list(remaining):
            if command_equivalent(candidate, command):
                ordered.append(candidate)
                remaining.remove(candidate)
    ordered.extend(remaining)
    return ordered


def command_mutation(command: str, explicit_mempool: object = None) -> dict[str, bool]:
    mutates_mempool = bool(explicit_mempool)
    if command in {
        "cynic activate",
        "cynic activate --json",
        "cynic bootstrap-request",
        "cynic bootstrap-request --json",
        "cynic mcp-reload",
        "cynic mcp-reload --json",
        "cynic gate-2",
        "cynic gate-2 --json",
        "cynic deploy",
        "cynic deploy --json",
    }:
        mutates_mempool = True
    if command in {
        "cynic bootstrap",
        "cynic apply",
        "cynic install-defaults",
    }:
        mutates_host = True
    else:
        mutates_host = command.startswith("sudo make ")
    return {
        "mutates_host": mutates_host,
        "mutates_mempool": mutates_mempool,
        "mutates_files": command.startswith("cynic storage bundle "),
    }


def operator_step_name(command: str, mutation: dict[str, bool], preferred: object = None) -> str:
    if isinstance(preferred, str) and preferred:
        return preferred
    if mutation["mutates_mempool"]:
        return "dispatch"
    if mutation["mutates_host"]:
        return "apply"
    if mutation["mutates_files"]:
        return "prepare"
    if command.endswith("--json"):
        return "prove"
    return "inspect"


def organism_operator_sequence(blockers: list[dict[str, object]], next_checks: list[str]) -> list[dict[str, object]]:
    sequence: list[dict[str, object]] = []
    seen: set[str] = set()

    def add(command: object, *, source: str, step: object = None, mutates_mempool: object = None) -> None:
        if not isinstance(command, str) or not command or command in {"cynic organism", "cynic organism --json"}:
            return
        if command in seen:
            return
        seen.add(command)
        mutation = command_mutation(command, mutates_mempool)
        sequence.append(
            {
                "step": operator_step_name(command, mutation, step),
                "command": command,
                "source": source,
                **mutation,
            }
        )

    for blocker in blockers:
        surface = str(blocker.get("surface", "organism"))
        proof_requirements = [
            proof
            for proof in blocker.get("proof_requirements", [])
            if isinstance(proof, dict) and proof.get("status") != "satisfied"
        ]
        for proof in proof_requirements:
            command = proof.get("command")
            if not isinstance(command, str) or not command_mutation(command)["mutates_host"]:
                continue
            add(
                command,
                source=f"{surface}.{proof.get('check', 'proof')}",
                mutates_mempool=proof.get("mutates_mempool"),
            )
        add(blocker.get("check"), source=f"{surface}.check")
        for proof in proof_requirements:
            if proof.get("mutates_mempool"):
                continue
            command = proof.get("command")
            if isinstance(command, str) and command_mutation(command)["mutates_host"]:
                continue
            add(
                command,
                source=f"{surface}.{proof.get('check', 'proof')}",
                mutates_mempool=proof.get("mutates_mempool"),
            )
        dispatch_plan = blocker.get("dispatch_plan")
        if isinstance(dispatch_plan, dict):
            for item in dispatch_plan.get("operator_sequence", []):
                if not isinstance(item, dict):
                    continue
                add(
                    item.get("command"),
                    source=f"{surface}.dispatch_plan",
                    step=item.get("step"),
                    mutates_mempool=item.get("mutates_mempool"),
                )
        for proof in proof_requirements:
            if not isinstance(proof, dict) or proof.get("status") == "satisfied":
                continue
            if not proof.get("mutates_mempool"):
                continue
            add(
                proof.get("command"),
                source=f"{surface}.{proof.get('check', 'proof')}",
                mutates_mempool=proof.get("mutates_mempool"),
            )
        action = blocker.get("action")
        if action:
            add(action, source=f"{surface}.dispatch", step="dispatch", mutates_mempool=True)

    for command in next_checks:
        add(command, source="organism.next_checks")
    return sequence
