"""Render non-mutating Hermes config patches for CYNIC fleet backends."""

from __future__ import annotations

from pathlib import Path
import json
import tomllib
from typing import Any

from organs.contracts.src.core.agent_framework_capabilities import utc_now_iso


HERMES_FLEET_CONFIG_PROJECTION_SCHEMA = "cynic.hermes_fleet_config_projection.v1"
HERMES_FLEET_CONFIG_DRIFT_SCHEMA = "cynic.hermes_fleet_config_drift.v1"


def render_hermes_fleet_config_projection(
    *,
    backends_toml: Path,
    registry_json: Path | None = None,
    expected_backend_id: str = "qwen36-27b-gpu",
    observed_at: str | None = None,
) -> dict[str, Any]:
    payload = tomllib.loads(backends_toml.read_text(encoding="utf-8"))
    backends = payload.get("backend")
    if not isinstance(backends, dict):
        raise RuntimeError(f"{backends_toml} does not declare [backend.*]")
    raw = backends.get(expected_backend_id)
    if not isinstance(raw, dict):
        raise RuntimeError(f"{backends_toml} does not declare backend.{expected_backend_id}")

    base_url = str(raw.get("base_url", "")).strip()
    resolved_base_url, endpoint_evidence = _resolve_base_url(
        base_url=base_url,
        backend=raw,
        registry_json=registry_json,
    )
    model = str(raw.get("model", "")).strip()
    key_env = str(raw.get("api_key_env", "CYNIC_SOVEREIGN_KEY")).strip() or "CYNIC_SOVEREIGN_KEY"
    context_size = int(raw.get("context_size", 0) or 0)
    supports_tool_use = bool(raw.get("supports_tool_use", False))
    sovereign = bool(raw.get("sovereign", False))
    if not resolved_base_url or not model:
        raise RuntimeError(f"backend.{expected_backend_id} must declare base_url and model")

    provider_name = expected_backend_id
    provider_slug = f"custom:{provider_name}"
    provider_entry: dict[str, Any] = {
        "name": provider_name,
        "base_url": resolved_base_url,
        "model": model,
        "key_env": key_env,
        "api_mode": "openai",
        "models": {
            model: {
                "context_length": context_size,
                "supports_tool_use": supports_tool_use,
            }
        },
    }
    config_patch = {
        "model": {
            "provider": provider_slug,
            "default": model,
            "base_url": resolved_base_url,
        },
        "custom_providers": [provider_entry],
    }
    return {
        "schema": HERMES_FLEET_CONFIG_PROJECTION_SCHEMA,
        "observed_at": observed_at or utc_now_iso(),
        "mutates": False,
        "backend_id": expected_backend_id,
        "provider_slug": provider_slug,
        "config_path": "~/.hermes/config.yaml",
        "secrets_path": "~/.hermes/.env",
        "required_secret_env": key_env,
        "endpoint_evidence": endpoint_evidence,
        "sovereign": sovereign,
        "supports_tool_use": supports_tool_use,
        "config_patch": config_patch,
        "yaml_snippet": _yaml_snippet(config_patch),
        "apply_note": "Merge config_patch into ~/.hermes/config.yaml and provide required_secret_env in ~/.hermes/.env or the service environment.",
    }


def render_hermes_fleet_config_projection_from_args(args: list[str]) -> dict[str, Any]:
    backends_toml = Path("backends.toml")
    registry_json: Path | None = Path("infra/registry.json")
    expected_backend = "qwen36-27b-gpu"
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
        if args[index] == "--registry" and index + 1 < len(args):
            registry_json = Path(args[index + 1]).expanduser()
            index += 2
            continue
        raise RuntimeError(
            "usage: hermes-fleet-config-projection [--backends-toml backends.toml] [--registry infra/registry.json] [--expected-backend qwen36-27b-gpu]"
        )
    return render_hermes_fleet_config_projection(
        backends_toml=backends_toml,
        registry_json=registry_json if registry_json and registry_json.exists() else None,
        expected_backend_id=expected_backend,
    )


def render_hermes_fleet_config_drift(
    *,
    backends_toml: Path,
    registry_json: Path | None = None,
    config_path: Path,
    expected_backend_id: str = "qwen36-27b-gpu",
    observed_at: str | None = None,
) -> dict[str, Any]:
    projection = render_hermes_fleet_config_projection(
        backends_toml=backends_toml,
        registry_json=registry_json,
        expected_backend_id=expected_backend_id,
        observed_at=observed_at,
    )
    expected = projection["config_patch"]
    actual, config_status = _read_config(config_path)
    violations = _config_violations(actual, expected) if actual is not None else [
        {"kind": config_status, "path": str(config_path)}
    ]
    status = "aligned" if not violations else "drift"
    return {
        "schema": HERMES_FLEET_CONFIG_DRIFT_SCHEMA,
        "observed_at": observed_at or utc_now_iso(),
        "mutates": False,
        "status": status,
        "config_path": str(config_path),
        "backend_id": expected_backend_id,
        "projection": projection,
        "violations": violations,
        "apply_required": bool(violations),
    }


def render_hermes_fleet_config_drift_from_args(args: list[str]) -> dict[str, Any]:
    backends_toml = Path("backends.toml")
    registry_json: Path | None = Path("infra/registry.json")
    config_path = Path("~/.hermes/config.yaml").expanduser()
    expected_backend = "qwen36-27b-gpu"
    index = 0
    while index < len(args):
        if args[index] == "--backends-toml" and index + 1 < len(args):
            backends_toml = Path(args[index + 1]).expanduser()
            index += 2
            continue
        if args[index] == "--registry" and index + 1 < len(args):
            registry_json = Path(args[index + 1]).expanduser()
            index += 2
            continue
        if args[index] == "--config" and index + 1 < len(args):
            config_path = Path(args[index + 1]).expanduser()
            index += 2
            continue
        if args[index] == "--expected-backend" and index + 1 < len(args):
            expected_backend = args[index + 1]
            index += 2
            continue
        raise RuntimeError(
            "usage: hermes-fleet-config-drift [--backends-toml backends.toml] [--registry infra/registry.json] [--config ~/.hermes/config.yaml] [--expected-backend qwen36-27b-gpu]"
        )
    return render_hermes_fleet_config_drift(
        backends_toml=backends_toml,
        registry_json=registry_json if registry_json and registry_json.exists() else None,
        config_path=config_path,
        expected_backend_id=expected_backend,
    )


def _resolve_base_url(
    *,
    base_url: str,
    backend: dict[str, Any],
    registry_json: Path | None,
) -> tuple[str, dict[str, Any]]:
    node = _backend_node(backend)
    evidence: dict[str, Any] = {
        "declared_base_url": base_url,
        "backend_node": node,
        "registry": str(registry_json) if registry_json else None,
        "resolved": False,
    }
    if "<" not in base_url:
        evidence["resolved_base_url"] = base_url
        evidence["resolved"] = True
        return base_url, evidence
    if not node or registry_json is None or not registry_json.exists():
        evidence["reason"] = "missing_registry_or_backend_node"
        return base_url, evidence

    endpoint = _node_endpoint(registry_json, node)
    if endpoint is None:
        evidence["reason"] = "node_not_found_in_registry"
        return base_url, evidence

    resolved = base_url.replace("<TAILSCALE_HOST_GPU>", endpoint["host"]).replace(
        "<TAILSCALE_HOST_PROXMOX>", endpoint["host"]
    )
    evidence.update(
        {
            "resolved": True,
            "resolved_host": endpoint["host"],
            "resolved_synapse_ip": endpoint.get("synapse_ip"),
            "resolved_base_url": resolved,
        }
    )
    return resolved, evidence


def _backend_node(backend: dict[str, Any]) -> str | None:
    remediation = backend.get("remediation")
    if isinstance(remediation, dict) and remediation.get("node"):
        return str(remediation["node"])
    if backend.get("fleet_node"):
        return str(backend["fleet_node"])
    return None


def _node_endpoint(registry_json: Path, node_name: str) -> dict[str, str | None] | None:
    registry = json.loads(registry_json.read_text(encoding="utf-8"))
    for node in registry.get("nodes", []):
        if not isinstance(node, dict):
            continue
        names = {str(node.get("name", "")), *(str(alias) for alias in node.get("aliases", []))}
        if node_name not in names:
            continue
        network = node.get("network", {}) if isinstance(node.get("network"), dict) else {}
        synapse = network.get("synapse")
        return {"host": f"{node.get('name')}.synapse.cynic", "synapse_ip": str(synapse)} if synapse else None
    mesh = registry.get("network", {}).get("nodes", {}) if isinstance(registry.get("network"), dict) else {}
    if isinstance(mesh, dict):
        node = mesh.get(node_name)
        if isinstance(node, dict):
            dns = node.get("dns")
            synapse = str(node["synapse"]) if node.get("synapse") else None
            if isinstance(dns, list) and dns:
                return {"host": str(dns[0]), "synapse_ip": synapse}
            if node.get("synapse"):
                return {"host": str(node["synapse"]), "synapse_ip": synapse}
    return None


def _yaml_snippet(config_patch: dict[str, Any]) -> str:
    model = config_patch["model"]
    entry = config_patch["custom_providers"][0]
    model_id = entry["model"]
    model_cfg = entry["models"][model_id]
    return "\n".join(
        [
            "model:",
            f"  provider: {model['provider']}",
            f"  default: {model['default']}",
            f"  base_url: {model['base_url']}",
            "custom_providers:",
            f"  - name: {entry['name']}",
            f"    base_url: {entry['base_url']}",
            f"    model: {model_id}",
            f"    key_env: {entry['key_env']}",
            f"    api_mode: {entry['api_mode']}",
            "    models:",
            f"      {model_id}:",
            f"        context_length: {model_cfg['context_length']}",
            f"        supports_tool_use: {str(model_cfg['supports_tool_use']).lower()}",
            "",
        ]
    )


def _read_config(path: Path) -> tuple[dict[str, Any] | None, str]:
    if not path.exists():
        return None, "missing_config"
    try:
        import yaml  # type: ignore
    except ImportError:
        return None, "yaml_parser_unavailable"
    try:
        loaded = yaml.safe_load(path.read_text(encoding="utf-8"))
    except Exception:
        return None, "config_parse_failed"
    return (loaded if isinstance(loaded, dict) else {}), "observed"


def _config_violations(actual: dict[str, Any], expected: dict[str, Any]) -> list[dict[str, Any]]:
    violations: list[dict[str, Any]] = []
    actual_model = actual.get("model") if isinstance(actual.get("model"), dict) else {}
    expected_model = expected["model"]
    for key in ("provider", "default", "base_url"):
        if actual_model.get(key) != expected_model.get(key):
            violations.append(
                {
                    "kind": f"model_{key}_mismatch",
                    "expected": expected_model.get(key),
                    "observed": actual_model.get(key),
                }
            )

    expected_provider = expected["custom_providers"][0]
    actual_provider = _find_provider(actual.get("custom_providers"), expected_provider["name"])
    if actual_provider is None:
        violations.append({"kind": "missing_custom_provider", "expected": expected_provider["name"]})
        return violations
    for key in ("base_url", "model", "key_env", "api_mode"):
        if actual_provider.get(key) != expected_provider.get(key):
            violations.append(
                {
                    "kind": f"custom_provider_{key}_mismatch",
                    "provider": expected_provider["name"],
                    "expected": expected_provider.get(key),
                    "observed": actual_provider.get(key),
                }
            )
    return violations


def _find_provider(providers: Any, name: str) -> dict[str, Any] | None:
    if not isinstance(providers, list):
        return None
    for entry in providers:
        if isinstance(entry, dict) and entry.get("name") == name:
            return entry
    return None
