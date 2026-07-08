from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class LxcExecRequest:
    node: str
    vmid: int
    purpose: str
    argv: tuple[str, ...]
    agent_id: str
    timeout_seconds: int = 30
    service: str | None = None

    @classmethod
    def from_json(cls, payload: dict[str, Any]) -> "LxcExecRequest":
        argv = payload.get("argv")
        if not isinstance(argv, list) or not all(isinstance(item, str) for item in argv):
            argv = []
        return cls(
            node=str(payload.get("node", "")),
            vmid=int(payload.get("vmid", 0) or 0),
            purpose=str(payload.get("purpose", "")),
            argv=tuple(argv),
            agent_id=str(payload.get("agent_id", "")),
            timeout_seconds=int(payload.get("timeout_seconds", 30) or 30),
            service=payload.get("service") if isinstance(payload.get("service"), str) else None,
        )

    def to_pct_argv(self) -> list[str]:
        return ["pct", "exec", str(self.vmid), "--", *self.argv]


@dataclass(frozen=True)
class BridgeValidation:
    ok: bool
    violations: tuple[str, ...]


class LxcExecBridgePolicy:
    def __init__(self, policy: dict[str, Any]) -> None:
        self.policy = policy

    @property
    def helper_path(self) -> str:
        return str(self.policy.get("helper_path", "/usr/local/libexec/cynic-lxc-exec-bridge"))

    @property
    def sudoers_subject(self) -> str:
        return str(self.policy.get("sudoers_subject", "cynic"))

    def sudoers_rule(self) -> str:
        return f"{self.sudoers_subject} ALL=(root) NOPASSWD: {self.helper_path}"

    def validate_request(self, request: LxcExecRequest, declared_services: set[str] | None = None) -> BridgeValidation:
        violations: list[str] = []
        declared_services = declared_services or set()

        if not request.node.strip():
            violations.append("missing_node")
        if request.vmid <= 0:
            violations.append("invalid_vmid")
        if not request.agent_id.strip():
            violations.append("missing_agent_id")
        if not request.argv:
            violations.append("missing_argv")
        if request.timeout_seconds > int(self.policy.get("max_timeout_seconds", 60)):
            violations.append("timeout_exceeds_policy")

        purpose_policy = self.policy.get("allowed_purposes", {}).get(request.purpose)
        if purpose_policy is None:
            violations.append("purpose_not_allowed")
            return BridgeValidation(ok=False, violations=tuple(violations))

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

        return BridgeValidation(ok=not violations, violations=tuple(sorted(set(violations))))

    def _service_from_argv(self, argv: tuple[str, ...]) -> str | None:
        if len(argv) < 3 or argv[0] != "systemctl":
            return None
        return argv[2].removesuffix(".service")
