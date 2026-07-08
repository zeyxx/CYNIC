"""Hermes fleet model state contract."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


HERMES_FLEET_MODEL_STATE_SCHEMA = "cynic.hermes_fleet_model_state.v1"


@dataclass(frozen=True)
class FleetBackend:
    backend_id: str
    provider: str
    model: str
    base_url: str
    fleet_node: str | None
    server: str | None
    sovereign: bool
    supports_tool_use: bool
    latency_class: str | None

    def __post_init__(self) -> None:
        if not self.backend_id.strip():
            raise RuntimeError("fleet backend id missing")
        if not self.provider.strip():
            raise RuntimeError(f"fleet backend {self.backend_id} provider missing")
        if not self.model.strip():
            raise RuntimeError(f"fleet backend {self.backend_id} model missing")
        if not self.base_url.strip():
            raise RuntimeError(f"fleet backend {self.backend_id} base_url missing")

    def to_dict(self) -> dict[str, Any]:
        return {
            "backend_id": self.backend_id,
            "provider": self.provider,
            "model": self.model,
            "base_url": self.base_url,
            "fleet_node": self.fleet_node,
            "server": self.server,
            "sovereign": self.sovereign,
            "supports_tool_use": self.supports_tool_use,
            "latency_class": self.latency_class,
        }


@dataclass(frozen=True)
class HermesObservedModel:
    provider: str | None
    model: str | None
    source: str

    def to_dict(self) -> dict[str, Any]:
        return {
            "provider": self.provider,
            "model": self.model,
            "source": self.source,
        }


@dataclass(frozen=True)
class HermesFleetModelState:
    observed_at: str
    expected_backend: FleetBackend
    observed_hermes: HermesObservedModel
    status: str
    violations: tuple[dict[str, Any], ...]
    evidence: tuple[dict[str, Any], ...] = ()

    def __post_init__(self) -> None:
        if self.status not in {"aligned", "mismatch", "degraded"}:
            raise RuntimeError(f"unsupported Hermes fleet model state status: {self.status}")
        if not self.observed_at.strip():
            raise RuntimeError("Hermes fleet model state observed_at missing")
        if self.status != "aligned" and not self.violations:
            raise RuntimeError("non-aligned Hermes fleet model state requires violations")

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema": HERMES_FLEET_MODEL_STATE_SCHEMA,
            "observed_at": self.observed_at,
            "status": self.status,
            "expected_backend": self.expected_backend.to_dict(),
            "observed_hermes": self.observed_hermes.to_dict(),
            "violations": list(self.violations),
            "evidence": list(self.evidence),
        }
