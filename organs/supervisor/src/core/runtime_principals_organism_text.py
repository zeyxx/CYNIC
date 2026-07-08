"""Text renderers for organism runtime principal reports."""

from __future__ import annotations

from organs.supervisor.src.core.organism_text_sections import (
    append_pattern_evolution,
    append_topology_summary,
)


def status_word(report: dict[str, object], ok: set[str] | None = None) -> str:
    allowed = ok or {"ok", "ready", "active", "fresh"}
    status_value = str(report.get("status", "unknown"))
    return status_value if status_value in allowed else f"{status_value} (blocked)"


def organism_blocker_check(surface: object) -> str | None:
    checks = {
        "cortex_runtime": "cynic status",
        "mcp_freshness": "cynic mcp-reload --dry-run",
        "storage_primary": "cynic storage status",
        "kernel_health": "cynic organism --json",
        "lifecycle_activation": "cynic actions",
        "lifecycle.bootstrap_request": "cynic bootstrap-request --dry-run",
        "lifecycle.mcp_reload": "cynic mcp-reload --dry-run",
        "lifecycle.gate_2": "cynic gate-2 --dry-run",
        "lifecycle.deploy": "cynic deploy --dry-run",
    }
    return checks.get(str(surface))


def organism_actions_text(report: dict[str, object]) -> str:
    lines = [
        "CYNIC organism actions",
        f"Status: {report.get('status', 'unknown')}",
        f"Pending actions: {report.get('action_count', 0)}",
    ]
    actions = report.get("actions", [])
    if actions:
        lines.append("Next:")
        for item in actions:
            if not isinstance(item, dict):
                continue
            target = item.get("target") or "organism"
            priority = item.get("priority")
            priority_part = f" p={priority}" if priority is not None else ""
            lines.append(f"  - {item.get('action')}: {target}{priority_part} -> {item.get('consumer')}")
    blockers = report.get("blockers", [])
    if blockers:
        lines.append("Blockers:")
        for item in blockers[:8]:
            if not isinstance(item, dict):
                continue
            code = item.get("code") or item.get("kind") or item.get("category")
            details = []
            for key in (
                "requirement",
                "required_status",
                "actual_status",
                "required",
                "actual",
                "resolver_organ",
                "resolver_consumer",
            ):
                value = item.get(key)
                if value is not None:
                    details.append(f"{key}={value}")
            detail_part = f" ({', '.join(details)})" if details else ""
            lines.append(f"  - {code}{detail_part}")
            native_check = item.get("native_check_command")
            if native_check:
                lines.append(f"    check: {native_check}")
            command = item.get("command")
            if command and command != native_check and not str(command).startswith("python3 "):
                lines.append(f"    check: {command}")
        if len(blockers) > 8:
            lines.append(f"  - ... {len(blockers) - 8} more")
    dispatch = report.get("dispatch_command")
    if dispatch:
        lines.append(f"Dispatch: {dispatch}")
    dispatch_plan = report.get("dispatch_plan")
    if isinstance(dispatch_plan, dict):
        lines.append("Dispatch plan:")
        lines.append(f"  requests: {dispatch_plan.get('request_count', 0)}")
        consumers = dispatch_plan.get("consumers")
        if isinstance(consumers, list) and consumers:
            lines.append(f"  consumers: {', '.join(str(item) for item in consumers)}")
        lines.append("  sequence:")
        for item in dispatch_plan.get("operator_sequence", []):
            if isinstance(item, dict):
                lines.append(f"    - {item.get('step')}: {item.get('command')}")
        read_back = dispatch_plan.get("read_back")
        if read_back:
            lines.append(f"  read-back: {read_back}")
    return "\n".join(lines) + "\n"


def activation_dispatch_text(report: dict[str, object]) -> str:
    summary = report.get("dispatch_summary")
    if not isinstance(summary, dict):
        summary = {}
    lines = [
        "CYNIC lifecycle activation dispatch",
        f"Status: {report.get('status', 'unknown')}",
        f"Plan status: {report.get('plan_status', 'unknown')}",
        f"Requested: {summary.get('requested', report.get('request_count', 'unknown'))}",
        f"Dispatched: {summary.get('dispatched', report.get('dispatched_count', 0))}",
        f"Skipped existing: {summary.get('skipped_existing', report.get('skipped_existing_count', 0))}",
        f"Read-back: {'required'}",
    ]
    task_ids = summary.get("task_ids", [])
    if task_ids:
        lines.append("Task ids:")
        for task_id in task_ids:
            lines.append(f"  - {task_id}")
    skipped = report.get("skipped_existing", [])
    if skipped:
        lines.append("Existing pending:")
        for item in skipped[:8]:
            if isinstance(item, dict):
                lines.append(f"  - {item.get('task_id')}: {item.get('action')} ({item.get('reason')})")
        if len(skipped) > 8:
            lines.append(f"  - ... {len(skipped) - 8} more")
    blockers = report.get("blockers", [])
    if blockers:
        lines.append("Carried blockers:")
        for item in blockers[:8]:
            if isinstance(item, dict):
                lines.append(f"  - {item.get('code') or item.get('kind') or item.get('category')}")
        if len(blockers) > 8:
            lines.append(f"  - ... {len(blockers) - 8} more")
    inspect_command = report.get("inspect_command")
    if inspect_command:
        lines.append(f"Inspect before dispatch: {inspect_command}")
    proof_commands = report.get("proof_commands", [])
    if proof_commands:
        lines.append("Proof commands:")
        for command in proof_commands:
            lines.append(f"  - {command}")
    done_when = report.get("done_when", [])
    if done_when:
        lines.append("Done when:")
        for item in done_when[:6]:
            lines.append(f"  - {item}")
    lines.append("Machine dispatch output: cynic activate --json")
    return "\n".join(lines) + "\n"


def lifecycle_profile_request_text(report: dict[str, object]) -> str:
    profile = report.get("profile") or "unknown"
    summary = report.get("dispatch_summary")
    if not isinstance(summary, dict):
        summary = {}
    task_ids = report.get("task_ids") or summary.get("task_ids") or []
    lines = [
        "CYNIC lifecycle profile request",
        f"Profile: {profile}",
        f"Status: {report.get('status', 'unknown')}",
        f"Command: {report.get('command', f'cynic {profile}')}",
        f"Mutates mempool: {report.get('mutates_mempool', True)}",
    ]
    if summary:
        lines.extend(
            [
                f"Requested: {summary.get('requested', 'unknown')}",
                f"Dispatched: {summary.get('dispatched', 'unknown')}",
                f"Skipped existing: {summary.get('skipped_existing', 'unknown')}",
            ]
        )
    if task_ids:
        lines.append("Task ids:")
        for task_id in task_ids:
            lines.append(f"  - {task_id}")
    elif report.get("task_id"):
        lines.append(f"Task id: {report.get('task_id')}")
    blockers = report.get("blockers", [])
    if blockers:
        lines.append("Carried blockers:")
        for item in blockers[:8]:
            if isinstance(item, dict):
                lines.append(f"  - {item.get('code') or item.get('kind') or item.get('category')}")
        if len(blockers) > 8:
            lines.append(f"  - ... {len(blockers) - 8} more")
    operator_sequence = report.get("operator_sequence", [])
    if operator_sequence:
        lines.append("Operator sequence:")
        for item in operator_sequence:
            if isinstance(item, dict):
                lines.append(f"  - {item.get('step')}: {item.get('command')}")
    proof_commands = report.get("proof_commands", [])
    if proof_commands:
        lines.append("Proof commands:")
        for command in proof_commands:
            lines.append(f"  - {command}")
    done_when = report.get("done_when", [])
    if done_when:
        lines.append("Done when:")
        for item in done_when[:6]:
            lines.append(f"  - {item}")
    lines.append(f"Machine dispatch output: cynic {profile} --json")
    return "\n".join(lines) + "\n"


def organism_text(report: dict[str, object]) -> str:
    runtime = report["runtime"]
    contract = report["contract"]
    cortex_line = str(
        runtime.get(
            "runtime_evidence_status",
            "ready" if runtime["runtime_ready"] else "not_ready",
        )
    )
    storage = report["storage_primary"]
    mcp = report["mcp_freshness"]
    kernel = report["kernel"]
    local_validation = report.get("local_validation", {"status": "unknown"})
    lifecycle = report["lifecycle"]
    activation = report["lifecycle_activation"]
    stack = report.get("stack", {})
    lines = [
        "CYNIC organism signs of life",
        f"Cortex runtime: {cortex_line}",
        f"L0-L7 stack: {status_word(stack, {'ok'})}",
        f"Local validation: {status_word(local_validation, {'ready'})}",
        f"Checkout: {runtime['canonical_project_root']}",
        f"Kernel health: {status_word(kernel, {'ok', 'unknown'})}",
        f"Storage primary: {status_word(storage, {'active'})}",
        f"MCP freshness: {status_word(mcp, {'fresh'})}",
        "Native lifecycle:",
        f"  bootstrap request: {status_word(lifecycle['bootstrap_request'], {'ok'})} -> cynic bootstrap-request",
        f"  mcp reload: {status_word(lifecycle['mcp_reload'], {'ok'})} -> cynic mcp-reload",
        f"  gate 2: {status_word(lifecycle['gate_2'], {'ok'})} -> cynic gate-2",
        f"  deploy: {status_word(lifecycle['deploy'], {'ok'})} -> cynic deploy",
        f"Activation pressure: {status_word(activation, {'ok', 'ready'})}",
        "Contract reports:",
    ]
    for item in contract["must_report"]:
        lines.append(f"  - {item}")
    _append_stack(lines, stack)
    if report["blockers"]:
        lines.append("Blockers:")
        for blocker in report["blockers"]:
            surface = blocker.get("surface")
            status_value = blocker.get("status")
            missing = blocker.get("missing")
            lines.append(f"  - {surface}: {status_value}")
            if missing:
                lines.append(f"    missing: {', '.join(str(item) for item in missing)}")
            proof_requirements = blocker.get("proof_requirements")
            if isinstance(proof_requirements, list) and proof_requirements:
                missing_proofs = [
                    item
                    for item in proof_requirements
                    if isinstance(item, dict) and item.get("status") != "satisfied"
                ]
                if missing_proofs:
                    lines.append("    proof:")
                    for item in missing_proofs[:6]:
                        mutates = " [mutates mempool]" if item.get("mutates_mempool") else ""
                        lines.append(
                            f"      - {item.get('check')}: {item.get('command')}{mutates} -> {item.get('expected')}"
                        )
            dependency_proofs = blocker.get("dependency_proofs")
            if isinstance(dependency_proofs, dict):
                _append_dependency_proof_gaps(lines, dependency_proofs)
            check = blocker.get("check") or organism_blocker_check(surface)
            if check:
                lines.append(f"    check: {check}")
            next_checks = blocker.get("next_checks")
            if isinstance(next_checks, list) and next_checks:
                lines.append("    next:")
                for command in next_checks:
                    lines.append(f"      - {command}")
            action = blocker.get("action")
            if action and action != check:
                lines.append(f"    dispatch: {action}")
    _append_next_checks(lines, report)
    _append_operator_sequence(lines, report)
    _append_activation_requests(lines, activation)
    return "\n".join(lines) + "\n"


def _append_stack(lines: list[str], stack: object) -> None:
    if not isinstance(stack, dict):
        return
    append_topology_summary(lines, stack.get("topology_summary"), title="Living topology:")
    layers = stack.get("layers", [])
    entities = stack.get("entities", [])
    if isinstance(layers, list) and isinstance(entities, list):
        counts: dict[str, int] = {}
        unhealthy: dict[str, int] = {}
        for entity in entities:
            if not isinstance(entity, dict):
                continue
            layer = str(entity.get("layer"))
            counts[layer] = counts.get(layer, 0) + 1
            if entity.get("health") in {"blocked", "degraded"}:
                unhealthy[layer] = unhealthy.get(layer, 0) + 1
        lines.append("Stack L0-L7:")
        for layer in layers:
            if not isinstance(layer, dict):
                continue
            layer_id = str(layer.get("id"))
            count = counts.get(layer_id, 0)
            bad = unhealthy.get(layer_id, 0)
            suffix = f", {bad} unhealthy" if bad else ""
            lines.append(f"  - {layer_id} {layer.get('name')}: {count} entities{suffix}")
    patterns = stack.get("patterns", [])
    append_pattern_evolution(lines, stack.get("pattern_evolution"))
    if isinstance(patterns, list) and patterns:
        lines.append("Fragmentation patterns:")
        for pattern in patterns[:8]:
            if not isinstance(pattern, dict):
                continue
            pattern_name = pattern.get("pattern_id") or pattern.get("pattern")
            lines.append(
                f"  - {pattern_name}: {pattern.get('severity')} -> {pattern.get('resolution')}"
            )
            if pattern.get("pattern") == "utility_duplicate_pattern":
                entities = pattern.get("entities")
                if isinstance(entities, dict):
                    for name, paths in list(entities.items())[:4]:
                        if isinstance(paths, list):
                            sample = ", ".join(str(path) for path in paths[:2])
                            suffix = f" (+{len(paths) - 2})" if len(paths) > 2 else ""
                            lines.append(f"    {name}: {sample}{suffix}")
    actions = stack.get("actions", [])
    if isinstance(actions, list) and actions:
        lines.append("Stack convergence actions:")
        for action in actions[:8]:
            if not isinstance(action, dict):
                continue
            flags = []
            if action.get("mutates_files"):
                flags.append("writes files")
            if action.get("mutates_mempool"):
                flags.append("mutates mempool")
            suffix = f" [{', '.join(flags)}]" if flags else ""
            consumer = action.get("consumer")
            consumer_part = f" -> {consumer}" if consumer else ""
            lines.append(
                f"  - {action.get('layer')} {action.get('entity')}: {action.get('action')}{consumer_part}{suffix}"
            )
            targets = action.get("coord_targets")
            if isinstance(targets, list) and targets:
                lines.append(f"    targets: {', '.join(str(item) for item in targets[:5])}")
            proofs = action.get("proof_commands")
            if isinstance(proofs, list) and proofs:
                lines.append(f"    proof: {', '.join(str(item) for item in proofs[:3])}")
    requests = stack.get("dispatchable_requests", [])
    if isinstance(requests, list) and requests:
        lines.append("Dispatchable convergence requests:")
        for request in requests[:5]:
            if not isinstance(request, dict):
                continue
            lines.append(
                f"  - {request.get('domain')} -> {request.get('consumer')} p={request.get('priority')}"
            )
            targets = request.get("coord_targets")
            if isinstance(targets, list) and targets:
                lines.append(f"    targets: {', '.join(str(item) for item in targets[:5])}")


def _append_dependency_proof_gaps(lines: list[str], dependency_proofs: dict[object, object]) -> None:
    for name, proofs in dependency_proofs.items():
        if not isinstance(proofs, list):
            continue
        missing_dependency_proofs = [
            item
            for item in proofs
            if isinstance(item, dict) and item.get("status") != "satisfied"
        ]
        if not missing_dependency_proofs:
            continue
        lines.append(f"    {name} proof gaps: {len(missing_dependency_proofs)}")
        for item in missing_dependency_proofs[:3]:
            lines.append(f"      - {item.get('check')}: {item.get('command')} -> {item.get('expected')}")


def _append_next_checks(lines: list[str], report: dict[str, object]) -> None:
    next_checks = report.get("next_checks")
    if isinstance(next_checks, list) and next_checks:
        lines.append("Next checks:")
        for command in next_checks:
            lines.append(f"  - {command}")


def _append_operator_sequence(lines: list[str], report: dict[str, object]) -> None:
    operator_sequence = report.get("operator_sequence")
    if isinstance(operator_sequence, list) and operator_sequence:
        lines.append("Operator sequence:")
        for item in operator_sequence[:14]:
            if not isinstance(item, dict):
                continue
            flags = []
            if item.get("mutates_host"):
                flags.append("mutates host")
            if item.get("mutates_mempool"):
                flags.append("mutates mempool")
            if item.get("mutates_files"):
                flags.append("writes files")
            suffix = f" [{', '.join(flags)}]" if flags else ""
            lines.append(f"  - {item.get('step')}: {item.get('command')}{suffix}")


def _append_activation_requests(lines: list[str], activation: dict[str, object]) -> None:
    activation_requests = activation.get("activation_request_summary", [])
    if not activation_requests:
        return
    lines.append("Next organ actions:")
    for request in activation_requests[:8]:
        if not isinstance(request, dict):
            continue
        target = request.get("node") or request.get("code") or ",".join(request.get("coord_targets", []))
        lines.append(f"  - {request.get('action')}: {target} -> {request.get('consumer')}")
    if len(activation_requests) > 8:
        lines.append(f"  - ... {len(activation_requests) - 8} more")
    dispatch = activation.get("activation_dispatch_command")
    if dispatch:
        lines.append(f"Dispatch: {dispatch}")
    lines.append("Details: cynic actions")
