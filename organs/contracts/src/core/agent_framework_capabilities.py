"""Framework-agnostic agent capability registry contracts."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime
from typing import Any


AGENT_FRAMEWORK_CAPABILITIES_SCHEMA = "cynic.agent_framework_capabilities.v1"

VALID_CONFIDENCE = frozenset({"observed", "declared", "inferred"})
VALID_SIDE_EFFECTS = frozenset(
    {
        "none",
        "kernel_observe",
        "kernel_task_dispatch",
        "repo_read",
        "repo_write_requires_claim",
        "network",
        "external_process",
    }
)


def utc_now_iso() -> str:
    return datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")


@dataclass(frozen=True)
class AgentFrameworkCapability:
    id: str
    input_schemas: tuple[str, ...]
    output_schemas: tuple[str, ...]
    side_effects: tuple[str, ...]
    confidence: str

    def __post_init__(self) -> None:
        if not self.id.strip():
            raise RuntimeError("agent framework capability id missing")
        if not self.input_schemas:
            raise RuntimeError(f"agent framework capability {self.id} input_schemas missing")
        if not self.output_schemas:
            raise RuntimeError(f"agent framework capability {self.id} output_schemas missing")
        if not self.side_effects:
            raise RuntimeError(f"agent framework capability {self.id} side_effects missing")
        if self.confidence not in VALID_CONFIDENCE:
            raise RuntimeError(f"unsupported agent framework capability confidence: {self.confidence}")
        unknown_effects = sorted(set(self.side_effects) - VALID_SIDE_EFFECTS)
        if unknown_effects:
            raise RuntimeError(
                f"unsupported agent framework capability side_effects: {','.join(unknown_effects)}"
            )

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "input_schemas": list(self.input_schemas),
            "output_schemas": list(self.output_schemas),
            "side_effects": list(self.side_effects),
            "confidence": self.confidence,
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AgentFrameworkCapability":
        return cls(
            id=str(data.get("id", "")).strip(),
            input_schemas=tuple(str(item) for item in data.get("input_schemas", ())),
            output_schemas=tuple(str(item) for item in data.get("output_schemas", ())),
            side_effects=tuple(str(item) for item in data.get("side_effects", ())),
            confidence=str(data.get("confidence", "")).strip(),
        )


@dataclass(frozen=True)
class AgentFrameworkCapabilities:
    framework: str
    version: str
    observed_at: str
    capabilities: tuple[AgentFrameworkCapability, ...]
    evidence: tuple[dict[str, Any], ...] = ()
    source_summary: dict[str, Any] | None = None
    coverage_gaps: tuple[dict[str, Any], ...] = ()

    def __post_init__(self) -> None:
        if not self.framework.strip():
            raise RuntimeError("agent framework name missing")
        if not self.version.strip():
            raise RuntimeError(f"agent framework {self.framework} version missing")
        if not self.observed_at.strip():
            raise RuntimeError(f"agent framework {self.framework} observed_at missing")

    def to_dict(self) -> dict[str, Any]:
        payload = {
            "schema": AGENT_FRAMEWORK_CAPABILITIES_SCHEMA,
            "framework": self.framework,
            "version": self.version,
            "observed_at": self.observed_at,
            "capabilities": [capability.to_dict() for capability in self.capabilities],
            "capability_count": len(self.capabilities),
            "evidence": list(self.evidence),
        }
        if self.source_summary is not None:
            payload["source_summary"] = self.source_summary
        if self.coverage_gaps:
            payload["coverage_gaps"] = list(self.coverage_gaps)
        return payload

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AgentFrameworkCapabilities":
        if data.get("schema") != AGENT_FRAMEWORK_CAPABILITIES_SCHEMA:
            raise RuntimeError("expected cynic.agent_framework_capabilities.v1")
        capabilities = data.get("capabilities", ())
        if not isinstance(capabilities, list):
            raise RuntimeError("agent framework capabilities must be a list")
        evidence = data.get("evidence", ())
        if not isinstance(evidence, list):
            raise RuntimeError("agent framework evidence must be a list")
        return cls(
            framework=str(data.get("framework", "")).strip(),
            version=str(data.get("version", "")).strip(),
            observed_at=str(data.get("observed_at", "")).strip(),
            capabilities=tuple(
                AgentFrameworkCapability.from_dict(item)
                for item in capabilities
                if isinstance(item, dict)
            ),
            evidence=tuple(item for item in evidence if isinstance(item, dict)),
            source_summary=data.get("source_summary") if isinstance(data.get("source_summary"), dict) else None,
            coverage_gaps=tuple(item for item in data.get("coverage_gaps", ()) if isinstance(item, dict)),
        )


def sorted_capabilities(
    capabilities: tuple[AgentFrameworkCapability, ...] | list[AgentFrameworkCapability],
) -> tuple[AgentFrameworkCapability, ...]:
    return tuple(sorted(capabilities, key=lambda item: item.id))
