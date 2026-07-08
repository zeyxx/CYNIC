"""Polymorphic agent execution envelope contracts."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


AGENT_EXECUTION_ENVELOPE_SCHEMA = "cynic.agent_execution_envelope.v1"
HERMES_EXECUTION_ROUTE_SCHEMA = "cynic.hermes_execution_route.v1"

VALID_ROUTE_RISKS = frozenset({"low", "medium", "high"})
VALID_ROUTE_TRANSPORTS = frozenset(
    {
        "cli",
        "cron",
        "gateway",
        "acp",
        "mcp",
        "plugin",
        "dog",
        "deterministic",
        "human_cortex",
    }
)
VALID_SIDEEFFECT_POLICIES = frozenset(
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


def _required_text(value: Any, field: str) -> str:
    text = str(value or "").strip()
    if not text:
        raise RuntimeError(f"{field} missing")
    return text


def _text_tuple(value: Any, field: str, *, required: bool = True) -> tuple[str, ...]:
    if value is None:
        items: tuple[str, ...] = ()
    elif isinstance(value, str):
        items = (value.strip(),) if value.strip() else ()
    else:
        items = tuple(str(item).strip() for item in value if str(item).strip())
    if required and not items:
        raise RuntimeError(f"{field} missing")
    return items


def _dict_tuple(value: Any, field: str) -> tuple[dict[str, Any], ...]:
    if value is None:
        return ()
    if not isinstance(value, list | tuple):
        raise RuntimeError(f"{field} must be a list")
    return tuple(item for item in value if isinstance(item, dict))


@dataclass(frozen=True)
class AgentExecutionUseCase:
    id: str
    intent: str
    why: tuple[str, ...]
    paths: tuple[str, ...]
    expected_outputs: tuple[str, ...]
    constraints: tuple[str, ...]
    fallback_routes: tuple[str, ...]
    what_else: tuple[str, ...]

    def __post_init__(self) -> None:
        _required_text(self.id, "agent execution use case id")
        _required_text(self.intent, f"agent execution use case {self.id} intent")
        _text_tuple(self.why, f"agent execution use case {self.id} why")
        _text_tuple(self.paths, f"agent execution use case {self.id} paths")
        _text_tuple(self.expected_outputs, f"agent execution use case {self.id} expected_outputs")
        _text_tuple(self.constraints, f"agent execution use case {self.id} constraints")
        _text_tuple(self.fallback_routes, f"agent execution use case {self.id} fallback_routes")
        _text_tuple(self.what_else, f"agent execution use case {self.id} what_else")

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "intent": self.intent,
            "why": list(self.why),
            "paths": list(self.paths),
            "expected_outputs": list(self.expected_outputs),
            "constraints": list(self.constraints),
            "fallback_routes": list(self.fallback_routes),
            "what_else": list(self.what_else),
        }

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AgentExecutionUseCase":
        use_case_id = _required_text(data.get("id"), "agent execution use case id")
        return cls(
            id=use_case_id,
            intent=_required_text(data.get("intent"), f"agent execution use case {use_case_id} intent"),
            why=_text_tuple(data.get("why"), f"agent execution use case {use_case_id} why"),
            paths=_text_tuple(data.get("paths"), f"agent execution use case {use_case_id} paths"),
            expected_outputs=_text_tuple(
                data.get("expected_outputs"),
                f"agent execution use case {use_case_id} expected_outputs",
            ),
            constraints=_text_tuple(data.get("constraints"), f"agent execution use case {use_case_id} constraints"),
            fallback_routes=_text_tuple(
                data.get("fallback_routes"),
                f"agent execution use case {use_case_id} fallback_routes",
            ),
            what_else=_text_tuple(data.get("what_else"), f"agent execution use case {use_case_id} what_else"),
        )


@dataclass(frozen=True)
class AgentExecutionRouteCandidate:
    id: str
    provider: str
    transport: str
    why: tuple[str, ...]
    required_capabilities: tuple[str, ...]
    allowed_toolsets: tuple[str, ...]
    forbidden_toolsets: tuple[str, ...]
    side_effect_policy: tuple[str, ...]
    risk: str
    evidence_required: tuple[str, ...]
    fallback_route_ids: tuple[str, ...] = ()
    provider_schema: str | None = None

    def __post_init__(self) -> None:
        _required_text(self.id, "agent execution route id")
        _required_text(self.provider, f"agent execution route {self.id} provider")
        if self.transport not in VALID_ROUTE_TRANSPORTS:
            raise RuntimeError(f"unsupported agent execution route transport: {self.transport}")
        _text_tuple(self.why, f"agent execution route {self.id} why")
        _text_tuple(self.required_capabilities, f"agent execution route {self.id} required_capabilities")
        _text_tuple(self.allowed_toolsets, f"agent execution route {self.id} allowed_toolsets")
        _text_tuple(self.evidence_required, f"agent execution route {self.id} evidence_required")
        if self.risk not in VALID_ROUTE_RISKS:
            raise RuntimeError(f"unsupported agent execution route risk: {self.risk}")
        unknown_effects = sorted(set(self.side_effect_policy) - VALID_SIDEEFFECT_POLICIES)
        if unknown_effects:
            raise RuntimeError(f"unsupported agent execution route side effects: {','.join(unknown_effects)}")

    def to_dict(self) -> dict[str, Any]:
        payload = {
            "id": self.id,
            "provider": self.provider,
            "transport": self.transport,
            "why": list(self.why),
            "required_capabilities": list(self.required_capabilities),
            "allowed_toolsets": list(self.allowed_toolsets),
            "forbidden_toolsets": list(self.forbidden_toolsets),
            "side_effect_policy": list(self.side_effect_policy),
            "risk": self.risk,
            "evidence_required": list(self.evidence_required),
            "fallback_route_ids": list(self.fallback_route_ids),
        }
        if self.provider_schema:
            payload["provider_schema"] = self.provider_schema
        return payload

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AgentExecutionRouteCandidate":
        route_id = _required_text(data.get("id"), "agent execution route id")
        return cls(
            id=route_id,
            provider=_required_text(data.get("provider"), f"agent execution route {route_id} provider"),
            transport=_required_text(data.get("transport"), f"agent execution route {route_id} transport"),
            why=_text_tuple(data.get("why"), f"agent execution route {route_id} why"),
            required_capabilities=_text_tuple(
                data.get("required_capabilities"),
                f"agent execution route {route_id} required_capabilities",
            ),
            allowed_toolsets=_text_tuple(data.get("allowed_toolsets"), f"agent execution route {route_id} allowed_toolsets"),
            forbidden_toolsets=_text_tuple(
                data.get("forbidden_toolsets"),
                f"agent execution route {route_id} forbidden_toolsets",
                required=False,
            ),
            side_effect_policy=_text_tuple(
                data.get("side_effect_policy"),
                f"agent execution route {route_id} side_effect_policy",
            ),
            risk=_required_text(data.get("risk"), f"agent execution route {route_id} risk"),
            evidence_required=_text_tuple(data.get("evidence_required"), f"agent execution route {route_id} evidence_required"),
            fallback_route_ids=_text_tuple(
                data.get("fallback_route_ids"),
                f"agent execution route {route_id} fallback_route_ids",
                required=False,
            ),
            provider_schema=str(data.get("provider_schema")).strip() if data.get("provider_schema") else None,
        )


@dataclass(frozen=True)
class AgentExecutionEnvelope:
    task_id: str
    kind: str
    domain: str
    objective: str
    provenance: dict[str, Any]
    use_cases: tuple[AgentExecutionUseCase, ...]
    route_candidates: tuple[AgentExecutionRouteCandidate, ...]
    constraints: tuple[str, ...]
    expected_output_schemas: tuple[str, ...]
    timeout_seconds: int | None = None
    budget: dict[str, Any] | None = None
    evidence: tuple[dict[str, Any], ...] = ()

    def __post_init__(self) -> None:
        _required_text(self.task_id, "agent execution envelope task_id")
        _required_text(self.kind, f"agent execution envelope {self.task_id} kind")
        _required_text(self.domain, f"agent execution envelope {self.task_id} domain")
        _required_text(self.objective, f"agent execution envelope {self.task_id} objective")
        if not self.provenance:
            raise RuntimeError(f"agent execution envelope {self.task_id} provenance missing")
        if not self.use_cases:
            raise RuntimeError(f"agent execution envelope {self.task_id} use_cases missing")
        if not self.route_candidates:
            raise RuntimeError(f"agent execution envelope {self.task_id} route_candidates missing")
        _text_tuple(self.constraints, f"agent execution envelope {self.task_id} constraints")
        _text_tuple(self.expected_output_schemas, f"agent execution envelope {self.task_id} expected_output_schemas")
        if self.timeout_seconds is not None and self.timeout_seconds <= 0:
            raise RuntimeError(f"agent execution envelope {self.task_id} timeout_seconds must be positive")

    def to_dict(self) -> dict[str, Any]:
        payload = {
            "schema": AGENT_EXECUTION_ENVELOPE_SCHEMA,
            "task_id": self.task_id,
            "kind": self.kind,
            "domain": self.domain,
            "objective": self.objective,
            "provenance": self.provenance,
            "use_cases": [item.to_dict() for item in self.use_cases],
            "route_candidates": [item.to_dict() for item in self.route_candidates],
            "constraints": list(self.constraints),
            "expected_output_schemas": list(self.expected_output_schemas),
            "evidence": list(self.evidence),
        }
        if self.timeout_seconds is not None:
            payload["timeout_seconds"] = self.timeout_seconds
        if self.budget is not None:
            payload["budget"] = self.budget
        return payload

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AgentExecutionEnvelope":
        if data.get("schema") != AGENT_EXECUTION_ENVELOPE_SCHEMA:
            raise RuntimeError("expected cynic.agent_execution_envelope.v1")
        use_cases = data.get("use_cases", ())
        routes = data.get("route_candidates", ())
        if not isinstance(use_cases, list):
            raise RuntimeError("agent execution envelope use_cases must be a list")
        if not isinstance(routes, list):
            raise RuntimeError("agent execution envelope route_candidates must be a list")
        timeout = data.get("timeout_seconds")
        return cls(
            task_id=_required_text(data.get("task_id"), "agent execution envelope task_id"),
            kind=_required_text(data.get("kind"), "agent execution envelope kind"),
            domain=_required_text(data.get("domain"), "agent execution envelope domain"),
            objective=_required_text(data.get("objective"), "agent execution envelope objective"),
            provenance=data.get("provenance") if isinstance(data.get("provenance"), dict) else {},
            use_cases=tuple(AgentExecutionUseCase.from_dict(item) for item in use_cases if isinstance(item, dict)),
            route_candidates=tuple(AgentExecutionRouteCandidate.from_dict(item) for item in routes if isinstance(item, dict)),
            constraints=_text_tuple(data.get("constraints"), "agent execution envelope constraints"),
            expected_output_schemas=_text_tuple(
                data.get("expected_output_schemas"),
                "agent execution envelope expected_output_schemas",
            ),
            timeout_seconds=int(timeout) if timeout is not None else None,
            budget=data.get("budget") if isinstance(data.get("budget"), dict) else None,
            evidence=_dict_tuple(data.get("evidence"), "agent execution envelope evidence"),
        )


def example_agent_execution_envelope() -> AgentExecutionEnvelope:
    return AgentExecutionEnvelope(
        task_id="example-cynic-task-1",
        kind="cortex",
        domain="framework-interop",
        objective="Route one CYNIC task through the best available agent execution path.",
        provenance={
            "source": "cynic-agent-task",
            "schema": "cynic.agent_task.v1",
            "agent_task_id": "example-cynic-task-1",
        },
        use_cases=(
            AgentExecutionUseCase(
                id="repo_review",
                intent="Review a bounded repository change and return structured findings.",
                why=(
                    "The task may require agent judgment over source code.",
                    "The result must return to CYNIC as structured task evidence.",
                ),
                paths=("hermes_cli", "deterministic_reviewer", "human_cortex"),
                expected_outputs=("cynic.git_matter_review.v1", "cynic.agent_task_result.v1"),
                constraints=("repo_write_requires_claim", "stdout_must_be_json"),
                fallback_routes=("deterministic_reviewer", "human_cortex"),
                what_else=(
                    "Could this be deterministic?",
                    "Could this be split into smaller tasks?",
                    "Does this need human approval?",
                ),
            ),
        ),
        route_candidates=(
            AgentExecutionRouteCandidate(
                id="hermes_cli",
                provider="hermes-agent",
                provider_schema=HERMES_EXECUTION_ROUTE_SCHEMA,
                transport="cli",
                why=("Hermes exposes a non-interactive command surface that can consume a compiled prompt.",),
                required_capabilities=("agent_task_execution", "repo_review"),
                allowed_toolsets=("file", "terminal"),
                forbidden_toolsets=("browser", "messaging", "wallet"),
                side_effect_policy=("repo_read", "external_process"),
                risk="medium",
                evidence_required=("exit_code", "stdout_json", "artifact_path"),
                fallback_route_ids=("deterministic_reviewer", "human_cortex"),
            ),
            AgentExecutionRouteCandidate(
                id="deterministic_reviewer",
                provider="cynic-deterministic",
                transport="deterministic",
                why=("Use deterministic checks when the required output does not need agent judgment.",),
                required_capabilities=("static_validation",),
                allowed_toolsets=("file",),
                forbidden_toolsets=("network", "browser", "messaging"),
                side_effect_policy=("repo_read",),
                risk="low",
                evidence_required=("exit_code", "artifact_path"),
                fallback_route_ids=("human_cortex",),
            ),
        ),
        constraints=("coordination_claim_before_repo_write", "normalize_result_to_cynic_agent_task_result_v1"),
        expected_output_schemas=("cynic.agent_task_result.v1",),
        timeout_seconds=900,
        budget={"max_routes": 3, "max_retries": 1},
        evidence=({"kind": "contract_example", "status": "draft"},),
    )
