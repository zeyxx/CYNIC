"""Executable Hermes command map discovery for organ-framework-map."""

from __future__ import annotations

from collections.abc import Callable
from pathlib import Path
import re
import subprocess
from typing import Any

from organs.contracts.src.adapters.hermes_fleet_model import (
    parse_hermes_status,
    resolve_fleet_backend,
    run_hermes_status,
)
from organs.contracts.src.core.agent_framework_capabilities import utc_now_iso


COMMAND_GROUPS = {
    "llm_provider_model": {
        "commands": {"model", "moa", "fallback", "status", "prompt-size", "auth", "login", "logout", "portal"},
        "cynic_integration_gap": "CYNIC still needs to enforce that Hermes provider, model, fallback, and MoA state resolve to declared fleet backends and sovereign policy.",
    },
    "agent_execution_and_coordination": {
        "commands": {"chat", "kanban", "cron", "project", "sessions", "journey", "learning", "curator", "claw"},
        "cynic_integration_gap": "Execution bloodstream events exist, but board graph, worker telemetry, and decomposition state are not yet fully mirrored into the Kernel.",
    },
    "tooling_and_surface_area": {
        "commands": {"tools", "mcp", "acp", "computer-use", "serve", "desktop", "gui", "dashboard", "proxy", "lsp"},
        "cynic_integration_gap": "CYNIC still lacks a canonical renderer from organ intent to Hermes-enabled toolsets and MCP/plugin exposure policy.",
    },
    "skills_plugins_memory_sessions": {
        "commands": {"skills", "plugins", "bundles", "memory", "memory-graph", "profile", "pairing"},
        "cynic_integration_gap": "Hermes local skills and memory are rich, but import/export and durable Kernel bridging remain incomplete.",
    },
    "messaging_gateway_and_servers": {
        "commands": {"gateway", "send", "slack", "whatsapp", "whatsapp-cloud", "webhook"},
        "cynic_integration_gap": "Outbound messaging still needs human-gated consent, approval queues, and per-channel evidence controls before cashflow automation.",
    },
    "maintenance_security_backup": {
        "commands": {"doctor", "security", "backup", "checkpoints", "config", "migrate", "setup", "postinstall", "dump", "debug", "version", "update", "uninstall", "hooks", "logs", "completion", "secrets", "import", "pets", "insights"},
        "cynic_integration_gap": "Maintenance surfaces exist, but CYNIC does not yet model them as a fully governed supply-chain and runtime hygiene contract per organ.",
    },
}


class HermesCommandMapDiscovery:
    def discover(
        self,
        *,
        source_root: Path | None,
        binary: str,
        source_inventory: dict[str, Any],
        repo_root: Path,
    ) -> dict[str, Any]:
        version_text = _run_stdout((binary, "--version"))
        help_text = _run_stdout((binary, "--help"))
        _, status_stdout, status_stderr = run_hermes_status(binary)
        status_text = "\n".join(part for part in (status_stdout, status_stderr) if part)

        parsed_help = _parse_root_help(help_text)
        merged_commands = _merge_commands(parsed_help, source_inventory.get("cli_commands", []))
        command_groups = _build_command_groups(merged_commands)

        return {
            "schema": "cynic.hermes_agent_command_map.v1",
            "generated_at": utc_now_iso(),
            "source": {
                "local_checkout": str(source_root) if source_root is not None else None,
                "installed_binary": binary,
                "observed_version": _first_line(version_text),
                "status_command": f"{binary} status --all",
                "status_redaction": "API keys, credential paths, and secret suffixes are intentionally redacted from this generated map.",
            },
            "current_local_runtime_observation": _status_summary(status_text),
            "cynic_expected_runtime_route": _expected_runtime_route(repo_root / "backends.toml"),
            "root_cli_commands": [item["name"] for item in merged_commands],
            "commands": merged_commands,
            "command_groups": command_groups,
        }


def _run_stdout(argv: tuple[str, ...]) -> str:
    try:
        completed = subprocess.run(
            argv,
            check=False,
            capture_output=True,
            text=True,
            timeout=20,
        )
    except OSError as exc:
        return str(exc)
    except subprocess.TimeoutExpired as exc:
        return (exc.stdout or "") + "\n" + (exc.stderr or "command timed out")
    return "\n".join(part for part in (completed.stdout, completed.stderr) if part)


def _first_line(text: str) -> str:
    for line in text.splitlines():
        stripped = line.strip()
        if stripped:
            return stripped
    return ""


def _parse_root_help(text: str) -> list[dict[str, Any]]:
    lines = text.splitlines()
    commands: list[dict[str, Any]] = []
    in_section = False
    current: dict[str, Any] | None = None
    for line in lines:
        if line.strip() == "positional arguments:":
            in_section = True
            current = None
            continue
        if not in_section:
            continue
        if not line.strip():
            continue
        if line.startswith("options:"):
            break
        match = re.match(r"^\s{4}([a-z0-9][a-z0-9_-]*)\s{2,}(.+?)\s*$", line)
        if match:
            current = {
                "name": match.group(1),
                "purpose": match.group(2).strip(),
            }
            commands.append(current)
            continue
        continuation = re.match(r"^\s{24,}(.+?)\s*$", line)
        if continuation and current is not None:
            current["purpose"] = f'{current["purpose"]} {continuation.group(1).strip()}'.strip()
    return commands


def _merge_commands(parsed_help: list[dict[str, Any]], source_commands: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_name: dict[str, dict[str, Any]] = {}
    for item in parsed_help:
        name = str(item.get("name", "")).strip()
        if not name:
            continue
        by_name[name] = {
            "name": name,
            "purpose": str(item.get("purpose", "")).strip() or None,
            "subcommands": [],
            "group": _group_for_command(name),
        }
    for item in source_commands:
        if not isinstance(item, dict):
            continue
        name = str(item.get("name", "")).strip()
        if not name:
            continue
        record = by_name.setdefault(
            name,
            {
                "name": name,
                "purpose": None,
                "subcommands": [],
                "group": _group_for_command(name),
            },
        )
        subcommands = sorted({str(value).strip() for value in item.get("subcommands", []) if str(value).strip()})
        if subcommands:
            record["subcommands"] = subcommands
    return sorted(by_name.values(), key=lambda item: item["name"])


def _group_for_command(name: str) -> str:
    for group_id, config in COMMAND_GROUPS.items():
        if name in config["commands"]:
            return group_id
    return "ungrouped"


def _build_command_groups(commands: list[dict[str, Any]]) -> dict[str, Any]:
    grouped: dict[str, dict[str, Any]] = {}
    for command in commands:
        group_id = str(command.get("group", "ungrouped"))
        entry = grouped.setdefault(
            group_id,
            {
                "commands": {},
                "cynic_integration_gap": COMMAND_GROUPS.get(group_id, {}).get(
                    "cynic_integration_gap",
                    "This Hermes surface is visible, but CYNIC has not yet assigned an explicit integration contract to it.",
                ),
            },
        )
        entry["commands"][command["name"]] = {
            "purpose": command.get("purpose"),
            "subcommands": command.get("subcommands", []),
        }
    return dict(sorted(grouped.items()))


def _status_summary(text: str) -> dict[str, Any]:
    observed = parse_hermes_status(text)
    configured_platforms = [
        line.split()[0].strip()
        for line in text.splitlines()
        if "✓ configured" in line and not line.strip().startswith("Status:")
    ]
    credential_present = "✓" in _section(text, "API Keys") or "✓" in _section(text, "Auth Providers")
    return {
        "provider": observed.provider,
        "model": observed.model,
        "gateway_status": _section_value(text, "Gateway Service", "Status"),
        "scheduled_jobs": _int_value(_section_value(text, "Scheduled Jobs", "Jobs")),
        "active_sessions": _int_value(_section_value(text, "Sessions", "Active")),
        "terminal_backend": _section_value(text, "Terminal Backend", "Backend"),
        "configured_messaging_platforms": configured_platforms,
        "credential_observation": (
            "At least one provider credential or API key is configured locally."
            if credential_present
            else "No provider credential observed in hermes status output."
        ),
    }


def _section(text: str, title: str) -> str:
    lines = text.splitlines()
    capture = False
    captured: list[str] = []
    header = f"◆ {title}"
    for line in lines:
        if line.strip() == header:
            capture = True
            continue
        if capture and line.strip().startswith("◆ "):
            break
        if capture:
            captured.append(line)
    return "\n".join(captured)


def _section_value(text: str, title: str, label: str) -> str | None:
    pattern = re.compile(rf"^\s*{re.escape(label)}:\s*(.+?)\s*$", re.MULTILINE)
    match = pattern.search(_section(text, title))
    if match is None:
        return None
    return match.group(1).strip()


def _int_value(value: str | None) -> int | None:
    if value is None:
        return None
    match = re.search(r"\d+", value)
    if match is None:
        return None
    return int(match.group(0))


def _expected_runtime_route(backends_toml: Path) -> dict[str, Any]:
    primary = _backend_route(backends_toml, "qwen36-27b-gpu")
    secondary = _backend_route(backends_toml, "qwen3-4b-core")
    return {
        "authority": "backends.toml plus fleet evidence",
        "principle": "Hermes should be propelled by a CYNIC fleet backend rather than arbitrary local provider picker state.",
        "primary_candidate": primary,
        "secondary_candidate": secondary,
    }


def _backend_route(backends_toml: Path, backend_id: str) -> dict[str, Any] | None:
    try:
        backend = resolve_fleet_backend(backends_toml, backend_id)
    except Exception:
        return None
    return {
        "backend": backend.backend_id,
        "fleet_node": backend.fleet_node,
        "server": backend.server,
        "api_shape": "OpenAI-compatible /v1" if backend.base_url else None,
        "model": backend.model,
        "sovereign": backend.sovereign,
        "supports_tool_use": backend.supports_tool_use,
        "latency_class": backend.latency_class,
    }
