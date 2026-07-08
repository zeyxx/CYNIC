"""Resolve Hermes runtime model state against CYNIC fleet backends."""

from __future__ import annotations

from pathlib import Path
import re
import subprocess
import tomllib
from typing import Any

from organs.contracts.src.core.agent_framework_capabilities import utc_now_iso
from organs.contracts.src.core.hermes_fleet_model_state import (
    FleetBackend,
    HermesFleetModelState,
    HermesObservedModel,
)


def run_hermes_status(binary: str = "hermes") -> tuple[int, str, str]:
    try:
        completed = subprocess.run(
            (binary, "status", "--all"),
            check=False,
            capture_output=True,
            text=True,
            timeout=20,
        )
    except OSError as exc:
        return 127, "", str(exc)
    except subprocess.TimeoutExpired as exc:
        return 124, exc.stdout or "", exc.stderr or "command timed out"
    return completed.returncode, completed.stdout, completed.stderr


def parse_hermes_status(text: str) -> HermesObservedModel:
    provider = _extract_status_value(text, "Provider")
    model = _extract_status_value(text, "Model")
    if model in {"(not set)", "not set", ""}:
        model = None
    if provider in {"(not set)", "not set", ""}:
        provider = None
    return HermesObservedModel(provider=provider, model=model, source="hermes status --all")


def resolve_fleet_backend(backends_toml: Path, backend_id: str) -> FleetBackend:
    payload = tomllib.loads(backends_toml.read_text(encoding="utf-8"))
    backends = payload.get("backend")
    if not isinstance(backends, dict):
        raise RuntimeError(f"{backends_toml} does not declare [backend.*]")
    raw = backends.get(backend_id)
    if not isinstance(raw, dict):
        raise RuntimeError(f"{backends_toml} does not declare backend.{backend_id}")

    llama_server = payload.get("llama-server", {})
    server = "llama-server" if isinstance(llama_server, dict) and llama_server else None
    remediation = raw.get("remediation") if isinstance(raw.get("remediation"), dict) else {}
    fleet_node = str(raw.get("fleet_node") or remediation.get("node") or "").strip() or None
    return FleetBackend(
        backend_id=backend_id,
        provider=backend_id,
        model=str(raw.get("model", "")).strip(),
        base_url=str(raw.get("base_url", "")).strip(),
        fleet_node=fleet_node,
        server=server,
        sovereign=bool(raw.get("sovereign", False)),
        supports_tool_use=bool(raw.get("supports_tool_use", False)),
        latency_class=str(raw.get("latency_class") or "").strip() or None,
    )


def evaluate_hermes_fleet_model_state(
    *,
    backends_toml: Path,
    expected_backend_id: str = "qwen36-27b-gpu",
    status_text: str | None = None,
    binary: str = "hermes",
    observed_at: str | None = None,
) -> HermesFleetModelState:
    expected = resolve_fleet_backend(backends_toml, expected_backend_id)
    evidence: list[dict[str, Any]] = [
        {
            "kind": "backend_config",
            "path": str(backends_toml),
            "backend_id": expected_backend_id,
            "status": "observed",
        }
    ]
    if status_text is None:
        returncode, stdout, stderr = run_hermes_status(binary)
        status_text = "\n".join((stdout, stderr))
        evidence.append(
            {
                "kind": "command",
                "argv": [binary, "status", "--all"],
                "returncode": returncode,
                "status": "observed" if returncode == 0 else "degraded",
            }
        )
    else:
        evidence.append({"kind": "command", "argv": ["hermes", "status", "--all"], "status": "provided_fixture"})

    observed = parse_hermes_status(status_text)
    violations = _violations(expected, observed)
    status = "aligned" if not violations else "mismatch"
    if observed.provider is None and observed.model is None:
        status = "degraded"

    return HermesFleetModelState(
        observed_at=observed_at or utc_now_iso(),
        expected_backend=expected,
        observed_hermes=observed,
        status=status,
        violations=tuple(violations),
        evidence=tuple(evidence),
    )


def evaluate_hermes_fleet_model_state_from_args(args: list[str]) -> HermesFleetModelState:
    backends_toml = Path("backends.toml")
    expected_backend = "qwen36-27b-gpu"
    binary = "hermes"
    index = 0
    while index < len(args):
        if args[index] == "--backends-toml" and index + 1 < len(args):
            backends_toml = Path(args[index + 1]).expanduser()
            index += 2
            continue
        if args[index] == "--expected-backend" and index + 1 < len(args):
            expected_backend = args[index + 1]
            index += 2
            continue
        if args[index] == "--binary" and index + 1 < len(args):
            binary = args[index + 1]
            index += 2
            continue
        raise RuntimeError(
            "usage: hermes-fleet-model-state [--backends-toml backends.toml] [--expected-backend qwen36-27b-gpu] [--binary hermes]"
        )
    return evaluate_hermes_fleet_model_state(
        backends_toml=backends_toml,
        expected_backend_id=expected_backend,
        binary=binary,
    )


def _extract_status_value(text: str, label: str) -> str | None:
    pattern = re.compile(rf"^\s*{re.escape(label)}:\s*(.+?)\s*$", re.MULTILINE)
    match = pattern.search(text)
    if match is None:
        return None
    return match.group(1).strip()


def _violations(expected: FleetBackend, observed: HermesObservedModel) -> list[dict[str, Any]]:
    violations: list[dict[str, Any]] = []
    if not expected.sovereign:
        violations.append(
            {
                "kind": "expected_backend_not_sovereign",
                "expected_backend": expected.backend_id,
            }
        )
    if not expected.supports_tool_use:
        violations.append(
            {
                "kind": "expected_backend_lacks_tool_use",
                "expected_backend": expected.backend_id,
            }
        )
    allowed_providers = {expected.provider, f"custom:{expected.provider}"}
    if observed.provider not in allowed_providers:
        violations.append(
            {
                "kind": "provider_mismatch",
                "expected": sorted(allowed_providers),
                "observed": observed.provider,
            }
        )
    if observed.model != expected.model:
        violations.append(
            {
                "kind": "model_mismatch",
                "expected": expected.model,
                "observed": observed.model,
            }
        )
    return violations
