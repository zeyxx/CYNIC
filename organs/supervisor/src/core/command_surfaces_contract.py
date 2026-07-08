#!/usr/bin/env python3
"""Legacy Python mirror for canonical command-surface validation.

The stable deterministic authority is migrating into `crates/cynic-mcp`.
Keep this module as a compatibility surface until supervisor wrappers consume
the Rust validator directly.
"""

from __future__ import annotations

import json
import subprocess
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

from organs.contracts.src.project_root import resolve_repo_root


PROJECT_ROOT = resolve_repo_root(Path(__file__).resolve())
CONTRACT_PATH = PROJECT_ROOT / "axioms" / "command-surfaces-v1.json"
EXPOSURE_PATH = PROJECT_ROOT / "infra" / "agent-exposure.json"
REQUIRED_ROLES = {
    "human_operator",
    "interactive_agent",
    "machine_fallback",
    "validation_dev",
    "runtime_daemon",
}
REQUIRED_COMMAND_FIELDS = {
    "name",
    "role",
    "domain",
    "canonical",
    "mutates",
    "mutates_mempool",
    "json_output_required",
}


@dataclass
class Finding:
    severity: str
    kind: str
    path: str
    detail: str


def load_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        payload = json.load(handle)
    return payload if isinstance(payload, dict) else {}


def validate_command_surfaces_contract(
    contract: dict[str, Any] | None = None,
    *,
    exposure: dict[str, Any] | None = None,
    make_targets: set[str] | None = None,
) -> list[Finding]:
    findings: list[Finding] = []
    if contract is None:
        if not CONTRACT_PATH.exists():
            return [
                Finding(
                    "critical",
                    "command_surfaces_contract_missing",
                    "axioms/command-surfaces-v1.json",
                    "command-surfaces-v1 must define canonical human/agent/fallback surfaces.",
                )
            ]
        contract = load_json(CONTRACT_PATH)
    if exposure is None and EXPOSURE_PATH.exists():
        exposure = load_json(EXPOSURE_PATH)
    exposure = exposure or {}
    make_targets = make_targets if make_targets is not None else _make_targets()

    if contract.get("schema") != "cynic.command_surfaces.v1":
        findings.append(
            Finding(
                "critical",
                "unexpected_command_surfaces_schema",
                "axioms/command-surfaces-v1.json",
                f"expected cynic.command_surfaces.v1, got {contract.get('schema')!r}",
            )
        )

    roles = contract.get("surface_roles")
    if not isinstance(roles, dict):
        findings.append(
            Finding(
                "critical",
                "command_surface_roles_missing",
                "axioms/command-surfaces-v1.json",
                "surface_roles must define human_operator, interactive_agent, machine_fallback, validation_dev, and runtime_daemon.",
            )
        )
        roles = {}
    missing_roles = sorted(REQUIRED_ROLES - set(roles.keys()))
    if missing_roles:
        findings.append(
            Finding(
                "critical",
                "command_surface_required_roles_missing",
                "axioms/command-surfaces-v1.json",
                f"missing roles: {', '.join(missing_roles)}",
            )
        )

    commands = contract.get("commands")
    if not isinstance(commands, list):
        findings.append(
            Finding(
                "critical",
                "command_surface_commands_missing",
                "axioms/command-surfaces-v1.json",
                "commands must be a list.",
            )
        )
        commands = []

    exposed_tools = _exposed_tools(exposure)
    for command in commands:
        if not isinstance(command, dict):
            findings.append(
                Finding(
                    "critical",
                    "command_surface_command_invalid",
                    "axioms/command-surfaces-v1.json",
                    "every command entry must be an object.",
                )
            )
            continue
        name = str(command.get("name", ""))
        missing = sorted(field for field in REQUIRED_COMMAND_FIELDS if field not in command)
        if missing:
            findings.append(
                Finding(
                    "critical",
                    "command_surface_command_metadata_missing",
                    name or "axioms/command-surfaces-v1.json",
                    f"command is missing fields: {', '.join(missing)}",
                )
            )
        role = command.get("role")
        if role not in roles:
            findings.append(
                Finding(
                    "critical",
                    "command_surface_unknown_role",
                    name,
                    f"role {role!r} is not declared in surface_roles.",
                )
            )
        if command.get("role") in {"interactive_agent", "machine_fallback"} and not command.get("json_output_required"):
            findings.append(
                Finding(
                    "critical",
                    "machine_command_without_json_output",
                    name,
                    "interactive-agent and machine-fallback surfaces must emit JSON/JSONL.",
                )
            )
        if command.get("mutates_mempool") and not command.get("read_back_required"):
            findings.append(
                Finding(
                    "critical",
                    "mempool_mutation_without_read_back",
                    name,
                    "commands that mutate the Kernel mempool must require read-back.",
                )
            )
        if command.get("role") == "interactive_agent" and name not in exposed_tools:
            findings.append(
                Finding(
                    "critical",
                    "interactive_agent_command_not_exposed",
                    name,
                    "interactive-agent commands must be declared in infra/agent-exposure.json.",
                )
            )
        if name.startswith("make "):
            target = name.split(maxsplit=1)[1]
            if target not in make_targets:
                findings.append(
                    Finding(
                        "critical",
                        "make_surface_target_missing",
                        name,
                        f"make target {target!r} is declared as a surface but not implemented.",
                    )
                )
            if command.get("canonical") and command.get("domain") == "lifecycle" and command.get("mutates"):
                findings.append(
                    Finding(
                        "critical",
                        "make_lifecycle_mutation_marked_canonical",
                        name,
                        "mutating lifecycle make targets must remain fallback/dev wrappers, not canonical control surfaces.",
                    )
                )
    return findings


def report() -> dict[str, Any]:
    findings = validate_command_surfaces_contract()
    return {
        "schema": "cynic.command_surfaces_validation.v1",
        "ok": not any(item.severity == "critical" for item in findings),
        "findings": [asdict(item) for item in findings],
    }


def _exposed_tools(exposure: dict[str, Any]) -> set[str]:
    names: set[str] = set()
    for domain in exposure.get("mcp_tool_domains", {}).values():
        if not isinstance(domain, dict):
            continue
        names.update(str(item) for item in domain.get("tools", []))
        names.update(str(item) for item in domain.get("fallback_tools", []))
    for server in exposure.get("mcp_servers", []):
        if not isinstance(server, dict):
            continue
        for tool in server.get("tools", []):
            if isinstance(tool, dict) and tool.get("name"):
                names.add(str(tool["name"]))
    return names


def _make_targets() -> set[str]:
    try:
        output = subprocess.check_output(
            ["make", "-pn"],
            cwd=str(PROJECT_ROOT),
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except (OSError, subprocess.CalledProcessError):
        return set()
    targets: set[str] = set()
    for line in output.splitlines():
        if ":" not in line or line.startswith(("\t", ".", "#")):
            continue
        target = line.split(":", 1)[0].strip()
        if target and " " not in target and not target.startswith("%"):
            targets.add(target)
    return targets


def main() -> int:
    payload = report()
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0 if payload["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
