#!/usr/bin/env python3
# Tier 2 INFRASTRUCTURE: Hermes X backend policy.
"""Hermes X inference backend policy.

Selection is contract-first: infra/registry.json defines organ-x backend order,
backends.toml defines the concrete backend endpoints and enablement.
The legacy CLI path is disabled by default.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover - Python <3.11 fallback
    import tomli as tomllib  # type: ignore


DEFAULT_BACKEND_PRIORITY = ["qwen25-7b-core", "qwen36-27b-gpu"]
LEGACY_ENV = "HERMES_X_ALLOW_ANTIGRAVITY_LEGACY"
LEGACY_ENV_COMPAT = "HERMES_X_ALLOW_GEMINI_LEGACY"
CONTRACT_ENV = "CYNIC_HERMES_X_CONTRACT_JSON"


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def _candidate_backend_files() -> list[Path]:
    env_path = os.environ.get("CYNIC_BACKENDS_TOML", "").strip()
    candidates: list[Path] = []
    if env_path:
        candidates.append(Path(env_path).expanduser())
    candidates.append(Path.home() / ".config" / "cynic" / "backends.toml")
    candidates.append(_repo_root() / "backends.toml")
    return candidates


def _candidate_contract_files() -> list[Path]:
    env_path = os.environ.get(CONTRACT_ENV, "").strip()
    candidates: list[Path] = []
    if env_path:
        candidates.append(Path(env_path).expanduser())
    candidates.append(_repo_root() / "infra" / "registry.json")
    return candidates


def _is_enabled(cfg: dict[str, Any]) -> bool:
    enabled = cfg.get("enabled")
    return enabled is not False


def load_backend_table() -> dict[str, dict[str, Any]]:
    """Load the backend registry from the first backends.toml we can find."""
    for path in _candidate_backend_files():
        if not path.exists():
            continue
        try:
            with path.open("rb") as fh:
                data = tomllib.load(fh)
        except Exception:
            continue
        backends = data.get("backend", {})
        if isinstance(backends, dict) and backends:
            return backends
    return {}


def load_organ_contract() -> dict[str, Any]:
    """Load the Hermes-X contract from infra/registry.json."""
    for path in _candidate_contract_files():
        if not path.exists():
            continue
        try:
            data = json.loads(path.read_text())
        except Exception:
            continue
        for organ in data.get("organs", []):
            if organ.get("name") == "organ-x":
                contract = organ.get("contract", {})
                if isinstance(contract, dict):
                    return contract
    return {}


def allow_legacy_backend() -> bool:
    raw = (os.environ.get(LEGACY_ENV) or os.environ.get(LEGACY_ENV_COMPAT, "")).strip().lower()
    return raw in {"1", "true", "yes", "on"}


def resolve_backend_state() -> dict[str, Any]:
    """Resolve Hermes-X backend policy once from the contract + backend table."""
    backends = load_backend_table()
    contract = load_organ_contract()
    legacy_backend = str(contract.get("legacy_backend", "antigravity-cli")).strip() or "antigravity-cli"
    legacy_command = str(contract.get("legacy_command", legacy_backend)).strip() or legacy_backend
    legacy_model = str(contract.get("legacy_model", "auto")).strip() or "auto"
    legacy_enabled = allow_legacy_backend()

    priority: list[str] = []
    for name in contract.get("primary_backends", []):
        if isinstance(name, str) and name and name not in priority:
            if name in backends and _is_enabled(backends[name]):
                priority.append(name)
    for name in contract.get("fallback_backends", []):
        if isinstance(name, str) and name and name not in priority:
            if name in backends and _is_enabled(backends[name]):
                priority.append(name)

    if legacy_enabled and legacy_backend in backends and _is_enabled(backends[legacy_backend]):
        if legacy_backend not in priority:
            priority.append(legacy_backend)

    if not priority:
        priority = [name for name in DEFAULT_BACKEND_PRIORITY if name in backends and _is_enabled(backends[name])]
        if legacy_enabled and legacy_backend in backends and _is_enabled(backends[legacy_backend]):
            if legacy_backend not in priority:
                priority.append(legacy_backend)

    if not priority:
        priority = list(DEFAULT_BACKEND_PRIORITY)
        if legacy_enabled:
            priority.append(legacy_backend)

    return {
        "backends": backends,
        "contract": contract,
        "legacy_backend": legacy_backend,
        "legacy_command": legacy_command,
        "legacy_model": legacy_model,
        "legacy_enabled": legacy_enabled,
        "priority": priority,
        "policy": {
            "contract_source": "infra/registry.json",
            "backend_registry": "backends.toml",
            "selection_policy": contract.get("selection_policy", "registry-first"),
            "primary_backend": priority[0] if priority else None,
            "fallback_backends": priority[1:],
            "legacy_backend": legacy_backend,
            "legacy_command": legacy_command,
            "legacy_model": legacy_model,
            "legacy_enabled": legacy_enabled,
        },
    }


def get_legacy_backend_name() -> str:
    return str(resolve_backend_state()["legacy_backend"])


def get_legacy_command() -> str:
    return str(resolve_backend_state()["legacy_command"])


def get_legacy_model() -> str:
    return str(resolve_backend_state()["legacy_model"])


def get_backend_config(name: str) -> dict[str, Any]:
    return load_backend_table().get(name, {})


def get_inference_backend_priority() -> list[str]:
    return list(resolve_backend_state()["priority"])


def load_available_models() -> list[str]:
    return get_inference_backend_priority()


def get_model_priority() -> list[str]:
    return get_inference_backend_priority()
