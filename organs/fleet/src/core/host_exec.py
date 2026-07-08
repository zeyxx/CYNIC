from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class HostExecRequest:
    node: str
    role: str
    purpose: str
    argv: tuple[str, ...]
    agent_id: str
    timeout_seconds: int = 30
    service: str | None = None

    @classmethod
    def from_json(cls, payload: dict[str, Any]) -> "HostExecRequest":
        argv = payload.get("command")
        if not isinstance(argv, list) or not all(isinstance(item, str) for item in argv):
            argv = []
        return cls(
            node=str(payload.get("node", "")),
            role=str(payload.get("role", "")),
            purpose=str(payload.get("purpose", "")),
            argv=tuple(argv),
            agent_id=str(payload.get("agent_id", "")),
            timeout_seconds=int(payload.get("timeout_seconds", 30) or 30),
            service=payload.get("service") if isinstance(payload.get("service"), str) else None,
        )


@dataclass(frozen=True)
class HostExecValidation:
    ok: bool
    violations: tuple[str, ...]


class HostExecPolicy:
    def __init__(self, policy: dict[str, Any]) -> None:
        self.policy = policy

    @property
    def helper_path(self) -> str:
        return str(self.policy.get("helper_path", "/usr/local/libexec/cynic-host-exec"))

    @property
    def sudoers_subject(self) -> str:
        return str(self.policy.get("sudoers_subject", "cynic"))

    def sudoers_rule(self) -> str:
        return f"{self.sudoers_subject} ALL=(root) NOPASSWD: {self.helper_path}"

    def validate_request(
        self,
        request: HostExecRequest,
        declared_services: set[str] | None = None,
    ) -> HostExecValidation:
        violations: list[str] = []
        declared_services = declared_services or set()

        if not request.node.strip():
            violations.append("missing_node")
        if not request.agent_id.strip():
            violations.append("missing_agent_id")
        if not request.argv:
            violations.append("missing_command")
        if request.timeout_seconds > int(self.policy.get("max_timeout_seconds", 60)):
            violations.append("timeout_exceeds_policy")

        purpose_policy = self.policy.get("allowed_purposes", {}).get(request.purpose)
        if purpose_policy is None:
            violations.append("purpose_not_allowed")
            return HostExecValidation(ok=False, violations=tuple(sorted(set(violations))))

        allowed_roles = purpose_policy.get("allowed_roles", [])
        if allowed_roles and request.role not in {str(item) for item in allowed_roles}:
            violations.append("role_not_allowed_for_purpose")

        forbidden_tokens = set(self.policy.get("forbidden_tokens", []))
        if any(token in forbidden_tokens for token in request.argv):
            violations.append("forbidden_token")
        if any(any(shell in token for shell in ("|", "&&", ";", ">", "$(", "`")) for token in request.argv):
            violations.append("shell_syntax_forbidden")

        allowed_prefixes = [
            tuple(str(part) for part in prefix)
            for prefix in purpose_policy.get("argv_prefixes", [])
        ]
        if not any(request.argv[: len(prefix)] == prefix for prefix in allowed_prefixes):
            violations.append("argv_prefix_not_allowed")

        if purpose_policy.get("requires_declared_service"):
            service = request.service or self._service_from_argv(request.argv)
            if service is None or service not in declared_services:
                violations.append("service_not_declared")

        return HostExecValidation(ok=not violations, violations=tuple(sorted(set(violations))))

    def _service_from_argv(self, argv: tuple[str, ...]) -> str | None:
        if len(argv) < 3 or argv[0] != "systemctl":
            return None
        return argv[2].removesuffix(".service")
